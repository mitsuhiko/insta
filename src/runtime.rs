use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::{Arc, Mutex};
use std::thread;

use lazy_static::lazy_static;

use crate::cargo::get_cargo_workspace;
use crate::output::{print_snapshot_diff_with_title, print_snapshot_summary_with_title};
use crate::settings::Settings;
use crate::snapshot::{MetaData, PendingInlineSnapshot, Snapshot, SnapshotContents};
use crate::utils::{is_ci, path_to_storage, style};

lazy_static! {
    static ref WORKSPACES: Mutex<BTreeMap<String, Arc<PathBuf>>> = Mutex::new(BTreeMap::new());
    static ref TEST_NAME_COUNTERS: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
    static ref TEST_NAME_CLASH_DETECTION: Mutex<BTreeMap<String, bool>> =
        Mutex::new(BTreeMap::new());
}

// This macro is basically eprintln but without being captured and
// hidden by the test runner.
macro_rules! elog {
    () => (write!(std::io::stderr()).ok());
    ($($arg:tt)*) => ({
        writeln!(std::io::stderr(), $($arg)*).ok();
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UpdateBehavior {
    InPlace,
    NewFile,
    NoUpdate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputBehavior {
    Diff,
    Summary,
    Minimal,
    Nothing,
}

fn update_snapshot_behavior(unseen: bool) -> UpdateBehavior {
    match env::var("INSTA_UPDATE").ok().as_deref() {
        None | Some("") | Some("auto") => {
            if is_ci() {
                UpdateBehavior::NoUpdate
            } else {
                UpdateBehavior::NewFile
            }
        }
        Some("always") | Some("1") => UpdateBehavior::InPlace,
        Some("new") => UpdateBehavior::NewFile,
        Some("unseen") => {
            if unseen {
                UpdateBehavior::NewFile
            } else {
                UpdateBehavior::InPlace
            }
        }
        Some("no") => UpdateBehavior::NoUpdate,
        _ => panic!("invalid value for INSTA_UPDATE"),
    }
}

fn memoize_snapshot_file(snapshot_file: &Path) {
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

fn output_snapshot_behavior() -> OutputBehavior {
    match env::var("INSTA_OUTPUT").ok().as_deref() {
        None | Some("") | Some("diff") => OutputBehavior::Diff,
        Some("summary") => OutputBehavior::Summary,
        Some("minimal") => OutputBehavior::Minimal,
        Some("none") => OutputBehavior::Nothing,
        _ => panic!("invalid value for INSTA_OUTPUT"),
    }
}

fn force_update_snapshots() -> bool {
    match env::var("INSTA_FORCE_UPDATE_SNAPSHOTS").ok().as_deref() {
        None | Some("") | Some("0") => false,
        Some("1") => true,
        _ => panic!("invalid value for INSTA_FORCE_UPDATE_SNAPSHOTS"),
    }
}

fn should_fail_in_tests() -> bool {
    match env::var("INSTA_FORCE_PASS").ok().as_deref() {
        None | Some("") | Some("0") => true,
        Some("1") => false,
        _ => panic!("invalid value for INSTA_FORCE_PASS"),
    }
}

pub fn get_snapshot_filename(
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

#[cfg(feature = "backtrace")]
fn test_name_from_backtrace(module_path: &str) -> Result<String, &'static str> {
    let backtrace = backtrace::Backtrace::new();
    let frames = backtrace.frames();
    let mut found_run_wrapper = false;

    for symbol in frames
        .iter()
        .rev()
        .flat_map(|x| x.symbols())
        .filter_map(|x| x.name())
        .map(|x| format!("{}", x))
    {
        if !found_run_wrapper {
            if symbol.starts_with("test::run_test::") {
                found_run_wrapper = true;
            }
        } else if symbol.starts_with(module_path) {
            let mut rv = &symbol[..symbol.len() - 19];
            if rv.ends_with("::{{closure}}") {
                rv = &rv[..rv.len() - 13];
            }
            return Ok(rv.to_string());
        }
    }

    Err(
        "Cannot determine test name from backtrace, no snapshot name \
        can be generated. Did you forget to enable debug info?",
    )
}

fn generate_snapshot_name_for_thread(module_path: &str) -> Result<String, &'static str> {
    let thread = thread::current();
    #[allow(unused_mut)]
    let mut name = Cow::Borrowed(
        thread
            .name()
            .ok_or("test thread is unnamed, no snapshot name can be generated.")?,
    );
    if name == "main" {
        #[cfg(feature = "backtrace")]
        {
            name = Cow::Owned(test_name_from_backtrace(module_path)?);
        }
        #[cfg(not(feature = "backtrace"))]
        {
            return Err("tests run with disabled concurrency, automatic snapshot \
                 name generation is not supported.  Consider using the \
                 \"backtrace\" feature of insta which tries to recover test \
                 names from the call stack.");
        }
    }

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

#[derive(Debug, PartialEq)]
enum SnapshotUpdateResult {
    UpdatedInPlace,
    WroteNewFile,
    NoUpdate,
}

fn update_snapshots(
    ctx: &SnapshotAssertionContext,
    new_snapshot: Snapshot,
) -> Result<SnapshotUpdateResult, Box<dyn Error>> {
    let unseen = ctx
        .snapshot_file
        .as_ref()
        .map_or(false, |x| fs::metadata(x).is_ok());
    let should_print = ctx.output_behavior != OutputBehavior::Nothing;

    match update_snapshot_behavior(unseen) {
        UpdateBehavior::InPlace => {
            if let Some(ref snapshot_file) = ctx.snapshot_file {
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
            Ok(SnapshotUpdateResult::UpdatedInPlace)
        }
        UpdateBehavior::NewFile => {
            if let Some(ref snapshot_file) = ctx.snapshot_file {
                let mut new_path = snapshot_file.to_path_buf();
                new_path.set_extension("snap.new");
                new_snapshot.save(&new_path)?;
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
                    ctx.old_snapshot.clone(),
                    ctx.assertion_line,
                )
                .save(ctx.pending_snapshots_path.as_ref().unwrap())?;
            }
            Ok(SnapshotUpdateResult::WroteNewFile)
        }
        UpdateBehavior::NoUpdate => Ok(SnapshotUpdateResult::NoUpdate),
    }
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

#[derive(Debug)]
struct SnapshotAssertionContext<'a> {
    cargo_workspace: Arc<PathBuf>,
    output_behavior: OutputBehavior,
    snapshot_name: Option<Cow<'a, str>>,
    snapshot_file: Option<PathBuf>,
    old_snapshot: Option<Snapshot>,
    pending_snapshots_path: Option<PathBuf>,
    assertion_line: u32,
}

fn prepare_snapshot_assertion<'a>(
    refval: ReferenceValue<'a>,
    manifest_dir: &str,
    module_path: &str,
    assertion_file: &str,
    assertion_line: u32,
) -> Result<SnapshotAssertionContext<'a>, Box<dyn Error>> {
    let cargo_workspace = get_cargo_workspace(manifest_dir);
    let (snapshot_name, snapshot_file, old_snapshot, pending_snapshots_path) = match refval {
        ReferenceValue::Named(snapshot_name) => {
            let snapshot_name = match snapshot_name {
                Some(snapshot_name) => add_suffix_to_snapshot_name(snapshot_name),
                None => generate_snapshot_name_for_thread(module_path)
                    .unwrap()
                    .into(),
            };
            let snapshot_file = get_snapshot_filename(
                module_path,
                &snapshot_name,
                &cargo_workspace,
                assertion_file,
            );
            let old_snapshot = if fs::metadata(&snapshot_file).is_ok() {
                Some(Snapshot::from_file(&snapshot_file)?)
            } else {
                None
            };
            (Some(snapshot_name), Some(snapshot_file), old_snapshot, None)
        }
        ReferenceValue::Inline(contents) => {
            let snapshot_name = generate_snapshot_name_for_thread(module_path)
                .ok()
                .map(Cow::Owned);
            let mut filename = cargo_workspace.join(assertion_file);
            filename.set_file_name(format!(
                ".{}.pending-snap",
                filename
                    .file_name()
                    .expect("no filename")
                    .to_str()
                    .expect("non unicode filename")
            ));
            (
                snapshot_name,
                None,
                Some(Snapshot::from_components(
                    module_path.replace("::", "__"),
                    None,
                    MetaData::default(),
                    SnapshotContents::from_inline(contents),
                )),
                Some(filename),
            )
        }
    };

    Ok(SnapshotAssertionContext {
        cargo_workspace,
        output_behavior: output_snapshot_behavior(),
        snapshot_name,
        snapshot_file,
        old_snapshot,
        pending_snapshots_path,
        assertion_line,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn assert_snapshot(
    refval: ReferenceValue<'_>,
    new_snapshot_value: &str,
    manifest_dir: &str,
    module_path: &str,
    assertion_file: &str,
    assertion_line: u32,
    expr: &str,
) -> Result<(), Box<dyn Error>> {
    let ctx = prepare_snapshot_assertion(
        refval,
        manifest_dir,
        module_path,
        assertion_file,
        assertion_line,
    )?;

    let new_snapshot_contents: SnapshotContents = new_snapshot_value.into();
    let new_snapshot = Snapshot::from_components(
        module_path.replace("::", "__"),
        ctx.snapshot_name.as_ref().map(|x| x.to_string()),
        MetaData {
            source: Some(path_to_storage(assertion_file)),
            expression: Some(expr.to_string()),
            input_file: Settings::with(|settings| {
                settings
                    .input_file()
                    .and_then(|x| ctx.cargo_workspace.join(x).canonicalize().ok())
                    .and_then(|s| {
                        s.strip_prefix(ctx.cargo_workspace.as_path())
                            .ok()
                            .map(|x| x.to_path_buf())
                    })
                    .map(path_to_storage)
            }),
        },
        new_snapshot_contents,
    );

    // memoize the snapshot file if requested.
    if let Some(ref snapshot_file) = ctx.snapshot_file {
        memoize_snapshot_file(snapshot_file);
    }

    // if the snapshot matches we're done.
    if let Some(ref old_snapshot) = ctx.old_snapshot {
        if old_snapshot.contents() == new_snapshot.contents() {
            // let's just make sure there are no more pending files lingering
            // around.
            if let Some(ref snapshot_file) = ctx.snapshot_file {
                let mut snapshot_file = snapshot_file.clone();
                snapshot_file.set_extension("snap.new");
                fs::remove_file(snapshot_file).ok();
            }
            // and add a null pending snapshot to a pending snapshot file if needed
            if let Some(ref pending_snapshots) = ctx.pending_snapshots_path {
                if fs::metadata(pending_snapshots).is_ok() {
                    PendingInlineSnapshot::new(None, None, assertion_line)
                        .save(pending_snapshots)?;
                }
            }

            if force_update_snapshots() {
                update_snapshots(&ctx, new_snapshot)?;
            }

            return Ok(());
        }
    }

    match ctx.output_behavior {
        OutputBehavior::Summary => {
            print_snapshot_summary_with_title(
                ctx.cargo_workspace.as_path(),
                &new_snapshot,
                ctx.old_snapshot.as_ref(),
                assertion_line,
                ctx.snapshot_file.as_deref(),
            );
        }
        OutputBehavior::Diff => {
            print_snapshot_diff_with_title(
                ctx.cargo_workspace.as_path(),
                &new_snapshot,
                ctx.old_snapshot.as_ref(),
                assertion_line,
                ctx.snapshot_file.as_deref(),
            );
        }
        _ => {}
    }

    let update_result = update_snapshots(&ctx, new_snapshot)?;

    if update_result == SnapshotUpdateResult::WroteNewFile
        && ctx.output_behavior != OutputBehavior::Nothing
    {
        println!(
            "{hint}",
            hint = style("To update snapshots run `cargo insta review`").dim(),
        );
    }

    if update_result != SnapshotUpdateResult::UpdatedInPlace && should_fail_in_tests() {
        panic!(
            "snapshot assertion for '{}' failed in line {}",
            ctx.snapshot_name
                .as_ref()
                .map_or("unnamed snapshot", |x| &*x),
            assertion_line
        );
    }

    Ok(())
}
