use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json;

use super::runtime::get_inline_snapshot_value;

lazy_static! {
    static ref RUN_ID: Uuid = Uuid::new_v4();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PendingInlineSnapshot {
    pub run_id: Uuid,
    pub line: u32,
    pub new: Snapshot,
    pub old: Option<Snapshot>,
}

impl PendingInlineSnapshot {
    pub fn new(new: Snapshot, old: Option<Snapshot>, line: u32) -> PendingInlineSnapshot {
        PendingInlineSnapshot {
            new,
            old,
            line,
            run_id: *RUN_ID,
        }
    }

    pub fn load_batch<P: AsRef<Path>>(p: P) -> Result<Vec<PendingInlineSnapshot>, Box<dyn Error>> {
        let f = BufReader::new(fs::File::open(p)?);
        let iter = serde_json::Deserializer::from_reader(f).into_iter::<PendingInlineSnapshot>();
        let mut rv = iter.collect::<Result<Vec<PendingInlineSnapshot>, _>>()?;

        // remove all but the last run
        if let Some(last_run_id) = rv.last().map(|x| x.run_id) {
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
    /// The timestamp of when the snapshot was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,
    /// The creator of the snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    /// The source file (relative to workspace root).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Optionally the expression that created the snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
}

impl MetaData {
    pub fn get_relative_source(&self, base: &Path) -> Option<PathBuf> {
        self.source.as_ref().map(|source| {
            base.join(source)
                .canonicalize()
                .ok()
                .and_then(|s| s.strip_prefix(base).ok().map(|x| x.to_path_buf()))
                .unwrap_or_else(|| base.to_path_buf())
        })
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
                            "created" => {
                                rv.created =
                                    Some(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
                            }
                            "creator" => rv.creator = Some(value.to_string()),
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
        self.snapshot_name.as_ref().map(|x| x.as_str())
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
        serde_yaml::to_writer(&mut f, &self.metadata)?;
        f.write_all(b"\n---\n")?;
        f.write_all(self.contents_str().as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
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
        denormalize_inline_snapshot(&self.0, indentation)
    }
}

impl From<&str> for SnapshotContents {
    fn from(value: &str) -> SnapshotContents {
        SnapshotContents(value.to_string())
    }
}

impl From<String> for SnapshotContents {
    fn from(value: String) -> SnapshotContents {
        SnapshotContents(value)
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
    let snapshot_contents = SnapshotContents("testing".to_string());
    assert_eq!(snapshot_contents.to_inline(0), r#""testing""#);
}

// from a snapshot to a string we want to write back
fn denormalize_inline_snapshot(snapshot: &str, indentation: usize) -> String {
    // TODO: add to SnapshotContents struct
    let mut out = String::new();
    let is_escape = snapshot.lines().count() > 1 || snapshot.contains(&['\\', '"'][..]);

    out.push_str(if is_escape { "r###\"" } else { "\"" });
    // if we have more than one line we want to change into the block
    // representation mode
    if snapshot.lines().count() > 1 {
        out.push_str("\n");
        out.extend(
            snapshot
                .lines()
                .map(|l| format!("{c: >width$}{l}\n", c = "", width = indentation, l = l)),
        );
        out.push_str(&format!("{c: >width$}", c = "", width = indentation));
    } else {
        out.push_str(snapshot);
    }

    out.push_str(if is_escape { "\"###" } else { "\"" });

    out
}

#[test]
fn test_denormalize_inline_snapshot() {
    let t = &"
a
b"[1..];
    assert_eq!(
        denormalize_inline_snapshot(t, 0),
        "r###\"
a
b
\"###"
    );

    let t = &"
a
b"[1..];
    assert_eq!(
        denormalize_inline_snapshot(t, 4),
        "r###\"
    a
    b
    \"###"
    );

    let t = &"
    a
    b"[1..];
    assert_eq!(
        denormalize_inline_snapshot(t, 0),
        "r###\"
    a
    b
\"###"
    );

    let t = "ab";
    assert_eq!(denormalize_inline_snapshot(t, 0), r##""ab""##);
}
