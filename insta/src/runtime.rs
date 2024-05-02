use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::{Arc, Mutex};

use crate::env::{
    get_cargo_workspace, get_tool_config, memoize_snapshot_file, snapshot_update_behavior,
    OutputBehavior, SnapshotUpdateBehavior, ToolConfig,
};
use crate::output::SnapshotPrinter;
use crate::settings::Settings;
use crate::snapshot::{MetaData, PendingInlineSnapshot, Snapshot, SnapshotContents};
use crate::utils::{path_to_storage, style};

lazy_static::lazy_static! {
    static ref TEST_NAME_COUNTERS: Mutex<BTreeMap<String, usize>> =
        Mutex::new(BTreeMap::new());
    static ref TEST_NAME_CLASH_DETECTION: Mutex<BTreeMap<String, bool>> =
        Mutex::new(BTreeMap::new());
    static ref INLINE_DUPLICATES: Mutex<BTreeSet<String>> =
        Mutex::new(BTreeSet::new());
}

thread_local! {
    static RECORDED_DUPLICATES: RefCell<Vec<BTreeMap<String, Snapshot>>> = RefCell::default()
}

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

fn is_doctest(function_name: &str) -> bool {
    function_name.starts_with("rust_out::main::_doctest")
}

fn detect_snapshot_name(
    function_name: &str,
    module_path: &str,
    inline: bool,
    is_doctest: bool,
) -> Result<String, &'static str> {
    let mut name = function_name;

    // simplify doctest names
    if is_doctest && !inline {
        panic!("Cannot determine reliable names for snapshot in doctests.  Please use explicit names instead.");
    }

    // clean test name first
    name = name.rsplit("::").next().unwrap();
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

    // The rest of the code just deals with duplicates, which we in some
    // cases do not want to guard against.
    if allow_duplicates() {
        return Ok(name.to_string());
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
    assertion_file: &str,
    snapshot_name: &str,
    cargo_workspace: &Path,
    base: &str,
    is_doctest: bool,
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
                    if is_doctest {
                        write!(
                            &mut f,
                            "doctest_{}__",
                            Path::new(assertion_file)
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .replace('.', "_")
                        )
                        .unwrap();
                    } else {
                        write!(&mut f, "{}__", module_path.replace("::", "__")).unwrap();
                    }
                }
                write!(
                    &mut f,
                    "{}.snap",
                    snapshot_name.replace(&['/', '\\'][..], "__")
                )
                .unwrap();
                f
            })
    })
}

#[derive(Debug)]
struct SnapshotAssertionContext<'a> {
    tool_config: Arc<ToolConfig>,
    cargo_workspace: Arc<PathBuf>,
    module_path: &'a str,
    snapshot_name: Option<Cow<'a, str>>,
    snapshot_file: Option<PathBuf>,
    duplication_key: Option<String>,
    old_snapshot: Option<Snapshot>,
    pending_snapshots_path: Option<PathBuf>,
    assertion_file: &'a str,
    assertion_line: u32,
    is_doctest: bool,
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
        let tool_config = get_tool_config(manifest_dir);
        let cargo_workspace = get_cargo_workspace(manifest_dir);
        let snapshot_name;
        let mut duplication_key = None;
        let mut snapshot_file = None;
        let mut old_snapshot = None;
        let mut pending_snapshots_path = None;
        let is_doctest = is_doctest(function_name);

        match refval {
            ReferenceValue::Named(name) => {
                let name = match name {
                    Some(name) => add_suffix_to_snapshot_name(name),
                    None => detect_snapshot_name(function_name, module_path, false, is_doctest)
                        .unwrap()
                        .into(),
                };
                if allow_duplicates() {
                    duplication_key = Some(format!("named:{}|{}", module_path, name));
                }
                let file = get_snapshot_filename(
                    module_path,
                    assertion_file,
                    &name,
                    &cargo_workspace,
                    assertion_file,
                    is_doctest,
                );
                if fs::metadata(&file).is_ok() {
                    old_snapshot = Some(Snapshot::from_file(&file)?);
                }
                snapshot_name = Some(name);
                snapshot_file = Some(file);
            }
            ReferenceValue::Inline(contents) => {
                if allow_duplicates() {
                    duplication_key = Some(format!(
                        "inline:{}|{}|{}",
                        function_name, assertion_file, assertion_line
                    ));
                } else {
                    prevent_inline_duplicate(function_name, assertion_file, assertion_line);
                }
                snapshot_name = detect_snapshot_name(function_name, module_path, true, is_doctest)
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
            tool_config,
            cargo_workspace,
            module_path,
            snapshot_name,
            snapshot_file,
            old_snapshot,
            pending_snapshots_path,
            assertion_file,
            assertion_line,
            duplication_key,
            is_doctest,
        })
    }

    /// Given a path returns the local path within the workspace.
    pub fn localize_path(&self, p: &Path) -> Option<PathBuf> {
        let workspace = self.cargo_workspace.canonicalize().ok()?;
        let p = self.cargo_workspace.join(p).canonicalize().ok()?;
        p.strip_prefix(&workspace).ok().map(|x| x.to_path_buf())
    }

    /// Creates the new snapshot from input values.
    pub fn new_snapshot(&self, contents: SnapshotContents, expr: &str) -> Snapshot {
        Snapshot::from_components(
            self.module_path.replace("::", "__"),
            self.snapshot_name.as_ref().map(|x| x.to_string()),
            Settings::with(|settings| MetaData {
                source: Some(path_to_storage(Path::new(self.assertion_file))),
                assertion_line: Some(self.assertion_line),
                description: settings.description().map(Into::into),
                expression: if settings.omit_expression() {
                    None
                } else {
                    Some(expr.to_string())
                },
                info: settings.info().map(ToOwned::to_owned),
                input_file: settings
                    .input_file()
                    .and_then(|x| self.localize_path(x))
                    .map(|x| path_to_storage(&x)),
            }),
            contents,
        )
    }

    /// Cleanup logic for passing snapshots.
    pub fn cleanup_passing(&self) -> Result<(), Box<dyn Error>> {
        // let's just make sure there are no more pending files lingering
        // around.
        if let Some(ref snapshot_file) = self.snapshot_file {
            let snapshot_file = snapshot_file.clone().with_extension("snap.new");
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
    ) -> Result<SnapshotUpdateBehavior, Box<dyn Error>> {
        let unseen = self
            .snapshot_file
            .as_ref()
            .map_or(false, |x| fs::metadata(x).is_ok());
        let should_print = self.tool_config.output_behavior() != OutputBehavior::Nothing;
        let snapshot_update = snapshot_update_behavior(&self.tool_config, unseen);

        match snapshot_update {
            SnapshotUpdateBehavior::InPlace => {
                if let Some(ref snapshot_file) = self.snapshot_file {
                    let saved = new_snapshot.save(snapshot_file)?;
                    if should_print && saved {
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
                        style(
                            "error: cannot update inline snapshots in-place \
                        (https://github.com/mitsuhiko/insta/issues/272)"
                        )
                        .red()
                        .bold(),
                    );
                }
            }
            SnapshotUpdateBehavior::NewFile => {
                if let Some(ref snapshot_file) = self.snapshot_file {
                    if let Some(new_path) = new_snapshot.save_new(snapshot_file)? {
                        if should_print {
                            elog!(
                                "{} {}",
                                style("stored new snapshot").green(),
                                style(new_path.display()).cyan().underlined(),
                            );
                        }
                    }
                } else if self.is_doctest {
                    if should_print {
                        elog!(
                            "{}",
                            style("warning: cannot update inline snapshots in doctests")
                                .red()
                                .bold(),
                        );
                    }

                // special case for pending inline snapshots.  Here we really only want
                // to write the contents if the snapshot contents changed as the metadata
                // is not retained for inline snapshots.  This used to have different
                // behavior in the past where we did indeed want to rewrite the snapshots
                // entirely since we used to change the canonical snapshot format, but now
                // this is significantly less likely to happen and seeing hundreds of unchanged
                // inline snapshots in the review screen is not a lot of fun.
                } else if self
                    .old_snapshot
                    .as_ref()
                    .map_or(true, |x| x.contents() != new_snapshot.contents())
                {
                    PendingInlineSnapshot::new(
                        Some(new_snapshot),
                        self.old_snapshot.clone(),
                        self.assertion_line,
                    )
                    .save(self.pending_snapshots_path.as_ref().unwrap())?;
                }
            }
            SnapshotUpdateBehavior::NoUpdate => {}
        }

        Ok(snapshot_update)
    }
}

fn prevent_inline_duplicate(function_name: &str, assertion_file: &str, assertion_line: u32) {
    let key = format!("{}|{}|{}", function_name, assertion_file, assertion_line);
    let mut set = INLINE_DUPLICATES.lock().unwrap();
    if set.contains(&key) {
        // drop the lock so we don't poison it
        drop(set);
        panic!(
            "Insta does not allow inline snapshot assertions in loops. \
            Wrap your assertions in allow_duplicates! to change this."
        );
    }
    set.insert(key);
}

/// This prints the information about the snapshot
fn print_snapshot_info(ctx: &SnapshotAssertionContext, new_snapshot: &Snapshot) {
    let mut printer = SnapshotPrinter::new(
        ctx.cargo_workspace.as_path(),
        ctx.old_snapshot.as_ref(),
        new_snapshot,
    );
    printer.set_line(Some(ctx.assertion_line));
    printer.set_snapshot_file(ctx.snapshot_file.as_deref());
    printer.set_title(Some("Snapshot Summary"));
    printer.set_show_info(true);
    match ctx.tool_config.output_behavior() {
        OutputBehavior::Summary => {
            printer.print();
        }
        OutputBehavior::Diff => {
            printer.set_show_diff(true);
            printer.print();
        }
        _ => {}
    }
}

#[cfg(feature = "glob")]
macro_rules! print_or_panic {
    ($fail_fast:expr, $($tokens:tt)*) => {{
        if (!$fail_fast) {
            eprintln!($($tokens)*);
            eprintln!();
        } else {
            panic!($($tokens)*);
        }
    }}
}

/// Finalizes the assertion based on the update result.
fn finalize_assertion(ctx: &SnapshotAssertionContext, update_result: SnapshotUpdateBehavior) {
    // if we are in glob mode, we want to adjust the finalization
    // so that we do not show the hints immediately.
    let fail_fast = {
        #[cfg(feature = "glob")]
        {
            if let Some(top) = crate::glob::GLOB_STACK.lock().unwrap().last() {
                top.fail_fast
            } else {
                true
            }
        }
        #[cfg(not(feature = "glob"))]
        {
            true
        }
    };

    if fail_fast
        && update_result == SnapshotUpdateBehavior::NewFile
        && ctx.tool_config.output_behavior() != OutputBehavior::Nothing
        && !ctx.is_doctest
    {
        println!(
            "{hint}",
            hint = style("To update snapshots run `cargo insta review`").dim(),
        );
    }

    if update_result != SnapshotUpdateBehavior::InPlace && !ctx.tool_config.force_pass() {
        if fail_fast && ctx.tool_config.output_behavior() != OutputBehavior::Nothing {
            println!(
                "{hint}",
                hint = style(
                    "Stopped on the first failure. Run `cargo insta test` to run all snapshots."
                )
                .dim(),
            );
        }

        // if we are in glob mode, count the failures and print the
        // errors instead of panicking.  The glob will then panic at
        // the end.
        #[cfg(feature = "glob")]
        {
            let mut stack = crate::glob::GLOB_STACK.lock().unwrap();
            if let Some(glob_collector) = stack.last_mut() {
                glob_collector.failed += 1;
                if update_result == SnapshotUpdateBehavior::NewFile
                    && ctx.tool_config.output_behavior() != OutputBehavior::Nothing
                {
                    glob_collector.show_insta_hint = true;
                }

                print_or_panic!(
                    fail_fast,
                    "snapshot assertion from glob for '{}' failed in line {}",
                    ctx.snapshot_name.as_deref().unwrap_or("unnamed snapshot"),
                    ctx.assertion_line
                );
                return;
            }
        }

        panic!(
            "snapshot assertion for '{}' failed in line {}",
            ctx.snapshot_name.as_deref().unwrap_or("unnamed snapshot"),
            ctx.assertion_line
        );
    }
}

fn record_snapshot_duplicate(
    results: &mut BTreeMap<String, Snapshot>,
    snapshot: &Snapshot,
    ctx: &SnapshotAssertionContext,
) {
    let key = ctx.duplication_key.as_deref().unwrap();
    if let Some(prev_snapshot) = results.get(key) {
        if prev_snapshot.contents() != snapshot.contents() {
            println!("Snapshots in allow-duplicates block do not match.");
            let mut printer =
                SnapshotPrinter::new(ctx.cargo_workspace.as_path(), Some(prev_snapshot), snapshot);
            printer.set_line(Some(ctx.assertion_line));
            printer.set_snapshot_file(ctx.snapshot_file.as_deref());
            printer.set_title(Some("Differences in Block"));
            printer.set_snapshot_hints("previous assertion", "current assertion");
            if ctx.tool_config.output_behavior() == OutputBehavior::Diff {
                printer.set_show_diff(true);
            }
            printer.print();
            panic!(
                "snapshot assertion for '{}' failed in line {}. Result \
                    does not match previous snapshot in allow-duplicates block.",
                ctx.snapshot_name.as_deref().unwrap_or("unnamed snapshot"),
                ctx.assertion_line
            );
        }
    } else {
        results.insert(key.to_string(), snapshot.clone());
    }
}

/// Do we allow recording of duplicates?
fn allow_duplicates() -> bool {
    RECORDED_DUPLICATES.with(|x| !x.borrow().is_empty())
}

/// Helper function to support perfect duplicate detection.
pub fn with_allow_duplicates<R, F>(f: F) -> R
where
    F: FnOnce() -> R,
{
    RECORDED_DUPLICATES.with(|x| x.borrow_mut().push(BTreeMap::new()));
    let rv = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    RECORDED_DUPLICATES.with(|x| x.borrow_mut().pop().unwrap());
    match rv {
        Ok(rv) => rv,
        Err(payload) => std::panic::resume_unwind(payload),
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
    let tool_config = get_tool_config(manifest_dir);

    // apply filters if they are available
    #[cfg(feature = "filters")]
    let new_snapshot_value =
        Settings::with(|settings| settings.filters().apply_to(new_snapshot_value));

    let new_snapshot = ctx.new_snapshot(new_snapshot_value.into(), expr);

    // memoize the snapshot file if requested.
    if let Some(ref snapshot_file) = ctx.snapshot_file {
        memoize_snapshot_file(snapshot_file);
    }

    // If we allow assertion with duplicates, we record the duplicate now.  This will
    // in itself fail the assertion if the previous visit of the same assertion macro
    // did not yield the same result.
    RECORDED_DUPLICATES.with(|x| {
        if let Some(results) = x.borrow_mut().last_mut() {
            record_snapshot_duplicate(results, &new_snapshot, &ctx);
        }
    });

    let pass = ctx
        .old_snapshot
        .as_ref()
        .map(|x| {
            if tool_config.require_full_match() {
                x.matches_fully(&new_snapshot)
            } else {
                x.matches(&new_snapshot)
            }
        })
        .unwrap_or(false);

    if pass {
        ctx.cleanup_passing()?;

        if tool_config.force_update_snapshots() {
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

/// Test snapshots in doctests.
///
/// ```
/// // this is only working on newer rust versions
/// extern crate rustc_version;
/// use rustc_version::{Version, version};
/// if version().unwrap() > Version::parse("1.72.0").unwrap() {
///     insta::assert_debug_snapshot!("named", vec![1, 2, 3, 4, 5]);
/// }
/// ```
///
/// ```should_panic
/// insta::assert_debug_snapshot!(vec![1, 2, 3, 4, 5]);
/// ```
///
/// ```
/// let some_string = "Coucou je suis un joli bug";
/// insta::assert_snapshot!(some_string, @"Coucou je suis un joli bug");
/// ```
///
/// ```
/// let some_string = "Coucou je suis un joli bug";
/// insta::assert_snapshot!(some_string, @"Coucou je suis un joli bug");
/// ```
const _DOCTEST1: bool = false;
