use std::env;
use std::path::{Path, PathBuf};
use std::process;

use console::{set_colors_enabled, style, Key, Term};
use failure::{err_msg, Error, Fail};
use insta::{print_snapshot_diff, Snapshot};
use structopt::clap::AppSettings;
use structopt::StructOpt;

use crate::cargo::{find_packages, get_cargo, get_package_metadata, Metadata, Operation, Package};

/// Close without message but exit code.
#[derive(Fail, Debug)]
#[fail(display = "exit with {}", _0)]
pub struct QuietExit(pub i32);

/// A helper utility to work with insta snapshots.
#[derive(StructOpt, Debug)]
#[structopt(
    bin_name = "cargo-insta",
    raw(
        setting = "AppSettings::ArgRequiredElseHelp",
        global_setting = "AppSettings::UnifiedHelpMessage",
        global_setting = "AppSettings::DeriveDisplayOrder",
        global_setting = "AppSettings::DontCollapseArgsInUsage"
    )
)]
pub struct Opts {
    /// Coloring: auto, always, never
    #[structopt(long, raw(global = "true"), value_name = "WHEN")]
    pub color: Option<String>,

    #[structopt(subcommand)]
    pub command: Command,
}

#[derive(StructOpt, Debug)]
#[structopt(bin_name = "cargo-insta")]
pub enum Command {
    /// Interactively review snapshots
    #[structopt(name = "review")]
    Review(ProcessCommand),
    /// Rejects all snapshots
    #[structopt(name = "reject")]
    Reject(ProcessCommand),
    /// Accept all snapshots
    #[structopt(name = "accept")]
    Accept(ProcessCommand),
    /// Run tests and then reviews
    #[structopt(name = "test")]
    Test(TestCommand),
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(rename_all = "kebab-case")]
pub struct PackageArgs {
    /// Path to Cargo.toml
    #[structopt(long, value_name = "PATH", parse(from_os_str))]
    pub manifest_path: Option<PathBuf>,

    /// Work on all packages in the workspace
    #[structopt(long)]
    pub all: bool,
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
pub struct ProcessCommand {
    #[structopt(flatten)]
    pub pkg_args: PackageArgs,
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
pub struct TestCommand {
    #[structopt(flatten)]
    pub pkg_args: PackageArgs,
    /// Package to run tests for
    #[structopt(short = "p", long)]
    pub package: Option<String>,
    /// Disable force-passing of snapshot tests
    #[structopt(long)]
    pub no_force_pass: bool,
    /// Prevent running all tests regardless of failure
    #[structopt(long)]
    pub fail_fast: bool,
    /// Space-separated list of features to activate
    #[structopt(long, value_name = "FEATURES")]
    pub features: Option<String>,
    /// Activate all available features
    #[structopt(long)]
    pub all_features: bool,
    /// Do not activate the `default` feature
    #[structopt(long)]
    pub no_default_features: bool,
    /// Follow up with review.
    #[structopt(long)]
    pub review: bool,
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
) -> Result<Operation, Error> {
    term.clear_screen()?;
    println!(
        "{}{}{} {} ({})",
        style("Reviewing [").bold(),
        style(format!("{}/{}", i, n)).yellow().bold(),
        style("]:").bold(),
        style(pkg.name()).dim(),
        pkg.version()
    );

    print_snapshot_diff(workspace_root, new, old, snapshot_file, line);

    println!();
    println!(
        "  {} accept   {}",
        style("a").green().bold(),
        style("keep the new snapshot").dim()
    );
    println!(
        "  {} reject   {}",
        style("r").red().bold(),
        style("keep the old snapshot").dim()
    );
    println!(
        "  {} skip     {}",
        style("s").yellow().bold(),
        style("keep both for now").dim()
    );

    loop {
        match term.read_key()? {
            Key::Char('a') | Key::Enter => break Ok(Operation::Accept),
            Key::Char('r') | Key::Escape => break Ok(Operation::Reject),
            Key::Char('s') | Key::Char(' ') => break Ok(Operation::Skip),
            _ => {}
        }
    }
}

fn handle_color(color: &Option<String>) -> Result<(), Error> {
    match color.as_ref().map(|x| x.as_str()).unwrap_or("auto") {
        "always" => set_colors_enabled(true),
        "auto" => {}
        "never" => set_colors_enabled(false),
        color => return Err(err_msg(format!("invalid value for --color: {}", color))),
    }
    Ok(())
}

fn handle_pkg_args(pkg_args: &PackageArgs) -> Result<(Metadata, Vec<Package>), Error> {
    let metadata = get_package_metadata(pkg_args.manifest_path.as_ref().map(|x| x.as_path()))?;
    let packages = find_packages(&metadata, pkg_args.all)?;
    Ok((metadata, packages))
}

fn process_snapshots(cmd: &ProcessCommand, op: Option<Operation>) -> Result<(), Error> {
    let term = Term::stdout();
    let (metadata, packages) = handle_pkg_args(&cmd.pkg_args)?;
    let mut snapshot_containers = vec![];
    for package in &packages {
        for snapshot_container in package.iter_snapshot_containers() {
            snapshot_containers.push((snapshot_container?, package));
        }
    }
    let snapshot_count = snapshot_containers.iter().map(|x| x.0.len()).sum();

    if snapshot_count == 0 {
        println!("{}: no snapshots to review", style("done").bold());
        return Ok(());
    }

    let mut accepted = vec![];
    let mut rejected = vec![];
    let mut skipped = vec![];
    let mut num = 0;

    for (snapshot_container, package) in snapshot_containers.iter_mut() {
        let snapshot_file = snapshot_container.snapshot_file().map(|x| x.to_path_buf());
        for snapshot_ref in snapshot_container.iter_snapshots() {
            num += 1;
            let op = match op {
                Some(op) => op,
                None => query_snapshot(
                    metadata.workspace_root(),
                    &term,
                    &snapshot_ref.new,
                    snapshot_ref.old.as_ref(),
                    package,
                    snapshot_ref.line,
                    num,
                    snapshot_count,
                    snapshot_file.as_ref().map(|x| x.as_path()),
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

    Ok(())
}

fn test_run(cmd: &TestCommand) -> Result<(), Error> {
    let mut proc = process::Command::new(get_cargo());
    proc.arg("test");
    if cmd.pkg_args.all {
        proc.arg("--all");
    }
    if let Some(ref pkg) = cmd.package {
        proc.arg("--package");
        proc.arg(pkg);
    }
    if let Some(ref manifest_path) = cmd.pkg_args.manifest_path {
        proc.arg("--manifest-path");
        proc.arg(manifest_path);
    }
    if !cmd.fail_fast {
        proc.arg("--no-fail-fast");
    }
    if !cmd.no_force_pass {
        proc.env("INSTA_FORCE_PASS", "1");
    }
    if cmd.review {
        proc.env("INSTA_UPDATE", "new");
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
    proc.arg("--");
    proc.arg("-q");
    let status = proc.status()?;

    if !status.success() {
        if cmd.review {
            eprintln!(
                "{} non snapshot tests failed, skipping review",
                style("warning:").bold().yellow()
            );
        }
        return Err(QuietExit(1).into());
    }

    if cmd.review {
        process_snapshots(
            &ProcessCommand {
                pkg_args: cmd.pkg_args.clone(),
            },
            None,
        )?
    }

    Ok(())
}

pub fn run() -> Result<(), Error> {
    // chop off cargo
    let mut args = env::args_os();
    args.next().unwrap();

    let opts = Opts::from_iter(args);
    handle_color(&opts.color)?;
    match opts.command {
        Command::Review(cmd) => process_snapshots(&cmd, None),
        Command::Accept(cmd) => process_snapshots(&cmd, Some(Operation::Accept)),
        Command::Reject(cmd) => process_snapshots(&cmd, Some(Operation::Reject)),
        Command::Test(cmd) => test_run(&cmd),
    }
}
