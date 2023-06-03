use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use serde::Deserialize;

use crate::utils::err_msg;

#[derive(Deserialize, Clone, Debug)]
pub struct Target {
    src_path: PathBuf,
    kind: HashSet<String>,
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

impl Package {
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn find_snapshot_roots<'a>(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();

        // the manifest path's parent is always a snapshot container.  For
        // a rationale see GH-70.  But generally a user would expect to be
        // able to put a snapshot into foo/snapshots instead of foo/src/snapshots.
        if let Some(manifest) = self.manifest_path.parent() {
            roots.push(manifest.to_path_buf());
        }

        // additionally check all targets.
        for target in &self.targets {
            // custom build scripts we can safely skip over.  In the past this
            // caused issues with duplicate paths but that's resolved in other
            // ways now.  We do not want to pick up snapshots in such places
            // though.
            if target.kind.contains("custom-build") {
                continue;
            }

            // this gives us the containing source folder.  Typically this is
            // something like crate/src.
            let root = target.src_path.parent().unwrap();
            roots.push(root.to_path_buf());
        }

        // reduce roots to avoid traversing into paths twice.  If we have both
        // /foo and /foo/bar as roots we would only walk into /foo.  Otherwise
        // we would encounter paths twice.  If we don't skip them here we run
        // into issues where the existence of a build script causes a snapshot
        // to be picked up twice since the same path is determined.  (GH-15)
        roots.sort_by_key(|x| x.as_os_str().len());
        let mut reduced_roots = vec![];
        for root in roots {
            if !reduced_roots.iter().any(|x| root.starts_with(x)) {
                reduced_roots.push(root);
            }
        }

        reduced_roots
    }
}

pub fn get_cargo() -> String {
    env::var("CARGO")
        .ok()
        .unwrap_or_else(|| "cargo".to_string())
}

pub fn get_package_metadata(manifest_path: Option<&Path>) -> Result<Metadata, Box<dyn Error>> {
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

fn get_default_manifest() -> Result<Option<PathBuf>, Box<dyn Error>> {
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

fn find_all_packages(metadata: &Metadata) -> Vec<Package> {
    metadata
        .packages
        .iter()
        .filter_map(|package| {
            if metadata.workspace_members.contains(&package.id) {
                Some(package.clone())
            } else {
                None
            }
        })
        .collect()
}

pub fn find_packages(metadata: &Metadata, all: bool) -> Result<Vec<Package>, Box<dyn Error>> {
    if all {
        Ok(find_all_packages(metadata))
    } else {
        let default_manifest = get_default_manifest()?
            .ok_or_else(|| {
                err_msg(
                    "Could not find `Cargo.toml` in the current folder or any parent directory.",
                )
            })?
            .canonicalize()?;
        let mut rv = vec![];
        for package in &metadata.packages {
            if package.manifest_path.canonicalize()? == default_manifest {
                rv.push(package.clone());
            }
        }
        if rv.is_empty() {
            // if we don't find anything we're in a workspace root that has no
            // root member in which case --all is implied.
            Ok(find_all_packages(metadata))
        } else {
            Ok(rv)
        }
    }
}
