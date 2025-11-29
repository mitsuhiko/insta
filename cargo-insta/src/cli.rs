use std::borrow::{Borrow, Cow};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::{collections::HashSet, fmt};
use std::{env, fs};
use std::{io, process};

use console::{set_colors_enabled, style, Key, Term};
use insta::_cargo_insta_support::{
    get_cargo, is_ci, SnapshotPrinter, SnapshotUpdate, TestRunner, ToolConfig,
    UnreferencedSnapshots,
};
use insta::{internals::SnapshotContents, Snapshot};
use itertools::Itertools;
use semver::Version;
use serde::Serialize;
use uuid::Uuid;

use crate::cargo::{find_snapshot_roots, Package};
use crate::container::{Operation, SnapshotContainer};
use crate::utils::cargo_insta_version;
use crate::utils::{err_msg, QuietExit};
use crate::walk::{find_pending_snapshots, make_snapshot_walker, FindFlags};

use clap::{Args, Parser, Subcommand, ValueEnum};

/// A helper utility to work with insta snapshots.
#[derive(Parser, Debug)]
#[command(
    bin_name = "cargo insta",
    arg_required_else_help = true,
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
    version,
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
    // TODO: this alias is confusing, I think we should remove — does "no" mean
    // "don't ignore files" or "not ignored files"?
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
    #[arg(short = 'F', long, value_name = "FEATURES")]
    features: Option<String>,
    /// Number of parallel jobs, defaults to # of CPUs
    #[arg(short = 'j', long)]
    jobs: Option<usize>,
    /// Build artifacts in release mode, with optimizations
    #[arg(short = 'r', long)]
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
    #[arg(long, hide = true)]
    accept_unseen: bool,
    /// Do not reject pending snapshots before run (deprecated).
    #[arg(long, hide = true)]
    keep_pending: bool,
    /// Update all snapshots even if they are still matching; implies `--accept`.
    #[arg(long)]
    force_update_snapshots: bool,
    /// Handle unreferenced snapshots after a successful test run.
    #[arg(long)]
    unreferenced: Option<UnreferencedSnapshots>,
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
    /// Fallback to cargo test if nextest is not available.
    ///
    /// Use --test-runner-fallback to enable, --test-runner-fallback=false to disable,
    /// or --no-test-runner-fallback as a shorthand for disabling
    #[arg(
        long = "test-runner-fallback",
        num_args(0..=1),
        default_missing_value = "true",
        overrides_with = "_no_test_runner_fallback"
    )]
    test_runner_fallback: Option<bool>,
    /// Don't fallback to cargo test if nextest is not available \[default\]
    #[arg(
        long = "no-test-runner-fallback",
        overrides_with = "test_runner_fallback"
    )]
    _no_test_runner_fallback: bool,
    /// Delete unreferenced snapshots after a successful test run (deprecated)
    #[arg(long, hide = true)]
    delete_unreferenced_snapshots: bool,
    /// Disable force-passing of snapshot tests (deprecated)
    #[arg(long, hide = true)]
    no_force_pass: bool,
    /// Disable running doctests when using nextest test runner
    #[arg(long, alias = "dnd")]
    disable_nextest_doctest: bool,
    #[command(flatten)]
    target_args: TargetArgs,
    #[command(flatten)]
    test_runner_options: TestRunnerOptions,
    /// Options passed to cargo test
    #[arg(last = true)]
    cargo_options: Vec<String>,
}

impl TestCommand {
    fn test_runner_fallback_value(&self) -> Option<bool> {
        // When _no_test_runner_fallback is true, it means --no-test-runner-fallback
        // was the last flag specified (clap's overrides_with sets the overridden flag
        // to its default value). Otherwise, use test_runner_fallback which may be
        // Some(true/false) if the flag was used, or None if no flag was provided.
        if self._no_test_runner_fallback {
            Some(false)
        } else {
            self.test_runner_fallback
        }
    }
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
    // Check if we're running in a TTY environment
    if !term.is_term() {
        return Err(err_msg(
            "Interactive review requires a terminal. For non-interactive snapshot management:\n\
            - Use `cargo insta pending-snapshots` to list pending snapshots\n\
            - Use `cargo insta review --snapshot <path>` to view a specific snapshot diff\n\
            - Use `cargo insta reject --snapshot <path>` to view and reject a specific snapshot\n\
            - Use `cargo insta accept` or `cargo insta reject` to accept/reject all snapshots\n\
            - Use `cargo insta accept --snapshot <path>` to accept a specific snapshot\n\
            Or run this command in a terminal environment.",
        ));
    }

    loop {
        term.clear_screen()?;

        println!(
            "{}{}{} {}@{}:",
            style("Reviewing [").bold(),
            style(format!("{i}/{n}")).yellow().bold(),
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

        let new_is_binary = new.contents().is_binary();
        let old_is_binary = old.map(|o| o.contents().is_binary()).unwrap_or(false);

        if new_is_binary || old_is_binary {
            println!(
                "  {} open       {}",
                style("o").cyan().bold(),
                style(if new_is_binary && old_is_binary {
                    "open snapshot files in external tool"
                } else if new_is_binary {
                    "open new snapshot file in external tool"
                } else {
                    "open old snapshot file in external tool"
                })
                .dim()
            );
        }

        // Add a subtle hint about uppercase shortcuts at the bottom
        println!();
        println!(
            "  {}",
            style("Tip: Use uppercase A/R/S to apply to all remaining snapshots").dim()
        );

        loop {
            match term.read_key()? {
                Key::Char('a') | Key::Enter => return Ok(Operation::Accept),
                Key::Char('A') => return Ok(Operation::AcceptAll),
                Key::Char('r') | Key::Escape => return Ok(Operation::Reject),
                Key::Char('R') => return Ok(Operation::RejectAll),
                Key::Char('s') | Key::Char(' ') => return Ok(Operation::Skip),
                Key::Char('S') => return Ok(Operation::SkipAll),
                Key::Char('i') => {
                    *show_info = !*show_info;
                    break;
                }
                Key::Char('d') => {
                    *show_diff = !*show_diff;
                    break;
                }
                Key::Char('o') => {
                    if let Some(old) = old {
                        if let Some(path) = old.build_binary_path(snapshot_file.unwrap()) {
                            open::that_detached(path)?;
                        }
                    }

                    if let Some(path) =
                        new.build_binary_path(snapshot_file.unwrap().with_extension("snap.new"))
                    {
                        open::that_detached(path)?;
                    }

                    // there's no break here because there's no need to re-output anything
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

#[derive(Debug)]
struct LocationInfo<'a> {
    tool_config: ToolConfig,
    workspace_root: PathBuf,
    /// Packages to test
    packages: Vec<Package>,
    exts: Vec<&'a str>,
    find_flags: FindFlags,
    /// The tested crate's insta version (i.e. not the `cargo-insta` binary
    /// that's running this code).
    insta_version: Version,
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
    let mut cmd = cargo_metadata::MetadataCommand::new();

    match (
        target_args.workspace_root.as_deref(),
        target_args.manifest_path.as_deref(),
    ) {
        (Some(_), Some(_)) => {
            return Err(err_msg(
                "both manifest-path and workspace-root provided.".to_string(),
            ))
        }
        (None, Some(manifest)) => {
            cmd.manifest_path(manifest);
        }
        (Some(root), None) => {
            cmd.current_dir(root);
        }
        (None, None) => {}
    };

    let metadata = cmd
        .exec()
        .map_err(|e| format!("failed to load cargo metadata: {e}. Command details: {cmd:?}"))?;
    let workspace_root = metadata.workspace_root.as_std_path().to_path_buf();
    let tool_config = ToolConfig::from_workspace(&workspace_root)?;

    let insta_version = metadata
        .packages
        .iter()
        .find(|package| package.name == "insta")
        .map(|package| package.version.clone())
        .ok_or_else(|| eprintln!("insta not found in cargo metadata; defaulting to 1.0.0"))
        .unwrap_or(Version::new(1, 0, 0));

    // If `--workspace` is passed, or there's no root package, we include all
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
            .map(|mut p| {
                // Dependencies aren't needed and bloat the object (but we can't pass
                // `--no-deps` to the original command as we collect the insta
                // version above...)
                p.dependencies = vec![];
                p
            })
            .collect()
    } else {
        vec![metadata.root_package().unwrap().clone()]
    };

    Ok(LocationInfo {
        workspace_root,
        packages,
        exts: target_args
        .extensions
        .iter()
        .map(|x| {
            if let Some(no_period) = x.strip_prefix(".") {
                eprintln!("`{x}` supplied as an extension. This will use `foo.{x}` as file names; likely you want `{no_period}` instead.")
            };
            x.as_str()
        })
        .collect(),
        find_flags: get_find_flags(&tool_config, target_args),
        tool_config,
        insta_version
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

/// Formats a snapshot key for use in filters and display.
/// Returns "path" for file snapshots or "path:line" for inline snapshots.
/// Converts absolute paths to workspace-relative paths.
fn format_snapshot_key(workspace_root: &Path, target_file: &Path, line: Option<u32>) -> String {
    let relative_path = target_file
        .strip_prefix(workspace_root)
        .unwrap_or(target_file);

    if let Some(line) = line {
        format!("{}:{}", relative_path.display(), line)
    } else {
        format!("{}", relative_path.display())
    }
}

/// Processes snapshot files for reviewing, accepting, or rejecting.
fn review_snapshots(
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
                show_undiscovered_hint(
                    loc.find_flags,
                    &snapshot_containers
                        .iter()
                        .map(|x| x.0.clone())
                        .collect_vec(),
                    &roots,
                    &loc.exts,
                );
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
    let mut apply_to_all: Option<Operation> = None;

    // Non-interactive mode: if we have a filter and no TTY, just show diffs.
    // Accept doesn't need display (it just accepts), but review and reject should show what they're affecting.
    let non_interactive_display = snapshot_filter.is_some()
        && !term.is_term()
        && (op.is_none() || matches!(op, Some(Operation::Reject)));

    for (snapshot_container, package) in snapshot_containers.iter_mut() {
        let target_file = snapshot_container.target_file().to_path_buf();
        let snapshot_file = snapshot_container.snapshot_file().map(|x| x.to_path_buf());
        for snapshot_ref in snapshot_container.iter_snapshots() {
            // if a filter is provided, check if the snapshot reference is included
            if let Some(filter) = snapshot_filter {
                let key = format_snapshot_key(&loc.workspace_root, &target_file, snapshot_ref.line);
                if !filter.contains(&key) {
                    skipped.push(snapshot_ref.summary());
                    continue;
                }
            }

            num += 1;

            // In non-interactive display mode, show the snapshot diff
            if non_interactive_display {
                println!(
                    "{}{}:",
                    style("Snapshot: ").bold(),
                    style(&snapshot_ref.summary()).yellow()
                );
                println!("  Package: {}@{}", package.name.as_str(), &package.version);
                println!();

                let mut printer = SnapshotPrinter::new(
                    &loc.workspace_root,
                    snapshot_ref.old.as_ref(),
                    &snapshot_ref.new,
                );
                printer.set_snapshot_file(snapshot_file.as_deref());
                printer.set_line(snapshot_ref.line);
                printer.set_show_info(true);
                printer.set_show_diff(true);
                printer.print();

                println!();

                // If we're in review mode (no op), just show instructions and skip
                if op.is_none() {
                    let key =
                        format_snapshot_key(&loc.workspace_root, &target_file, snapshot_ref.line);
                    println!("To accept: cargo insta accept --snapshot '{}'", key);
                    println!("To reject: cargo insta reject --snapshot '{}'", key);
                    println!();

                    skipped.push(snapshot_ref.summary());
                    continue;
                }
                // Otherwise fall through to apply the operation (reject)
                // Note: Only reject mode reaches here because review mode returns early above
            }

            let op = match (op, apply_to_all) {
                (Some(op), _) => op, // Use provided op if any (from CLI)
                (_, Some(op)) => op, // Use apply_to_all if set from previous choice
                _ => {
                    // Otherwise prompt for user choice
                    let choice = query_snapshot(
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
                    )?;

                    // For "All" operations, set the apply_to_all flag and convert to single operation
                    match choice {
                        Operation::AcceptAll => {
                            apply_to_all = Some(Operation::Accept);
                            Operation::Accept
                        }
                        Operation::RejectAll => {
                            apply_to_all = Some(Operation::Reject);
                            Operation::Reject
                        }
                        Operation::SkipAll => {
                            apply_to_all = Some(Operation::Skip);
                            Operation::Skip
                        }
                        op => op,
                    }
                }
            };

            match op {
                Operation::Accept | Operation::AcceptAll => {
                    snapshot_ref.op = Operation::Accept;
                    accepted.push(snapshot_ref.summary());
                }
                Operation::Reject | Operation::RejectAll => {
                    snapshot_ref.op = Operation::Reject;
                    rejected.push(snapshot_ref.summary());
                }
                Operation::Skip | Operation::SkipAll => {
                    skipped.push(snapshot_ref.summary());
                }
            }
        }
        snapshot_container.commit()?;
    }

    if op.is_none() && apply_to_all.is_none() {
        term.clear_screen()?;
    }

    if !quiet {
        println!("{}", style("insta review finished").bold());
        if !accepted.is_empty() {
            println!("{}:", style("accepted").green());
            for item in accepted {
                println!("  {item}");
            }
        }
        if !rejected.is_empty() {
            println!("{}:", style("rejected").red());
            for item in rejected {
                println!("  {item}");
            }
        }
        if !skipped.is_empty() {
            println!("{}:", style("skipped").yellow());
            for item in skipped {
                println!("  {item}");
            }
        }
    }

    Ok(())
}

/// Check if any of the packages have doctests
fn has_doctests(packages: &[Package]) -> bool {
    for package in packages {
        for target in &package.targets {
            // Skip non-source targets
            if target.kind.iter().any(|kind| kind == "custom-build") {
                continue;
            }

            // Check if the target source file exists and contains doctests
            if let Ok(content) = fs::read_to_string(&target.src_path) {
                // Look for doc comment blocks with code blocks
                if content.contains("/// ```") || content.contains("//! ```") {
                    return true;
                }
            }
        }
    }
    false
}

/// Run the tests
fn test_run(mut cmd: TestCommand, color: ColorWhen) -> Result<(), Box<dyn Error>> {
    let loc = handle_target_args(&cmd.target_args, &cmd.test_runner_options.package)?;

    if cmd.accept_unseen {
        eprintln!(
            "{} If this option is materially helpful to you, please add a comment at https://github.com/mitsuhiko/insta/issues/659.", 
            style("`--accept-unseen` is pending deprecation.").bold().yellow()
        )
    }

    // Based on any configs in the config file, update the test command. Default
    // is `SnapshotUpdate::Auto`.
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
    // `--force-update-snapshots` implies `--accept`
    if cmd.force_update_snapshots {
        cmd.accept = true;
    }
    if cmd.no_force_pass {
        cmd.check = true;
        eprintln!(
            "{}: `--no-force-pass` is deprecated. Please use --check to immediately raise an error on any non-matching snapshots.",
            style("warning").bold().yellow()
        )
    }
    if cmd.keep_pending {
        eprintln!(
            "{}: `--keep-pending` is deprecated; its behavior is implied: pending snapshots are never removed before a test run.",
            style("warning").bold().yellow()
        )
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
        eprintln!("Warning: `--delete-unreferenced-snapshots` is deprecated. Use `--unreferenced=delete` instead.");
        cmd.unreferenced = Some(UnreferencedSnapshots::Delete);
    }

    // If unreferenced wasn't specified, use the config file setting
    cmd.unreferenced = cmd
        .unreferenced
        .or_else(|| Some(loc.tool_config.test_unreferenced()));

    // Prioritize the command line over the tool config
    let test_runner = match cmd.test_runner {
        TestRunner::Auto => loc.tool_config.test_runner(),
        TestRunner::CargoTest => TestRunner::CargoTest,
        TestRunner::Nextest => TestRunner::Nextest,
    };
    let test_runner = test_runner.resolve_fallback(
        cmd.test_runner_fallback_value()
            .unwrap_or(loc.tool_config.test_runner_fallback()),
    );

    let (mut proc, snapshot_ref_file, prevents_doc_run) =
        prepare_test_runner(&cmd, test_runner, color, &[], None, &loc)?;

    // Set up warnings file for collecting warnings from test processes.
    // This is necessary because test runners like nextest suppress stdout/stderr
    // from passing tests by default.
    let warnings_file = env::temp_dir().join(format!("insta-warnings-{}", Uuid::new_v4()));
    proc.env("INSTA_WARNINGS_FILE", &warnings_file);

    if let Some(workspace_root) = &cmd.target_args.workspace_root {
        proc.current_dir(workspace_root);
    }

    // Run the tests
    let status = proc.status()?;
    let mut success = status.success();

    // nextest currently cannot run doctests, run them with regular tests. We'd
    // like to deprecate this; see discussion at https://github.com/mitsuhiko/insta/pull/438
    //
    // Note that unlike `cargo test`, `cargo test --doctest` will run doctests
    // even on crates that specify `doctests = false`. But I don't think there's
    // a way to replicate the `cargo test` behavior.
    if matches!(cmd.test_runner, TestRunner::Nextest)
        && !prevents_doc_run
        && !cmd.disable_nextest_doctest
    {
        // Check if there are doctests and show warning
        if has_doctests(&loc.packages) {
            eprintln!(
                "{}: insta won't run a separate doctest process when using nextest in the future. \
                 Pass `--disable-nextest-doctest` (or `--dnd`) to update to this behavior now and silence this warning.",
                style("warning").bold().yellow()
            );
        }

        let (mut proc, _, _) = prepare_test_runner(
            &cmd,
            &TestRunner::CargoTest,
            color,
            &["--doc"],
            snapshot_ref_file.as_deref(),
            &loc,
        )?;
        // Use the same warnings file for doctests
        proc.env("INSTA_WARNINGS_FILE", &warnings_file);
        success = success && proc.status()?.success();
    }

    // Display any warnings collected during tests (deduplicated)
    if warnings_file.exists() {
        if let Ok(contents) = fs::read_to_string(&warnings_file) {
            let mut seen = std::collections::BTreeSet::new();
            for line in contents.lines().map(str::trim).filter(|l| !l.is_empty()) {
                if seen.insert(line.to_owned()) {
                    eprintln!("{}", line);
                }
            }
        }
        fs::remove_file(&warnings_file).ok();
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
            handle_unreferenced_snapshots(
                snapshot_ref_path.borrow(),
                &loc,
                // we set this to `Some` above so can't be `None`
                cmd.unreferenced.unwrap(),
            )?;
        }
    }

    if cmd.review || cmd.accept {
        review_snapshots(
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
        let snapshot_containers = snapshot_containers.into_iter().map(|x| x.0).collect_vec();
        let snapshot_count = snapshot_containers.iter().map(|x| x.len()).sum::<usize>();
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

    let snapshot_files_from_test = fs::read_to_string(snapshot_ref_path)
        .map(|s| {
            s.lines()
                .filter_map(|line| fs::canonicalize(line).ok())
                .collect()
        })
        .or_else(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                // if the file was not created, no test referenced
                // snapshots (though we also check for this in the calling
                // function, so maybe duplicative...)
                Ok(HashSet::new())
            } else {
                Err(err)
            }
        })?;

    let mut encountered_any = false;

    for package in loc.packages.clone() {
        let unreferenced_snapshots = make_snapshot_walker(
            package.manifest_path.parent().unwrap().as_std_path(),
            &loc.exts,
            FindFlags {
                include_ignored: true,
                include_hidden: true,
            },
        )
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .filter_map(|e| e.path().canonicalize().ok())
        // The path isn't in the list which the tests wrote to, so it's
        // unreferenced.
        .filter(|path| !snapshot_files_from_test.contains(path))
        // we don't want to delete the new or pending-snap files, partly because
        // we use their presence to determine if a test created a snapshot and
        // so `insta test` should fail
        .filter(|path| {
            path.extension()
                .map(|x| x != "new" && x != "pending-snap")
                .unwrap_or(true)
        });

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
                // If it's an inline pending snapshot, then don't attempt to
                // load it, since these are in a different format; just delete
                if path.extension() == Some(std::ffi::OsStr::new("pending-snap")) {
                    if let Err(e) = fs::remove_file(path) {
                        eprintln!("Failed to remove file: {e}");
                    }
                } else {
                    let snapshot = match Snapshot::from_file(&path) {
                        Ok(snapshot) => snapshot,
                        Err(e) => {
                            eprintln!("Error loading snapshot at {:?}: {}", &path, e);
                            continue;
                        }
                    };

                    if let Some(binary_path) = snapshot.build_binary_path(&path) {
                        fs::remove_file(&binary_path).ok();
                    }

                    fs::remove_file(&path).ok();
                }
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

/// Create and setup a `Command`, translating our configs into env vars & cli options
// TODO: possibly we can clean this function up a bit, reduce the number of args
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn prepare_test_runner<'snapshot_ref>(
    cmd: &TestCommand,
    test_runner: &TestRunner,
    color: ColorWhen,
    extra_args: &[&str],
    snapshot_ref_file: Option<&'snapshot_ref Path>,
    loc: &LocationInfo,
) -> Result<(process::Command, Option<Cow<'snapshot_ref, Path>>, bool), Box<dyn Error>> {
    let mut proc = match test_runner {
        TestRunner::CargoTest | TestRunner::Auto => {
            let mut proc = process::Command::new(get_cargo());
            proc.arg("test");
            proc
        }
        TestRunner::Nextest => {
            let mut proc = get_cargo_nextest_command();
            proc.arg("run");
            proc
        }
    };

    // An env var to indicate we're running under cargo-insta
    proc.env("INSTA_CARGO_INSTA", "1");
    proc.env("INSTA_CARGO_INSTA_VERSION", cargo_insta_version());

    let snapshot_ref_file = if cmd.unreferenced != Some(UnreferencedSnapshots::Ignore) {
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
    if let Some(ref manifest_path) = &cmd.target_args.manifest_path {
        proc.arg("--manifest-path");
        proc.arg(manifest_path);
    }
    if !cmd.fail_fast {
        proc.arg("--no-fail-fast");
    }
    if !cmd.check {
        proc.env("INSTA_FORCE_PASS", "1");
    }

    proc.env(
        "INSTA_UPDATE",
        // Don't set `INSTA_UPDATE=force` for `--force-update-snapshots` on
        // older versions
        if loc.insta_version >= Version::new(1,41,0) {
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
    if cmd.force_update_snapshots && loc.insta_version < Version::new(1, 40, 0) {
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
    proc.args(["--color", color.to_string().as_str()]);
    proc.args(extra_args);
    // Items after this are passed to the test runner
    proc.arg("--");
    if !cmd.no_quiet && matches!(test_runner, TestRunner::CargoTest) {
        proc.arg("-q");
    }
    proc.args(&cmd.cargo_options);
    // Currently libtest uses a different approach to color, so we need to pass
    // it again to get output from the test runner as well as cargo. See
    // https://github.com/rust-lang/cargo/issues/1983 for more
    // We also only want to do this if we override auto as some custom test runners
    // do not handle --color and then we at least fix the default case.
    // https://github.com/mitsuhiko/insta/issues/473
    if matches!(color, ColorWhen::Auto)
        && matches!(test_runner, TestRunner::CargoTest | TestRunner::Auto)
    {
        proc.arg(format!("--color={color}"));
    };
    Ok((proc, snapshot_ref_file, prevents_doc_run))
}

fn get_cargo_nextest_command() -> std::process::Command {
    let cargo_nextest = env::var_os("INSTA_CARGO_NEXTEST_BIN");
    match cargo_nextest.as_deref() {
        Some(cargo_nextest_bin_path) => process::Command::new(cargo_nextest_bin_path),
        None => {
            let mut proc = process::Command::new(get_cargo());
            proc.arg("nextest");
            proc
        }
    }
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

    let mut snapshot_keys = vec![];

    for (snapshot_container, _package) in snapshot_containers.iter_mut() {
        let target_file = snapshot_container.target_file().to_path_buf();
        let is_inline = snapshot_container.snapshot_file().is_none();
        for snapshot_ref in snapshot_container.iter_snapshots() {
            let key = format_snapshot_key(&loc.workspace_root, &target_file, snapshot_ref.line);

            if cmd.as_json {
                let old_snapshot = snapshot_ref.old.as_ref().map(|x| match x.contents() {
                    SnapshotContents::Text(x) => x.to_string(),
                    _ => unreachable!(),
                });
                let new_snapshot = match snapshot_ref.new.contents() {
                    SnapshotContents::Text(x) => x.to_string(),
                    _ => unreachable!(),
                };

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
            } else {
                snapshot_keys.push(key);
            }
        }
    }

    if !cmd.as_json {
        if snapshot_keys.is_empty() {
            println!("No pending snapshots.");
        } else {
            println!("Pending snapshots:");
            for key in &snapshot_keys {
                println!("  {}", key);
            }
            println!();
            println!(
                "To review a snapshot: cargo insta review --snapshot '{}'",
                snapshot_keys[0]
            );
            println!(
                "To accept a snapshot: cargo insta accept --snapshot '{}'",
                snapshot_keys[0]
            );
            println!(
                "To reject a snapshot: cargo insta reject --snapshot '{}'",
                snapshot_keys[0]
            );
            println!();
            println!("To review all interactively: cargo insta review");
            println!("To accept all: cargo insta accept");
            println!("To reject all: cargo insta reject");
        }
    }

    Ok(())
}

fn show_undiscovered_hint(
    find_flags: FindFlags,
    snapshot_containers: &[SnapshotContainer],
    roots: &HashSet<PathBuf>,
    extensions: &[&str],
) {
    // there is nothing to do if we already search everything.
    if find_flags.include_hidden && find_flags.include_ignored {
        return;
    }

    let found_snapshots = snapshot_containers
        .iter()
        .filter_map(|x| x.snapshot_file())
        .map(|x| x.to_path_buf())
        .collect::<HashSet<_>>();

    let all_snapshots: HashSet<_> = roots
        .iter()
        .flat_map(|root| {
            make_snapshot_walker(
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
                extensions
                    .iter()
                    .any(|ext| fname.ends_with(&format!(".{ext}.new")))
                    || fname.ends_with(".pending-snap")
            })
            .map(|x| x.path().to_path_buf())
        })
        .collect();

    let missed_snapshots = all_snapshots.difference(&found_snapshots).collect_vec();

    // we did not find any extra snapshots
    if missed_snapshots.is_empty() {
        return;
    }

    let (args, paths) = match (find_flags.include_ignored, find_flags.include_hidden) {
        (false, true) => ("--include-ignored", "ignored"),
        (true, false) => ("--include-hidden", "hidden"),
        (false, false) => (
            "--include-ignored and --include-hidden",
            "ignored or hidden",
        ),
        (true, true) => unreachable!(),
    };

    eprintln!(
        "{}: {}",
        style("warning").yellow().bold(),
        format_args!(
            "found undiscovered pending snapshots in some paths which are not picked up by cargo \
            insta. Use {} if you have snapshots in {} paths. Files:\n{}",
            args,
            paths,
            missed_snapshots
                .iter()
                .map(|x| x.display().to_string())
                .join("\n")
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
            review_snapshots(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_cargo_nextest_command_from_env_variables() {
        env::set_var("INSTA_CARGO_NEXTEST_BIN", "/a/custom/path/to/cargo-nextest");
        let command = get_cargo_nextest_command();
        assert_eq!(
            command.get_program().to_string_lossy(),
            "/a/custom/path/to/cargo-nextest"
        );
        assert_eq!(command.get_args().len(), 0);
        env::remove_var("INSTA_CARGO_NEXTEST_BIN");

        env::set_var("CARGO", "/a/path/to/cargo");
        let command = get_cargo_nextest_command();
        assert_eq!(command.get_program().to_string_lossy(), "/a/path/to/cargo");
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        assert_eq!(args, vec!["nextest"]);
        env::remove_var("CARGO");
    }
}
