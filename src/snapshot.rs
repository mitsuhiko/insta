use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::content::{self, Content};

use once_cell::sync::Lazy;

static RUN_ID: Lazy<String> = Lazy::new(|| {
    if let Ok(run_id) = env::var("NEXTEST_RUN_ID") {
        run_id
    } else {
        let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        format!("{}-{}", d.as_secs(), d.subsec_nanos())
    }
});

#[derive(Debug)]
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
        let contents = fs::read_to_string(p)?;

        let mut rv: Vec<Self> = contents
            .lines()
            .map(|line| {
                let value = Content::from_yaml(line)?;
                Self::from_content(value)
            })
            .collect::<Result<_, Box<dyn Error>>>()?;

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
        let mut s = self.as_content().as_json()?;
        s.push('\n');
        f.write_all(s.as_bytes())?;
        Ok(())
    }

    fn from_content(content: Content) -> Result<PendingInlineSnapshot, Box<dyn Error>> {
        if let Content::Map(map) = content {
            let mut map = content::utils::into_unordered_struct_fields(map)?;

            let run_id = content::utils::pop_str(&mut map, "run_id")?;
            let line = content::utils::pop_u32(&mut map, "line")?;
            let new = match map.remove("new") {
                None | Some(Content::None) => None,
                Some(non_null) => Some(Snapshot::from_content(non_null)?),
            };
            let old = match map.remove("old") {
                None | Some(Content::None) => None,
                Some(non_null) => Some(Snapshot::from_content(non_null)?),
            };

            Ok(PendingInlineSnapshot {
                run_id,
                line,
                new,
                old,
            })
        } else {
            Err(content::Error::UnexpectedDataType.into())
        }
    }

    fn as_content(&self) -> Content {
        let fields = vec![
            ("run_id", Content::from(self.run_id.as_str())),
            ("line", Content::from(self.line)),
            (
                "new",
                match &self.new {
                    Some(snap) => snap.as_content(),
                    None => Content::None,
                },
            ),
            (
                "old",
                match &self.old {
                    Some(snap) => snap.as_content(),
                    None => Content::None,
                },
            ),
        ];

        Content::Struct("PendingInlineSnapshot", fields)
    }
}

/// Snapshot metadata information.
#[derive(Debug, Default, Clone)]
pub struct MetaData {
    /// The source file (relative to workspace root).
    pub(crate) source: Option<String>,
    /// The source line if available.
    pub(crate) assertion_line: Option<u32>,
    /// Optional human readable (non formatted) snapshot description.
    pub(crate) description: Option<String>,
    /// Optionally the expression that created the snapshot.
    pub(crate) expression: Option<String>,
    /// An optional arbitrary structured info object.
    pub(crate) info: Option<Content>,
    /// Reference to the input file.
    pub(crate) input_file: Option<String>,
}

impl MetaData {
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

    /// Returns the description that created the snapshot.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref().filter(|x| !x.is_empty())
    }

    /// Returns the embedded info.
    #[doc(hidden)]
    pub fn private_info(&self) -> Option<&Content> {
        self.info.as_ref()
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

    fn from_content(content: Content) -> Result<MetaData, Box<dyn Error>> {
        if let Content::Map(map) = content {
            let mut map = content::utils::into_unordered_struct_fields(map)?;

            let source = content::utils::pop_nullable_str(&mut map, "source")?;
            let assertion_line = content::utils::pop_nullable_u32(&mut map, "assertion_line")?;
            let description = content::utils::pop_nullable_str(&mut map, "description")?;
            let expression = content::utils::pop_nullable_str(&mut map, "expression")?;
            let info = map.remove("info");
            let input_file = content::utils::pop_nullable_str(&mut map, "input_file")?;

            Ok(MetaData {
                source,
                assertion_line,
                description,
                expression,
                info,
                input_file,
            })
        } else {
            Err(content::Error::UnexpectedDataType.into())
        }
    }

    fn as_content(&self) -> Content {
        let mut fields = Vec::new();
        if let Some(source) = self.source.as_deref() {
            fields.push(("source", Content::from(source)));
        }
        if let Some(expression) = self.expression.as_deref() {
            fields.push(("expression", Content::from(expression)));
        }
        if let Some(line) = self.assertion_line {
            fields.push(("assertion_line", Content::from(line)));
        }
        if let Some(description) = self.description.as_deref() {
            fields.push(("description", Content::from(description)));
        }
        if let Some(info) = &self.info {
            fields.push(("info", info.to_owned()));
        }
        if let Some(input_file) = self.input_file.as_deref() {
            fields.push(("input_file", Content::from(input_file)));
        }

        Content::Struct("MetaData", fields)
    }
}

/// A helper to work with stored snapshots.
#[derive(Debug, Clone)]
pub struct Snapshot {
    module_name: String,
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
            let content = Content::from_yaml(&buf)?;
            MetaData::from_content(content)?
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

    fn from_content(content: Content) -> Result<Snapshot, Box<dyn Error>> {
        if let Content::Map(map) = content {
            let mut map = content::utils::into_unordered_struct_fields(map)?;

            let module_name = content::utils::pop_str(&mut map, "module_name")?;
            let snapshot_name = content::utils::pop_nullable_str(&mut map, "snapshot_name")?;
            let metadata = MetaData::from_content(
                map.remove("metadata").ok_or(content::Error::MissingField)?,
            )?;
            let snapshot = SnapshotContents(content::utils::pop_str(&mut map, "snapshot")?);

            Ok(Snapshot {
                module_name,
                snapshot_name,
                metadata,
                snapshot,
            })
        } else {
            Err(content::Error::UnexpectedDataType.into())
        }
    }

    fn as_content(&self) -> Content {
        let mut fields = vec![("module_name", Content::from(self.module_name.as_str()))];
        if let Some(name) = self.snapshot_name.as_deref() {
            fields.push(("snapshot_name", Content::from(name)));
        }
        fields.push(("metadata", self.metadata.as_content()));
        fields.push(("snapshot", Content::from(self.snapshot.0.as_str())));

        Content::Struct("Content", fields)
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
        let blob = md.as_content().as_yaml();
        f.write_all(blob.as_bytes())?;
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
#[derive(Debug, Clone)]
pub struct SnapshotContents(String);

impl SnapshotContents {
    pub fn from_inline(value: &str) -> SnapshotContents {
        SnapshotContents(get_inline_snapshot_value(value))
    }

    pub fn to_inline(&self, indentation: usize) -> String {
        let contents = &self.0;
        let mut out = String::new();
        let is_escape = contents.contains(&['\n', '\\', '"'][..]);

        out.push_str(if is_escape { "r###\"" } else { "\"" });
        // if we have more than one line we want to change into the block
        // representation mode
        if contents.contains('\n') {
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

impl<'a> From<Cow<'a, str>> for SnapshotContents {
    fn from(value: Cow<'a, str>) -> Self {
        match value {
            Cow::Borrowed(s) => SnapshotContents::from(s),
            Cow::Owned(s) => SnapshotContents::from(s),
        }
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

    let t = &"
    ab
"[1..];
    assert_eq!(
        SnapshotContents(t.to_string()).to_inline(0),
        "r###\"
    ab
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
