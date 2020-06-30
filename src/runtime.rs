use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str;
use std::sync::Mutex;
use std::thread;

use console::style;
use difference::{Changeset, Difference};
use lazy_static::lazy_static;

use serde::Deserialize;

use crate::settings::Settings;
use crate::snapshot::{MetaData, PendingInlineSnapshot, Snapshot, SnapshotContents};
use crate::utils::is_ci;

lazy_static! {
    static ref WORKSPACES: Mutex<BTreeMap<String, &'static Path>> = Mutex::new(BTreeMap::new());
    static ref TEST_NAME_COUNTERS: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
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

#[cfg(windows)]
fn path_to_storage<P: AsRef<Path>>(path: P) -> String {
    path.as_ref().to_str().unwrap().replace('\\', "/")
}

#[cfg(not(windows))]
fn path_to_storage<P: AsRef<Path>>(path: P) -> String {
    path.as_ref().to_string_lossy().into()
}

fn format_rust_expression(value: &str) -> Cow<'_, str> {
    const PREFIX: &str = "const x:() = ";
    const SUFFIX: &str = ";\n";
    if let Ok(mut proc) = Command::new("rustfmt")
        .arg("--emit=stdout")
        .arg("--edition=2018")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        {
            let stdin = proc.stdin.as_mut().unwrap();
            stdin.write_all(PREFIX.as_bytes()).unwrap();
            stdin.write_all(value.as_bytes()).unwrap();
            stdin.write_all(SUFFIX.as_bytes()).unwrap();
        }
        if let Ok(output) = proc.wait_with_output() {
            if output.status.success() {
                // slice between after the prefix and before the suffix
                // (currently 14 from the start and 2 before the end, respectively)
                let start = PREFIX.len() + 1;
                let end = output.stdout.len() - SUFFIX.len();
                return str::from_utf8(&output.stdout[start..end])
                    .unwrap()
                    .to_owned()
                    .into();
            }
        }
    }
    Cow::Borrowed(value)
}

#[test]
fn test_format_rust_expression() {
    use crate::assert_snapshot;
    assert_snapshot!(format_rust_expression("vec![1,2,3]"), @"vec![1, 2, 3]");
    assert_snapshot!(format_rust_expression("vec![1,2,3].iter()"), @"vec![1, 2, 3].iter()");
    assert_snapshot!(format_rust_expression(r#"    "aoeu""#), @r###""aoeu""###);
    assert_snapshot!(format_rust_expression(r#"  "aoe😄""#), @r###""aoe😄""###);
    assert_snapshot!(format_rust_expression("😄😄😄😄😄"), @"😄😄😄😄😄")
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

fn get_cargo() -> String {
    env::var("CARGO")
        .ok()
        .unwrap_or_else(|| "cargo".to_string())
}

pub fn get_cargo_workspace(manifest_dir: &str) -> &Path {
    // we really do not care about poisoning here.
    let mut workspaces = WORKSPACES.lock().unwrap_or_else(|x| x.into_inner());
    if let Some(rv) = workspaces.get(manifest_dir) {
        rv
    } else {
        #[derive(Deserialize)]
        struct Manifest {
            workspace_root: String,
        }
        let output = std::process::Command::new(get_cargo())
            .arg("metadata")
            .arg("--format-version=1")
            .arg("--no-deps")
            .current_dir(manifest_dir)
            .output()
            .unwrap();
        let manifest: Manifest = serde_json::from_slice(&output.stdout).unwrap();
        let path = Box::leak(Box::new(PathBuf::from(manifest.workspace_root)));
        workspaces.insert(manifest_dir.to_string(), path.as_path());
        workspaces.get(manifest_dir).unwrap()
    }
}

fn print_changeset(changeset: &Changeset, expr: Option<&str>) {
    let Changeset { ref diffs, .. } = *changeset;
    #[derive(PartialEq, Debug)]
    enum Mode {
        Same,
        Add,
        Rem,
    }

    #[derive(PartialEq, Debug)]
    enum Lineno {
        NotPresent,
        Present(usize),
    }

    impl fmt::Display for Lineno {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                Lineno::NotPresent => f.pad(""),
                Lineno::Present(lineno) => fmt::Display::fmt(&lineno, f),
            }
        }
    }

    let mut lines = vec![];

    let mut lineno_a = 1;
    let mut lineno_b = 1;

    for diff in diffs.iter() {
        match *diff {
            Difference::Same(ref x) => {
                for line in x.split('\n') {
                    lines.push((
                        Mode::Same,
                        Lineno::Present(lineno_a),
                        Lineno::Present(lineno_b),
                        line.trim_end(),
                    ));
                    lineno_a += 1;
                    lineno_b += 1;
                }
            }
            Difference::Add(ref x) => {
                for line in x.split('\n') {
                    lines.push((
                        Mode::Add,
                        Lineno::NotPresent,
                        Lineno::Present(lineno_b),
                        line.trim_end(),
                    ));
                    lineno_b += 1;
                }
            }
            Difference::Rem(ref x) => {
                for line in x.split('\n') {
                    lines.push((
                        Mode::Rem,
                        Lineno::Present(lineno_a),
                        Lineno::NotPresent,
                        line.trim_end(),
                    ));
                    lineno_a += 1;
                }
            }
        }
    }

    let width = console::Term::stdout().size().1 as usize;

    if let Some(expr) = expr {
        println!("{:─^1$}", "", width,);
        println!("{}", style(format_rust_expression(expr)));
    }
    println!("────────────┬{:─^1$}", "", width.saturating_sub(13),);
    let mut has_changes = false;
    for (i, (mode, lineno_a, lineno_b, line)) in lines.iter().enumerate() {
        match mode {
            Mode::Add => {
                has_changes = true;
                println!(
                    "{:>5} {:>5} │{}{}",
                    style(lineno_a).dim(),
                    style(lineno_b).dim().bold(),
                    style("+").green(),
                    style(line).green()
                );
            }
            Mode::Rem => {
                has_changes = true;
                println!(
                    "{:>5} {:>5} │{}{}",
                    style(lineno_a).dim(),
                    style(lineno_b).dim().bold(),
                    style("-").red(),
                    style(line).red()
                );
            }
            Mode::Same => {
                if lines[i.saturating_sub(5)..(i + 5).min(lines.len())]
                    .iter()
                    .any(|x| x.0 != Mode::Same)
                {
                    println!(
                        "{:>5} {:>5} │ {}",
                        style(lineno_a).dim(),
                        style(lineno_b).dim().bold(),
                        style(line).dim()
                    );
                }
            }
        }
    }

    if !has_changes {
        println!(
            "{:>5} {:>5} │{}",
            "",
            style("-").dim(),
            style(" snapshots are matching").cyan(),
        );
    }

    println!("────────────┴{:─^1$}", "", width.saturating_sub(13),);
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
            .join(format!(
                "{}__{}.snap",
                module_path.replace("::", "__"),
                snapshot_name.replace("/", "__").replace("\\", "__")
            ))
    })
}

/// Prints the summary of a snapshot
pub fn print_snapshot_summary(
    workspace_root: &Path,
    snapshot: &Snapshot,
    snapshot_file: Option<&Path>,
    line: Option<u32>,
) {
    if let Some(snapshot_file) = snapshot_file {
        let snapshot_file = workspace_root
            .join(snapshot_file)
            .strip_prefix(workspace_root)
            .ok()
            .map(|x| x.to_path_buf())
            .unwrap_or_else(|| snapshot_file.to_path_buf());
        println!(
            "Snapshot file: {}",
            style(snapshot_file.display()).cyan().underlined()
        );
    }
    if let Some(name) = snapshot.snapshot_name() {
        println!("Snapshot: {}", style(name).yellow());
    } else {
        println!("Snapshot: {}", style("<inline>").dim());
    }

    if let Some(ref value) = snapshot.metadata().get_relative_source(workspace_root) {
        println!(
            "Source: {}{}",
            style(value.display()).cyan(),
            if let Some(line) = line {
                format!(":{}", style(line).bold())
            } else {
                "".to_string()
            }
        );
    }

    if let Some(ref value) = snapshot.metadata().input_file() {
        println!("Input file: {}", style(value).cyan());
    }
}

/// Prints a diff against an old snapshot.
pub fn print_snapshot_diff(
    workspace_root: &Path,
    new: &Snapshot,
    old_snapshot: Option<&Snapshot>,
    snapshot_file: Option<&Path>,
    line: Option<u32>,
) {
    print_snapshot_summary(workspace_root, new, snapshot_file, line);
    let changeset = Changeset::new(
        old_snapshot.as_ref().map_or("", |x| x.contents_str()),
        &new.contents_str(),
        "\n",
    );
    if old_snapshot.is_some() {
        println!("{}", style("-old snapshot").red());
        println!("{}", style("+new results").green());
    } else {
        println!("{}", style("+new results").green());
    }
    print_changeset(&changeset, new.metadata().expression.as_deref());
}

fn print_snapshot_diff_with_title(
    workspace_root: &Path,
    new_snapshot: &Snapshot,
    old_snapshot: Option<&Snapshot>,
    line: u32,
    snapshot_file: Option<&Path>,
) {
    let width = console::Term::stdout().size().1 as usize;
    println!(
        "{title:━^width$}",
        title = style(" Snapshot Differences ").bold(),
        width = width
    );
    print_snapshot_diff(
        workspace_root,
        new_snapshot,
        old_snapshot,
        snapshot_file,
        Some(line),
    );
}

fn print_snapshot_summary_with_title(
    workspace_root: &Path,
    new_snapshot: &Snapshot,
    old_snapshot: Option<&Snapshot>,
    line: u32,
    snapshot_file: Option<&Path>,
) {
    let _old_snapshot = old_snapshot;
    let width = console::Term::stdout().size().1 as usize;
    println!(
        "{title:━^width$}",
        title = style(" Snapshot Summary ").bold(),
        width = width
    );
    print_snapshot_summary(workspace_root, new_snapshot, snapshot_file, Some(line));
    println!("{title:━^width$}", title = "", width = width);
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
    if name.starts_with("test_") {
        name = &name[5..];
    }

    // next check if we need to add a suffix
    let name = add_suffix_to_snapshot_name(Cow::Borrowed(name));
    let key = format!("{}::{}", module_path.replace("::", "__"), name);

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

/// Helper function that returns the real inline snapshot value from a given
/// frozen value string.  If the string starts with the '⋮' character
/// (optionally prefixed by whitespace) the alternative serialization format
/// is picked which has slightly improved indentation semantics.
pub(super) fn get_inline_snapshot_value(frozen_value: &str) -> String {
    // TODO: could move this into the SnapshotContents `from_inline` method
    // (the only call site)

    if frozen_value.trim_start().starts_with('⋮') {
        // legacy format - retain so old snapshots still work
        let mut buf = String::new();
        let mut line_iter = frozen_value.lines();
        let mut indentation = 0;

        for line in &mut line_iter {
            let line_trimmed = line.trim_start();
            if line_trimmed.is_empty() {
                continue;
            }
            indentation = line.len() - line_trimmed.len();
            // 3 because '⋮' is three utf-8 bytes long
            buf.push_str(&line_trimmed[3..]);
            buf.push('\n');
            break;
        }

        for line in &mut line_iter {
            if let Some(prefix) = line.get(..indentation) {
                if !prefix.trim().is_empty() {
                    return "".to_string();
                }
            }
            if let Some(remainder) = line.get(indentation..) {
                if remainder.starts_with('⋮') {
                    // 3 because '⋮' is three utf-8 bytes long
                    buf.push_str(&remainder[3..]);
                    buf.push('\n');
                } else if remainder.trim().is_empty() {
                    continue;
                } else {
                    return "".to_string();
                }
            }
        }

        buf.trim_end().to_string()
    } else {
        normalize_inline_snapshot(frozen_value)
    }
}

#[test]
fn test_inline_snapshot_value_newline() {
    // https://github.com/mitsuhiko/insta/issues/39
    assert_eq!(get_inline_snapshot_value("\n"), "");
}

fn count_leading_spaces(value: &str) -> usize {
    value.chars().take_while(|x| x.is_whitespace()).count()
}

fn min_indentation(snapshot: &str) -> usize {
    let lines = snapshot.trim_end().lines();

    if lines.clone().count() <= 1 {
        // not a multi-line string
        return 0;
    }

    lines
        .skip_while(|l| l.is_empty())
        .map(count_leading_spaces)
        .min()
        .unwrap_or(0)
}

#[test]
fn test_min_indentation() {
    let t = r#"
   1
   2
    "#;
    assert_eq!(min_indentation(t), 3);

    let t = r#"
            1
    2"#;
    assert_eq!(min_indentation(t), 4);

    let t = r#"
            1
            2
    "#;
    assert_eq!(min_indentation(t), 12);

    let t = r#"
   1
   2
"#;
    assert_eq!(min_indentation(t), 3);

    let t = r#"
        a 
    "#;
    assert_eq!(min_indentation(t), 8);

    let t = "";
    assert_eq!(min_indentation(t), 0);

    let t = r#"
    a 
    b
c
    "#;
    assert_eq!(min_indentation(t), 0);

    let t = r#"
a 
    "#;
    assert_eq!(min_indentation(t), 0);

    let t = "
    a";
    assert_eq!(min_indentation(t), 4);

    let t = r#"a
  a"#;
    assert_eq!(min_indentation(t), 0);
}

// Removes excess indentation, removes excess whitespace at start & end
fn normalize_inline_snapshot(snapshot: &str) -> String {
    let indentation = min_indentation(snapshot);
    snapshot
        .trim_end()
        .lines()
        .skip_while(|l| l.is_empty())
        .map(|l| &l[indentation..])
        .collect::<Vec<&str>>()
        .join("\n")
}

#[test]
fn test_normalize_inline_snapshot() {
    // here we do exact matching (rather than `assert_snapshot`)
    // to ensure we're not incorporating the modifications this library makes
    let t = r#"
   1
   2
    "#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
1
2"###[1..]
    );

    let t = r#"
            1
    2"#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
        1
2"###[1..]
    );

    let t = r#"
            1
            2
    "#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
1
2"###[1..]
    );

    let t = r#"
   1
   2
"#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
1
2"###[1..]
    );

    let t = r#"
        a 
    "#;
    assert_eq!(normalize_inline_snapshot(t), "a");

    let t = "";
    assert_eq!(normalize_inline_snapshot(t), "");

    let t = r#"
    a 
    b
c
    "#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
    a 
    b
c"###[1..]
    );

    let t = r#"
a 
    "#;
    assert_eq!(normalize_inline_snapshot(t), "a");

    let t = "
    a";
    assert_eq!(normalize_inline_snapshot(t), "a");

    let t = r#"a
  a"#;
    assert_eq!(
        normalize_inline_snapshot(t),
        r###"
a
  a"###[1..]
    );
}

fn update_snapshots(
    snapshot_file: Option<&Path>,
    new: Snapshot,
    old: Option<Snapshot>,
    line: u32,
    pending_snapshots: Option<PathBuf>,
    output_behavior: OutputBehavior,
) -> Result<(), Box<dyn Error>> {
    let unseen = snapshot_file.map_or(false, |x| fs::metadata(x).is_ok());
    let should_print = output_behavior != OutputBehavior::Nothing;

    match update_snapshot_behavior(unseen) {
        UpdateBehavior::InPlace => {
            if let Some(ref snapshot_file) = snapshot_file {
                new.save(snapshot_file)?;
                if should_print {
                    eprintln!(
                        "{} {}",
                        if unseen {
                            style("created previously unseen snapshot").green()
                        } else {
                            style("updated snapshot").green()
                        },
                        style(snapshot_file.display()).cyan().underlined(),
                    );
                }
                return Ok(());
            } else if should_print {
                eprintln!(
                    "{}",
                    style("error: cannot update inline snapshots in-place")
                        .red()
                        .bold(),
                );
            }
        }
        UpdateBehavior::NewFile => {
            if let Some(ref snapshot_file) = snapshot_file {
                let mut new_path = snapshot_file.to_path_buf();
                new_path.set_extension("snap.new");
                new.save(&new_path)?;
                if should_print {
                    eprintln!(
                        "{} {}",
                        style("stored new snapshot").green(),
                        style(new_path.display()).cyan().underlined(),
                    );
                }
            } else {
                PendingInlineSnapshot::new(Some(new), old, line)
                    .save(pending_snapshots.unwrap())?;
            }
        }
        UpdateBehavior::NoUpdate => {}
    }

    Ok(())
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

#[allow(clippy::too_many_arguments)]
pub fn assert_snapshot(
    refval: ReferenceValue<'_>,
    new_snapshot: &str,
    manifest_dir: &str,
    module_path: &str,
    file: &str,
    line: u32,
    expr: &str,
) -> Result<(), Box<dyn Error>> {
    let cargo_workspace = get_cargo_workspace(manifest_dir);
    let output_behavior = output_snapshot_behavior();

    let (snapshot_name, snapshot_file, old, pending_snapshots) = match refval {
        ReferenceValue::Named(snapshot_name) => {
            let snapshot_name = match snapshot_name {
                Some(snapshot_name) => add_suffix_to_snapshot_name(snapshot_name),
                None => generate_snapshot_name_for_thread(module_path)
                    .unwrap()
                    .into(),
            };
            let snapshot_file =
                get_snapshot_filename(module_path, &snapshot_name, &cargo_workspace, file);
            let old = if fs::metadata(&snapshot_file).is_ok() {
                Some(Snapshot::from_file(&snapshot_file)?)
            } else {
                None
            };
            (Some(snapshot_name), Some(snapshot_file), old, None)
        }
        ReferenceValue::Inline(contents) => {
            let snapshot_name = generate_snapshot_name_for_thread(module_path)
                .ok()
                .map(Cow::Owned);
            let mut filename = cargo_workspace.join(file);
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

    let new_snapshot_contents: SnapshotContents = new_snapshot.into();
    let new = Snapshot::from_components(
        module_path.replace("::", "__"),
        snapshot_name.as_ref().map(|x| x.to_string()),
        MetaData {
            source: Some(path_to_storage(file)),
            expression: Some(expr.to_string()),
            input_file: Settings::with(|settings| {
                settings
                    .input_file()
                    .and_then(|x| cargo_workspace.join(x).canonicalize().ok())
                    .and_then(|s| {
                        s.strip_prefix(cargo_workspace)
                            .ok()
                            .map(|x| x.to_path_buf())
                    })
                    .map(path_to_storage)
            }),
        },
        new_snapshot_contents,
    );

    // if the snapshot matches we're done.
    if let Some(ref old_snapshot) = old {
        if old_snapshot.contents() == new.contents() {
            // let's just make sure there are no more pending files lingering
            // around.
            if let Some(ref snapshot_file) = snapshot_file {
                let mut snapshot_file = snapshot_file.clone();
                snapshot_file.set_extension("snap.new");
                fs::remove_file(snapshot_file).ok();
            }
            // and add a null pending snapshot to a pending snapshot file if needed
            if let Some(ref pending_snapshots) = pending_snapshots {
                if fs::metadata(pending_snapshots).is_ok() {
                    PendingInlineSnapshot::new(None, None, line).save(pending_snapshots)?;
                }
            }

            if force_update_snapshots() {
                update_snapshots(
                    snapshot_file.as_deref(),
                    new,
                    old,
                    line,
                    pending_snapshots,
                    output_behavior,
                )?;
            }

            return Ok(());
        }
    }

    match output_behavior {
        OutputBehavior::Summary => {
            print_snapshot_summary_with_title(
                cargo_workspace,
                &new,
                old.as_ref(),
                line,
                snapshot_file.as_deref(),
            );
        }
        OutputBehavior::Diff => {
            print_snapshot_diff_with_title(
                cargo_workspace,
                &new,
                old.as_ref(),
                line,
                snapshot_file.as_deref(),
            );
        }
        _ => {}
    }

    update_snapshots(
        snapshot_file.as_deref(),
        new,
        old,
        line,
        pending_snapshots,
        output_behavior,
    )?;

    if output_behavior != OutputBehavior::Nothing {
        println!(
            "{hint}",
            hint = style("To update snapshots run `cargo insta review`").dim(),
        );
    }

    if should_fail_in_tests() {
        panic!(
            "snapshot assertion for '{}' failed in line {}",
            snapshot_name.as_ref().map_or("unnamed snapshot", |x| &*x),
            line
        );
    }

    Ok(())
}
