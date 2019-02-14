use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;

use chrono::{Local, Utc};
use console::{style, StyledObject};
use difference::{Changeset, Difference};
use failure::Error;
use lazy_static::lazy_static;

use ci_info::is_ci;
use serde::Deserialize;
use serde_json;

use crate::snapshot::{MetaData, PendingInlineSnapshot, Snapshot};

lazy_static! {
    static ref WORKSPACES: Mutex<BTreeMap<String, &'static Path>> = Mutex::new(BTreeMap::new());
    static ref COLOR_MAP: AtomicIsize = {
        let c = if let Ok(c) = env::var("INSTA_COLORS") {
            if c == "dalton" {
                ColorMap::Dalton
            } else {
                ColorMap::Normal
            }
        } else {
            ColorMap::Normal
        };
        AtomicIsize::new(c as isize)
    };
}

#[allow(unused)]
pub enum ColorMap {
    Normal = 0,
    Dalton = 1,
}

#[allow(unused)]
impl ColorMap {
    pub fn success<D>(&self, o: D) -> StyledObject<D> {
        let o = style(o);
        match self {
            ColorMap::Normal => o.green(),
            ColorMap::Dalton => o.blue().bold(),
        }
    }
    pub fn failure<D>(&self, o: D) -> StyledObject<D> {
        let o = style(o);
        match self {
            ColorMap::Normal => o.red(),
            ColorMap::Dalton => o.yellow(),
        }
    }
    pub fn skip<D>(&self, o: D) -> StyledObject<D> {
        let o = style(o);
        match self {
            ColorMap::Normal => o.yellow(),
            ColorMap::Dalton => o.white().bold(),
        }
    }

    #[inline]
    pub fn set_current(c: ColorMap) {
        COLOR_MAP.store(c as isize, Ordering::Relaxed);
    }
    #[inline]
    pub fn current() -> ColorMap {
        match COLOR_MAP.load(Ordering::Relaxed) {
            1 => ColorMap::Dalton,
            _ => ColorMap::Normal,
        }
    }
}

enum UpdateBehavior {
    InPlace,
    NewFile,
    NoUpdate,
}

#[cfg(windows)]
fn path_to_storage<P: AsRef<Path>>(path: P) -> String {
    path.as_ref().to_str().unwrap().replace('\\', "/").into()
}

#[cfg(not(windows))]
fn path_to_storage<P: AsRef<Path>>(path: P) -> String {
    path.as_ref().to_string_lossy().into()
}

fn format_rust_expression(value: &str) -> Cow<'_, str> {
    if let Ok(mut proc) = Command::new("rustfmt")
        .arg("--emit=stdout")
        .arg("--edition=2018")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        {
            let stdin = proc.stdin.as_mut().unwrap();
            stdin.write_all(b"const x:() = ").unwrap();
            stdin.write_all(value.as_bytes()).unwrap();
            stdin.write_all(b";").unwrap();
        }
        if let Ok(output) = proc.wait_with_output() {
            let mut buf = String::new();
            let mut rv = String::new();
            let mut reader = BufReader::new(&output.stdout[..]);
            reader.read_line(&mut buf).unwrap();

            rv.push_str(&buf[14..]);
            loop {
                buf.clear();
                let read = reader.read_line(&mut buf).unwrap();
                if read == 0 {
                    break;
                }
                rv.push_str(&buf);
            }
            rv.truncate(rv.trim_end().len() - 1);
            return Cow::Owned(rv);
        }
    }
    Cow::Borrowed(value)
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
    // we really do not care about locking here.
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
    #[derive(PartialEq)]
    enum Mode {
        Same,
        Add,
        Rem,
    }
    let mut lines = vec![];

    let mut lineno = 1;
    for diff in diffs.iter() {
        match *diff {
            Difference::Same(ref x) => {
                for line in x.split('\n') {
                    lines.push((Mode::Same, lineno, line.trim_end()));
                    lineno += 1;
                }
            }
            Difference::Add(ref x) => {
                for line in x.split('\n') {
                    lines.push((Mode::Add, lineno, line.trim_end()));
                    lineno += 1;
                }
            }
            Difference::Rem(ref x) => {
                for line in x.split('\n') {
                    lines.push((Mode::Rem, lineno, line.trim_end()));
                    lineno += 1;
                }
            }
        }
    }

    let width = console::Term::stdout().size().1 as usize;

    if let Some(expr) = expr {
        println!("{:─^1$}", "", width,);
        println!("{}", style(format_rust_expression(expr)).dim());
    }
    println!(
        "──────┬{:─^1$}",
        "",
        width.saturating_sub(7),
    );
    let cm = ColorMap::current();
    for (i, (mode, lineno, line)) in lines.iter().enumerate() {
        match mode {
            Mode::Add => println!(
                "{:>5} │{}{}",
                style(lineno).dim().bold(),
                cm.success("+"),
                cm.success(line)
            ),
            Mode::Rem => println!(
                "{:>5} │{}{}",
                style(lineno).dim().bold(),
                cm.failure("-"),
                cm.failure(line)
            ),
            Mode::Same => {
                if lines[i.saturating_sub(5)..(i + 5).min(lines.len())]
                    .iter()
                    .any(|x| x.0 != Mode::Same)
                {
                    println!(
                        "{:>5} │ {}",
                        style(lineno).dim().bold(),
                        style(line).dim()
                    );
                }
            }
        }
    }
    println!(
        "──────┴{:─^1$}",
        "",
        width.saturating_sub(7),
    );
}

pub fn get_snapshot_filename(
    module_name: &str,
    snapshot_name: &str,
    cargo_workspace: &Path,
    base: &str,
) -> PathBuf {
    let root = Path::new(cargo_workspace);
    let base = Path::new(base);
    root.join(base.parent().unwrap())
        .join("snapshots")
        .join(format!("{}__{}.snap", module_name, snapshot_name))
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
        old_snapshot.as_ref().map_or("", |x| x.contents()),
        &new.contents(),
        "\n",
    );
    let cm = ColorMap::current();
    if let Some(old_snapshot) = old_snapshot {
        if let Some(ref value) = old_snapshot.metadata().created {
            println!(
                "Old: {}",
                style(value.with_timezone(&Local).to_rfc2822()).cyan()
            );
        }
        if let Some(ref value) = new.metadata().created {
            println!(
                "New: {}",
                style(value.with_timezone(&Local).to_rfc2822()).cyan()
            );
        }
        println!();
        println!("{}", cm.failure("-old snapshot"));
        println!("{}", cm.success("+new results"));
    } else {
        println!("Old: {}", cm.failure("n.a."));
        if let Some(ref value) = new.metadata().created {
            println!(
                "New: {}",
                style(value.with_timezone(&Local).to_rfc2822()).cyan()
            );
        }
        println!();
        println!("{}", cm.success("+new results"));
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

pub enum ReferenceValue<'a> {
    Named(&'a str),
    Inline(&'a str),
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
) -> Result<(), Error> {
    let module_name = module_path.rsplit("::").next().unwrap();
    let cargo_workspace = get_cargo_workspace(manifest_dir);

    let (snapshot_name, snapshot_file, old, pending_snapshots) = match refval {
        ReferenceValue::Named(snapshot_name) => {
            let snapshot_file =
                get_snapshot_filename(module_name, snapshot_name, &cargo_workspace, file);
            let old = if fs::metadata(&snapshot_file).is_ok() {
                Some(Snapshot::from_file(&snapshot_file)?)
            } else {
                None
            };
            (Some(snapshot_name), Some(snapshot_file), old, None)
        }
        ReferenceValue::Inline(contents) => {
            let mut filename = cargo_workspace.join(file);
            let created = fs::metadata(&filename)?.created().ok().map(|x| x.into());
            filename.set_file_name(format!(
                ".{}.pending-snap",
                filename
                    .file_name()
                    .expect("no filename")
                    .to_str()
                    .expect("non unicode filename")
            ));
            (
                None,
                None,
                Some(Snapshot::from_components(
                    module_name.to_string(),
                    None,
                    MetaData {
                        created,
                        ..MetaData::default()
                    },
                    contents.to_string(),
                )),
                Some(filename),
            )
        }
    };

    // if the snapshot matches we're done.
    if old.as_ref().map_or(false, |x| x.contents() == new_snapshot) {
        return Ok(());
    }

    let new = Snapshot::from_components(
        module_name.to_string(),
        snapshot_name.map(|x| x.to_string()),
        MetaData {
            created: Some(Utc::now()),
            creator: Some(format!(
                "{}@{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            )),
            source: Some(path_to_storage(file)),
            expression: Some(expr.to_string()),
        },
        new_snapshot.to_string(),
    );

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

    let cm = ColorMap::current();
    match update_snapshot_behavior() {
        UpdateBehavior::InPlace => {
            if let Some(ref snapshot_file) = snapshot_file {
                new.save(snapshot_file)?;
                eprintln!(
                    "  {} {}\n",
                    cm.success("updated snapshot"),
                    style(snapshot_file.display()).cyan().underlined(),
                );
                return Ok(());
            } else {
                eprintln!(
                    "  {}",
                    cm.failure("error: cannot update inline snapshots in-place")
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
                    cm.success("stored new snapshot"),
                    style(new_path.display()).cyan().underlined(),
                );
            } else {
                PendingInlineSnapshot::new(new, old, line).save(pending_snapshots.unwrap())?;
            }
        }
        UpdateBehavior::NoUpdate => {}
    }

    if should_fail_in_tests() {
        assert!(
            false,
            "snapshot assertion for '{}' failed in line {}",
            snapshot_name.unwrap_or("inline snapshot"),
            line
        );
    }

    Ok(())
}
