use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Utc;
use console::{style, Color};
use difference::Changeset;
use failure::Error;
use lazy_static::lazy_static;

use serde::Deserialize;
use serde_json;
#[cfg(feature = "serialization")]
use {serde::Serialize, serde_yaml};

lazy_static! {
    static ref WORKSPACES: Mutex<BTreeMap<String, &'static Path>> = Mutex::new(BTreeMap::new());
}

enum UpdateBehavior {
    InPlace,
    NewFile,
    NoUpdate,
}

fn update_snapshot_behavior() -> UpdateBehavior {
    match env::var("INSTA_UPDATE").ok().as_ref().map(|x| x.as_str()) {
        None | Some("") => UpdateBehavior::NoUpdate,
        Some("new") => UpdateBehavior::NewFile,
        Some("1") => UpdateBehavior::InPlace,
        _ => panic!("invalid value for INSTA_UPDATE"),
    }
}

fn get_cargo_workspace(manifest_dir: &str) -> &Path {
    let mut workspaces = WORKSPACES.lock().unwrap();
    if let Some(rv) = workspaces.get(manifest_dir) {
        rv
    } else {
        #[derive(Deserialize)]
        struct Manifest {
            workspace_root: String,
        }
        let output = std::process::Command::new(env!("CARGO"))
            .arg("metadata")
            .arg("--format-version=1")
            .current_dir(manifest_dir)
            .output()
            .unwrap();
        let manifest: Manifest = serde_json::from_slice(&output.stdout).unwrap();
        let path = Box::leak(Box::new(PathBuf::from(manifest.workspace_root)));
        workspaces.insert(manifest_dir.to_string(), path.as_path());
        workspaces.get(manifest_dir).unwrap()
    }
}

pub fn get_snapshot_filename(
    name: &str,
    manifest_dir: &str,
    module_path: &str,
    base: &str,
) -> PathBuf {
    let cargo_workspace = get_cargo_workspace(manifest_dir);
    let root = Path::new(cargo_workspace);
    let base = Path::new(base);
    root.join(base.parent().unwrap())
        .join("snapshots")
        .join(format!(
            "{}__{}.snap",
            module_path.rsplit("::").next().unwrap(),
            name
        ))
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
        self.save_impl(&self.path)
    }

    pub fn save_new(&self) -> Result<PathBuf, Error> {
        let mut path = self.path.to_path_buf();
        path.set_extension("snap.new");
        self.save_impl(&path)?;
        Ok(path)
    }

    pub fn save_impl(&self, path: &Path) -> Result<(), Error> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(&folder)?;
        }
        let mut f = fs::File::create(&path)?;
        for (key, value) in self.metadata.iter() {
            writeln!(f, "{}: {}", key, value)?;
        }
        f.write_all(b"\n")?;
        f.write_all(self.snapshot.as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
    }
}

pub fn print_snapshot_diff(name: &str, old_snapshot: Option<&Snapshot>, new_snapshot: &Snapshot) {
    let file = style(new_snapshot.path.display()).underlined().fg(
        if fs::metadata(&new_snapshot.path).is_ok() {
            Color::Cyan
        } else {
            Color::Red
        },
    );

    println!(
        "{title:-^width$}\nSnapshot: {name}\nFile: {file}",
        name = style(name).yellow(),
        file = file,
        title = style(" Snapshot Information ").bold(),
        width = 74
    );

    if let Some(old_snapshot) = old_snapshot {
        let title = Changeset::new("- got this run", "+ expected snapshot", "\n");
        let changeset = Changeset::new(&new_snapshot.snapshot, &old_snapshot.snapshot, "\n");

        if let Some(value) = old_snapshot.metadata.get("Created") {
            println!("Created: {}", style(value).cyan());
        }
        if let Some(value) = old_snapshot.metadata.get("Creator") {
            println!("Creator: {}", style(value).cyan());
        }

        println!(
            "{title:-^width$}",
            title = style(" Snapshot Differences ").bold(),
            width = 74
        );
        println!("{}", title);
        println!("{}", changeset);
    } else {
        println!(
            "{title:-^width$}",
            title = style(" New Snapshot ").bold(),
            width = 74
        );
        println!("{}", style(&new_snapshot.snapshot).dim());
        println!();
    }

    println!("{title:-^width$}", title = style("").bold(), width = 74);
    println!(
        "{hint}",
        hint = style("To update the snapshots re-run the tests with INSTA_UPDATE=1.").dim(),
    );
}

pub fn assert_snapshot(
    name: &str,
    new_snapshot: &str,
    manifest_dir: &str,
    module_path: &str,
    file: &str,
    line: u32,
) -> Result<(), Error> {
    let snapshot_file = get_snapshot_filename(name, manifest_dir, module_path, file);
    let old = Snapshot::from_file(&snapshot_file).ok();

    // if the snapshot matches we're done.
    if old.as_ref().map_or(false, |x| x.snapshot == new_snapshot) {
        return Ok(());
    }

    let mut metadata = BTreeMap::new();
    metadata.insert("Created".to_string(), Utc::now().to_rfc3339());
    metadata.insert(
        "Creator".to_string(),
        format!("{}@{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
    );
    metadata.insert("Source".to_string(), file.to_string());
    let new = Snapshot {
        path: snapshot_file.to_path_buf(),
        metadata,
        snapshot: new_snapshot.to_string(),
    };

    print_snapshot_diff(name, old.as_ref(), &new);

    match update_snapshot_behavior() {
        UpdateBehavior::InPlace => {
            new.save()?;
            writeln!(
                std::io::stderr(),
                "  {} {}\n",
                style("updated snapshot").green(),
                style(snapshot_file.display()).cyan().underlined(),
            )?;
        }
        UpdateBehavior::NewFile => {
            let new_path = new.save_new()?;
            writeln!(
                std::io::stderr(),
                "  {} {}\n",
                style("stored new snapshot").green(),
                style(new_path.display()).cyan().underlined(),
            )?;
        }
        UpdateBehavior::NoUpdate => {
            assert!(
                false,
                "snapshot assertion for '{}' failed in line {}",
                name, line
            );
        }
    }

    Ok(())
}

#[cfg(feature = "serialization")]
pub fn serialize_value<S: Serialize>(s: &S) -> String {
    serde_yaml::to_string(s).unwrap()
}
