use std::borrow::Cow;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

use crate::env::{
    force_pass, force_update_snapshots, get_cargo_workspace, get_output_behavior,
    get_snapshot_update_behavior, memoize_snapshot_file, OutputBehavior, SnapshotUpdate,
};
use crate::output::{print_snapshot_diff_with_title, print_snapshot_summary_with_title};
use crate::settings::Settings;
use crate::snapshot::{MetaData, PendingInlineSnapshot, Snapshot, SnapshotContents};
use crate::utils::style;

static TEST_NAME_COUNTERS: Lazy<Mutex<BTreeMap<String, usize>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));
static TEST_NAME_CLASH_DETECTION: Lazy<Mutex<BTreeMap<String, bool>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

// This macro is basically eprintln but without being captured and
// hidden by the test runner.
macro_rules! elog {
    () => (write!(std::io::stderr()).ok());
    ($($arg:tt)*) => ({
        writeln!(std::io::stderr(), $($arg)*).ok();
    })
}

/// Special marker to use an automatic name.
///
/// This can be passed as a snapshot name in a macro to explicitly tell
/// insta to use the automatic name.  This is useful in ambiguous syntax
/// situations.
#[derive(Debug)]
pub struct AutoName;

impl From<AutoName> for ReferenceValue<'static> {
    fn from(_value: AutoName) -> ReferenceValue<'static> {
        ReferenceValue::Named(None)
    }
}

impl From<Option<String>> for ReferenceValue<'static> {
    fn from(value: Option<String>) -> ReferenceValue<'static> {
        ReferenceValue::Named(value.map(Cow::Owned))
    }
}

impl From<String> for ReferenceValue<'static> {
    fn from(value: String) -> ReferenceValue<'static> {
        ReferenceValue::Named(Some(Cow::Owned(value)))
    }
}

impl<'a> From<Option<&'a str>> for ReferenceValue<'a> {
    fn from(value: Option<&'a str>) -> ReferenceValue<'a> {
        ReferenceValue::Named(value.map(Cow::Borrowed))
    }
}

impl<'a> From<&'a str> for ReferenceValue<'a> {
    fn from(value: &'a str) -> ReferenceValue<'a> {
        ReferenceValue::Named(Some(Cow::Borrowed(value)))
    }
}

pub enum ReferenceValue<'a> {
    Named(Option<Cow<'a, str>>),
    Inline(&'a str),
}

fn detect_snapshot_name(function_name: &str, module_path: &str) -> Result<String, &'static str> {
    let name = Cow::Borrowed(function_name);

    // clean test name first
    let mut name = name.rsplit("::").next().unwrap();
    let mut test_prefixed = false;
    if name.starts_with("test_") {
        name = &name[5..];
        test_prefixed = true;
    }

    // next check if we need to add a suffix
    let name = add_suffix_to_snapshot_name(Cow::Borrowed(name));
    let key = format!("{}::{}", module_path.replace("::", "__"), name);

    // because fn foo and fn test_foo end up with the same snapshot name we
    // make sure we detect this here and raise an error.
    let mut name_clash_detection = TEST_NAME_CLASH_DETECTION
        .lock()
        .unwrap_or_else(|x| x.into_inner());
    match name_clash_detection.get(&key) {
        None => {
            name_clash_detection.insert(key.clone(), test_prefixed);
        }
        Some(&was_test_prefixed) => {
            if was_test_prefixed != test_prefixed {
                panic!(
                    "Insta snapshot name clash detected between '{}' \
                     and 'test_{}' in '{}'. Rename one function.",
                    name, name, module_path
                );
            }
        }
    }

    // if the snapshot name clashes we need to increment a counter.
    // we really do not care about poisoning here.
    let mut counters = TEST_NAME_COUNTERS.lock().unwrap_or_else(|x| x.into_inner());
    let test_idx = counters.get(&key).cloned().unwrap_or(0) + 1;
    let rv = if test_idx == 1 {
        name.to_string()
    } else {
        format!("{}-{}", name, test_idx)
    };
    counters.insert(key, test_idx);

    Ok(rv)
}

/// If there is a suffix on the settings, append it to the snapshot name.
fn add_suffix_to_snapshot_name(name: Cow<'_, str>) -> Cow<'_, str> {
    Settings::with(|settings| {
        settings
            .snapshot_suffix()
            .map(|suffix| Cow::Owned(format!("{}@{}", name, suffix)))
            .unwrap_or_else(|| name)
    })
}

fn get_snapshot_filename(
    module_path: &str,
    snapshot_name: &str,
    cargo_workspace: &Path,
    base: &str,
) -> PathBuf {
    let root = Path::new(cargo_workspace);
    let base = Path::new(base);
    Settings::with(|settings| {
        root.join(base.parent().unwrap())
            .join(settings.snapshot_path())
            .join({
                use std::fmt::Write;
                let mut f = String::new();
                if settings.prepend_module_to_snapshot() {
                    write!(&mut f, "{}__", module_path.replace("::", "__")).unwrap();
                }
                write!(
                    &mut f,
                    "{}.snap",
                    snapshot_name.replace("/", "__").replace("\\", "__")
                )
                .unwrap();
                f
            })
    })
}

#[derive(Debug)]
struct SnapshotAssertionContext<'a> {
    cargo_workspace: Arc<PathBuf>,
    module_path: &'a str,
    snapshot_name: Option<Cow<'a, str>>,
    snapshot_file: Option<PathBuf>,
    old_snapshot: Option<Snapshot>,
    pending_snapshots_path: Option<PathBuf>,
    assertion_file: &'a str,
    assertion_line: u32,
}

impl<'a> SnapshotAssertionContext<'a> {
    fn prepare(
        refval: ReferenceValue<'a>,
        manifest_dir: &'a str,
        function_name: &'a str,
        module_path: &'a str,
        assertion_file: &'a str,
        assertion_line: u32,
    ) -> Result<SnapshotAssertionContext<'a>, Box<dyn Error>> {
        let cargo_workspace = get_cargo_workspace(manifest_dir);
        let snapshot_name;
        let mut snapshot_file = None;
        let mut old_snapshot = None;
        let mut pending_snapshots_path = None;

        match refval {
            ReferenceValue::Named(name) => {
                let name = match name {
                    Some(name) => add_suffix_to_snapshot_name(name),
                    None => detect_snapshot_name(function_name, module_path)
                        .unwrap()
                        .into(),
                };
                let file =
                    get_snapshot_filename(module_path, &name, &cargo_workspace, assertion_file);
                if fs::metadata(&file).is_ok() {
                    old_snapshot = Some(Snapshot::from_file(&file)?);
                }
                snapshot_name = Some(name);
                snapshot_file = Some(file);
            }
            ReferenceValue::Inline(contents) => {
                snapshot_name = detect_snapshot_name(function_name, module_path)
                    .ok()
                    .map(Cow::Owned);
                let mut pending_file = cargo_workspace.join(assertion_file);
                pending_file.set_file_name(format!(
                    ".{}.pending-snap",
                    pending_file
                        .file_name()
                        .expect("no filename")
                        .to_str()
                        .expect("non unicode filename")
                ));
                pending_snapshots_path = Some(pending_file);
                old_snapshot = Some(Snapshot::from_components(
                    module_path.replace("::", "__"),
                    None,
                    MetaData::default(),
                    SnapshotContents::from_inline(contents),
                ));
            }
        };

        Ok(SnapshotAssertionContext {
            cargo_workspace,
            module_path,
            snapshot_name,
            snapshot_file,
            old_snapshot,
            pending_snapshots_path,
            assertion_file,
            assertion_line,
        })
    }

    /// Given a path returns the local path within the workspace.
    pub fn localize_path(&self, p: &Path) -> Option<PathBuf> {
        self.cargo_workspace
            .join(p)
            .canonicalize()
            .ok()
            .and_then(|s| {
                s.strip_prefix(self.cargo_workspace.as_path())
                    .ok()
                    .map(|x| x.to_path_buf())
            })
    }

    /// Creates the new snapshot from input values.
    pub fn new_snapshot(&self, contents: SnapshotContents, expr: &str) -> Snapshot {
        Snapshot::from_components(
            self.module_path.replace("::", "__"),
            self.snapshot_name.as_ref().map(|x| x.to_string()),
            MetaData::new(
                self.assertion_file,
                expr,
                Some(self.assertion_line),
                Settings::with(|s| s.input_file().and_then(|x| self.localize_path(x))),
            ),
            contents,
        )
    }

    /// Cleanup logic for passing snapshots.
    pub fn cleanup_passing(&self) -> Result<(), Box<dyn Error>> {
        // let's just make sure there are no more pending files lingering
        // around.
        if let Some(ref snapshot_file) = self.snapshot_file {
            let mut snapshot_file = snapshot_file.clone();
            snapshot_file.set_extension("snap.new");
            fs::remove_file(snapshot_file).ok();
        }

        // and add a null pending snapshot to a pending snapshot file if needed
        if let Some(ref pending_snapshots) = self.pending_snapshots_path {
            if fs::metadata(pending_snapshots).is_ok() {
                PendingInlineSnapshot::new(None, None, self.assertion_line)
                    .save(pending_snapshots)?;
            }
        }
        Ok(())
    }

    /// Writes the changes of the snapshot back.
    pub fn update_snapshot(
        &self,
        new_snapshot: Snapshot,
    ) -> Result<SnapshotUpdate, Box<dyn Error>> {
        let unseen = self
            .snapshot_file
            .as_ref()
            .map_or(false, |x| fs::metadata(x).is_ok());
        let should_print = get_output_behavior() != OutputBehavior::Nothing;
        let snapshot_update = get_snapshot_update_behavior(unseen);

        match snapshot_update {
            SnapshotUpdate::InPlace => {
                if let Some(ref snapshot_file) = self.snapshot_file {
                    new_snapshot.save(snapshot_file)?;
                    if should_print {
                        elog!(
                            "{} {}",
                            if unseen {
                                style("created previously unseen snapshot").green()
                            } else {
                                style("updated snapshot").green()
                            },
                            style(snapshot_file.display()).cyan().underlined(),
                        );
                    }
                } else if should_print {
                    elog!(
                        "{}",
                        style("error: cannot update inline snapshots in-place")
                            .red()
                            .bold(),
                    );
                }
            }
            SnapshotUpdate::NewFile => {
                if let Some(ref snapshot_file) = self.snapshot_file {
                    let mut new_path = snapshot_file.to_path_buf();
                    new_path.set_extension("snap.new");
                    new_snapshot.save_new(&new_path)?;
                    if should_print {
                        elog!(
                            "{} {}",
                            style("stored new snapshot").green(),
                            style(new_path.display()).cyan().underlined(),
                        );
                    }
                } else {
                    PendingInlineSnapshot::new(
                        Some(new_snapshot),
                        self.old_snapshot.clone(),
                        self.assertion_line,
                    )
                    .save(self.pending_snapshots_path.as_ref().unwrap())?;
                }
            }
            SnapshotUpdate::NoUpdate => {}
        }

        Ok(snapshot_update)
    }
}

/// This prints the information about the snapshot
fn print_snapshot_info(ctx: &SnapshotAssertionContext, new_snapshot: &Snapshot) {
    match get_output_behavior() {
        OutputBehavior::Summary => {
            print_snapshot_summary_with_title(
                ctx.cargo_workspace.as_path(),
                new_snapshot,
                ctx.old_snapshot.as_ref(),
                ctx.assertion_line,
                ctx.snapshot_file.as_deref(),
            );
        }
        OutputBehavior::Diff => {
            print_snapshot_diff_with_title(
                ctx.cargo_workspace.as_path(),
                new_snapshot,
                ctx.old_snapshot.as_ref(),
                ctx.assertion_line,
                ctx.snapshot_file.as_deref(),
            );
        }
        _ => {}
    }
}

/// Finalizes the assertion based on the update result.
fn finalize_assertion(ctx: &SnapshotAssertionContext, update_result: SnapshotUpdate) {
    if update_result == SnapshotUpdate::NewFile && get_output_behavior() != OutputBehavior::Nothing
    {
        println!(
            "{hint}",
            hint = style("To update snapshots run `cargo insta review`").dim(),
        );
    }

    if update_result != SnapshotUpdate::InPlace && !force_pass() {
        panic!(
            "snapshot assertion for '{}' failed in line {}",
            ctx.snapshot_name
                .as_ref()
                .map_or("unnamed snapshot", |x| &*x),
            ctx.assertion_line
        );
    }
}

/// This function is invoked from the macros to run the main assertion logic.
///
/// This will create the assertion context, run the main logic to assert
/// on snapshots and write changes to the pending snapshot files.  It will
/// also print the necessary bits of information to the output and fail the
/// assertion with a panic if needed.
#[allow(clippy::too_many_arguments)]
pub fn assert_snapshot(
    refval: ReferenceValue<'_>,
    new_snapshot_value: &str,
    manifest_dir: &str,
    function_name: &str,
    module_path: &str,
    assertion_file: &str,
    assertion_line: u32,
    expr: &str,
) -> Result<(), Box<dyn Error>> {
    let ctx = SnapshotAssertionContext::prepare(
        refval,
        manifest_dir,
        function_name,
        module_path,
        assertion_file,
        assertion_line,
    )?;

    let new_snapshot = ctx.new_snapshot(new_snapshot_value.into(), expr);

    // memoize the snapshot file if requested.
    if let Some(ref snapshot_file) = ctx.snapshot_file {
        memoize_snapshot_file(snapshot_file);
    }

    // pass if the snapshots are missing
    if ctx.old_snapshot.as_ref().map(|x| x.contents()) == Some(new_snapshot.contents()) {
        ctx.cleanup_passing()?;

        if force_update_snapshots() {
            ctx.update_snapshot(new_snapshot)?;
        }
    // otherwise print information and update snapshots.
    } else {
        print_snapshot_info(&ctx, &new_snapshot);
        let update_result = ctx.update_snapshot(new_snapshot)?;
        finalize_assertion(&ctx, update_result);
    }

    Ok(())
}
