use std::path::{Path, PathBuf};

use console::{set_colors_enabled, style, Key, Term};
use failure::{err_msg, Error};
use structopt::clap::AppSettings;
use structopt::StructOpt;

use crate::cargo::{find_packages, get_package_metadata, Metadata, Package, SnapshotRef};

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
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
pub struct PackageArgs {
    /// Review all packages
    #[structopt(long)]
    pub all: bool,

    /// Path to Cargo.toml
    #[structopt(long, value_name = "PATH", parse(from_os_str))]
    pub manifest_path: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
pub struct ProcessCommand {
    #[structopt(flatten)]
    pub pkg_args: PackageArgs,
}

#[derive(Clone, Copy, Debug)]
enum Operation {
    Accept,
    Reject,
    Skip,
}

fn process_snapshot(
    term: &Term,
    cargo_workspace: &Path,
    snapshot_ref: &SnapshotRef,
    pkg: &Package,
    i: usize,
    n: usize,
) -> Result<Operation, Error> {
    let old = snapshot_ref.load_old()?;
    let new = snapshot_ref.load_new()?;

    let path = snapshot_ref
        .path()
        .strip_prefix(cargo_workspace)
        .ok()
        .unwrap_or_else(|| snapshot_ref.path());

    term.clear_screen()?;
    println!(
        "{}{}{} {} in {} ({})",
        style("Reviewing [").bold(),
        style(format!("{}/{}", i, n)).yellow().bold(),
        style("]:").bold(),
        style(path.display()).cyan().underlined().bold(),
        style(pkg.name()).dim(),
        pkg.version()
    );

    println!("Snapshot: {}", style(new.snapshot_name()).yellow());
    new.print_changes(old.as_ref());

    println!("");
    println!(
        "  {} accept   {}",
        style("A").green().bold(),
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
    let snapshots: Vec<_> = packages
        .iter()
        .flat_map(|p| p.iter_snapshots().map(move |s| (s, p)))
        .collect();

    if snapshots.is_empty() {
        println!("{}: no snapshots to review", style("done").bold());
        return Ok(());
    }

    let mut accepted = vec![];
    let mut rejected = vec![];
    let mut skipped = vec![];

    for (idx, (snapshot, package)) in snapshots.iter().enumerate() {
        let op = match op {
            Some(op) => op,
            None => process_snapshot(
                &term,
                metadata.workspace_root(),
                snapshot,
                package,
                idx + 1,
                snapshots.len(),
            )?,
        };
        let path = snapshot.path().to_path_buf();
        match op {
            Operation::Accept => {
                snapshot.accept()?;
                accepted.push(path);
            }
            Operation::Reject => {
                snapshot.reject()?;
                rejected.push(path);
            }
            Operation::Skip => {
                skipped.push(path);
            }
        }
    }

    if op.is_none() {
        term.clear_screen()?;
    }

    println!("{}", style("insta review finished").bold());
    if !accepted.is_empty() {
        println!("{}:", style("accepted").green());
        for item in accepted {
            println!("  {}", item.display());
        }
    }
    if !rejected.is_empty() {
        println!("{}:", style("rejected").red());
        for item in rejected {
            println!("  {}", item.display());
        }
    }
    if !skipped.is_empty() {
        println!("{}:", style("skipped").yellow());
        for item in skipped {
            println!("  {}", item.display());
        }
    }

    Ok(())
}

pub fn run() -> Result<(), Error> {
    let opts = Opts::from_args();
    handle_color(&opts.color)?;
    match opts.command {
        Command::Review(cmd) => process_snapshots(&cmd, None),
        Command::Accept(cmd) => process_snapshots(&cmd, Some(Operation::Accept)),
        Command::Reject(cmd) => process_snapshots(&cmd, Some(Operation::Reject)),
    }
}
