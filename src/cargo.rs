use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use serde::Deserialize;

lazy_static! {
    static ref WORKSPACES: Mutex<BTreeMap<String, Arc<PathBuf>>> = Mutex::new(BTreeMap::new());
}

fn get_cargo() -> String {
    env::var("CARGO")
        .ok()
        .unwrap_or_else(|| "cargo".to_string())
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
            let output = std::process::Command::new(get_cargo())
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
