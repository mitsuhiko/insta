use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process;

use failure::{err_msg, Error};
pub use insta::Snapshot;
use serde::Deserialize;
use walkdir::{DirEntry, WalkDir};

#[derive(Deserialize, Clone, Debug)]
pub struct Target {
    src_path: PathBuf,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Package {
    name: String,
    version: String,
    id: String,
    manifest_path: PathBuf,
    targets: Vec<Target>,
}

#[derive(Deserialize, Debug)]
pub struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    workspace_root: String,
}

impl Metadata {
    pub fn workspace_root(&self) -> &Path {
        Path::new(&self.workspace_root)
    }
}

#[derive(Deserialize, Debug)]
struct ProjectLocation {
    root: PathBuf,
}

#[derive(Debug)]
pub struct SnapshotRef {
    old_path: PathBuf,
    new_path: PathBuf,
}

impl SnapshotRef {
    fn new(new_path: PathBuf) -> SnapshotRef {
        let mut old_path = new_path.clone();
        old_path.set_extension("");
        SnapshotRef { old_path, new_path }
    }

    pub fn path(&self) -> &Path {
        &self.old_path
    }

    pub fn load_old(&self) -> Result<Option<Snapshot>, Error> {
        if fs::metadata(&self.old_path).is_err() {
            Ok(None)
        } else {
            Snapshot::from_file(&self.old_path).map(Some)
        }
    }

    pub fn load_new(&self) -> Result<Snapshot, Error> {
        Snapshot::from_file(&self.new_path)
    }

    pub fn accept(&self) -> Result<(), Error> {
        fs::rename(&self.new_path, &self.old_path)?;
        Ok(())
    }

    pub fn discard(&self) -> Result<(), Error> {
        fs::remove_file(&self.new_path)?;
        Ok(())
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

impl Package {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn iter_snapshots(&self) -> impl Iterator<Item = SnapshotRef> {
        let mut roots = HashSet::new();
        for target in &self.targets {
            let root = target.src_path.parent().unwrap();
            if !roots.contains(root) {
                roots.insert(root.to_path_buf());
            }
        }
        roots.into_iter().flat_map(|root| {
            WalkDir::new(root.clone())
                .into_iter()
                .filter_entry(|e| !is_hidden(e))
                .filter_map(|e| e.ok())
                .filter(move |e| {
                    e.file_name().to_string_lossy().ends_with(".snap.new")
                        && e.path()
                            .strip_prefix(&root)
                            .unwrap()
                            .components()
                            .any(|c| match c {
                                Component::Normal(dir) => dir.to_str() == Some("snapshots"),
                                _ => false,
                            })
                })
                .map(|e| SnapshotRef::new(e.into_path()))
        })
    }
}

fn get_cargo() -> String {
    env::var("CARGO")
        .ok()
        .unwrap_or_else(|| "cargo".to_string())
}

pub fn get_package_metadata(manifest_path: Option<&Path>) -> Result<Metadata, Error> {
    let mut cmd = process::Command::new(get_cargo());
    cmd.arg("metadata")
        .arg("--no-deps")
        .arg("--format-version=1");
    if let Some(manifest_path) = manifest_path {
        if !fs::metadata(manifest_path)
            .ok()
            .map_or(false, |x| x.is_file())
        {
            return Err(err_msg(
                "the manifest-path must be a path to a Cargo.toml file",
            ));
        }
        cmd.arg("--manifest-path").arg(manifest_path.as_os_str());
    }
    let output = cmd.output()?;
    if !output.status.success() {
        let msg = String::from_utf8_lossy(&output.stderr);
        return Err(err_msg(format!(
            "cargo erroried getting metadata: {}",
            msg.trim()
        )));
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn get_default_manifest() -> Result<Option<PathBuf>, Error> {
    let output = process::Command::new(get_cargo())
        .arg("locate-project")
        .output()?;
    if output.status.success() {
        let loc: ProjectLocation = serde_json::from_slice(&output.stdout)?;
        Ok(Some(loc.root))
    } else {
        Ok(None)
    }
}

pub fn find_packages(metadata: &Metadata, all: bool) -> Result<Vec<Package>, Error> {
    let mut rv = vec![];
    if all {
        for package in &metadata.packages {
            if metadata.workspace_members.contains(&package.id) {
                rv.push(package.clone());
            }
        }
    } else {
        let default_manifest = get_default_manifest()?
            .ok_or_else(|| {
                err_msg(
                    "Could not find `Cargo.toml` in the current folder or any parent directory.",
                )
            })?
            .canonicalize()?;
        for package in &metadata.packages {
            if package.manifest_path.canonicalize()? == default_manifest {
                rv.push(package.clone());
            }
        }
        if rv.is_empty() {
            return Err(err_msg("Unexpectedly did not find Cargo.toml in workspace"));
        }
    }
    Ok(rv)
}
