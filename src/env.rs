use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{env, fs};

use crate::content::{yaml, Content};
use crate::utils::is_ci;

lazy_static::lazy_static! {
    static ref WORKSPACES: Mutex<BTreeMap<String, Arc<PathBuf>>> = Mutex::new(BTreeMap::new());
    static ref TOOL_CONFIGS: Mutex<BTreeMap<String, Arc<ToolConfig>>> = Mutex::new(BTreeMap::new());
}

pub fn get_tool_config(manifest_dir: &str) -> Arc<ToolConfig> {
    let mut configs = TOOL_CONFIGS.lock().unwrap();
    if let Some(rv) = configs.get(manifest_dir) {
        return rv.clone();
    }
    let config = Arc::new(ToolConfig::load(manifest_dir));
    configs.insert(manifest_dir.to_string(), config.clone());
    config
}

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

/// Represents a tool configuration.
#[derive(Debug)]
pub struct ToolConfig {
    values: Content,
}

impl ToolConfig {
    /// Loads the tool config for a specific manifest.
    pub fn load(manifest_dir: &str) -> ToolConfig {
        let cargo_workspace = get_cargo_workspace(manifest_dir);
        let path = cargo_workspace.join(".config/insta.yaml");
        let values = match fs::read_to_string(path) {
            Ok(s) => yaml::parse_str(&s).expect("failed to deserialize tool config"),
            Err(err) if matches!(err.kind(), io::ErrorKind::NotFound) => {
                Content::Map(Default::default())
            }
            Err(err) => panic!("failed to read tool config: {}", err),
        };
        ToolConfig { values }
    }

    fn resolve(&self, path: &[&str]) -> Option<&Content> {
        path.iter()
            .try_fold(&self.values, |node, segment| match node.resolve_inner() {
                Content::Map(fields) => fields
                    .iter()
                    .find(|x| x.0.as_str() == Some(segment))
                    .map(|x| &x.1),
                Content::Struct(_, fields) | Content::StructVariant(_, _, _, fields) => {
                    fields.iter().find(|x| x.0 == *segment).map(|x| &x.1)
                }
                _ => None,
            })
    }

    fn get_bool(&self, path: &[&str]) -> Option<bool> {
        self.resolve(path).and_then(|x| x.as_bool())
    }

    fn get_str(&self, path: &[&str]) -> Option<&str> {
        self.resolve(path).and_then(|x| x.as_str())
    }

    /// Is insta told to force update snapshots?
    pub fn force_update_snapshots(&self) -> bool {
        match env::var("INSTA_FORCE_UPDATE_SNAPSHOTS").ok().as_deref() {
            None | Some("") => self
                .get_bool(&["snapshots", "force_update"])
                .unwrap_or(false),
            Some("0") => false,
            Some("1") => true,
            _ => panic!("invalid value for INSTA_FORCE_UPDATE_SNAPSHOTS"),
        }
    }

    /// Is insta instructed to fail in tests?
    pub fn force_pass(&self) -> bool {
        match env::var("INSTA_FORCE_PASS").ok().as_deref() {
            None | Some("") => self.get_bool(&["behavior", "force_pass"]).unwrap_or(false),
            Some("0") => false,
            Some("1") => true,
            _ => panic!("invalid value for INSTA_FORCE_PASS"),
        }
    }

    /// Returns the intended output behavior for insta.
    pub fn get_output_behavior(&self) -> OutputBehavior {
        let env_var = env::var("INSTA_OUTPUT").ok();
        let val = match env_var.as_deref() {
            None | Some("") => self.get_str(&["behavior", "output"]).unwrap_or("diff"),
            Some(val) => val,
        };
        match val {
            "diff" => OutputBehavior::Diff,
            "summary" => OutputBehavior::Summary,
            "minimal" => OutputBehavior::Minimal,
            "none" => OutputBehavior::Nothing,
            _ => panic!("invalid value for INSTA_OUTPUT"),
        }
    }

    /// Returns the intended snapshot update behavior.
    pub fn get_snapshot_update_behavior(&self, unseen: bool) -> SnapshotUpdate {
        let env_var = env::var("INSTA_UPDATE").ok();
        let val = match env_var.as_deref() {
            None | Some("") => self.get_str(&["behavior", "update"]).unwrap_or("auto"),
            Some(val) => val,
        };
        match val {
            "auto" => {
                if is_ci() {
                    SnapshotUpdate::NoUpdate
                } else {
                    SnapshotUpdate::NewFile
                }
            }
            "always" | "1" => SnapshotUpdate::InPlace,
            "new" => SnapshotUpdate::NewFile,
            "unseen" => {
                if unseen {
                    SnapshotUpdate::NewFile
                } else {
                    SnapshotUpdate::InPlace
                }
            }
            "no" => SnapshotUpdate::NoUpdate,
            _ => panic!("invalid value for INSTA_UPDATE"),
        }
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
            let docs =
                yaml_rust::YamlLoader::load_from_str(std::str::from_utf8(&output.stdout).unwrap())
                    .unwrap();
            let manifest = docs.get(0).expect("Unable to parse cargo manifest");
            let workspace_root = PathBuf::from(manifest["workspace_root"].as_str().unwrap());
            Arc::new(workspace_root)
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
