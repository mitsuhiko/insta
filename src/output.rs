use std::{path::Path, time::Duration};

use similar::{Algorithm, ChangeTag, TextDiff};

use crate::snapshot::Snapshot;
use crate::utils::{format_rust_expression, style, term_width};

/// Prints the summary of a snapshot
pub fn print_snapshot_summary(
    workspace_root: &Path,
    snapshot: &Snapshot,
    snapshot_file: Option<&Path>,
    mut line: Option<u32>,
) {
    // default to old assertion line from snapshot.
    if line.is_none() {
        line = snapshot.metadata().assertion_line();
    }

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
    mut line: Option<u32>,
) {
    // default to old assertion line from snapshot.
    if line.is_none() {
        line = new.metadata().assertion_line();
    }

    print_snapshot_summary(workspace_root, new, snapshot_file, line);
    let old_contents = old_snapshot.as_ref().map_or("", |x| x.contents_str());
    let new_contents = new.contents_str();
    if !old_contents.is_empty() {
        println!("{}", style("-old snapshot").red());
        println!("{}", style("+new results").green());
    } else {
        println!("{}", style("+new results").green());
    }
    print_changeset(
        old_contents,
        new_contents,
        new.metadata().expression.as_deref(),
    );
}

pub fn print_snapshot_diff_with_title(
    workspace_root: &Path,
    new_snapshot: &Snapshot,
    old_snapshot: Option<&Snapshot>,
    line: u32,
    snapshot_file: Option<&Path>,
) {
    let width = term_width();
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

pub fn print_snapshot_summary_with_title(
    workspace_root: &Path,
    new_snapshot: &Snapshot,
    old_snapshot: Option<&Snapshot>,
    line: u32,
    snapshot_file: Option<&Path>,
) {
    let _old_snapshot = old_snapshot;
    let width = term_width();
    println!(
        "{title:━^width$}",
        title = style(" Snapshot Summary ").bold(),
        width = width
    );
    print_snapshot_summary(workspace_root, new_snapshot, snapshot_file, Some(line));
    println!("{title:━^width$}", title = "", width = width);
}

pub fn print_changeset(old: &str, new: &str, expr: Option<&str>) {
    let width = term_width();
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .timeout(Duration::from_millis(500))
        .diff_lines(old, new);

    if let Some(expr) = expr {
        println!("{:─^1$}", "", width,);
        println!("{}", style(format_rust_expression(expr)));
    }
    println!("────────────┬{:─^1$}", "", width.saturating_sub(13));
    let mut has_changes = false;
    for (idx, group) in diff.grouped_ops(4).iter().enumerate() {
        if idx > 0 {
            println!("┈┈┈┈┈┈┈┈┈┈┈┈┼{:┈^1$}", "", width.saturating_sub(13));
        }
        for op in group {
            for change in diff.iter_inline_changes(&op) {
                match change.tag() {
                    ChangeTag::Insert => {
                        has_changes = true;
                        print!(
                            "{:>5} {:>5} │{}",
                            "",
                            style(change.new_index().unwrap()).cyan().dim().bold(),
                            style("+").green(),
                        );
                        for &(emphasized, change) in change.values() {
                            if emphasized {
                                print!("{}", style(change).green().underlined());
                            } else {
                                print!("{}", style(change).green());
                            }
                        }
                    }
                    ChangeTag::Delete => {
                        has_changes = true;
                        print!(
                            "{:>5} {:>5} │{}",
                            style(change.old_index().unwrap()).cyan().dim(),
                            "",
                            style("-").red(),
                        );
                        for &(emphasized, change) in change.values() {
                            if emphasized {
                                print!("{}", style(change).red().underlined());
                            } else {
                                print!("{}", style(change).red());
                            }
                        }
                    }
                    ChangeTag::Equal => {
                        print!(
                            "{:>5} {:>5} │ ",
                            style(change.old_index().unwrap()).cyan().dim(),
                            style(change.new_index().unwrap()).cyan().dim().bold(),
                        );
                        for &(_, change) in change.values() {
                            print!("{}", style(change).dim());
                        }
                    }
                }
                if change.missing_newline() {
                    println!();
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
