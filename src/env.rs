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
pub enum SnapshotUpdateBehavior {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SnapshotUpdateSetting {
    Always,
    Auto,
    Unseen,
    New,
    No,
}

/// Represents a tool configuration.
#[derive(Debug)]
pub struct ToolConfig {
    force_update_snapshots: bool,
    force_pass: bool,
    output: OutputBehavior,
    snapshot_update: SnapshotUpdateSetting,
    #[allow(unused)]
    glob_fail_fast: bool,
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

        let force_update_snapshots = match env::var("INSTA_FORCE_UPDATE_SNAPSHOTS").as_deref() {
            Err(_) | Ok("") => resolve(&values, &["behavior", "force_update"])
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            Ok("0") => false,
            Ok("1") => true,
            _ => panic!("invalid value for INSTA_FORCE_UPDATE_SNAPSHOTS"),
        };

        let force_pass = match env::var("INSTA_FORCE_PASS").as_deref() {
            Err(_) | Ok("") => resolve(&values, &["behavior", "force_pass"])
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            Ok("0") => false,
            Ok("1") => true,
            _ => panic!("invalid value for INSTA_FORCE_PASS"),
        };

        let output = {
            let env_var = env::var("INSTA_OUTPUT");
            let val = match env_var.as_deref() {
                Err(_) | Ok("") => resolve(&values, &["behavior", "output"])
                    .and_then(|x| x.as_str())
                    .unwrap_or("diff"),
                Ok(val) => val,
            };
            match val {
                "diff" => OutputBehavior::Diff,
                "summary" => OutputBehavior::Summary,
                "minimal" => OutputBehavior::Minimal,
                "none" => OutputBehavior::Nothing,
                _ => panic!("invalid value for INSTA_OUTPUT"),
            }
        };

        let snapshot_update = {
            let env_var = env::var("INSTA_UPDATE");
            let val = match env_var.as_deref() {
                Err(_) | Ok("") => resolve(&values, &["behavior", "update"])
                    .and_then(|x| x.as_str())
                    .unwrap_or("auto"),
                Ok(val) => val,
            };
            match val {
                "auto" => SnapshotUpdateSetting::Auto,
                "always" | "1" => SnapshotUpdateSetting::Always,
                "new" => SnapshotUpdateSetting::New,
                "unseen" => SnapshotUpdateSetting::Unseen,
                "no" => SnapshotUpdateSetting::No,
                _ => panic!("invalid value for INSTA_UPDATE"),
            }
        };

        let glob_fail_fast = match env::var("INSTA_GLOB_FAIL_FAST").as_deref() {
            Err(_) | Ok("") => resolve(&values, &["behavior", "glob_fail_fast"])
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            Ok("1") => true,
            Ok("0") => false,
            _ => panic!("invalid value for INSTA_GLOB_FAIL_FAST"),
        };

        ToolConfig {
            force_update_snapshots,
            force_pass,
            output,
            snapshot_update,
            glob_fail_fast,
        }
    }

    /// Is insta told to force update snapshots?
    pub fn force_update_snapshots(&self) -> bool {
        self.force_update_snapshots
    }

    /// Is insta instructed to fail in tests?
    pub fn force_pass(&self) -> bool {
        self.force_pass
    }

    /// Returns the intended output behavior for insta.
    pub fn output_behavior(&self) -> OutputBehavior {
        self.output
    }

    /// Returns the intended snapshot update behavior.
    pub fn snapshot_update_behavior(&self, unseen: bool) -> SnapshotUpdateBehavior {
        match self.snapshot_update {
            SnapshotUpdateSetting::Always => SnapshotUpdateBehavior::InPlace,
            SnapshotUpdateSetting::Auto => {
                if is_ci() {
                    SnapshotUpdateBehavior::NoUpdate
                } else {
                    SnapshotUpdateBehavior::NewFile
                }
            }
            SnapshotUpdateSetting::Unseen => {
                if unseen {
                    SnapshotUpdateBehavior::NewFile
                } else {
                    SnapshotUpdateBehavior::InPlace
                }
            }
            SnapshotUpdateSetting::New => SnapshotUpdateBehavior::NewFile,
            SnapshotUpdateSetting::No => SnapshotUpdateBehavior::NoUpdate,
        }
    }

    /// Returns the value of glob_fail_fast
    #[allow(unused)]
    pub fn glob_fail_fast(&self) -> bool {
        self.glob_fail_fast
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

fn resolve<'a>(value: &'a Content, path: &[&str]) -> Option<&'a Content> {
    path.iter()
        .try_fold(value, |node, segment| match node.resolve_inner() {
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
