use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{env, fs};

use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::utils::is_ci;

static WORKSPACES: Lazy<Mutex<BTreeMap<String, Arc<PathBuf>>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

/// How snapshots are supposed to be updated
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotUpdate {
    /// Snapshots are updated in-place
    InPlace,
    /// Snapshots are placed in a new file with a .new suffix
    NewFile,
    /// Snapshots are not updated at all.
    NoUpdate,
}

/// Controls how information is supposed to be displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputBehavior {
    /// Diff only
    Diff,
    /// Short summary
    Summary,
    /// The most minimal output
    Minimal,
    /// No output at all
    Nothing,
}

/// Is insta told to force update snapshots?
pub fn force_update_snapshots() -> bool {
    match env::var("INSTA_FORCE_UPDATE_SNAPSHOTS").ok().as_deref() {
        None | Some("") | Some("0") => false,
        Some("1") => true,
        _ => panic!("invalid value for INSTA_FORCE_UPDATE_SNAPSHOTS"),
    }
}

/// Is insta instructed to fail in tests?
pub fn force_pass() -> bool {
    match env::var("INSTA_FORCE_PASS").ok().as_deref() {
        None | Some("") | Some("0") => false,
        Some("1") => true,
        _ => panic!("invalid value for INSTA_FORCE_PASS"),
    }
}

/// Returns the intended output behavior for insta.
pub fn get_output_behavior() -> OutputBehavior {
    match env::var("INSTA_OUTPUT").ok().as_deref() {
        None | Some("") | Some("diff") => OutputBehavior::Diff,
        Some("summary") => OutputBehavior::Summary,
        Some("minimal") => OutputBehavior::Minimal,
        Some("none") => OutputBehavior::Nothing,
        _ => panic!("invalid value for INSTA_OUTPUT"),
    }
}

/// Returns the intended snapshot update behavior.
pub fn get_snapshot_update_behavior(unseen: bool) -> SnapshotUpdate {
    match env::var("INSTA_UPDATE").ok().as_deref() {
        None | Some("") | Some("auto") => {
            if is_ci() {
                SnapshotUpdate::NoUpdate
            } else {
                SnapshotUpdate::NewFile
            }
        }
        Some("always") | Some("1") => SnapshotUpdate::InPlace,
        Some("new") => SnapshotUpdate::NewFile,
        Some("unseen") => {
            if unseen {
                SnapshotUpdate::NewFile
            } else {
                SnapshotUpdate::InPlace
            }
        }
        Some("no") => SnapshotUpdate::NoUpdate,
        _ => panic!("invalid value for INSTA_UPDATE"),
    }
}

/// Returns the cargo workspace for a manifest
pub fn get_cargo_workspace(manifest_dir: &str) -> Arc<PathBuf> {
    // we really do not care about poisoning here.
    let mut workspaces = WORKSPACES.lock().unwrap_or_else(|x| x.into_inner());
    if let Some(rv) = workspaces.get(manifest_dir) {
        rv.clone()
    } else {
        // If INSTA_WORKSPACE_ROOT environment variable is set, use the value
        // as-is. This is useful for those users where the compiled in
        // CARGO_MANIFEST_DIR points to some transient location. This can easily
        // happen if the user builds the test in one directory but then tries to
        // run it in another: even if sources are available in the new
        // directory, in the past we would always go with the compiled-in value.
        // The compiled-in directory may not even exist anymore.
        let path = if let Ok(workspace_root) = std::env::var("INSTA_WORKSPACE_ROOT") {
            Arc::new(PathBuf::from(workspace_root))
        } else {
            #[derive(Deserialize)]
            struct Manifest {
                workspace_root: PathBuf,
            }
            let output = std::process::Command::new(
                env::var("CARGO")
                    .ok()
                    .unwrap_or_else(|| "cargo".to_string()),
            )
            .arg("metadata")
            .arg("--format-version=1")
            .arg("--no-deps")
            .current_dir(manifest_dir)
            .output()
            .unwrap();
            let manifest: Manifest = serde_json::from_slice(&output.stdout).unwrap();
            Arc::new(manifest.workspace_root)
        };
        workspaces.insert(manifest_dir.to_string(), path.clone());
        path
    }
}

/// Memoizes a snapshot file in the reference file.
pub fn memoize_snapshot_file(snapshot_file: &Path) {
    if let Ok(path) = env::var("INSTA_SNAPSHOT_REFERENCES_FILE") {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(path)
            .unwrap();
        f.write_all(format!("{}\n", snapshot_file.display()).as_bytes())
            .unwrap();
    }
}
