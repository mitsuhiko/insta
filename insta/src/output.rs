use std::borrow::Cow;
use std::process::Command;
use std::{env, path::Path, time::Duration};

use similar::{Algorithm, ChangeTag, TextDiff};

use crate::content::yaml;
use crate::snapshot::{MetaData, Snapshot, SnapshotContents};
use crate::utils::{format_rust_expression, style, term_width};

/// Snapshot printer utility.
pub struct SnapshotPrinter<'a> {
    workspace_root: &'a Path,
    old_snapshot: Option<&'a Snapshot>,
    new_snapshot: &'a Snapshot,
    old_snapshot_hint: &'a str,
    new_snapshot_hint: &'a str,
    show_info: bool,
    show_diff: bool,
    title: Option<&'a str>,
    line: Option<u32>,
    snapshot_file: Option<&'a Path>,
}

impl<'a> SnapshotPrinter<'a> {
    pub fn new(
        workspace_root: &'a Path,
        old_snapshot: Option<&'a Snapshot>,
        new_snapshot: &'a Snapshot,
    ) -> SnapshotPrinter<'a> {
        SnapshotPrinter {
            workspace_root,
            old_snapshot,
            new_snapshot,
            old_snapshot_hint: "old snapshot",
            new_snapshot_hint: "new results",
            show_info: false,
            show_diff: false,
            title: None,
            line: None,
            snapshot_file: None,
        }
    }

    pub fn set_snapshot_hints(&mut self, old: &'a str, new: &'a str) {
        self.old_snapshot_hint = old;
        self.new_snapshot_hint = new;
    }

    pub fn set_show_info(&mut self, yes: bool) {
        self.show_info = yes;
    }

    pub fn set_show_diff(&mut self, yes: bool) {
        self.show_diff = yes;
    }

    pub fn set_title(&mut self, title: Option<&'a str>) {
        self.title = title;
    }

    pub fn set_line(&mut self, line: Option<u32>) {
        self.line = line;
    }

    pub fn set_snapshot_file(&mut self, file: Option<&'a Path>) {
        self.snapshot_file = file;
    }

    pub fn print(&self) {
        if let Some(title) = self.title {
            let width = term_width();
            println!(
                "{title:━^width$}",
                title = style(format!(" {title} ")).bold(),
                width = width
            );
        }
        self.print_snapshot_diff();
    }

    fn print_snapshot_diff(&self) {
        self.print_snapshot_summary();
        if self.show_diff {
            self.print_changeset();
        } else {
            self.print_snapshot();
        }
    }

    fn print_snapshot_summary(&self) {
        print_snapshot_summary(
            self.workspace_root,
            self.new_snapshot,
            self.snapshot_file,
            self.line,
        );
    }

    fn print_info(&self) {
        print_info(self.new_snapshot.metadata());
    }

    fn print_snapshot(&self) {
        print_line(term_width());

        let width = term_width();
        if self.show_info {
            self.print_info();
        }
        println!("Snapshot Contents:");

        match self.new_snapshot.contents() {
            SnapshotContents::Text(new_contents) => {
                let new_contents = new_contents.to_string();

                println!("──────┬{:─^1$}", "", width.saturating_sub(7));
                for (idx, line) in new_contents.lines().enumerate() {
                    println!("{:>5} │ {}", style(idx + 1).cyan().dim().bold(), line);
                }
                println!("──────┴{:─^1$}", "", width.saturating_sub(7));
            }
            SnapshotContents::Binary(_) => {
                println!(
                    "{}",
                    encode_file_link_escape(
                        &self
                            .new_snapshot
                            .build_binary_path(
                                self.snapshot_file.unwrap().with_extension("snap.new")
                            )
                            .unwrap()
                    )
                );
            }
        }
    }

    fn print_changeset(&self) {
        let width = term_width();
        print_line(width);

        if self.show_info {
            self.print_info();
        }

        if let Some(old_snapshot) = self.old_snapshot {
            if old_snapshot.contents().is_binary() {
                println!(
                    "{}",
                    style(format_args!(
                        "-{}: {}",
                        self.old_snapshot_hint,
                        encode_file_link_escape(
                            &old_snapshot
                                .build_binary_path(self.snapshot_file.unwrap())
                                .unwrap()
                        ),
                    ))
                    .red()
                );
            }
        }

        if self.new_snapshot.contents().is_binary() {
            println!(
                "{}",
                style(format_args!(
                    "+{}: {}",
                    self.new_snapshot_hint,
                    encode_file_link_escape(
                        &self
                            .new_snapshot
                            .build_binary_path(
                                self.snapshot_file.unwrap().with_extension("snap.new")
                            )
                            .unwrap()
                    ),
                ))
                .green()
            );
        }

        if let Some((old, new)) = match (
            self.old_snapshot.as_ref().map(|o| o.contents()),
            self.new_snapshot.contents(),
        ) {
            (Some(SnapshotContents::Binary(_)) | None, SnapshotContents::Text(new)) => {
                Some((None, Some(new.to_string())))
            }
            (Some(SnapshotContents::Text(old)), SnapshotContents::Binary { .. }) => {
                Some((Some(old.to_string()), None))
            }
            (Some(SnapshotContents::Text(old)), SnapshotContents::Text(new)) => {
                Some((Some(old.to_string()), Some(new.to_string())))
            }
            _ => None,
        } {
            let old_text = old.as_deref().unwrap_or("");
            let new_text = new.as_deref().unwrap_or("");

            // Check for external diff tool
            if let Ok(tool) = env::var("INSTA_DIFF_TOOL") {
                if !tool.is_empty()
                    && invoke_external_diff_tool(&tool, old_text, new_text, self.snapshot_file)
                {
                    println!(); // Add spacing after external tool output
                    return;
                }
            }

            let newlines_matter = newlines_matter(old_text, new_text);
            let diff = TextDiff::configure()
                .algorithm(Algorithm::Patience)
                .timeout(Duration::from_millis(500))
                .diff_lines(old_text, new_text);

            if old.is_some() {
                println!(
                    "{}",
                    style(format_args!("-{}", self.old_snapshot_hint)).red()
                );
            }

            if new.is_some() {
                println!(
                    "{}",
                    style(format_args!("+{}", self.new_snapshot_hint)).green()
                );
            }

            println!("────────────┬{:─^1$}", "", width.saturating_sub(13));

            // This is to make sure that binary and text snapshots are never reported as being
            // equal (that would otherwise happen if the text snapshot is an empty string).
            let mut has_changes = old.is_none() || new.is_none();

            for (idx, group) in diff.grouped_ops(4).iter().enumerate() {
                if idx > 0 {
                    println!("┈┈┈┈┈┈┈┈┈┈┈┈┼{:┈^1$}", "", width.saturating_sub(13));
                }
                for op in group {
                    for change in diff.iter_inline_changes(op) {
                        match change.tag() {
                            ChangeTag::Insert => {
                                has_changes = true;
                                print!(
                                    "{:>5} {:>5} │{}",
                                    "",
                                    style(change.new_index().unwrap() + 1).cyan().dim().bold(),
                                    style("+").green(),
                                );
                                for &(emphasized, change) in change.values() {
                                    let change = render_invisible(change, newlines_matter);
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
                                    style(change.old_index().unwrap() + 1).cyan().dim(),
                                    "",
                                    style("-").red(),
                                );
                                for &(emphasized, change) in change.values() {
                                    let change = render_invisible(change, newlines_matter);
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
                                    style(change.old_index().unwrap() + 1).cyan().dim(),
                                    style(change.new_index().unwrap() + 1).cyan().dim().bold(),
                                );
                                for &(_, change) in change.values() {
                                    let change = render_invisible(change, newlines_matter);
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

            println!("────────────┴{:─^1$}", "", width.saturating_sub(13));
        }
    }
}

/// Prints the summary of a snapshot
pub fn print_snapshot_summary(
    workspace_root: &Path,
    snapshot: &Snapshot,
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
    if let Some(name) = snapshot.snapshot_name() {
        println!("Snapshot: {}", style(name).yellow());
    } else {
        println!("Snapshot: {}", style("<inline>").dim());
    }

    if let Some(ref value) = snapshot.metadata().get_relative_source(workspace_root) {
        println!(
            "Source: {}{}",
            style(value.display()).cyan(),
            line.or(
                // default to old assertion line from snapshot.
                snapshot.metadata().assertion_line()
            )
            .map(|line| format!(":{}", style(line).bold()))
            .unwrap_or_default()
        );
    }

    if let Some(ref value) = snapshot.metadata().input_file() {
        println!("Input file: {}", style(value).cyan());
    }
}

fn print_line(width: usize) {
    println!("{:─^1$}", "", width);
}

fn trailing_newline(s: &str) -> &str {
    if s.ends_with("\r\n") {
        "\r\n"
    } else if s.ends_with('\r') {
        "\r"
    } else if s.ends_with('\n') {
        "\n"
    } else {
        ""
    }
}

fn detect_newlines(s: &str) -> (bool, bool, bool) {
    let mut last_char = None;
    let mut detected_crlf = false;
    let mut detected_cr = false;
    let mut detected_lf = false;

    for c in s.chars() {
        if c == '\n' {
            if last_char.take() == Some('\r') {
                detected_crlf = true;
            } else {
                detected_lf = true;
            }
        }
        if last_char == Some('\r') {
            detected_cr = true;
        }
        last_char = Some(c);
    }
    if last_char == Some('\r') {
        detected_cr = true;
    }

    (detected_cr, detected_crlf, detected_lf)
}

fn newlines_matter(left: &str, right: &str) -> bool {
    if trailing_newline(left) != trailing_newline(right) {
        return true;
    }

    let (cr1, crlf1, lf1) = detect_newlines(left);
    let (cr2, crlf2, lf2) = detect_newlines(right);

    !matches!(
        (cr1 || cr2, crlf1 || crlf2, lf1 || lf2),
        (false, false, false) | (true, false, false) | (false, true, false) | (false, false, true)
    )
}

fn render_invisible(s: &str, newlines_matter: bool) -> Cow<'_, str> {
    if newlines_matter || s.find(&['\x1b', '\x07', '\x08', '\x7f'][..]).is_some() {
        Cow::Owned(
            s.replace('\r', "␍\r")
                .replace('\n', "␊\n")
                .replace("␍\r␊\n", "␍␊\r\n")
                .replace('\x07', "␇")
                .replace('\x08', "␈")
                .replace('\x1b', "␛")
                .replace('\x7f', "␡"),
        )
    } else {
        Cow::Borrowed(s)
    }
}

fn print_info(metadata: &MetaData) {
    let width = term_width();
    if let Some(expr) = metadata.expression() {
        println!("Expression: {}", style(format_rust_expression(expr)));
        print_line(width);
    }
    if let Some(descr) = metadata.description() {
        println!("{descr}");
        print_line(width);
    }
    if let Some(info) = metadata.private_info() {
        let out = yaml::to_string(info);
        // TODO: does the yaml output always start with '---'?
        println!("{}", out.trim().strip_prefix("---").unwrap().trim_start());
        print_line(width);
    }
}

/// Encodes a path as an OSC-8 escape sequence. This makes it a clickable link in supported
/// terminal emulators.
fn encode_file_link_escape(path: &Path) -> String {
    assert!(path.is_absolute());
    format!(
        "\x1b]8;;file://{}\x1b\\{}\x1b]8;;\x1b\\",
        path.display(),
        path.display()
    )
}

/// Invokes an external diff tool with the old and new snapshot contents.
///
/// Returns `true` if the external tool was successfully invoked, `false` if it failed
/// (in which case the caller should fall back to the built-in diff).
///
/// This function is public for testing purposes.
#[doc(hidden)]
pub fn invoke_external_diff_tool(
    tool: &str,
    old_content: &str,
    new_content: &str,
    snapshot_file: Option<&Path>,
) -> bool {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("warning: failed to create temp dir for diff tool: {err}");
            return false;
        }
    };

    // Use snapshot file stem for naming (helps diff tools with syntax detection).
    // Fall back to generic name - these are ephemeral temp files anyway.
    let base_name = snapshot_file
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("snapshot");
    let old_path = dir.path().join(format!("{base_name}.old.snap"));
    let new_path = dir.path().join(format!("{base_name}.new.snap"));

    // Write old content
    if let Err(err) = std::fs::write(&old_path, old_content) {
        eprintln!("warning: failed to write old snapshot to temp file: {err}");
        return false;
    }

    // Write new content
    if let Err(err) = std::fs::write(&new_path, new_content) {
        eprintln!("warning: failed to write new snapshot to temp file: {err}");
        return false;
    }

    // Invoke the diff tool from the temp directory so paths are relative/clean.
    // We capture stdout/stderr and print them ourselves so the output goes through
    // the same capture mechanism as the built-in diff (important for cargo test).
    let old_filename = old_path.file_name().unwrap();
    let new_filename = new_path.file_name().unwrap();

    // Split tool string to support arguments (e.g., "delta --side-by-side")
    let mut parts = tool.split_whitespace();
    let cmd = match parts.next() {
        Some(cmd) => cmd,
        None => return false,
    };
    let mut command = Command::new(cmd);
    command.args(parts);
    command.current_dir(dir.path());
    command.arg(old_filename);
    command.arg(new_filename);

    match command.output() {
        Ok(output) => {
            // Print captured output through normal channels so it gets captured
            // by cargo test when appropriate (just like the built-in diff)
            if !output.stdout.is_empty() {
                print!("{}", String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                eprint!("{}", String::from_utf8_lossy(&output.stderr));
            }
            // Non-zero exit is normal for diff tools when files differ
            true
        }
        Err(err) => {
            eprintln!("warning: failed to invoke diff tool `{tool}`: {err}");
            false
        }
    }
    // Temp dir is cleaned up when `dir` goes out of scope
}

#[test]
fn test_invisible() {
    assert_eq!(
        render_invisible("\r\n\x1b\r\x07\x08\x7f\n", true),
        "␍␊\r\n␛␍\r␇␈␡␊\n"
    );
}
