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
use serde_json;

use crate::settings::Settings;
use crate::snapshot::{MetaData, PendingInlineSnapshot, Snapshot, SnapshotContents};
use crate::utils::is_ci;

lazy_static! {
    static ref WORKSPACES: Mutex<BTreeMap<String, &'static Path>> = Mutex::new(BTreeMap::new());
    static ref TEST_NAME_COUNTERS: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
}

enum UpdateBehavior {
    InPlace,
    NewFile,
    NoUpdate,
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

fn update_snapshot_behavior() -> UpdateBehavior {
    match env::var("INSTA_UPDATE").ok().as_ref().map(|x| x.as_str()) {
        None | Some("") | Some("auto") => {
            if is_ci() {
                UpdateBehavior::NoUpdate
            } else {
                UpdateBehavior::NewFile
            }
        }
        Some("always") | Some("1") => UpdateBehavior::InPlace,
        Some("new") => UpdateBehavior::NewFile,
        Some("no") => UpdateBehavior::NoUpdate,
        _ => panic!("invalid value for INSTA_UPDATE"),
    }
}

fn force_update_snapshots() -> bool {
    match env::var("INSTA_FORCE_UPDATE_SNAPSHOTS")
        .ok()
        .as_ref()
        .map(|x| x.as_str())
    {
        None | Some("") | Some("0") => false,
        Some("1") => true,
        _ => panic!("invalid value for INSTA_FORCE_UPDATE_SNAPSHOTS"),
    }
}

fn should_fail_in_tests() -> bool {
    match env::var("INSTA_FORCE_PASS")
        .ok()
        .as_ref()
        .map(|x| x.as_str())
    {
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

fn get_cargo_workspace(manifest_dir: &str) -> &Path {
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
            .join(format!("{}__{}.snap", module_path, snapshot_name))
    })
}

/// Prints a diff against an old snapshot.
pub fn print_snapshot_diff(
    workspace_root: &Path,
    new: &Snapshot,
    old_snapshot: Option<&Snapshot>,
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
    if let Some(name) = new.snapshot_name() {
        println!("Snapshot: {}", style(name).yellow());
    } else {
        println!("Snapshot: {}", style("<inline>").dim());
    }

    if let Some(ref value) = new.metadata().get_relative_source(workspace_root) {
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
    print_changeset(
        &changeset,
        new.metadata().expression.as_ref().map(|x| x.as_str()),
    );
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

fn generate_snapshot_name_for_thread(module_path: &str) -> String {
    let thread = thread::current();
    let mut name = thread
        .name()
        .expect("test thread is unnamed, no snapshot name can be generated");
    name = name.rsplit("::").next().unwrap();
    // we really do not care about poisoning here.
    let key = format!("{}::{}", module_path, name);
    let mut counters = TEST_NAME_COUNTERS.lock().unwrap_or_else(|x| x.into_inner());
    let test_idx = counters.get(&key).cloned().unwrap_or(0) + 1;
    if name.starts_with("test_") {
        name = &name[5..];
    }
    let rv = if test_idx == 1 {
        name.to_string()
    } else {
        format!("{}-{}", name, test_idx)
    };
    counters.insert(key, test_idx);
    rv
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
) -> Result<(), Box<dyn Error>> {
    match update_snapshot_behavior() {
        UpdateBehavior::InPlace => {
            if let Some(ref snapshot_file) = snapshot_file {
                new.save(snapshot_file)?;
                eprintln!(
                    "  {} {}\n",
                    style("updated snapshot").green(),
                    style(snapshot_file.display()).cyan().underlined(),
                );
                return Ok(());
            } else {
                eprintln!(
                    "  {}",
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
                eprintln!(
                    "  {} {}\n",
                    style("stored new snapshot").green(),
                    style(new_path.display()).cyan().underlined(),
                );
            } else {
                PendingInlineSnapshot::new(Some(new), old, line)
                    .save(pending_snapshots.unwrap())?;
            }
        }
        UpdateBehavior::NoUpdate => {}
    }

    Ok(())
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
    let module_path = module_path.replace("::", "__");
    let cargo_workspace = get_cargo_workspace(manifest_dir);

    let (snapshot_name, snapshot_file, old, pending_snapshots) = match refval {
        ReferenceValue::Named(snapshot_name) => {
            let snapshot_name: Cow<str> = match snapshot_name {
                Some(snapshot_name) => snapshot_name,
                None => (generate_snapshot_name_for_thread(&module_path).into()),
            };
            let snapshot_file =
                get_snapshot_filename(&module_path, &snapshot_name, &cargo_workspace, file);
            let old = if fs::metadata(&snapshot_file).is_ok() {
                Some(Snapshot::from_file(&snapshot_file)?)
            } else {
                None
            };
            (snapshot_name, Some(snapshot_file), old, None)
        }
        ReferenceValue::Inline(contents) => {
            let snapshot_name = generate_snapshot_name_for_thread(&module_path);
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
                Cow::Owned(snapshot_name),
                None,
                Some(Snapshot::from_components(
                    module_path.to_string(),
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
        module_path.to_string(),
        Some(snapshot_name.to_string()),
        MetaData {
            source: Some(path_to_storage(file)),
            expression: Some(expr.to_string()),
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
                    snapshot_file.as_ref().map(|x| x.as_path()),
                    new,
                    old,
                    line,
                    pending_snapshots,
                )?;
            }

            return Ok(());
        }
    }

    print_snapshot_diff_with_title(
        cargo_workspace,
        &new,
        old.as_ref(),
        line,
        snapshot_file.as_ref().map(|x| x.as_path()),
    );
    println!(
        "{hint}",
        hint = style("To update snapshots run `cargo insta review`").dim(),
    );

    update_snapshots(
        snapshot_file.as_ref().map(|x| x.as_path()),
        new,
        old,
        line,
        pending_snapshots,
    )?;

    if should_fail_in_tests() {
        panic!(
            "snapshot assertion for '{}' failed in line {}",
            snapshot_name, line
        );
    }

    Ok(())
}
