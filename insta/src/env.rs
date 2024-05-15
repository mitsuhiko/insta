use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{env, fmt, fs};

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
    let config =
        Arc::new(ToolConfig::from_manifest_dir(manifest_dir).expect("failed to load tool config"));
    configs.insert(manifest_dir.to_string(), config.clone());
    config
}

/// The test runner to use.
#[cfg(feature = "_cargo_insta_internal")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestRunner {
    Auto,
    CargoTest,
    Nextest,
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

/// Unreferenced snapshots flag
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "_cargo_insta_internal")]
pub enum UnreferencedSnapshots {
    Auto,
    Reject,
    Delete,
    Warn,
    Ignore,
}

/// Snapshot update flag
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotUpdate {
    Always,
    Auto,
    Unseen,
    New,
    No,
}

#[derive(Debug)]
pub enum Error {
    Deserialize(crate::content::Error),
    Env(&'static str),
    #[allow(unused)]
    Config(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Deserialize(_) => write!(f, "failed to deserialize tool config"),
            Error::Env(var) => write!(f, "invalid value for env var '{}'", var),
            Error::Config(var) => write!(f, "invalid value for config '{}'", var),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Deserialize(ref err) => Some(err),
            _ => None,
        }
    }
}

/// Represents a tool configuration.
#[derive(Debug)]
pub struct ToolConfig {
    force_update_snapshots: bool,
    force_pass: bool,
    require_full_match: bool,
    output: OutputBehavior,
    snapshot_update: SnapshotUpdate,
    #[cfg(feature = "glob")]
    glob_fail_fast: bool,
    #[cfg(feature = "_cargo_insta_internal")]
    test_runner: TestRunner,
    #[cfg(feature = "_cargo_insta_internal")]
    test_unreferenced: UnreferencedSnapshots,
    #[cfg(feature = "_cargo_insta_internal")]
    auto_review: bool,
    #[cfg(feature = "_cargo_insta_internal")]
    auto_accept_unseen: bool,
    #[cfg(feature = "_cargo_insta_internal")]
    review_include_ignored: bool,
    #[cfg(feature = "_cargo_insta_internal")]
    review_include_hidden: bool,
    #[cfg(feature = "_cargo_insta_internal")]
    review_warn_undiscovered: bool,
}

impl ToolConfig {
    /// Loads the tool config for a specific manifest.
    pub fn from_manifest_dir(manifest_dir: &str) -> Result<ToolConfig, Error> {
        ToolConfig::from_workspace(&get_cargo_workspace(manifest_dir))
    }

    /// Loads the tool config from a cargo workspace.
    pub fn from_workspace(workspace_dir: &Path) -> Result<ToolConfig, Error> {
        let mut cfg = None;
        for choice in &[".config/insta.yaml", "insta.yaml", ".insta.yaml"] {
            let path = workspace_dir.join(choice);
            match fs::read_to_string(&path) {
                Ok(s) => {
                    cfg = Some(yaml::parse_str(&s, &path).map_err(Error::Deserialize)?);
                    break;
                }
                // ideally we would not swallow all errors here but unfortunately there are
                // some cases where we cannot detect the error properly.
                // Eg we can see NotADirectory here as kind, but on stable rust it cannot
                // be matched on.
                Err(_) => continue,
            }
        }
        let cfg = cfg.unwrap_or_else(|| Content::Map(Default::default()));

        // support for the deprecated environment variable.  This is implemented in a way that
        // cargo-insta can support older and newer insta versions alike.  It will set both
        // variables.  However if only `INSTA_FORCE_UPDATE_SNAPSHOTS` is set, we will emit
        // a deprecation warning.
        if env::var("INSTA_FORCE_UPDATE").is_err() {
            if let Ok("1") = env::var("INSTA_FORCE_UPDATE_SNAPSHOTS").as_deref() {
                eprintln!("INSTA_FORCE_UPDATE_SNAPSHOTS is deprecated, use INSTA_FORCE_UPDATE");
                env::set_var("INSTA_FORCE_UPDATE", "1");
            }
        }

        Ok(ToolConfig {
            force_update_snapshots: match env::var("INSTA_FORCE_UPDATE").as_deref() {
                Err(_) | Ok("") => resolve(&cfg, &["behavior", "force_update"])
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false),
                Ok("0") => false,
                Ok("1") => true,
                _ => return Err(Error::Env("INSTA_FORCE_UPDATE")),
            },
            require_full_match: match env::var("INSTA_REQUIRE_FULL_MATCH").as_deref() {
                Err(_) | Ok("") => resolve(&cfg, &["behavior", "require_full_match"])
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false),
                Ok("0") => false,
                Ok("1") => true,
                _ => return Err(Error::Env("INSTA_REQUIRE_FULL_MATCH")),
            },
            force_pass: match env::var("INSTA_FORCE_PASS").as_deref() {
                Err(_) | Ok("") => resolve(&cfg, &["behavior", "force_pass"])
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false),
                Ok("0") => false,
                Ok("1") => true,
                _ => return Err(Error::Env("INSTA_FORCE_PASS")),
            },
            output: {
                let env_var = env::var("INSTA_OUTPUT");
                let val = match env_var.as_deref() {
                    Err(_) | Ok("") => resolve(&cfg, &["behavior", "output"])
                        .and_then(|x| x.as_str())
                        .unwrap_or("diff"),
                    Ok(val) => val,
                };
                match val {
                    "diff" => OutputBehavior::Diff,
                    "summary" => OutputBehavior::Summary,
                    "minimal" => OutputBehavior::Minimal,
                    "none" => OutputBehavior::Nothing,
                    _ => return Err(Error::Env("INSTA_OUTPUT")),
                }
            },
            snapshot_update: {
                let env_var = env::var("INSTA_UPDATE");
                let val = match env_var.as_deref() {
                    Err(_) | Ok("") => resolve(&cfg, &["behavior", "update"])
                        .and_then(|x| x.as_str())
                        .unwrap_or("auto"),
                    Ok(val) => val,
                };
                match val {
                    "auto" => SnapshotUpdate::Auto,
                    "always" | "1" => SnapshotUpdate::Always,
                    "new" => SnapshotUpdate::New,
                    "unseen" => SnapshotUpdate::Unseen,
                    "no" => SnapshotUpdate::No,
                    _ => return Err(Error::Env("INSTA_UPDATE")),
                }
            },
            #[cfg(feature = "glob")]
            glob_fail_fast: match env::var("INSTA_GLOB_FAIL_FAST").as_deref() {
                Err(_) | Ok("") => resolve(&cfg, &["behavior", "glob_fail_fast"])
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false),
                Ok("1") => true,
                Ok("0") => false,
                _ => return Err(Error::Env("INSTA_GLOB_FAIL_FAST")),
            },
            #[cfg(feature = "_cargo_insta_internal")]
            test_runner: {
                let env_var = env::var("INSTA_TEST_RUNNER");
                match env_var.as_deref() {
                    Err(_) | Ok("") => resolve(&cfg, &["test", "runner"])
                        .and_then(|x| x.as_str())
                        .unwrap_or("auto"),
                    Ok(val) => val,
                }
                .parse::<TestRunner>()
                .map_err(|_| Error::Env("INSTA_TEST_RUNNER"))?
            },
            #[cfg(feature = "_cargo_insta_internal")]
            test_unreferenced: {
                resolve(&cfg, &["test", "unreferenced"])
                    .and_then(|x| x.as_str())
                    .unwrap_or("ignore")
                    .parse::<UnreferencedSnapshots>()
                    .map_err(|_| Error::Config("unreferenced"))?
            },
            #[cfg(feature = "_cargo_insta_internal")]
            auto_review: resolve(&cfg, &["test", "auto_review"])
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            #[cfg(feature = "_cargo_insta_internal")]
            auto_accept_unseen: resolve(&cfg, &["test", "auto_accept_unseen"])
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            #[cfg(feature = "_cargo_insta_internal")]
            review_include_hidden: resolve(&cfg, &["review", "include_hidden"])
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            #[cfg(feature = "_cargo_insta_internal")]
            review_include_ignored: resolve(&cfg, &["review", "include_ignored"])
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            #[cfg(feature = "_cargo_insta_internal")]
            review_warn_undiscovered: resolve(&cfg, &["review", "warn_undiscovered"])
                .and_then(|x| x.as_bool())
                .unwrap_or(true),
        })
    }

    /// Is insta told to force update snapshots?
    pub fn force_update_snapshots(&self) -> bool {
        self.force_update_snapshots
    }

    /// Should we fail if metadata doesn't match?
    pub fn require_full_match(&self) -> bool {
        self.require_full_match
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
    pub fn snapshot_update(&self) -> SnapshotUpdate {
        self.snapshot_update
    }

    /// Returns the value of glob_fail_fast
    #[cfg(feature = "glob")]
    pub fn glob_fail_fast(&self) -> bool {
        self.glob_fail_fast
    }
}

#[cfg(feature = "_cargo_insta_internal")]
impl ToolConfig {
    /// Returns the intended test runner
    pub fn test_runner(&self) -> TestRunner {
        self.test_runner
    }

    pub fn test_unreferenced(&self) -> UnreferencedSnapshots {
        self.test_unreferenced
    }

    /// Returns the auto review flag.
    pub fn auto_review(&self) -> bool {
        self.auto_review
    }

    /// Returns the auto accept unseen flag.
    pub fn auto_accept_unseen(&self) -> bool {
        self.auto_accept_unseen
    }

    pub fn review_include_hidden(&self) -> bool {
        self.review_include_hidden
    }

    pub fn review_include_ignored(&self) -> bool {
        self.review_include_ignored
    }

    pub fn review_warn_undiscovered(&self) -> bool {
        self.review_warn_undiscovered
    }
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

/// Returns the intended snapshot update behavior.
pub fn snapshot_update_behavior(tool_config: &ToolConfig, unseen: bool) -> SnapshotUpdateBehavior {
    match tool_config.snapshot_update() {
        SnapshotUpdate::Always => SnapshotUpdateBehavior::InPlace,
        SnapshotUpdate::Auto => {
            if is_ci() {
                SnapshotUpdateBehavior::NoUpdate
            } else {
                SnapshotUpdateBehavior::NewFile
            }
        }
        SnapshotUpdate::Unseen => {
            if unseen {
                SnapshotUpdateBehavior::NewFile
            } else {
                SnapshotUpdateBehavior::InPlace
            }
        }
        SnapshotUpdate::New => SnapshotUpdateBehavior::NewFile,
        SnapshotUpdate::No => SnapshotUpdateBehavior::NoUpdate,
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
            let docs = crate::content::yaml::vendored::yaml::YamlLoader::load_from_str(
                std::str::from_utf8(&output.stdout).unwrap(),
            )
            .unwrap();
            let manifest = docs.first().expect("Unable to parse cargo manifest");
            let workspace_root = PathBuf::from(manifest["workspace_root"].as_str().unwrap());
            Arc::new(workspace_root)
        };
        workspaces.insert(manifest_dir.to_string(), path.clone());
        path
    }
}

#[cfg(feature = "_cargo_insta_internal")]
impl std::str::FromStr for TestRunner {
    type Err = ();

    fn from_str(value: &str) -> Result<TestRunner, ()> {
        match value {
            "auto" => Ok(TestRunner::Auto),
            "cargo-test" => Ok(TestRunner::CargoTest),
            "nextest" => Ok(TestRunner::Nextest),
            _ => Err(()),
        }
    }
}

#[cfg(feature = "_cargo_insta_internal")]
impl std::str::FromStr for UnreferencedSnapshots {
    type Err = ();

    fn from_str(value: &str) -> Result<UnreferencedSnapshots, ()> {
        match value {
            "auto" => Ok(UnreferencedSnapshots::Auto),
            "reject" | "error" => Ok(UnreferencedSnapshots::Reject),
            "delete" => Ok(UnreferencedSnapshots::Delete),
            "warn" => Ok(UnreferencedSnapshots::Warn),
            "ignore" => Ok(UnreferencedSnapshots::Ignore),
            _ => Err(()),
        }
    }
}

/// Memoizes a snapshot file in the reference file.
pub fn memoize_snapshot_file(snapshot_file: &Path) {
    if let Ok(path) = env::var("INSTA_SNAPSHOT_REFERENCES_FILE") {
        let mut f = fs::OpenOptions::new()
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
