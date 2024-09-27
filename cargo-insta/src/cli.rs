use std::borrow::{Borrow, Cow};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::{collections::HashSet, fmt};
use std::{env, fs};
use std::{io, process};

use console::{set_colors_enabled, style, Key, Term};
use insta::Snapshot;
use insta::_cargo_insta_support::{
    is_ci, SnapshotPrinter, SnapshotUpdate, TestRunner, ToolConfig, UnreferencedSnapshots,
};
use semver::Version;
use serde::Serialize;
use uuid::Uuid;

use crate::cargo::{find_snapshot_roots, Package};
use crate::container::{Operation, SnapshotContainer};
use crate::utils::cargo_insta_version;
use crate::utils::INSTA_VERSION;
use crate::utils::{err_msg, QuietExit};
use crate::walk::{find_pending_snapshots, make_snapshot_walker, FindFlags};

use clap::{Args, Parser, Subcommand, ValueEnum};

/// A helper utility to work with insta snapshots.
#[derive(Parser, Debug)]
#[command(
    bin_name = "cargo insta",
    arg_required_else_help = true,
    // TODO: do we want these?
    disable_colored_help = true,
    disable_version_flag = true,
    next_line_help = true
)]
struct Opts {
    /// Coloring
    #[arg(long, global = true, value_name = "WHEN", env = "CARGO_TERM_COLOR")]
    color: Option<ColorWhen>,

    #[command(subcommand)]
    command: Command,
}

#[derive(ValueEnum, Copy, Clone, Debug)]
enum ColorWhen {
    Auto,
    Always,
    Never,
}

impl fmt::Display for ColorWhen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorWhen::Auto => write!(f, "auto"),
            ColorWhen::Always => write!(f, "always"),
            ColorWhen::Never => write!(f, "never"),
        }
    }
}

#[derive(Subcommand, Debug)]
#[command(
    after_help = "For the online documentation of the latest version, see https://insta.rs/docs/cli/."
)]
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Interactively review snapshots
    #[command(alias = "verify")]
    Review(ProcessCommand),
    /// Rejects all snapshots
    Reject(ProcessCommand),
    /// Accept all snapshots
    #[command(alias = "approve")]
    Accept(ProcessCommand),
    /// Run tests and then reviews
    Test(TestCommand),
    /// Print a summary of all pending snapshots.
    PendingSnapshots(PendingSnapshotsCommand),
    /// Shows a specific snapshot
    Show(ShowCommand),
}

#[derive(Args, Debug, Clone)]
struct TargetArgs {
    /// Path to `Cargo.toml`
    #[arg(long, value_name = "PATH")]
    manifest_path: Option<PathBuf>,
    /// Explicit path to the workspace root
    #[arg(long, value_name = "PATH")]
    workspace_root: Option<PathBuf>,
    /// Sets the extensions to consider. Defaults to `snap`.
    #[arg(short = 'e', long, value_name = "EXTENSIONS", num_args = 1.., value_delimiter = ',', default_value = "snap")]
    extensions: Vec<String>,
    /// Work on all packages in the workspace
    #[arg(long)]
    workspace: bool,
    /// Alias for `--workspace` (deprecated)
    #[arg(long)]
    all: bool,
    /// Also walk into ignored paths.
    #[arg(long, alias = "no-ignore")]
    include_ignored: bool,
    /// Also include hidden paths.
    #[arg(long)]
    include_hidden: bool,
}

#[derive(Args, Debug)]
struct ProcessCommand {
    #[command(flatten)]
    target_args: TargetArgs,
    /// Limits the operation to one or more snapshots.
    #[arg(long = "snapshot")]
    snapshot_filter: Option<Vec<String>>,
    /// Do not print to stdout.
    #[arg(short = 'q', long)]
    quiet: bool,
}

#[derive(Args, Debug)]
#[command(rename_all = "kebab-case", next_help_heading = "Test Runner Options")]
struct TestRunnerOptions {
    /// Test only this package's library unit tests
    #[arg(long)]
    lib: bool,
    /// Test only the specified binary
    #[arg(long)]
    bin: Option<String>,
    /// Test all binaries
    #[arg(long)]
    bins: bool,
    /// Test only the specified example
    #[arg(long)]
    example: Option<String>,
    /// Test all examples
    #[arg(long)]
    examples: bool,
    /// Test only the specified test targets
    #[arg(long)]
    test: Vec<String>,
    /// Test all tests
    #[arg(long)]
    tests: bool,
    /// Package to run tests for
    #[arg(short = 'p', long)]
    package: Vec<String>,
    /// Exclude packages from the test
    #[arg(long, value_name = "SPEC")]
    exclude: Vec<String>,
    /// Space-separated list of features to activate
    #[arg(long, value_name = "FEATURES")]
    features: Option<String>,
    /// Number of parallel jobs, defaults to # of CPUs
    #[arg(short = 'j', long)]
    jobs: Option<usize>,
    /// Build artifacts in release mode, with optimizations
    #[arg(long)]
    release: bool,
    /// Build artifacts with the specified profile
    #[arg(long)]
    profile: Option<String>,
    /// Test all targets (does not include doctests)
    #[arg(long)]
    all_targets: bool,
    /// Activate all available features
    #[arg(long)]
    all_features: bool,
    /// Do not activate the `default` feature
    #[arg(long)]
    no_default_features: bool,
    /// Build for the target triple
    #[arg(long)]
    target: Option<String>,
}

#[derive(Args, Debug)]
#[command(rename_all = "kebab-case")]
struct TestCommand {
    /// Accept all snapshots after test.
    #[arg(long, conflicts_with_all = ["review", "check"])]
    accept: bool,
    /// Instructs the test command to just assert.
    #[arg(long, conflicts_with_all = ["review"])]
    check: bool,
    /// Follow up with review.
    #[arg(long)]
    review: bool,
    /// Accept all new (previously unseen).
    #[arg(long)]
    accept_unseen: bool,
    /// Do not reject pending snapshots before run.
    #[arg(long)]
    keep_pending: bool,
    /// Update all snapshots even if they are still matching.
    #[arg(long)]
    force_update_snapshots: bool,
    /// Handle unreferenced snapshots after a successful test run.
    #[arg(long, default_value = "ignore")]
    unreferenced: UnreferencedSnapshots,
    /// Filters to apply to the insta glob feature.
    #[arg(long)]
    glob_filter: Vec<String>,
    /// Require metadata as well as snapshots' contents to match.
    #[arg(long)]
    require_full_match: bool,
    /// Prevent running all tests regardless of failure
    #[arg(long)]
    fail_fast: bool,
    /// Do not pass the quiet flag (`-q`) to tests.
    #[arg(short = 'Q', long)]
    no_quiet: bool,
    /// Picks the test runner.
    #[arg(long, default_value = "auto")]
    test_runner: TestRunner,
    #[arg(long)]
    test_runner_fallback: Option<bool>,
    /// Delete unreferenced snapshots after a successful test run.
    #[arg(long, hide = true)]
    delete_unreferenced_snapshots: bool,
    /// Disable force-passing of snapshot tests (deprecated)
    #[arg(long, hide = true)]
    no_force_pass: bool,
    #[command(flatten)]
    target_args: TargetArgs,
    #[command(flatten)]
    test_runner_options: TestRunnerOptions,
    /// Options passed to cargo test
    #[arg(last = true)]
    cargo_options: Vec<String>,
}

#[derive(Args, Debug)]
#[command(rename_all = "kebab-case")]
struct PendingSnapshotsCommand {
    #[command(flatten)]
    target_args: TargetArgs,
    /// Changes the output from human readable to JSON.
    #[arg(long)]
    as_json: bool,
}

#[derive(Args, Debug)]
#[command(rename_all = "kebab-case")]
struct ShowCommand {
    #[command(flatten)]
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
    pkg: &Package,
    line: Option<u32>,
    i: usize,
    n: usize,
    snapshot_file: Option<&Path>,
    show_info: &mut bool,
    show_diff: &mut bool,
) -> Result<Operation, Box<dyn Error>> {
    loop {
        term.clear_screen()?;

        println!(
            "{}{}{} {}@{}:",
            style("Reviewing [").bold(),
            style(format!("{}/{}", i, n)).yellow().bold(),
            style("]").bold(),
            pkg.name.as_str(),
            &pkg.version,
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

fn handle_color(color: Option<ColorWhen>) {
    match color {
        Some(ColorWhen::Always) => {
            set_colors_enabled(true);
        }
        Some(ColorWhen::Never) => {
            set_colors_enabled(false);
        }
        Some(ColorWhen::Auto) | None => {}
    }
}

struct LocationInfo<'a> {
    tool_config: ToolConfig,
    workspace_root: PathBuf,
    /// Packages to test
    packages: Vec<Package>,
    exts: Vec<&'a str>,
    find_flags: FindFlags,
}

fn get_find_flags(tool_config: &ToolConfig, target_args: &TargetArgs) -> FindFlags {
    FindFlags {
        include_ignored: target_args.include_ignored || tool_config.review_include_ignored(),
        include_hidden: target_args.include_hidden || tool_config.review_include_hidden(),
    }
}

fn handle_target_args<'a>(
    target_args: &'a TargetArgs,
    // Empty if none are selected, implying cargo default
    packages: &'a [String],
) -> Result<LocationInfo<'a>, Box<dyn Error>> {
    let exts: Vec<&str> = target_args.extensions.iter().map(|x| x.as_str()).collect();

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
        (Some(root), None) => {
            let assumed_manifest = root.join("Cargo.toml");
            if assumed_manifest.is_file() {
                (None, Some(Cow::Owned(assumed_manifest)))
            } else {
                (Some(root), None)
            }
        }
        (None, None) => (None, None),
    };

    let mut cmd = cargo_metadata::MetadataCommand::new();

    // If a manifest path is provided, set it in the command
    if let Some(manifest_path) = manifest_path {
        cmd.manifest_path(manifest_path);
    }
    if let Some(workspace_root) = workspace_root {
        cmd.current_dir(workspace_root);
    }
    let metadata = cmd.no_deps().exec()?;
    let workspace_root = metadata.workspace_root.as_std_path().to_path_buf();
    let tool_config = ToolConfig::from_workspace(&workspace_root)?;

    // If `--all` is passed, or there's no root package, we include all
    // packages. If packages are specified, we filter from all packages.
    // Otherwise we use just the root package.
    //
    // (Once we're OK running on Cargo 1.71, we can replace `.root_package` with
    // `.default_workspace_packages`.)
    let packages = if metadata.root_package().is_none()
        || (target_args.all || target_args.workspace)
        || !packages.is_empty()
    {
        metadata
            .workspace_packages()
            .into_iter()
            .filter(|p| packages.is_empty() || packages.contains(&p.name))
            .cloned()
            .collect()
    } else {
        vec![metadata.root_package().unwrap().clone()]
    };

    Ok(LocationInfo {
        workspace_root,
        packages,
        exts,
        find_flags: get_find_flags(&tool_config, target_args),
        tool_config,
    })
}

#[allow(clippy::type_complexity)]
fn load_snapshot_containers<'a>(
    loc: &'a LocationInfo,
) -> Result<(Vec<(SnapshotContainer, &'a Package)>, HashSet<PathBuf>), Box<dyn Error>> {
    let mut roots = HashSet::new();
    let mut snapshot_containers = vec![];

    debug_assert!(!loc.packages.is_empty());

    for package in &loc.packages {
        for root in find_snapshot_roots(package) {
            roots.insert(root.clone());
            for snapshot_container in find_pending_snapshots(&root, &loc.exts, loc.find_flags) {
                snapshot_containers.push((snapshot_container?, package));
            }
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
                    package,
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

fn test_run(mut cmd: TestCommand, color: ColorWhen) -> Result<(), Box<dyn Error>> {
    let loc = handle_target_args(&cmd.target_args, &cmd.test_runner_options.package)?;
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
        SnapshotUpdate::Force => {
            cmd.force_update_snapshots = true;
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
        cmd.unreferenced = UnreferencedSnapshots::Delete;
    }

    // Prioritize the command line over the tool config
    let test_runner = match cmd.test_runner {
        TestRunner::Auto => loc.tool_config.test_runner(),
        TestRunner::CargoTest => TestRunner::CargoTest,
        TestRunner::Nextest => TestRunner::Nextest,
    };
    // Prioritize the command line over the tool config
    let test_runner_fallback = cmd
        .test_runner_fallback
        .unwrap_or(loc.tool_config.test_runner_fallback());

    let (mut proc, snapshot_ref_file, prevents_doc_run) = prepare_test_runner(
        test_runner,
        test_runner_fallback,
        cmd.unreferenced,
        &cmd,
        color,
        &[],
        None,
    )?;

    if !cmd.keep_pending {
        process_snapshots(true, None, &loc, Some(Operation::Reject))?;
    }

    let status = proc.status()?;
    let mut success = status.success();

    // nextest currently cannot run doctests, run them with regular tests.
    //
    // Note that unlike `cargo test`, `cargo test --doctest` will run doctests
    // even on crates that specify `doctests = false`. But I don't think there's
    // a way to replicate the `cargo test` behavior.
    if matches!(cmd.test_runner, TestRunner::Nextest) && !prevents_doc_run {
        let (mut proc, _, _) = prepare_test_runner(
            TestRunner::CargoTest,
            false,
            cmd.unreferenced,
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
        if let Some(ref snapshot_ref_path) = snapshot_ref_file {
            handle_unreferenced_snapshots(snapshot_ref_path.borrow(), &loc, cmd.unreferenced)?;
        }
    }

    if cmd.review || cmd.accept {
        process_snapshots(
            false,
            None,
            &loc,
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

/// Scan for any snapshots that were not referenced by any test.
fn handle_unreferenced_snapshots(
    snapshot_ref_path: &Path,
    loc: &LocationInfo<'_>,
    unreferenced: UnreferencedSnapshots,
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
    match fs::read_to_string(snapshot_ref_path) {
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

    for package in loc.packages.clone() {
        let unreferenced_snapshots = make_snapshot_walker(
            package.manifest_path.parent().unwrap().as_std_path(),
            &[".snap"],
            FindFlags {
                include_ignored: true,
                include_hidden: true,
            },
        )
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|name| name.ends_with(".snap"))
                .unwrap_or(false)
        })
        .filter_map(|e| e.path().canonicalize().ok())
        .filter(|path| !files.contains(path));

        for path in unreferenced_snapshots {
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
            eprintln!("  {}", path.display());
            if matches!(action, Action::Delete) {
                fs::remove_file(&path).ok();
            }
        }
    }

    fs::remove_file(snapshot_ref_path).ok();

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
    test_runner_fallback: bool,
    unreferenced: UnreferencedSnapshots,
    cmd: &TestCommand,
    color: ColorWhen,
    extra_args: &[&str],
    snapshot_ref_file: Option<&'snapshot_ref Path>,
) -> Result<(process::Command, Option<Cow<'snapshot_ref, Path>>, bool), Box<dyn Error>> {
    let cargo = env::var_os("CARGO");
    let cargo = cargo
        .as_deref()
        .unwrap_or_else(|| std::ffi::OsStr::new("cargo"));
    let test_runner = match test_runner {
        TestRunner::CargoTest | TestRunner::Auto => test_runner,
        TestRunner::Nextest => {
            // Fall back to `cargo test` if `cargo nextest` isn't installed and
            // `test_runner_fallback` is true (but don't run the cargo command
            // unless that's an option)
            if !test_runner_fallback
                || std::process::Command::new("cargo")
                    .arg("nextest")
                    .arg("--version")
                    .output()
                    .map(|output| output.status.success())
                    .unwrap_or(false)
            {
                TestRunner::Nextest
            } else {
                TestRunner::Auto
            }
        }
    };
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
    proc.env("INSTA_CARGO_INSTA_VERSION", cargo_insta_version());

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
    if cmd.test_runner_options.lib {
        proc.arg("--lib");
        prevents_doc_run = true;
    }
    if let Some(ref bin) = cmd.test_runner_options.bin {
        proc.arg("--bin");
        proc.arg(bin);
        prevents_doc_run = true;
    }
    if cmd.test_runner_options.bins {
        proc.arg("--bins");
        prevents_doc_run = true;
    }
    if let Some(ref example) = cmd.test_runner_options.example {
        proc.arg("--example");
        proc.arg(example);
        prevents_doc_run = true;
    }
    if cmd.test_runner_options.examples {
        proc.arg("--examples");
        prevents_doc_run = true;
    }
    for test in &cmd.test_runner_options.test {
        proc.arg("--test");
        proc.arg(test);
        prevents_doc_run = true;
    }
    if cmd.test_runner_options.tests {
        proc.arg("--tests");
        prevents_doc_run = true;
    }
    for pkg in &cmd.test_runner_options.package {
        proc.arg("--package");
        proc.arg(pkg);
    }
    for spec in &cmd.test_runner_options.exclude {
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
    if !cmd.check {
        proc.env("INSTA_FORCE_PASS", "1");
    } else if !cmd.no_force_pass {
        proc.env("INSTA_FORCE_PASS", "1");
        // If we're not running under cargo insta, raise a warning that this option
        // is deprecated. (cargo insta still uses it when running under `--check`,
        // but this will stop soon too)
        eprintln!(
            "{}: `--no-force-pass` is deprecated. Please use --check to immediately raise an error on any non-matching snapshots.",
            style("warning").bold().yellow()
    );
    }

    proc.env(
        "INSTA_UPDATE",
        // Don't set `INSTA_UPDATE=force` for `--force-update-snapshots` on
        // older versions
        if *INSTA_VERSION >= Version::new(1,41,0) {
            match (cmd.check, cmd.accept_unseen, cmd.force_update_snapshots) {
                (true, false, false) => "no",
                (false, true, false) => "unseen",
                (false, false, false) => "new",
                (false, _, true) => "force",
                _ => return Err(err_msg(format!("invalid combination of flags: check={}, accept-unseen={}, force-update-snapshots={}", cmd.check, cmd.accept_unseen, cmd.force_update_snapshots))),
            }
        } else {
            match (cmd.check, cmd.accept_unseen) {
                (true, _) => "no",
                (_, true) => "unseen",
                (_, false) => "new",
            }
        }
    );
    if cmd.force_update_snapshots && *INSTA_VERSION < Version::new(1, 40, 0) {
        // Currently compatible with older versions of insta.
        proc.env("INSTA_FORCE_UPDATE_SNAPSHOTS", "1");
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
    if cmd.test_runner_options.release {
        proc.arg("--release");
    }
    if let Some(ref profile) = cmd.test_runner_options.profile {
        proc.arg("--profile");
        proc.arg(profile);
    }
    if cmd.test_runner_options.all_targets {
        proc.arg("--all-targets");
    }
    if let Some(n) = cmd.test_runner_options.jobs {
        // use -j instead of --jobs since both nextest and cargo test use it
        proc.arg("-j");
        proc.arg(n.to_string());
    }
    if let Some(ref features) = cmd.test_runner_options.features {
        proc.arg("--features");
        proc.arg(features);
    }
    if cmd.test_runner_options.all_features {
        proc.arg("--all-features");
    }
    if cmd.test_runner_options.no_default_features {
        proc.arg("--no-default-features");
    }
    if let Some(ref target) = cmd.test_runner_options.target {
        proc.arg("--target");
        proc.arg(target);
    }
    proc.arg("--color");
    proc.arg(color.to_string());
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
    if matches!(color, ColorWhen::Auto)
        && matches!(test_runner, TestRunner::CargoTest | TestRunner::Auto)
    {
        proc.arg(format!("--color={}", color));
    };
    Ok((proc, snapshot_ref_file, prevents_doc_run))
}

fn show_cmd(cmd: ShowCommand) -> Result<(), Box<dyn Error>> {
    let loc = handle_target_args(&cmd.target_args, &[])?;
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
        FileSnapshot {
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

    let loc = handle_target_args(&cmd.target_args, &[])?;
    let (mut snapshot_containers, _) = load_snapshot_containers(&loc)?;

    for (snapshot_container, _package) in snapshot_containers.iter_mut() {
        let target_file = snapshot_container.target_file().to_path_buf();
        let is_inline = snapshot_container.snapshot_file().is_none();
        for snapshot_ref in snapshot_container.iter_snapshots() {
            if cmd.as_json {
                let old_snapshot = snapshot_ref.old.as_ref().map(|x| x.contents_string());
                let new_snapshot = snapshot_ref.new.contents_string();
                let info = if is_inline {
                    SnapshotKey::InlineSnapshot {
                        path: &target_file,
                        line: snapshot_ref.line.unwrap(),
                        old_snapshot: old_snapshot.as_deref(),
                        new_snapshot: &new_snapshot,
                        expression: snapshot_ref.new.metadata().expression(),
                    }
                } else {
                    SnapshotKey::FileSnapshot { path: &target_file }
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
    snapshot_containers: &[(SnapshotContainer, &Package)],
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

    let opts = Opts::parse_from(args);

    handle_color(opts.color);
    match opts.command {
        Command::Review(ref cmd) | Command::Accept(ref cmd) | Command::Reject(ref cmd) => {
            process_snapshots(
                cmd.quiet,
                cmd.snapshot_filter.as_deref(),
                &handle_target_args(&cmd.target_args, &[])?,
                match opts.command {
                    Command::Review(_) => None,
                    Command::Accept(_) => Some(Operation::Accept),
                    Command::Reject(_) => Some(Operation::Reject),
                    _ => unreachable!(),
                },
            )
        }
        Command::Test(cmd) => test_run(cmd, opts.color.unwrap_or(ColorWhen::Auto)),
        Command::Show(cmd) => show_cmd(cmd),
        Command::PendingSnapshots(cmd) => pending_snapshots_cmd(cmd),
    }
}
