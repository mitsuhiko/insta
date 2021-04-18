use std::error::Error;
use std::fs;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::loader::{DefaultSnapfileFormatter, SnapfileFormatter};
use super::runtime::get_inline_snapshot_value;

lazy_static! {
    static ref RUN_ID: String = {
        let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        format!("{}-{}", d.as_secs(), d.subsec_nanos())
    };
}

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
    pub source: Option<String>,
    /// Optionally the expression that created the snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    /// Reference to the input file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_file: Option<String>,
}

impl MetaData {
    /// Returns the absolute source path.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
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
    pub(crate) metadata: MetaData,
    snapshot: SnapshotContents,
}

impl Snapshot {
    /// Loads a snapshot from a file.
    pub fn from_file<P: AsRef<Path>>(p: P) -> Result<Snapshot, Box<dyn Error>> {
        let mut f = BufReader::new(fs::File::open(p.as_ref())?);
        DefaultSnapfileFormatter::deserialize(&mut f, p.as_ref().file_name().unwrap())
    }

    /// Creates a snapshot from its parts.
    pub fn from_components(
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

    pub(crate) fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let path = path.as_ref();
        if let Some(folder) = path.parent() {
            fs::create_dir_all(&folder)?;
        }
        let mut f = fs::File::create(&path)?;
        DefaultSnapfileFormatter::serialize(self, &mut f)
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

    pub fn as_str(&self) -> &str {
        &self.0
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
