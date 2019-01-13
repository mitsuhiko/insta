use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::io::Write;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use chrono::Utc;
use console::style;
use difference::Changeset;
use failure::Error;

#[cfg(feature = "serialization")]
use {serde::Serialize, serde_yaml};

struct RunHint<'a>(&'a Path, Option<&'a Snapshot>);

impl<'a> fmt::Display for RunHint<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\n{title:-^width$}\nSnapshot: {file}\n",
            file = style(self.0.display()).cyan().underlined(),
            title = style(" Snapshot Information ").bold(),
            width = 74
        )?;

        if let Some(ref old) = self.1 {
            for (key, value) in old.metadata.iter() {
                write!(f, "{}: {}\n", key, style(value).cyan())?;
            }
        }

        write!(
            f,
            "\n{hint}\n",
            hint = style("To update the snapshots re-run the tests with INSTA_UPDATE=1.").dim(),
        )?;
        Ok(())
    }
}

fn should_update_snapshot() -> bool {
    match env::var("INSTA_UPDATE").ok().as_ref().map(|x| x.as_str()) {
        None | Some("") => false,
        Some("1") => true,
        _ => panic!("invalid value for INSTA_UPDATE"),
    }
}

pub fn get_snapshot_filename(name: &str, module_path: &str, base: &str) -> PathBuf {
    let path = Path::new(base);
    path.parent()
        .unwrap()
        .join("snapshots")
        .join(format!("{}__{}.snap", module_path.rsplit("::").next().unwrap(), name))
}

#[derive(Debug)]
pub struct Snapshot {
    path: PathBuf,
    metadata: BTreeMap<String, String>,
    snapshot: String,
}

impl Snapshot {
    pub fn from_file<P: AsRef<Path>>(p: &P) -> Result<Snapshot, Error> {
        let mut f = BufReader::new(fs::File::open(p)?);
        let mut buf = String::new();
        let mut metadata = BTreeMap::new();

        loop {
            buf.clear();
            f.read_line(&mut buf)?;
            if buf.trim().is_empty() {
                break;
            }
            let mut iter = buf.splitn(2, ':');
            if let Some(key) = iter.next() {
                if let Some(value) = iter.next() {
                    metadata.insert(key.to_string(), value.trim().to_string());
                }
            }
        }

        buf.clear();
        f.read_to_string(&mut buf)?;
        if buf.ends_with('\n') {
            buf.truncate(buf.len() - 1);
        }

        Ok(Snapshot {
            path: p.as_ref().to_path_buf(),
            metadata,
            snapshot: buf,
        })
    }

    pub fn save(&self) -> Result<(), Error> {
        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(&folder)?;
        }
        let mut f = fs::File::create(&self.path)?;
        for (key, value) in self.metadata.iter() {
            write!(f, "{}: {}\n", key, value)?;
        }
        f.write_all(b"\n")?;
        f.write_all(self.snapshot.as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
    }
}

pub fn assert_snapshot(
    name: &str,
    new_snapshot: &str,
    module_path: &str,
    file: &str,
    line: u32,
) -> Result<(), Error> {
    let snapshot_file = get_snapshot_filename(name, module_path, file);
    let old = Snapshot::from_file(&snapshot_file).ok();

    // if the snapshot matches we're done.
    if old.as_ref().map_or(false, |x| x.snapshot == new_snapshot) {
        return Ok(());
    }

    if should_update_snapshot() {
        let mut metadata = BTreeMap::new();
        metadata.insert("Created".to_string(), Utc::now().to_rfc3339());
        metadata.insert(
            "Creator".to_string(),
            format!("{}@{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        );
        metadata.insert("Source".to_string(), file.to_string());
        let snapshot = Snapshot {
            path: snapshot_file.to_path_buf(),
            metadata: metadata,
            snapshot: new_snapshot.to_string(),
        };
        snapshot.save()?;

        match old {
            Some(ref old) => {
                let title = Changeset::new("- old snapshot", "+ new snapshot", "\n");
                let changeset = Changeset::new(&old.snapshot, new_snapshot, "\n");
                writeln!(
                    std::io::stderr(),
                    "  {} {}\n{}\n{}",
                    style("updated snapshot").green(),
                    style(snapshot_file.display()).cyan().underlined(),
                    title,
                    changeset,
                )?;
            }
            None => {
                writeln!(
                    std::io::stderr(),
                    "  {} {}",
                    style("created snapshot").green(),
                    style(snapshot_file.display()).cyan().underlined()
                )?;
            }
        }
    } else {
        match old.as_ref().map(|x| &x.snapshot) {
            None => panic!(
                "Missing snapshot '{}' in line {}{}",
                name,
                line,
                RunHint(&snapshot_file, old.as_ref()),
            ),
            Some(ref old_snapshot) => {
                let title = Changeset::new("- got this run", "+ expected snapshot", "\n");
                let changeset = Changeset::new(new_snapshot, old_snapshot, "\n");
                assert!(
                    false,
                    "snapshot '{}' mismatched in line {}:\n{}\n{}{}",
                    name,
                    line,
                    title,
                    changeset,
                    RunHint(&snapshot_file, old.as_ref()),
                );
            }
        }
    }

    Ok(())
}

#[cfg(feature = "serialization")]
pub fn serialize_value<S: Serialize>(s: &S) -> String {
    serde_yaml::to_string(s).unwrap()
}
