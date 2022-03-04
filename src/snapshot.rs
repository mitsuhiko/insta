use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::utils::path_to_storage;

static RUN_ID: Lazy<String> = Lazy::new(|| {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}-{}", d.as_secs(), d.subsec_nanos())
});

#[derive(Debug, Serialize, Deserialize)]
pub struct PendingInlineSnapshot {
    pub run_id: String,
    pub line: u32,
    pub new: Option<Snapshot>,
    pub old: Option<Snapshot>,
}

impl PendingInlineSnapshot {
    pub fn new(new: Option<Snapshot>, old: Option<Snapshot>, line: u32) -> PendingInlineSnapshot {
        PendingInlineSnapshot {
            new,
            old,
            line,
            run_id: RUN_ID.clone(),
        }
    }

    pub fn load_batch<P: AsRef<Path>>(p: P) -> Result<Vec<PendingInlineSnapshot>, Box<dyn Error>> {
        let f = BufReader::new(fs::File::open(p)?);
        let iter = serde_json::Deserializer::from_reader(f).into_iter::<PendingInlineSnapshot>();
        let mut rv = iter.collect::<Result<Vec<PendingInlineSnapshot>, _>>()?;

        // remove all but the last run
        if let Some(last_run_id) = rv.last().map(|x| x.run_id.clone()) {
            rv.retain(|x| x.run_id == last_run_id);
        }

        Ok(rv)
    }

    pub fn save_batch<P: AsRef<Path>>(
        p: P,
        batch: &[PendingInlineSnapshot],
    ) -> Result<(), Box<dyn Error>> {
        fs::remove_file(&p).ok();
        for snap in batch {
            snap.save(&p)?;
        }
        Ok(())
    }

    pub fn save<P: AsRef<Path>>(&self, p: P) -> Result<(), Box<dyn Error>> {
        let mut f = fs::OpenOptions::new().create(true).append(true).open(p)?;
        let mut s = serde_json::to_string(self)?;
        s.push('\n');
        f.write_all(s.as_bytes())?;
        Ok(())
    }
}

/// Snapshot metadata information.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct MetaData {
    /// The source file (relative to workspace root).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<String>,
    /// The source line if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) assertion_line: Option<u32>,
    /// Optionally the expression that created the snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) expression: Option<String>,
    /// Reference to the input file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) input_file: Option<String>,
}

impl MetaData {
    /// Creates a new metadata from the given inputs.
    pub(crate) fn new(
        source: &str,
        expr: &str,
        assertion_line: Option<u32>,
        input_file: Option<PathBuf>,
    ) -> MetaData {
        MetaData {
            source: Some(path_to_storage(source)),
            expression: Some(expr.to_string()),
            assertion_line,
            input_file: input_file.map(path_to_storage),
        }
    }

    /// Returns the absolute source path.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Returns the assertion line.
    pub fn assertion_line(&self) -> Option<u32> {
        self.assertion_line
    }

    /// Returns the expression that created the snapshot.
    pub fn expression(&self) -> Option<&str> {
        self.expression.as_deref()
    }

    /// Returns the relative source path.
    pub fn get_relative_source(&self, base: &Path) -> Option<PathBuf> {
        self.source.as_ref().map(|source| {
            base.join(source)
                .canonicalize()
                .ok()
                .and_then(|s| s.strip_prefix(base).ok().map(|x| x.to_path_buf()))
                .unwrap_or_else(|| base.to_path_buf())
        })
    }

    /// Returns the input file reference.
    pub fn input_file(&self) -> Option<&str> {
        self.input_file.as_deref()
    }
}

/// A helper to work with stored snapshots.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Snapshot {
    module_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot_name: Option<String>,
    metadata: MetaData,
    snapshot: SnapshotContents,
}

impl Snapshot {
    /// Loads a snapshot from a file.
    pub fn from_file<P: AsRef<Path>>(p: P) -> Result<Snapshot, Box<dyn Error>> {
        let mut f = BufReader::new(fs::File::open(p.as_ref())?);
        let mut buf = String::new();

        f.read_line(&mut buf)?;

        // yaml format
        let metadata = if buf.trim_end() == "---" {
            loop {
                let read = f.read_line(&mut buf)?;
                if read == 0 {
                    break;
                }
                if buf[buf.len() - read..].trim_end() == "---" {
                    buf.truncate(buf.len() - read);
                    break;
                }
            }
            serde_yaml::from_str(&buf)?
        // legacy format
        } else {
            let mut rv = MetaData::default();
            loop {
                buf.clear();
                let read = f.read_line(&mut buf)?;
                if read == 0 || buf.trim_end().is_empty() {
                    buf.truncate(buf.len() - read);
                    break;
                }
                let mut iter = buf.splitn(2, ':');
                if let Some(key) = iter.next() {
                    if let Some(value) = iter.next() {
                        let value = value.trim();
                        match key.to_lowercase().as_str() {
                            "expression" => rv.expression = Some(value.to_string()),
                            "source" => rv.source = Some(value.into()),
                            _ => {}
                        }
                    }
                }
            }
            rv
        };

        buf.clear();
        for (idx, line) in f.lines().enumerate() {
            let line = line?;
            if idx > 0 {
                buf.push('\n');
            }
            buf.push_str(&line);
        }

        let module_name = p
            .as_ref()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap_or("")
            .split("__")
            .next()
            .unwrap_or("<unknown>")
            .to_string();

        let snapshot_name = p
            .as_ref()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap_or("")
            .split('.')
            .next()
            .unwrap_or("")
            .splitn(2, "__")
            .nth(1)
            .map(|x| x.to_string());

        Ok(Snapshot::from_components(
            module_name,
            snapshot_name,
            metadata,
            buf.into(),
        ))
    }

    /// Creates an empty snapshot.
    pub(crate) fn from_components(
        module_name: String,
        snapshot_name: Option<String>,
        metadata: MetaData,
        snapshot: SnapshotContents,
    ) -> Snapshot {
        Snapshot {
            module_name,
            snapshot_name,
            metadata,
            snapshot,
        }
    }

    /// Returns the module name.
    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    /// Returns the snapshot name.
    pub fn snapshot_name(&self) -> Option<&str> {
        self.snapshot_name.as_deref()
    }

    /// The metadata in the snapshot.
    pub fn metadata(&self) -> &MetaData {
        &self.metadata
    }

    /// The snapshot contents
    pub fn contents(&self) -> &SnapshotContents {
        &self.snapshot
    }

    /// The snapshot contents as a &str
    pub fn contents_str(&self) -> &str {
        &self.snapshot.0
    }

    fn save_with_metadata<P: AsRef<Path>>(
        &self,
        path: P,
        md: &MetaData,
    ) -> Result<(), Box<dyn Error>> {
        let path = path.as_ref();
        if let Some(folder) = path.parent() {
            fs::create_dir_all(&folder)?;
        }
        let mut f = fs::File::create(&path)?;
        serde_yaml::to_writer(&mut f, md)?;
        f.write_all(b"---\n")?;
        f.write_all(self.contents_str().as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
    }

    /// Saves the snapshot.
    #[doc(hidden)]
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        // we do not want to retain the assertion line on the metadata when storing
        // as a regular snapshot.
        if self.metadata.assertion_line.is_some() {
            let mut metadata = self.metadata.clone();
            metadata.assertion_line = None;
            self.save_with_metadata(path, &metadata)
        } else {
            self.save_with_metadata(path, &self.metadata)
        }
    }

    /// Same as `save` but also holds information only relevant for `.new` files.
    pub(crate) fn save_new<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        self.save_with_metadata(path, &self.metadata)
    }
}

/// The contents of a Snapshot
// Could be Cow, but I think limited savings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotContents(String);

impl SnapshotContents {
    pub fn from_inline(value: &str) -> SnapshotContents {
        SnapshotContents(get_inline_snapshot_value(value))
    }

    pub fn to_inline(&self, indentation: usize) -> String {
        let contents = &self.0;
        let mut out = String::new();
        let is_escape = contents.lines().count() > 1 || contents.contains(&['\\', '"'][..]);

        out.push_str(if is_escape { "r###\"" } else { "\"" });
        // if we have more than one line we want to change into the block
        // representation mode
        if contents.lines().count() > 1 {
            out.extend(
                contents
                    .lines()
                    // newline needs to be at the start, since we don't want the end
                    // finishing with a newline - the closing suffix should be on the same line
                    .map(|l| {
                        format!(
                            "\n{:width$}{l}",
                            "",
                            width = if l.is_empty() { 0 } else { indentation },
                            l = l
                        )
                    })
                    // `lines` removes the final line ending - add back
                    .chain(Some(format!("\n{:width$}", "", width = indentation)).into_iter()),
            );
        } else {
            out.push_str(contents);
        }

        out.push_str(if is_escape { "\"###" } else { "\"" });

        out
    }
}

impl From<&str> for SnapshotContents {
    fn from(value: &str) -> SnapshotContents {
        // make sure we have unix newlines consistently
        SnapshotContents(value.replace("\r\n", "\n"))
    }
}

impl From<String> for SnapshotContents {
    fn from(value: String) -> SnapshotContents {
        // make sure we have unix newlines consistently
        SnapshotContents(value.replace("\r\n", "\n"))
    }
}

impl From<SnapshotContents> for String {
    fn from(value: SnapshotContents) -> String {
        value.0
    }
}

impl PartialEq for SnapshotContents {
    fn eq(&self, other: &Self) -> bool {
        self.0.trim_end() == other.0.trim_end()
    }
}

fn count_leading_spaces(value: &str) -> usize {
    value.chars().take_while(|x| x.is_whitespace()).count()
}

fn min_indentation(snapshot: &str) -> usize {
    let lines = snapshot.trim_end().lines();

    if lines.clone().count() <= 1 {
        // not a multi-line string
        return 0;
    }

    lines
        .filter(|l| !l.is_empty())
        .map(count_leading_spaces)
        .min()
        .unwrap_or(0)
}

// Removes excess indentation, removes excess whitespace at start & end
// and changes newlines to \n.
fn normalize_inline_snapshot(snapshot: &str) -> String {
    let indentation = min_indentation(snapshot);
    snapshot
        .trim_end()
        .lines()
        .skip_while(|l| l.is_empty())
        .map(|l| l.get(indentation..).unwrap_or(""))
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Helper function that returns the real inline snapshot value from a given
/// frozen value string.  If the string starts with the '⋮' character
/// (optionally prefixed by whitespace) the alternative serialization format
/// is picked which has slightly improved indentation semantics.
///
/// This also changes all newlines to \n
fn get_inline_snapshot_value(frozen_value: &str) -> String {
    // TODO: could move this into the SnapshotContents `from_inline` method
    // (the only call site)

    if frozen_value.trim_start().starts_with('⋮') {
        // legacy format - retain so old snapshots still work
        let mut buf = String::new();
        let mut line_iter = frozen_value.lines();
        let mut indentation = 0;

        for line in &mut line_iter {
            let line_trimmed = line.trim_start();
            if line_trimmed.is_empty() {
                continue;
            }
            indentation = line.len() - line_trimmed.len();
            // 3 because '⋮' is three utf-8 bytes long
            buf.push_str(&line_trimmed[3..]);
            buf.push('\n');
            break;
        }

        for line in &mut line_iter {
            if let Some(prefix) = line.get(..indentation) {
                if !prefix.trim().is_empty() {
                    return "".to_string();
                }
            }
            if let Some(remainder) = line.get(indentation..) {
                if remainder.starts_with('⋮') {
                    // 3 because '⋮' is three utf-8 bytes long
                    buf.push_str(&remainder[3..]);
                    buf.push('\n');
                } else if remainder.trim().is_empty() {
                    continue;
                } else {
                    return "".to_string();
                }
            }
        }

        buf.trim_end().to_string()
    } else {
        normalize_inline_snapshot(frozen_value)
    }
}

#[test]
fn test_snapshot_contents() {
    use similar_asserts::assert_eq;
    let snapshot_contents = SnapshotContents("testing".to_string());
    assert_eq!(snapshot_contents.to_inline(0), r#""testing""#);

    let t = &"
a
b"[1..];
    assert_eq!(
        SnapshotContents(t.to_string()).to_inline(0),
        "r###\"
a
b
\"###"
    );

    let t = &"
a
b"[1..];
    assert_eq!(
        SnapshotContents(t.to_string()).to_inline(4),
        "r###\"
    a
    b
    \"###"
    );

    let t = &"
    a
    b"[1..];
    assert_eq!(
        SnapshotContents(t.to_string()).to_inline(0),
        "r###\"
    a
    b
\"###"
    );

    let t = &"
    a

    b"[1..];
    assert_eq!(
        SnapshotContents(t.to_string()).to_inline(0),
        "r###\"
    a

    b
\"###"
    );

    let t = "ab";
    assert_eq!(SnapshotContents(t.to_string()).to_inline(0), r##""ab""##);
}

#[test]
fn test_normalize_inline_snapshot() {
    use similar_asserts::assert_eq;
    // here we do exact matching (rather than `assert_snapshot`)
    // to ensure we're not incorporating the modifications this library makes
    let t = r#"
   1
   2
    "#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
1
2"###[1..]
    );

    let t = r#"
            1
    2"#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
        1
2"###[1..]
    );

    let t = r#"
            1
            2
    "#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
1
2"###[1..]
    );

    let t = r#"
   1
   2
"#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
1
2"###[1..]
    );

    let t = r#"
        a
    "#;
    assert_eq!(normalize_inline_snapshot(t), "a");

    let t = "";
    assert_eq!(normalize_inline_snapshot(t), "");

    let t = r#"
    a
    b
c
    "#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
    a
    b
c"###[1..]
    );

    let t = r#"
a
    "#;
    assert_eq!(normalize_inline_snapshot(t), "a");

    let t = "
    a";
    assert_eq!(normalize_inline_snapshot(t), "a");

    let t = r#"a
  a"#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
a
  a"###[1..]
    );
}

#[test]
fn test_min_indentation() {
    use similar_asserts::assert_eq;
    let t = r#"
   1
   2
    "#;
    assert_eq!(min_indentation(t), 3);

    let t = r#"
            1
    2"#;
    assert_eq!(min_indentation(t), 4);

    let t = r#"
            1
            2
    "#;
    assert_eq!(min_indentation(t), 12);

    let t = r#"
   1
   2
"#;
    assert_eq!(min_indentation(t), 3);

    let t = r#"
        a
    "#;
    assert_eq!(min_indentation(t), 8);

    let t = "";
    assert_eq!(min_indentation(t), 0);

    let t = r#"
    a
    b
c
    "#;
    assert_eq!(min_indentation(t), 0);

    let t = r#"
a
    "#;
    assert_eq!(min_indentation(t), 0);

    let t = "
    a";
    assert_eq!(min_indentation(t), 4);

    let t = r#"a
  a"#;
    assert_eq!(min_indentation(t), 0);
}

#[test]
fn test_inline_snapshot_value_newline() {
    // https://github.com/mitsuhiko/insta/issues/39
    assert_eq!(get_inline_snapshot_value("\n"), "");
}
