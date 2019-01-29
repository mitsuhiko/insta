use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use chrono::Utc;
use console::{style, Color};
use difference::{Changeset, Difference};
use failure::Error;
use lazy_static::lazy_static;

use ci_info::is_ci;
use serde::Deserialize;
use serde_json;

lazy_static! {
    static ref WORKSPACES: Mutex<BTreeMap<String, &'static Path>> = Mutex::new(BTreeMap::new());
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
            stdin.write_all(b"fn _x(){").unwrap();
            stdin.write_all(value.as_bytes()).unwrap();
            stdin.write_all(b"}").unwrap();
        }
        if let Ok(output) = proc.wait_with_output() {
            let mut buf = String::new();
            let mut rv = String::new();
            let mut reader = BufReader::new(&output.stdout[..]);
            reader.read_line(&mut buf).unwrap();

            buf.clear();
            reader.read_line(&mut buf).unwrap();

            let indentation = buf.len() - buf.trim_start().len();
            rv.push_str(&buf[indentation..]);
            loop {
                buf.clear();
                let read = reader.read_line(&mut buf).unwrap();
                if read == 0 {
                    break;
                }
                rv.push_str(buf.get(indentation..).unwrap_or(""));
            }
            rv.truncate(rv.trim_end().len());
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
    let mut workspaces = WORKSPACES.lock().unwrap();
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

fn print_changeset_diff(changeset: &Changeset, expr: Option<&str>) {
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
                for line in x.lines() {
                    lines.push((Mode::Same, lineno, line));
                    lineno += 1;
                }
            }
            Difference::Add(ref x) => {
                for line in x.lines() {
                    lines.push((Mode::Add, lineno, line));
                    lineno += 1;
                }
            }
            Difference::Rem(ref x) => {
                for line in x.lines() {
                    lines.push((Mode::Rem, lineno, line));
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
    for (i, (mode, lineno, line)) in lines.iter().enumerate() {
        match mode {
            Mode::Add => println!(
                "{:>5} │{}{}",
                style(lineno).dim().bold(),
                style("+").green(),
                style(line).green()
            ),
            Mode::Rem => println!(
                "{:>5} │{}{}",
                style(lineno).dim().bold(),
                style("-").red(),
                style(line).red()
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
    name: &str,
    cargo_workspace: &Path,
    module_path: &str,
    base: &str,
) -> PathBuf {
    let root = Path::new(cargo_workspace);
    let base = Path::new(base);
    root.join(base.parent().unwrap())
        .join("snapshots")
        .join(format!(
            "{}__{}.snap",
            module_path.rsplit("::").next().unwrap(),
            name
        ))
}

/// A helper to work with stored snapshots.
#[derive(Debug)]
pub struct Snapshot {
    path: PathBuf,
    metadata: BTreeMap<String, String>,
    snapshot: String,
}

impl Snapshot {
    /// Loads a snapshot from a file.
    pub fn from_file<P: AsRef<Path>>(p: P) -> Result<Snapshot, Error> {
        let mut f = BufReader::new(fs::File::open(p.as_ref())?);
        let mut buf = String::new();

        f.read_line(&mut buf)?;

        // yaml format
        let metadata = if buf.trim_end() == "---" {
            loop {
                let read = f.read_line(&mut buf)?;
                if read == 0 {
                    break;
                }
                if buf[buf.len() - read..].trim_end() == "---" {
                    buf.truncate(buf.len() - read);
                    break;
                }
            }
            serde_yaml::from_str(&buf)?
        // legacy format
        } else {
            let mut rv = BTreeMap::new();
            loop {
                buf.clear();
                let read = f.read_line(&mut buf)?;
                if read == 0 || buf.trim_end().is_empty() {
                    buf.truncate(buf.len() - read);
                    break;
                }
                let mut iter = buf.splitn(2, ':');
                if let Some(key) = iter.next() {
                    if let Some(value) = iter.next() {
                        rv.insert(key.to_lowercase(), value.to_string());
                    }
                }
            }
            rv
        };

        buf.clear();
        for (idx, line) in f.lines().enumerate() {
            let line = line?;
            if idx > 0 {
                buf.push('\n');
            }
            buf.push_str(&line);
        }

        Ok(Snapshot {
            path: p.as_ref().to_path_buf(),
            metadata,
            snapshot: buf,
        })
    }

    /// The path of the snapshot
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Relative path to the workspace root.
    pub fn relative_path(&self, root: &Path) -> &Path {
        self.path.strip_prefix(root).ok().unwrap_or(&self.path)
    }

    /// Returns the module name.
    pub fn module_name(&self) -> &str {
        self.path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap_or("")
            .split("__")
            .next()
            .unwrap()
    }

    /// Returns the snapshot name.
    pub fn snapshot_name(&self) -> &str {
        self.path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap_or("")
            .split('.')
            .next()
            .unwrap_or("")
            .splitn(2, "__")
            .nth(1)
            .unwrap_or("unknown")
    }

    /// The metadata in the snapshot.
    pub fn metadata(&self) -> &BTreeMap<String, String> {
        &self.metadata
    }

    /// The snapshot contents
    pub fn contents(&self) -> &str {
        &self.snapshot
    }

    /// Prints a diff against an old snapshot.
    pub fn print_changes(&self, old_snapshot: Option<&Snapshot>) {
        if let Some(value) = self.metadata.get("source") {
            println!("Source: {}", style(value).cyan());
        }
        if let Some(value) = self.metadata.get("created") {
            println!("New: {}", style(value).cyan());
        }
        let changeset = Changeset::new(
            old_snapshot.as_ref().map_or("", |x| x.contents()),
            &self.snapshot,
            "\n",
        );
        if let Some(old_snapshot) = old_snapshot {
            if let Some(value) = old_snapshot.metadata.get("created") {
                println!("Old: {}", style(value).cyan());
            }
            println!();
            println!("{}", style("-old snapshot").red());
            println!("{}", style("+new results").green());
        } else {
            println!("Old: {}", style("n.a.").red());
            println!();
            println!("{}", style("+new results").green());
        }
        print_changeset_diff(
            &changeset,
            self.metadata.get("expression").map(|x| x.as_str()),
        );
    }

    fn save(&self) -> Result<(), Error> {
        self.save_impl(&self.path)
    }

    fn save_new(&self) -> Result<PathBuf, Error> {
        let mut path = self.path.to_path_buf();
        path.set_extension("snap.new");
        self.save_impl(&path)?;
        Ok(path)
    }

    fn save_impl(&self, path: &Path) -> Result<(), Error> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(&folder)?;
        }
        let mut f = fs::File::create(&path)?;
        serde_yaml::to_writer(&mut f, &self.metadata)?;
        f.write_all(b"\n---\n")?;
        f.write_all(self.snapshot.as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
    }
}

fn print_snapshot_diff(
    cargo_workspace: &Path,
    name: &str,
    old_snapshot: Option<&Snapshot>,
    new_snapshot: &Snapshot,
) {
    let width = console::Term::stdout().size().1 as usize;

    let file = style(new_snapshot.relative_path(&cargo_workspace).display())
        .underlined()
        .fg(if fs::metadata(&new_snapshot.path).is_ok() {
            Color::Cyan
        } else {
            Color::Red
        });

    println!(
        "{title:━^width$}\nFile: {file}\nSnapshot: {name}",
        name = style(name).yellow(),
        file = file,
        title = style(" Snapshot Differences ").bold(),
        width = width
    );

    new_snapshot.print_changes(old_snapshot);
}

pub fn assert_snapshot(
    name: &str,
    new_snapshot: &str,
    manifest_dir: &str,
    module_path: &str,
    file: &str,
    line: u32,
    expr: &str,
) -> Result<(), Error> {
    let cargo_workspace = get_cargo_workspace(manifest_dir);
    let snapshot_file = get_snapshot_filename(name, &cargo_workspace, module_path, file);
    let old = Snapshot::from_file(&snapshot_file).ok();

    // if the snapshot matches we're done.
    if old.as_ref().map_or(false, |x| x.snapshot == new_snapshot) {
        return Ok(());
    }

    let mut metadata = BTreeMap::new();
    metadata.insert("created".to_string(), Utc::now().to_rfc3339());
    metadata.insert(
        "creator".to_string(),
        format!("{}@{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
    );
    metadata.insert("source".to_string(), path_to_storage(file));
    metadata.insert("expression".to_string(), expr.to_string());
    let new = Snapshot {
        path: snapshot_file.to_path_buf(),
        metadata,
        snapshot: new_snapshot.to_string(),
    };

    print_snapshot_diff(cargo_workspace, name, old.as_ref(), &new);
    println!(
        "{hint}",
        hint = style("To update snapshots re-run the tests with INSTA_UPDATE=yes or use `cargo insta review`").dim(),
    );

    match update_snapshot_behavior() {
        UpdateBehavior::InPlace => {
            new.save()?;
            eprintln!(
                "  {} {}\n",
                style("updated snapshot").green(),
                style(snapshot_file.display()).cyan().underlined(),
            );
            return Ok(());
        }
        UpdateBehavior::NewFile => {
            let new_path = new.save_new()?;
            eprintln!(
                "  {} {}\n",
                style("stored new snapshot").green(),
                style(new_path.display()).cyan().underlined(),
            );
        }
        UpdateBehavior::NoUpdate => {}
    }

    if should_fail_in_tests() {
        assert!(
            false,
            "snapshot assertion for '{}' failed in line {}",
            name, line
        );
    }

    Ok(())
}
