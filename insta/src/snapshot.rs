use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::content::{self, json, yaml, Content};

lazy_static::lazy_static! {
    static ref RUN_ID: String = {
        if let Ok(run_id) = env::var("NEXTEST_RUN_ID") {
            run_id
        } else {
            let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            format!("{}-{}", d.as_secs(), d.subsec_nanos())
        }
    };
}

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

    #[cfg(feature = "_cargo_insta_internal")]
    pub fn load_batch(p: &Path) -> Result<Vec<PendingInlineSnapshot>, Box<dyn Error>> {
        let contents =
            fs::read_to_string(p).map_err(|e| content::Error::FileIo(e, p.to_path_buf()))?;

        let mut rv: Vec<Self> = contents
            .lines()
            .map(|line| {
                let value = yaml::parse_str(line, p)?;
                Self::from_content(value)
            })
            .collect::<Result<_, Box<dyn Error>>>()?;

        // remove all but the last run
        if let Some(last_run_id) = rv.last().map(|x| x.run_id.clone()) {
            rv.retain(|x| x.run_id == last_run_id);
        }

        Ok(rv)
    }

    #[cfg(feature = "_cargo_insta_internal")]
    pub fn save_batch(p: &Path, batch: &[PendingInlineSnapshot]) -> Result<(), Box<dyn Error>> {
        fs::remove_file(p).ok();
        for snap in batch {
            snap.save(p)?;
        }
        Ok(())
    }

    pub fn save(&self, p: &Path) -> Result<(), Box<dyn Error>> {
        let mut f = fs::OpenOptions::new().create(true).append(true).open(p)?;
        let mut s = json::to_string(&self.as_content());
        s.push('\n');
        f.write_all(s.as_bytes())?;
        Ok(())
    }

    #[cfg(feature = "_cargo_insta_internal")]
    fn from_content(content: Content) -> Result<PendingInlineSnapshot, Box<dyn Error>> {
        if let Content::Map(map) = content {
            let mut run_id = None;
            let mut line = None;
            let mut old = None;
            let mut new = None;

            for (key, value) in map.into_iter() {
                match key.as_str() {
                    Some("run_id") => run_id = value.as_str().map(|x| x.to_string()),
                    Some("line") => line = value.as_u64().map(|x| x as u32),
                    Some("old") if !value.is_nil() => old = Some(Snapshot::from_content(value)?),
                    Some("new") if !value.is_nil() => new = Some(Snapshot::from_content(value)?),
                    _ => {}
                }
            }

            Ok(PendingInlineSnapshot {
                run_id: run_id.ok_or(content::Error::MissingField)?,
                line: line.ok_or(content::Error::MissingField)?,
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
#[derive(Debug, Default, Clone, PartialEq)]
pub struct MetaData {
    /// The source file (relative to workspace root).
    pub(crate) source: Option<String>,
    /// The source line, if available. This is used by pending snapshots, but trimmed
    /// before writing to the final `.snap` files in [`MetaData::trim_for_persistence`].
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
            let mut source = None;
            let mut assertion_line = None;
            let mut description = None;
            let mut expression = None;
            let mut info = None;
            let mut input_file = None;

            for (key, value) in map.into_iter() {
                match key.as_str() {
                    Some("source") => source = value.as_str().map(|x| x.to_string()),
                    Some("assertion_line") => assertion_line = value.as_u64().map(|x| x as u32),
                    Some("description") => description = value.as_str().map(Into::into),
                    Some("expression") => expression = value.as_str().map(Into::into),
                    Some("info") if !value.is_nil() => info = Some(value),
                    Some("input_file") => input_file = value.as_str().map(Into::into),
                    _ => {}
                }
            }

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
        if let Some(line) = self.assertion_line {
            fields.push(("assertion_line", Content::from(line)));
        }
        if let Some(description) = self.description.as_deref() {
            fields.push(("description", Content::from(description)));
        }
        if let Some(expression) = self.expression.as_deref() {
            fields.push(("expression", Content::from(expression)));
        }
        if let Some(info) = &self.info {
            fields.push(("info", info.to_owned()));
        }
        if let Some(input_file) = self.input_file.as_deref() {
            fields.push(("input_file", Content::from(input_file)));
        }

        Content::Struct("MetaData", fields)
    }

    /// Trims the metadata of fields that we don't save to `.snap` files; we
    /// only use for display while reviewing
    fn trim_for_persistence(&self) -> Cow<'_, MetaData> {
        if self.assertion_line.is_some() {
            let mut rv = self.clone();
            rv.assertion_line = None;
            Cow::Owned(rv)
        } else {
            Cow::Borrowed(self)
        }
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
    pub fn from_file(p: &Path) -> Result<Snapshot, Box<dyn Error>> {
        let mut f = BufReader::new(fs::File::open(p)?);
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
            let content = yaml::parse_str(&buf, p)?;
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

        let (snapshot_name, module_name) = names_of_path(p);

        Ok(Snapshot::from_components(
            module_name,
            Some(snapshot_name),
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

    #[cfg(feature = "_cargo_insta_internal")]
    fn from_content(content: Content) -> Result<Snapshot, Box<dyn Error>> {
        if let Content::Map(map) = content {
            let mut module_name = None;
            let mut snapshot_name = None;
            let mut metadata = None;
            let mut snapshot = None;

            for (key, value) in map.into_iter() {
                match key.as_str() {
                    Some("module_name") => module_name = value.as_str().map(|x| x.to_string()),
                    Some("snapshot_name") => snapshot_name = value.as_str().map(|x| x.to_string()),
                    Some("metadata") => metadata = Some(MetaData::from_content(value)?),
                    Some("snapshot") => {
                        snapshot = Some(SnapshotContents(
                            value
                                .as_str()
                                .ok_or(content::Error::UnexpectedDataType)?
                                .to_string(),
                        ))
                    }
                    _ => {}
                }
            }

            Ok(Snapshot {
                module_name: module_name.ok_or(content::Error::MissingField)?,
                snapshot_name,
                metadata: metadata.ok_or(content::Error::MissingField)?,
                snapshot: snapshot.ok_or(content::Error::MissingField)?,
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

    /// Snapshot contents match another snapshot's.
    pub fn matches(&self, other: &Snapshot) -> bool {
        self.contents() == other.contents()
    }

    /// Snapshot contents _and_ metadata match another snapshot's.
    pub fn matches_fully(&self, other: &Snapshot) -> bool {
        self.matches(other)
            && self.metadata.trim_for_persistence() == other.metadata.trim_for_persistence()
    }

    /// The snapshot contents as a &str
    pub fn contents_str(&self) -> &str {
        self.snapshot.as_str()
    }

    fn serialize_snapshot(&self, md: &MetaData) -> String {
        let mut buf = yaml::to_string(&md.as_content());
        buf.push_str("---\n");
        buf.push_str(self.contents_str());
        buf.push('\n');
        buf
    }

    fn save_with_metadata(
        &self,
        path: &Path,
        ref_file: Option<&Path>,
        md: &MetaData,
    ) -> Result<bool, Box<dyn Error>> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }

        let serialized_snapshot = self.serialize_snapshot(md);

        // check the reference file for contents.  Note that we always want to
        // compare snapshots that were trimmed to persistence here.
        if let Ok(old) = fs::read_to_string(ref_file.unwrap_or(path)) {
            let persisted = match md.trim_for_persistence() {
                Cow::Owned(trimmed) => Cow::Owned(self.serialize_snapshot(&trimmed)),
                Cow::Borrowed(_) => Cow::Borrowed(&serialized_snapshot),
            };
            if old == persisted.as_str() {
                return Ok(false);
            }
        }

        fs::write(path, serialized_snapshot)?;
        Ok(true)
    }

    /// Saves the snapshot.
    ///
    /// Returns `true` if the snapshot was saved.  This will return `false` if there
    /// was already a snapshot with matching contents.
    #[doc(hidden)]
    pub fn save(&self, path: &Path) -> Result<bool, Box<dyn Error>> {
        self.save_with_metadata(path, None, &self.metadata.trim_for_persistence())
    }

    /// Same as `save` but instead of writing a normal snapshot file this will write
    /// a `.snap.new` file with additional information.
    ///
    /// If the existing snapshot matches the new file, then `None` is returned, otherwise
    /// the name of the new snapshot file.
    pub(crate) fn save_new(&self, path: &Path) -> Result<Option<PathBuf>, Box<dyn Error>> {
        let new_path = path.to_path_buf().with_extension("snap.new");
        if self.save_with_metadata(&new_path, Some(path), &self.metadata)? {
            Ok(Some(new_path))
        } else {
            Ok(None)
        }
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

    /// Returns the snapshot contents as string with surrounding whitespace removed.
    pub fn as_str(&self) -> &str {
        self.0
            .trim_start_matches(|x| x == '\r' || x == '\n')
            .trim_end()
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
                    .chain(Some(format!("\n{:width$}", "", width = indentation))),
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
        self.as_str() == other.as_str()
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

/// Extracts the module and snapshot name from a snapshot path
fn names_of_path(path: &Path) -> (String, String) {
    // The final part of the snapshot file name is the test name; the
    // initial parts are the module name
    let parts: Vec<&str> = path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap_or("")
        .rsplitn(2, "__")
        .collect();

    match parts.as_slice() {
        [snapshot_name, module_name] => (snapshot_name.to_string(), module_name.to_string()),
        [snapshot_name] => (snapshot_name.to_string(), String::new()),
        _ => (String::new(), "<unknown>".to_string()),
    }
}

#[test]
fn test_names_of_path() {
    assert_debug_snapshot!(
        names_of_path(Path::new("/src/snapshots/insta_tests__tests__name_foo.snap")), @r###"
    (
        "name_foo",
        "insta_tests__tests",
    )
    "###
    );
    assert_debug_snapshot!(
        names_of_path(Path::new("/src/snapshots/name_foo.snap")), @r###"
    (
        "name_foo",
        "",
    )
    "###
    );
    assert_debug_snapshot!(
        names_of_path(Path::new("foo/src/snapshots/go1.20.5.snap")), @r###"
    (
        "go1.20.5",
        "",
    )
    "###
    );
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
                if let Some(rest) = remainder.strip_prefix('⋮') {
                    buf.push_str(rest);
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
    assert_eq!(SnapshotContents(t.to_string()).to_inline(0), r#""ab""#);
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

#[test]
fn test_parse_yaml_error() {
    use std::env::temp_dir;
    let mut temp = temp_dir();
    temp.push("bad.yaml");
    let mut f = fs::File::create(temp.clone()).unwrap();

    let invalid = r#"---
    This is invalid yaml:
     {
        {
    ---
    "#;

    f.write_all(invalid.as_bytes()).unwrap();

    let error = format!("{}", Snapshot::from_file(temp.as_path()).unwrap_err());
    assert!(error.contains("Failed parsing the YAML from"));
    assert!(error.contains("/bad.yaml"));
}

/// Check that snapshots don't take ownership of the value
#[test]
fn test_ownership() {
    // Range is non-copy
    use std::ops::Range;
    let r = Range { start: 0, end: 10 };
    assert_debug_snapshot!(r, @"0..10");
    assert_debug_snapshot!(r, @"0..10");
}
