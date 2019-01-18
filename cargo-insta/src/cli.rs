use std::path::{Path, PathBuf};

use console::{style, Key, Term, set_colors_enabled};
use failure::{Error, err_msg};
use structopt::clap::AppSettings;
use structopt::StructOpt;

use crate::cargo::{find_packages, get_package_metadata, Package, SnapshotRef};

#[derive(StructOpt, Debug)]
#[structopt(bin_name = "cargo-insta")]
pub enum Opts {
    /// Review snapshots
    #[structopt(
        name = "review",
        raw(
            setting = "AppSettings::UnifiedHelpMessage",
            setting = "AppSettings::DeriveDisplayOrder",
            setting = "AppSettings::DontCollapseArgsInUsage"
        )
    )]
    Review(ReviewCommand),
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
pub struct ReviewCommand {
    /// Review all packages
    #[structopt(long)]
    pub all: bool,

    /// Path to Cargo.toml
    #[structopt(long, value_name = "PATH", parse(from_os_str))]
    pub manifest_path: Option<PathBuf>,

    /// Coloring: auto, always, never
    #[structopt(long, value_name = "WHEN")]
    pub color: Option<String>,
}

fn review_snapshot(
    term: &Term,
    cargo_workspace: &Path,
    snapshot_ref: &SnapshotRef,
    pkg: &Package,
    i: usize,
    n: usize,
) -> Result<Option<bool>, Error> {
    term.clear_screen()?;

    let old = snapshot_ref.load_old()?;
    let new = snapshot_ref.load_new()?;

    let path = snapshot_ref
        .path()
        .strip_prefix(cargo_workspace)
        .ok()
        .unwrap_or_else(|| snapshot_ref.path());

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
            Key::Char('a') | Key::Enter => {
                snapshot_ref.accept()?;
                return Ok(Some(true));
            }
            Key::Char('r') | Key::Escape => {
                snapshot_ref.discard()?;
                return Ok(Some(false));
            }
            Key::Char('s') | Key::Char(' ') => {
                return Ok(None);
            }
            _ => {}
        }
    }
}

fn handle_color(color: &Option<String>) -> Result<(), Error> {
    match color.as_ref().map(|x| x.as_str()).unwrap_or("auto") {
        "always" => set_colors_enabled(true),
        "auto" => {},
        "never" => set_colors_enabled(false),
        color => return Err(err_msg(format!("invalid value for --color: {}", color)))
    }
    Ok(())
}

fn review_packages(cmd: &ReviewCommand) -> Result<(), Error> {
    handle_color(&cmd.color)?;
    let term = Term::stdout();
    let manifest = get_package_metadata(cmd.manifest_path.as_ref().map(|x| x.as_path()))?;
    let packages = find_packages(&manifest, cmd.all)?;
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
        let action = review_snapshot(
            &term,
            manifest.workspace_root(),
            snapshot,
            package,
            idx + 1,
            snapshots.len(),
        )?;
        match action {
            Some(true) => accepted.push(snapshot.path().to_path_buf()),
            Some(false) => rejected.push(snapshot.path().to_path_buf()),
            None => skipped.push(snapshot.path().to_path_buf()),
        }
    }

    term.clear_screen()?;

    println!("{}", style("insta review finished").bold());
    if !accepted.is_empty() {
        println!("{}:", style("accept").green());
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
    match opts {
        Opts::Review(cmd) => review_packages(&cmd),
    }
}
