use std::borrow::{Borrow, Cow};
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::{env, fs};
use std::{io, process};

use console::{set_colors_enabled, style, Key, Term};
use insta::Snapshot;
use insta::_cargo_insta_support::{
    is_ci, SnapshotPrinter, SnapshotUpdate, TestRunner, ToolConfig, UnreferencedSnapshots,
};
use serde::Serialize;
use structopt::clap::AppSettings;
use structopt::StructOpt;
use uuid::Uuid;

use crate::cargo::{find_snapshot_roots, get_metadata, Metadata, Package};
use crate::container::{Operation, SnapshotContainer};
use crate::utils::{err_msg, QuietExit};
use crate::walk::{find_snapshots, make_deletion_walker, make_snapshot_walker, FindFlags};

/// A helper utility to work with insta snapshots.
#[derive(StructOpt, Debug)]
#[structopt(
    bin_name = "cargo insta",
    setting = AppSettings::ArgRequiredElseHelp,
    global_setting = AppSettings::ColorNever,
    global_setting = AppSettings::UnifiedHelpMessage,
    global_setting = AppSettings::DeriveDisplayOrder,
    global_setting = AppSettings::DontCollapseArgsInUsage
)]
struct Opts {
    /// Coloring
    #[structopt(long, global = true, value_name = "WHEN", possible_values=&["auto", "always", "never"])]
    color: Option<String>,

    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt, Debug)]
#[structopt(
    bin_name = "cargo insta",
    after_help = "For the online documentation of the latest version, see https://insta.rs/docs/cli/."
)]
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Interactively review snapshots
    #[structopt(name = "review", alias = "verify")]
    Review(ProcessCommand),
    /// Rejects all snapshots
    #[structopt(name = "reject")]
    Reject(ProcessCommand),
    /// Accept all snapshots
    #[structopt(name = "accept", alias = "approve")]
    Accept(ProcessCommand),
    /// Run tests and then reviews
    #[structopt(name = "test")]
    Test(TestCommand),
    /// Print a summary of all pending snapshots.
    #[structopt(name = "pending-snapshots")]
    PendingSnapshots(PendingSnapshotsCommand),
    /// Shows a specific snapshot
    #[structopt(name = "show")]
    Show(ShowCommand),
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(rename_all = "kebab-case")]
struct TargetArgs {
    /// Path to Cargo.toml
    #[structopt(long, value_name = "PATH", parse(from_os_str))]
    manifest_path: Option<PathBuf>,
    /// Explicit path to the workspace root
    #[structopt(long, value_name = "PATH", parse(from_os_str))]
    workspace_root: Option<PathBuf>,
    /// Sets the extensions to consider.  Defaults to `.snap`
    #[structopt(short = "e", long, value_name = "EXTENSIONS", multiple = true)]
    extensions: Vec<String>,
    /// Work on all packages in the workspace
    #[structopt(long)]
    workspace: bool,
    /// Alias for --workspace (deprecated)
    #[structopt(long)]
    all: bool,
    /// Also walk into ignored paths.
    #[structopt(long, alias = "no-ignore")]
    include_ignored: bool,
    /// Also include hidden paths.
    #[structopt(long)]
    include_hidden: bool,
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
struct ProcessCommand {
    #[structopt(flatten)]
    target_args: TargetArgs,
    /// Limits the operation to one or more snapshots.
    #[structopt(long = "snapshot")]
    snapshot_filter: Option<Vec<String>>,
    /// Do not print to stdout.
    #[structopt(short = "q", long)]
    quiet: bool,
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
struct TestCommand {
    #[structopt(flatten)]
    target_args: TargetArgs,
    /// Test only this package's library unit tests
    #[structopt(long)]
    lib: bool,
    /// Test only the specified binary
    #[structopt(long)]
    bin: Option<String>,
    /// Test all binaries
    #[structopt(long)]
    bins: bool,
    /// Test only the specified example
    #[structopt(long)]
    example: Option<String>,
    /// Test all examples
    #[structopt(long)]
    examples: bool,
    /// Test only the specified test targets
    #[structopt(long)]
    test: Vec<String>,
    /// Test all tests
    #[structopt(long)]
    tests: bool,
    /// Package to run tests for
    #[structopt(short = "p", long)]
    package: Vec<String>,
    /// Exclude packages from the test
    #[structopt(long, value_name = "SPEC")]
    exclude: Option<String>,
    /// Disable force-passing of snapshot tests
    #[structopt(long)]
    no_force_pass: bool,
    /// Prevent running all tests regardless of failure
    #[structopt(long)]
    fail_fast: bool,
    /// Space-separated list of features to activate
    #[structopt(long, value_name = "FEATURES")]
    features: Option<String>,
    /// Number of parallel jobs, defaults to # of CPUs
    #[structopt(short = "j", long)]
    jobs: Option<usize>,
    /// Build artifacts in release mode, with optimizations
    #[structopt(long)]
    release: bool,
    /// Build artifacts with the specified profile
    #[structopt(long)]
    profile: Option<String>,
    /// Test all targets (does not include doctests)
    #[structopt(long)]
    all_targets: bool,
    /// Activate all available features
    #[structopt(long)]
    all_features: bool,
    /// Do not activate the `default` feature
    #[structopt(long)]
    no_default_features: bool,
    /// Build for the target triple
    #[structopt(long)]
    target: Option<String>,
    /// Follow up with review.
    #[structopt(long)]
    review: bool,
    /// Accept all snapshots after test.
    #[structopt(long, conflicts_with = "review")]
    accept: bool,
    /// Accept all new (previously unseen).
    #[structopt(long)]
    accept_unseen: bool,
    /// Instructs the test command to just assert.
    #[structopt(long)]
    check: bool,
    /// Do not reject pending snapshots before run.
    #[structopt(long)]
    keep_pending: bool,
    /// Update all snapshots even if they are still matching.
    #[structopt(long)]
    force_update_snapshots: bool,
    /// Require metadata as well as snapshots' contents to match (experimental).
    #[structopt(long)]
    require_full_match: bool,
    /// Handle unreferenced snapshots after a successful test run.
    #[structopt(long, default_value="ignore", possible_values=&["ignore", "warn", "reject", "delete", "auto"])]
    unreferenced: String,
    /// Delete unreferenced snapshots after a successful test run.
    #[structopt(long, hidden = true)]
    delete_unreferenced_snapshots: bool,
    /// Filters to apply to the insta glob feature.
    #[structopt(long)]
    glob_filter: Vec<String>,
    /// Do not pass the quiet flag (`-q`) to tests.
    #[structopt(short = "Q", long)]
    no_quiet: bool,
    /// Picks the test runner.
    #[structopt(long, default_value="auto", possible_values=&["auto", "cargo-test", "nextest"])]
    test_runner: String,
    /// Options passed to cargo test
    // Sets raw to true so that `--` is required
    #[structopt(name = "CARGO_TEST_ARGS", raw(true))]
    cargo_options: Vec<String>,
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
struct PendingSnapshotsCommand {
    #[structopt(flatten)]
    target_args: TargetArgs,
    /// Changes the output from human readable to JSON.
    #[structopt(long)]
    as_json: bool,
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
struct ShowCommand {
    #[structopt(flatten)]
    target_args: TargetArgs,
    /// The path to the snapshot file.
    path: PathBuf,
}

#[allow(clippy::too_many_arguments)]
fn query_snapshot(
    workspace_root: &Path,
    term: &Term,
    new: &Snapshot,
    old: Option<&Snapshot>,
    pkg: Option<&Package>,
    line: Option<u32>,
    i: usize,
    n: usize,
    snapshot_file: Option<&Path>,
    show_info: &mut bool,
    show_diff: &mut bool,
) -> Result<Operation, Box<dyn Error>> {
    loop {
        term.clear_screen()?;
        let (pkg_name, pkg_version): (_, &dyn Display) = if let Some(pkg) = pkg {
            (pkg.name.as_str(), &pkg.version)
        } else {
            ("unknown package", &"unknown version")
        };

        println!(
            "{}{}{} {}@{}:",
            style("Reviewing [").bold(),
            style(format!("{}/{}", i, n)).yellow().bold(),
            style("]").bold(),
            pkg_name,
            pkg_version,
        );

        let mut printer = SnapshotPrinter::new(workspace_root, old, new);
        printer.set_snapshot_file(snapshot_file);
        printer.set_line(line);
        printer.set_show_info(*show_info);
        printer.set_show_diff(*show_diff);
        printer.print();

        println!();
        println!(
            "  {} accept     {}",
            style("a").green().bold(),
            style("keep the new snapshot").dim()
        );

        if old.is_some() {
            println!(
                "  {} reject     {}",
                style("r").red().bold(),
                style("retain the old snapshot").dim()
            );
        } else {
            println!(
                "  {} reject     {}",
                style("r").red().bold(),
                style("reject the new snapshot").dim()
            );
        }

        println!(
            "  {} skip       {}",
            style("s").yellow().bold(),
            style("keep both for now").dim()
        );
        println!(
            "  {} {} info  {}",
            style("i").cyan().bold(),
            if *show_info { "hide" } else { "show" },
            style("toggles extended snapshot info").dim()
        );
        println!(
            "  {} {} diff  {}",
            style("d").cyan().bold(),
            if *show_diff { "hide" } else { "show" },
            style("toggle snapshot diff").dim()
        );

        loop {
            match term.read_key()? {
                Key::Char('a') | Key::Enter => return Ok(Operation::Accept),
                Key::Char('r') | Key::Escape => return Ok(Operation::Reject),
                Key::Char('s') | Key::Char(' ') => return Ok(Operation::Skip),
                Key::Char('i') => {
                    *show_info = !*show_info;
                    break;
                }
                Key::Char('d') => {
                    *show_diff = !*show_diff;
                    break;
                }
                _ => {}
            }
        }
    }
}

fn handle_color(color: Option<&str>) -> Result<&'static str, Box<dyn Error>> {
    match &*color
        .map(Cow::Borrowed)
        .or_else(|| std::env::var("CARGO_TERM_COLOR").ok().map(Cow::Owned))
        .unwrap_or(Cow::Borrowed("auto"))
    {
        "always" => {
            set_colors_enabled(true);
            Ok("always")
        }
        "auto" => Ok("auto"),
        "never" => {
            set_colors_enabled(false);
            Ok("never")
        }
        color => Err(err_msg(format!("invalid value for --color: {}", color))),
    }
}

struct LocationInfo<'a> {
    tool_config: ToolConfig,
    workspace_root: PathBuf,
    packages: Option<Vec<Package>>,
    exts: Vec<&'a str>,
    find_flags: FindFlags,
}

fn get_find_flags(tool_config: &ToolConfig, target_args: &TargetArgs) -> FindFlags {
    FindFlags {
        include_ignored: target_args.include_ignored || tool_config.review_include_ignored(),
        include_hidden: target_args.include_hidden || tool_config.review_include_hidden(),
    }
}

fn handle_target_args(target_args: &TargetArgs) -> Result<LocationInfo<'_>, Box<dyn Error>> {
    let mut exts: Vec<&str> = target_args.extensions.iter().map(|x| x.as_str()).collect();
    if exts.is_empty() {
        exts.push("snap");
    }

    // if a workspace root is provided we first check if it points to a `Cargo.toml`.  If it
    // does we instead treat it as manifest path.  If both are provided we fail with an error
    // as this would indicate an error.
    let (workspace_root, manifest_path) = match (
        target_args.workspace_root.as_deref(),
        target_args.manifest_path.as_deref(),
    ) {
        (Some(_), Some(_)) => {
            return Err(err_msg(
                "both manifest-path and workspace-root provided.".to_string(),
            ))
        }
        (None, Some(manifest)) => (None, Some(Cow::Borrowed(manifest))),
        (Some(root), manifest_path) => {
            let assumed_manifest = root.join("Cargo.toml");
            if assumed_manifest.is_file() {
                (None, Some(Cow::Owned(assumed_manifest)))
            } else {
                (Some(root), manifest_path.map(Cow::Borrowed))
            }
        }
        (None, None) => (None, None),
    };

    if let Some(workspace_root) = workspace_root {
        let tool_config = ToolConfig::from_workspace(workspace_root)?;
        Ok(LocationInfo {
            workspace_root: workspace_root.to_owned(),
            packages: None,
            exts,
            find_flags: get_find_flags(&tool_config, target_args),
            tool_config,
        })
    } else {
        let Metadata {
            packages,
            workspace_root,
            ..
        } = get_metadata(
            manifest_path.as_deref(),
            target_args.all || target_args.workspace,
        )?;
        let workspace_root = workspace_root.as_std_path().to_path_buf();
        let tool_config = ToolConfig::from_workspace(&workspace_root)?;
        Ok(LocationInfo {
            workspace_root,
            packages: Some(packages),
            exts,
            find_flags: get_find_flags(&tool_config, target_args),
            tool_config,
        })
    }
}

#[allow(clippy::type_complexity)]
fn load_snapshot_containers<'a>(
    loc: &'a LocationInfo,
) -> Result<
    (
        Vec<(SnapshotContainer, Option<&'a Package>)>,
        HashSet<PathBuf>,
    ),
    Box<dyn Error>,
> {
    let mut roots = HashSet::new();
    let mut snapshot_containers = vec![];
    if let Some(ref packages) = loc.packages {
        for package in packages.iter() {
            for root in find_snapshot_roots(package) {
                roots.insert(root.clone());
                for snapshot_container in find_snapshots(&root, &loc.exts, loc.find_flags) {
                    snapshot_containers.push((snapshot_container?, Some(package)));
                }
            }
        }
    } else {
        roots.insert(loc.workspace_root.clone());
        for snapshot_container in find_snapshots(&loc.workspace_root, &loc.exts, loc.find_flags) {
            snapshot_containers.push((snapshot_container?, None));
        }
    }
    snapshot_containers.sort_by(|a, b| a.0.snapshot_sort_key().cmp(&b.0.snapshot_sort_key()));
    Ok((snapshot_containers, roots))
}

fn process_snapshots(
    quiet: bool,
    snapshot_filter: Option<&[String]>,
    loc: &LocationInfo<'_>,
    op: Option<Operation>,
) -> Result<(), Box<dyn Error>> {
    let term = Term::stdout();

    let (mut snapshot_containers, roots) = load_snapshot_containers(loc)?;

    let snapshot_count = snapshot_containers.iter().map(|x| x.0.len()).sum();

    if snapshot_count == 0 {
        if !quiet {
            println!("{}: no snapshots to review", style("done").bold());
            if loc.tool_config.review_warn_undiscovered() {
                show_undiscovered_hint(loc.find_flags, &snapshot_containers, &roots, &loc.exts);
            }
        }
        return Ok(());
    }

    let mut accepted = vec![];
    let mut rejected = vec![];
    let mut skipped = vec![];
    let mut num = 0;
    let mut show_info = true;
    let mut show_diff = true;

    for (snapshot_container, package) in snapshot_containers.iter_mut() {
        let target_file = snapshot_container.target_file().to_path_buf();
        let snapshot_file = snapshot_container.snapshot_file().map(|x| x.to_path_buf());
        for snapshot_ref in snapshot_container.iter_snapshots() {
            // if a filter is provided, check if the snapshot reference is included
            if let Some(filter) = snapshot_filter {
                let key = if let Some(line) = snapshot_ref.line {
                    format!("{}:{}", target_file.display(), line)
                } else {
                    format!("{}", target_file.display())
                };
                if !filter.contains(&key) {
                    skipped.push(snapshot_ref.summary());
                    continue;
                }
            }

            num += 1;
            let op = match op {
                Some(op) => op,
                None => query_snapshot(
                    &loc.workspace_root,
                    &term,
                    &snapshot_ref.new,
                    snapshot_ref.old.as_ref(),
                    *package,
                    snapshot_ref.line,
                    num,
                    snapshot_count,
                    snapshot_file.as_deref(),
                    &mut show_info,
                    &mut show_diff,
                )?,
            };
            match op {
                Operation::Accept => {
                    snapshot_ref.op = Operation::Accept;
                    accepted.push(snapshot_ref.summary());
                }
                Operation::Reject => {
                    snapshot_ref.op = Operation::Reject;
                    rejected.push(snapshot_ref.summary());
                }
                Operation::Skip => {
                    skipped.push(snapshot_ref.summary());
                }
            }
        }
        snapshot_container.commit()?;
    }

    if op.is_none() {
        term.clear_screen()?;
    }

    if !quiet {
        println!("{}", style("insta review finished").bold());
        if !accepted.is_empty() {
            println!("{}:", style("accepted").green());
            for item in accepted {
                println!("  {}", item);
            }
        }
        if !rejected.is_empty() {
            println!("{}:", style("rejected").red());
            for item in rejected {
                println!("  {}", item);
            }
        }
        if !skipped.is_empty() {
            println!("{}:", style("skipped").yellow());
            for item in skipped {
                println!("  {}", item);
            }
        }
    }

    Ok(())
}

fn test_run(mut cmd: TestCommand, color: &str) -> Result<(), Box<dyn Error>> {
    let loc = handle_target_args(&cmd.target_args)?;
    match loc.tool_config.snapshot_update() {
        SnapshotUpdate::Auto => {
            if is_ci() {
                cmd.check = true;
            }
        }
        SnapshotUpdate::New | SnapshotUpdate::No => {}
        SnapshotUpdate::Always => {
            if !cmd.accept && !cmd.accept_unseen && !cmd.review {
                cmd.review = false;
                cmd.accept = true;
            }
        }
        SnapshotUpdate::Unseen => {
            if !cmd.accept {
                cmd.accept_unseen = true;
                cmd.review = true;
                cmd.accept = false;
            }
        }
    }

    // --check always implies --no-force-pass as otherwise this command does not
    // make a lot of sense.
    if cmd.check {
        cmd.no_force_pass = true
    }

    // the tool config can also indicate that --accept-unseen should be picked
    // automatically unless instructed otherwise.
    if loc.tool_config.auto_accept_unseen() && !cmd.accept && !cmd.review {
        cmd.accept_unseen = true;
    }
    if loc.tool_config.auto_review() && !cmd.review && !cmd.accept {
        cmd.review = true;
    }

    // Legacy command
    if cmd.delete_unreferenced_snapshots {
        println!("Warning: `--delete-unreferenced-snapshots` is deprecated. Use `--unreferenced=delete` instead.");
        cmd.unreferenced = "delete".into();
    }

    let test_runner = cmd
        .test_runner
        .parse()
        .map_err(|_| err_msg("invalid test runner preference"))?;

    let unreferenced = cmd
        .unreferenced
        .parse()
        .map_err(|_| err_msg("invalid value for --unreferenced"))?;

    let (mut proc, snapshot_ref_file, prevents_doc_run) =
        prepare_test_runner(test_runner, unreferenced, &cmd, color, &[], None)?;

    if !cmd.keep_pending {
        process_snapshots(true, None, &loc, Some(Operation::Reject))?;
    }

    let status = proc.status()?;
    let mut success = status.success();

    // nextest currently cannot run doctests, run them with regular tests
    if matches!(test_runner, TestRunner::Nextest) && !prevents_doc_run {
        let (mut proc, _, _) = prepare_test_runner(
            TestRunner::CargoTest,
            unreferenced,
            &cmd,
            color,
            &["--doc"],
            snapshot_ref_file.as_deref(),
        )?;
        success = success && proc.status()?.success();
    }

    if !success && cmd.review {
        eprintln!(
            "{} non snapshot tests failed, skipping review",
            style("warning:").bold().yellow()
        );
        return Err(QuietExit(1).into());
    }

    // handle unreferenced snapshots if we were instructed to do so and the
    // tests ran successfully
    if success {
        if let Some(ref path) = snapshot_ref_file {
            handle_unreferenced_snapshots(path.borrow(), &loc, unreferenced, &cmd.package[..])?;
        }
    }

    if cmd.review || cmd.accept {
        process_snapshots(
            false,
            None,
            &handle_target_args(&cmd.target_args)?,
            if cmd.accept {
                Some(Operation::Accept)
            } else {
                None
            },
        )?
    } else {
        let (snapshot_containers, roots) = load_snapshot_containers(&loc)?;
        let snapshot_count = snapshot_containers.iter().map(|x| x.0.len()).sum::<usize>();
        if snapshot_count > 0 {
            eprintln!(
                "{}: {} snapshot{} to review",
                style("info").bold(),
                style(snapshot_count).yellow(),
                if snapshot_count != 1 { "s" } else { "" }
            );
            eprintln!("use `cargo insta review` to review snapshots");
            return Err(QuietExit(1).into());
        } else {
            println!("{}: no snapshots to review", style("info").bold());
            if loc.tool_config.review_warn_undiscovered() {
                show_undiscovered_hint(loc.find_flags, &snapshot_containers, &roots, &loc.exts);
            }
        }
    }

    if !success {
        Err(QuietExit(1).into())
    } else {
        Ok(())
    }
}

fn handle_unreferenced_snapshots(
    path: &Path,
    loc: &LocationInfo<'_>,
    unreferenced: UnreferencedSnapshots,
    packages: &[String],
) -> Result<(), Box<dyn Error>> {
    enum Action {
        Delete,
        Reject,
        Warn,
    }

    let action = match unreferenced {
        UnreferencedSnapshots::Auto => {
            if is_ci() {
                Action::Reject
            } else {
                Action::Delete
            }
        }
        UnreferencedSnapshots::Reject => Action::Reject,
        UnreferencedSnapshots::Delete => Action::Delete,
        UnreferencedSnapshots::Warn => Action::Warn,
        UnreferencedSnapshots::Ignore => return Ok(()),
    };

    let mut files = HashSet::new();
    match fs::read_to_string(path) {
        Ok(s) => {
            for line in s.lines() {
                if let Ok(path) = fs::canonicalize(line) {
                    files.insert(path);
                }
            }
        }
        Err(err) => {
            // if the file was not created, no test referenced
            // snapshots.
            if err.kind() != io::ErrorKind::NotFound {
                return Err(err.into());
            }
        }
    }

    let mut encountered_any = false;
    for entry in make_deletion_walker(&loc.workspace_root, loc.packages.as_deref(), packages) {
        let rel_path = match entry {
            Ok(ref entry) => entry.path(),
            _ => continue,
        };
        if !rel_path.is_file()
            || !rel_path
                .file_name()
                .map_or(false, |x| x.to_str().unwrap_or("").ends_with(".snap"))
        {
            continue;
        }

        if let Ok(path) = fs::canonicalize(rel_path) {
            if files.contains(&path) {
                continue;
            }
            if !encountered_any {
                match action {
                    Action::Delete => {
                        eprintln!("{}: deleted unreferenced snapshots:", style("info").bold());
                    }
                    _ => {
                        eprintln!(
                            "{}: encountered unreferenced snapshots:",
                            style("warning").bold()
                        );
                    }
                }
                encountered_any = true;
            }
            eprintln!("  {}", rel_path.display());
            if matches!(action, Action::Delete) {
                fs::remove_file(path).ok();
            }
        }
    }

    fs::remove_file(path).ok();

    if !encountered_any {
        eprintln!("{}: no unreferenced snapshots found", style("info").bold());
    } else if matches!(action, Action::Reject) {
        return Err(err_msg("aborting because of unreferenced snapshots"));
    }

    Ok(())
}

#[allow(clippy::type_complexity)]
fn prepare_test_runner<'snapshot_ref>(
    test_runner: TestRunner,
    unreferenced: UnreferencedSnapshots,
    cmd: &TestCommand,
    color: &str,
    extra_args: &[&str],
    snapshot_ref_file: Option<&'snapshot_ref Path>,
) -> Result<(process::Command, Option<Cow<'snapshot_ref, Path>>, bool), Box<dyn Error>> {
    let cargo = env::var_os("CARGO");
    let cargo = cargo
        .as_deref()
        .unwrap_or_else(|| std::ffi::OsStr::new("cargo"));
    let mut proc = match test_runner {
        TestRunner::CargoTest | TestRunner::Auto => {
            let mut proc = process::Command::new(cargo);
            proc.arg("test");
            proc
        }
        TestRunner::Nextest => {
            let mut proc = process::Command::new(cargo);
            proc.arg("nextest");
            proc.arg("run");
            proc
        }
    };

    // An env var to indicate we're running under cargo-insta
    proc.env("INSTA_CARGO_INSTA", "1");

    let snapshot_ref_file = if unreferenced != UnreferencedSnapshots::Ignore {
        match snapshot_ref_file {
            Some(path) => Some(Cow::Borrowed(path)),
            None => {
                let snapshot_ref_file = env::temp_dir().join(Uuid::new_v4().to_string());
                proc.env("INSTA_SNAPSHOT_REFERENCES_FILE", &snapshot_ref_file);
                Some(Cow::Owned(snapshot_ref_file))
            }
        }
    } else {
        None
    };
    let mut prevents_doc_run = false;
    if cmd.target_args.all || cmd.target_args.workspace {
        proc.arg("--all");
    }
    if cmd.lib {
        proc.arg("--lib");
        prevents_doc_run = true;
    }
    if let Some(ref bin) = cmd.bin {
        proc.arg("--bin");
        proc.arg(bin);
        prevents_doc_run = true;
    }
    if cmd.bins {
        proc.arg("--bins");
        prevents_doc_run = true;
    }
    if let Some(ref example) = cmd.example {
        proc.arg("--example");
        proc.arg(example);
        prevents_doc_run = true;
    }
    if cmd.examples {
        proc.arg("--examples");
        prevents_doc_run = true;
    }
    for test in &cmd.test {
        proc.arg("--test");
        proc.arg(test);
        prevents_doc_run = true;
    }
    if cmd.tests {
        proc.arg("--tests");
        prevents_doc_run = true;
    }
    for pkg in &cmd.package {
        proc.arg("--package");
        proc.arg(pkg);
    }
    if let Some(ref spec) = cmd.exclude {
        proc.arg("--exclude");
        proc.arg(spec);
    }
    if let Some(ref manifest_path) = cmd.target_args.manifest_path {
        proc.arg("--manifest-path");
        proc.arg(manifest_path);
    }
    if !cmd.fail_fast {
        proc.arg("--no-fail-fast");
    }
    if !cmd.no_force_pass {
        proc.env("INSTA_FORCE_PASS", "1");
    }
    proc.env(
        "INSTA_UPDATE",
        match (cmd.check, cmd.accept_unseen) {
            (true, _) => "no",
            (_, true) => "unseen",
            (_, false) => "new",
        },
    );
    if cmd.force_update_snapshots {
        // for old versions of insta
        proc.env("INSTA_FORCE_UPDATE_SNAPSHOTS", "1");
        // for newer versions of insta
        proc.env("INSTA_FORCE_UPDATE", "1");
    }
    if cmd.require_full_match {
        proc.env("INSTA_REQUIRE_FULL_MATCH", "1");
    }
    let glob_filter =
        cmd.glob_filter
            .iter()
            .map(|x| x.as_str())
            .fold(String::new(), |mut s, item| {
                if !s.is_empty() {
                    s.push(';');
                }
                s.push_str(item);
                s
            });
    if !glob_filter.is_empty() {
        proc.env("INSTA_GLOB_FILTER", glob_filter);
    }
    if cmd.release {
        proc.arg("--release");
    }
    if let Some(ref profile) = cmd.profile {
        proc.arg("--profile");
        proc.arg(profile);
    }
    if cmd.all_targets {
        proc.arg("--all-targets");
    }
    if let Some(n) = cmd.jobs {
        // use -j instead of --jobs since both nextest and cargo test use it
        proc.arg("-j");
        proc.arg(n.to_string());
    }
    if let Some(ref features) = cmd.features {
        proc.arg("--features");
        proc.arg(features);
    }
    if cmd.all_features {
        proc.arg("--all-features");
    }
    if cmd.no_default_features {
        proc.arg("--no-default-features");
    }
    if let Some(ref target) = cmd.target {
        proc.arg("--target");
        proc.arg(target);
    }
    proc.arg("--color");
    proc.arg(color);
    proc.args(extra_args);
    // Items after this are passed to the test runner
    proc.arg("--");
    if !cmd.no_quiet && matches!(test_runner, TestRunner::CargoTest) {
        proc.arg("-q");
    }
    if !cmd.cargo_options.is_empty() {
        proc.args(&cmd.cargo_options);
    }
    // Currently libtest uses a different approach to color, so we need to pass
    // it again to get output from the test runner as well as cargo. See
    // https://github.com/rust-lang/cargo/issues/1983 for more
    // We also only want to do this if we override auto as some custom test runners
    // do not handle --color and then we at least fix the default case.
    // https://github.com/mitsuhiko/insta/issues/473
    if color != "auto" && matches!(test_runner, TestRunner::CargoTest | TestRunner::Auto) {
        proc.arg(format!("--color={}", color));
    };
    Ok((proc, snapshot_ref_file, prevents_doc_run))
}

fn show_cmd(cmd: ShowCommand) -> Result<(), Box<dyn Error>> {
    let loc = handle_target_args(&cmd.target_args)?;
    let snapshot = Snapshot::from_file(&cmd.path)?;
    let mut printer = SnapshotPrinter::new(&loc.workspace_root, None, &snapshot);
    printer.set_snapshot_file(Some(&cmd.path));
    printer.set_show_info(true);
    printer.set_show_diff(false);
    printer.print();
    Ok(())
}

fn pending_snapshots_cmd(cmd: PendingSnapshotsCommand) -> Result<(), Box<dyn Error>> {
    #[derive(Serialize, Debug)]
    #[serde(rename_all = "snake_case", tag = "type")]
    enum SnapshotKey<'a> {
        NamedSnapshot {
            path: &'a Path,
        },
        InlineSnapshot {
            path: &'a Path,
            line: u32,
            old_snapshot: Option<&'a str>,
            new_snapshot: &'a str,
            expression: Option<&'a str>,
        },
    }

    let loc = handle_target_args(&cmd.target_args)?;
    let (mut snapshot_containers, _) = load_snapshot_containers(&loc)?;

    for (snapshot_container, _package) in snapshot_containers.iter_mut() {
        let target_file = snapshot_container.target_file().to_path_buf();
        let is_inline = snapshot_container.snapshot_file().is_none();
        for snapshot_ref in snapshot_container.iter_snapshots() {
            if cmd.as_json {
                let info = if is_inline {
                    SnapshotKey::InlineSnapshot {
                        path: &target_file,
                        line: snapshot_ref.line.unwrap(),
                        old_snapshot: snapshot_ref.old.as_ref().map(|x| x.contents_str()),
                        new_snapshot: snapshot_ref.new.contents_str(),
                        expression: snapshot_ref.new.metadata().expression(),
                    }
                } else {
                    SnapshotKey::NamedSnapshot { path: &target_file }
                };
                println!("{}", serde_json::to_string(&info).unwrap());
            } else if is_inline {
                println!("{}:{}", target_file.display(), snapshot_ref.line.unwrap());
            } else {
                println!("{}", target_file.display());
            }
        }
    }

    Ok(())
}

fn show_undiscovered_hint(
    find_flags: FindFlags,
    snapshot_containers: &[(SnapshotContainer, Option<&Package>)],
    roots: &HashSet<PathBuf>,
    extensions: &[&str],
) {
    // there is nothing to do if we already search everything.
    if find_flags.include_hidden && find_flags.include_ignored {
        return;
    }

    let mut found_extra = false;
    let found_snapshots = snapshot_containers
        .iter()
        .filter_map(|x| x.0.snapshot_file())
        .collect::<HashSet<_>>();

    for root in roots {
        for snapshot in make_snapshot_walker(
            root,
            extensions,
            FindFlags {
                include_ignored: true,
                include_hidden: true,
            },
        )
        .filter_map(|e| e.ok())
        .filter(|x| {
            let fname = x.file_name().to_string_lossy();
            fname.ends_with(".snap.new") || fname.ends_with(".pending-snap")
        }) {
            if !found_snapshots.contains(snapshot.path()) {
                found_extra = true;
                break;
            }
        }
    }

    // we did not find any extra snapshots
    if !found_extra {
        return;
    }

    let (args, paths) = match (find_flags.include_ignored, find_flags.include_hidden) {
        (true, false) => ("--include-ignored", "ignored"),
        (false, true) => ("--include-hidden", "hidden"),
        (false, false) => (
            "--include-ignored and --include-hidden",
            "ignored or hidden",
        ),
        (true, true) => unreachable!(),
    };

    println!(
        "{}: {}",
        style("warning").yellow().bold(),
        format_args!(
            "found undiscovered snapshots in some paths which are not picked up by cargo \
            insta. Use {} if you have snapshots in {} paths.",
            args, paths,
        )
    );
}

pub(crate) fn run() -> Result<(), Box<dyn Error>> {
    // chop off cargo
    let mut args: Vec<_> = env::args_os().collect();
    if env::var("CARGO").is_ok() && args.get(1).and_then(|x| x.to_str()) == Some("insta") {
        args.remove(1);
    }

    let opts = Opts::from_iter(args);

    let color = handle_color(opts.color.as_deref())?;
    match opts.command {
        Command::Review(ref cmd) | Command::Accept(ref cmd) | Command::Reject(ref cmd) => {
            process_snapshots(
                cmd.quiet,
                cmd.snapshot_filter.as_deref(),
                &handle_target_args(&cmd.target_args)?,
                match opts.command {
                    Command::Review(_) => None,
                    Command::Accept(_) => Some(Operation::Accept),
                    Command::Reject(_) => Some(Operation::Reject),
                    _ => unreachable!(),
                },
            )
        }
        Command::Test(cmd) => test_run(cmd, color),
        Command::Show(cmd) => show_cmd(cmd),
        Command::PendingSnapshots(cmd) => pending_snapshots_cmd(cmd),
    }
}
