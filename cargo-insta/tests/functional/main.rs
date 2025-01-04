/// Integration tests which allow creating a full repo, running `cargo-insta`
/// and then checking the output.
///
/// By default, the output of the inner test is forwarded to the outer test with
/// a colored prefix. If we want to assert the inner test contains some output,
/// we need to disable that forwarding with `Stdio::piped()` like:
///
/// ```rust
/// let output = test_project
///     .insta_cmd()
///     .args(["test"])
///     .stderr(Stdio::piped())
///
/// assert!(
///     String::from_utf8_lossy(&output.stderr).contains("info: 2 snapshots to review"),
///    "{}",
///     String::from_utf8_lossy(&output.stderr)
/// );
/// ```
///
/// Often we want to see output from the test commands we run here; for example
/// a `dbg` statement we add while debugging. Cargo by default hides the output
/// of passing tests.
/// - Like any test, to forward the output of a passing outer test (i.e. one of
///   the `#[test]`s in this file) to the terminal, pass `--nocapture` to the
///   test runner, like `cargo insta test -- --nocapture`.
/// - To forward the output of a passing inner test (i.e. the test commands we
///   create and run within an outer test) to the output of an outer test, pass
///   `--nocapture` in the command we create; for example `.args(["test",
///   "--accept", "--", "--nocapture"])`.
///   - We also need to pass `--nocapture` to the outer test to forward that to
///     the terminal, per the previous bullet.
///
/// Note that the packages must have different names, or we'll see interference
/// between the tests.
///
/// > That seems to be because they all share the same `target` directory, which
/// > cargo will confuse for each other if they share the same name. I haven't
/// > worked out why — this is the case even if the files are the same between
/// > two tests but with different commands — and those files exist in different
/// > temporary workspace dirs. (We could try to enforce different names, or
/// > give up using a consistent target directory for a cache, but it would slow
/// > down repeatedly running the tests locally. To demonstrate the effect, name
/// > crates the same... This also causes issues when running the same tests
/// > concurrently.
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::thread;

use console::style;
use ignore::WalkBuilder;
use insta::assert_snapshot;
use itertools::Itertools;
use similar::udiff::unified_diff;
use tempfile::TempDir;

mod binary;
mod delete_pending;
mod inline;
mod workspace;

/// Wraps a formatting function to be used as a `Stdio`
struct OutputFormatter<F>(F)
where
    F: Fn(&str) -> String + Send + 'static;

impl<F> From<OutputFormatter<F>> for Stdio
where
    F: Fn(&str) -> String + Send + 'static,
{
    // Creates a pipe, spawns a thread to read from the pipe, applies the
    // formatting function to each line, and prints the result.
    fn from(output: OutputFormatter<F>) -> Stdio {
        let (read_end, write_end) = os_pipe::pipe().unwrap();

        thread::spawn(move || {
            let mut reader = BufReader::new(read_end);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 0 {
                print!("{}", (output.0)(&line));
                line.clear();
            }
        });

        Stdio::from(write_end)
    }
}

struct TestFiles {
    files: HashMap<PathBuf, String>,
}

impl TestFiles {
    fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    fn add_file<P: AsRef<Path>>(mut self, path: P, content: String) -> Self {
        self.files.insert(path.as_ref().to_path_buf(), content);
        self
    }

    /// Adds a standard `Cargo.toml` (some tests may need to add_file themselves
    /// with a different format)
    fn add_cargo_toml(self, name: &str) -> Self {
        self.add_file(
            "Cargo.toml",
            format!(
                r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
doctest = false

[dependencies]
insta = {{ path = '$PROJECT_PATH' }}
"#,
                name
            ),
        )
    }

    fn create_project(self) -> TestProject {
        TestProject::new(self.files)
    }
}

/// Path of the [`insta`] crate in this repo, which we use as a dependency in the test project
fn insta_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("insta")
        .to_path_buf()
}

/// A shared `target` directory for all tests to use, to allow caching.
fn target_dir() -> PathBuf {
    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| insta_path().join("target"))
        .join("test-projects");
    fs::create_dir_all(&target_dir).unwrap();
    target_dir
}

struct TestProject {
    /// Temporary directory where the project is created
    workspace_dir: PathBuf,
    /// Original files when the project is created.
    files: HashMap<PathBuf, String>,
    /// File tree when the test is created.
    file_tree: String,
}

impl TestProject {
    fn new(files: HashMap<PathBuf, String>) -> TestProject {
        let workspace_dir = TempDir::new().unwrap().into_path();

        // Create files and replace $PROJECT_PATH in all files
        for (path, content) in &files {
            let full_path = workspace_dir.join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let replaced_content = content.replace("$PROJECT_PATH", insta_path().to_str().unwrap());
            fs::write(full_path, replaced_content).unwrap();
        }

        TestProject {
            files,
            file_tree: Self::current_file_tree(&workspace_dir),
            workspace_dir,
        }
    }
    fn clean_env(cmd: &mut Command) {
        // Remove environment variables so we don't inherit anything (such as
        // `INSTA_FORCE_PASS` or `CARGO_INSTA_*`) from a cargo-insta process
        // which runs this integration test.
        for (key, _) in env::vars() {
            if key.starts_with("CARGO_INSTA") || key.starts_with("INSTA") {
                cmd.env_remove(&key);
            }
        }
        // Turn off CI flag so that cargo insta test behaves as we expect
        // under normal operation
        cmd.env("CI", "0");
        // And any others that can affect the output
        cmd.env_remove("CARGO_TERM_COLOR");
        cmd.env_remove("CLICOLOR_FORCE");
        cmd.env_remove("RUSTDOCFLAGS");
    }

    fn insta_cmd(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-insta"));
        Self::clean_env(&mut command);

        command.current_dir(self.workspace_dir.as_path());
        // Use the same target directory as other tests, consistent across test
        // runs. This makes the compilation much faster (though do some tests
        // tread on the toes of others? We could have a different cache for each
        // project if so...)
        command.env("CARGO_TARGET_DIR", target_dir());

        let workspace_name = self
            .workspace_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let stdout_name = workspace_name.clone();
        let stderr_name = workspace_name;

        command
            .stdout(OutputFormatter(move |line| {
                format!("{} {}", style(&stdout_name).green(), line)
            }))
            .stderr(OutputFormatter(move |line| {
                format!("{} {}", style(&stderr_name).yellow(), line)
            }));

        command
    }

    fn diff(&self, file_path: &str) -> String {
        let original_content = self.files.get(Path::new(file_path)).unwrap();
        let file_path_buf = self.workspace_dir.join(file_path);
        let updated_content = fs::read_to_string(&file_path_buf).unwrap();

        unified_diff(
            similar::Algorithm::Patience,
            original_content,
            &updated_content,
            3,
            Some((
                &format!("Original: {}", file_path),
                &format!("Updated: {}", file_path),
            )),
        )
    }

    fn current_file_tree(workspace_dir: &Path) -> String {
        WalkBuilder::new(workspace_dir)
            .hidden(false)
            .filter_entry(|e| e.path().file_name() != Some(std::ffi::OsStr::new("target")))
            .build()
            .filter_map(|e| e.ok())
            .sorted_by(|a, b| a.path().cmp(b.path()))
            .map(|entry| {
                let path = entry
                    .path()
                    .strip_prefix(workspace_dir)
                    .unwrap_or(entry.path());
                // Required for Windows compatibility
                let path_str = path.to_str().map(|s| s.replace('\\', "/")).unwrap();
                format!("{}{}", "  ".repeat(entry.depth()), path_str)
            })
            .chain(std::iter::once(String::new()))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn file_tree_diff(&self) -> String {
        unified_diff(
            similar::Algorithm::Patience,
            &self.file_tree.clone(),
            Self::current_file_tree(&self.workspace_dir).as_ref(),
            3,
            Some(("Original file tree", "Updated file tree")),
        )
    }

    fn update_file<P: AsRef<Path>>(&self, path: P, content: String) {
        fs::write(self.workspace_dir.join(path), content).unwrap();
    }
}

#[test]
fn test_force_update_snapshots() {
    fn create_test_force_update_project(name: &str, insta_dependency: &str) -> TestProject {
        TestFiles::new()
            .add_file(
                "Cargo.toml",
                format!(
                    r#"
[package]
name = "test_force_update_{}"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = {}
"#,
                    name, insta_dependency
                )
                .to_string(),
            )
            .add_file(
                "src/lib.rs",
                r#"
#[test]
fn test_snapshot_with_newline() {
    insta::assert_snapshot!("force_update", "Hello, world!");
}
"#
                .to_string(),
            )
            .add_file(
                format!(
                    "src/snapshots/test_force_update_{}__force_update.snap",
                    name
                ),
                r#"
---
source: src/lib.rs
expression: 
---
Hello, world!


"#
                .to_string(),
            )
            .create_project()
    }

    let test_current_insta =
        create_test_force_update_project("current", "{ path = '$PROJECT_PATH' }");
    let test_insta_1_40_0 = create_test_force_update_project("1_40_0", "\"1.40.0\"");

    // Test with current insta version
    assert!(&test_current_insta
        .insta_cmd()
        .args(["test", "--accept", "--force-update-snapshots"])
        .output()
        .unwrap()
        .status
        .success());

    // Test with insta 1.40.0
    assert!(&test_insta_1_40_0
        .insta_cmd()
        .args(["test", "--accept", "--force-update-snapshots"])
        .output()
        .unwrap()
        .status
        .success());

    // Check that both versions updated the snapshot correctly
    assert_snapshot!(test_current_insta.diff("src/snapshots/test_force_update_current__force_update.snap"), @r#"
    --- Original: src/snapshots/test_force_update_current__force_update.snap
    +++ Updated: src/snapshots/test_force_update_current__force_update.snap
    @@ -1,8 +1,5 @@
    -
     ---
     source: src/lib.rs
    -expression: 
    +expression: "\"Hello, world!\""
     ---
     Hello, world!
    -
    -
    "#);

    assert_snapshot!(test_insta_1_40_0.diff("src/snapshots/test_force_update_1_40_0__force_update.snap"), @r#"
    --- Original: src/snapshots/test_force_update_1_40_0__force_update.snap
    +++ Updated: src/snapshots/test_force_update_1_40_0__force_update.snap
    @@ -1,8 +1,6 @@
    -
     ---
     source: src/lib.rs
    -expression: 
    +expression: "\"Hello, world!\""
    +snapshot_kind: text
     ---
     Hello, world!
    -
    -
    "#);
}

#[test]
fn test_force_update_inline_snapshot_linebreaks() {
    let test_project = TestFiles::new()
        .add_cargo_toml("force-update-inline-linebreaks")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_linebreaks() {
    insta::assert_snapshot!("foo", @r####"
    foo
    
    "####);
}
"#####
                .to_string(),
        )
        .create_project();

    // Run the test with --force-update-snapshots and --accept
    let output = test_project
        .insta_cmd()
        .args(["test", "--force-update-snapshots", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // Linebreaks should be reset
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#####"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -1,8 +1,5 @@
     
     #[test]
     fn test_linebreaks() {
    -    insta::assert_snapshot!("foo", @r####"
    -    foo
    -    
    -    "####);
    +    insta::assert_snapshot!("foo", @"foo");
     }
    "#####);
}

#[test]
fn test_force_update_inline_snapshot_hashes() {
    let test_project = TestFiles::new()
        .add_cargo_toml("force-update-inline-hashes")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_excessive_hashes() {
    insta::assert_snapshot!("foo", @r####"foo"####);
}
"#####
                .to_string(),
        )
        .create_project();

    // Run the test with --force-update-snapshots and --accept
    let output = test_project
        .insta_cmd()
        .args(["test", "--force-update-snapshots", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // `--force-update-snapshots` should remove the hashes
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#####"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -1,5 +1,5 @@
     
     #[test]
     fn test_excessive_hashes() {
    -    insta::assert_snapshot!("foo", @r####"foo"####);
    +    insta::assert_snapshot!("foo", @"foo");
     }
    "#####);
}

#[test]
fn test_inline_snapshot_indent() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "inline-indent"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_wrong_indent_force() {
    insta::assert_snapshot!(r#"
    foo
    foo
    "#, @r#"
                foo
                foo
    "#);
}
"#####
                .to_string(),
        )
        .create_project();

    // ...and that it passes with `--require-full-match`. Note that ideally this
    // would fail, but we can't read the desired indent without serde, which is
    // in `cargo-insta` only. So this tests the current state rather than the
    // ideal state (and I don't think there's a reasonable way to get the ideal state)
    // Now confirm that `--require-full-match` passes
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--check",
            "--require-full-match",
            "--",
            "--nocapture",
        ])
        .output()
        .unwrap();
    assert!(&output.status.success());
}

#[test]
fn test_matches_fully_linebreaks() {
    // Until #563 merges, we should be OK with different leading newlines, even
    // in exact / full match mode.
    let test_project = TestFiles::new()
        .add_cargo_toml("exact-match-inline")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_additional_linebreak() {
    // Additional newline here
    insta::assert_snapshot!(r#"

    (
        "name_foo",
        "insta_tests__tests",
    )
    "#, @r#"
    (
        "name_foo",
        "insta_tests__tests",
    )
    "#);
}
"#####
                .to_string(),
        )
        .create_project();

    // Confirm the test passes despite the indent
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--check",
            "--require-full-match",
            "--",
            "--nocapture",
        ])
        .output()
        .unwrap();
    assert!(&output.status.success());
}

#[test]
fn test_snapshot_name_clash() {
    let test_project = TestFiles::new()
        .add_cargo_toml("snapshot_name_clash_test")
        .add_file(
            "src/lib.rs",
            r#"
use insta::assert_debug_snapshot;

#[test]
fn test_foo_always_missing() {
    assert_debug_snapshot!(42);
}

#[test]
fn foo_always_missing() {
    assert_debug_snapshot!(42);
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // The test should fail due to the name clash
    assert!(!output.status.success());

    let error_output = String::from_utf8_lossy(&output.stderr);

    // Check for the name clash error message
    assert!(error_output.contains("Insta snapshot name clash detected between 'foo_always_missing' and 'test_foo_always_missing' in 'snapshot_name_clash_test'. Rename one function."));
}

#[test]
fn test_unreferenced_delete() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_unreferenced_delete")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("Hello, world!");
}
"#
            .to_string(),
        )
        .create_project();

    // Run tests to create snapshots
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // Manually add an unreferenced snapshot
    let unreferenced_snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/test_unreferenced_delete__unused_snapshot.snap");
    std::fs::create_dir_all(unreferenced_snapshot_path.parent().unwrap()).unwrap();
    std::fs::write(
        &unreferenced_snapshot_path,
        r#"---
source: src/lib.rs
expression: "Unused snapshot"
---
Unused snapshot
"#,
    )
    .unwrap();

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_unreferenced_delete__snapshot.snap
    +      src/snapshots/test_unreferenced_delete__unused_snapshot.snap
    ");

    // Run cargo insta test with --unreferenced=delete
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--unreferenced=delete",
            "--accept",
            "--",
            "--nocapture",
        ])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // We should now see the unreferenced snapshot deleted
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_unreferenced_delete__snapshot.snap
    ");
}

#[test]
fn test_hidden_snapshots() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_hidden_snapshots")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("Hello, world!");
}
"#
            .to_string(),
        )
        .add_file(
            "src/snapshots/test_hidden_snapshots__snapshot.snap",
            r#"---
source: src/lib.rs
expression: "\"Hello, world!\""
---
Hello, world!
"#
            .to_string(),
        )
        .add_file(
            "src/snapshots/.hidden/hidden_snapshot.snap.new",
            r#"---
source: src/lib.rs
expression: "Hidden snapshot"
---
Hidden snapshot
"#
            .to_string(),
        )
        .create_project();

    // Run test without --include-hidden flag
    let output = test_project
        .insta_cmd()
        .args(["test"])
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("found undiscovered pending snapshots")
            && stderr.contains("--include-hidden"),
        "{}",
        stderr
    );

    // Run test with --include-hidden flag
    let output = test_project
        .insta_cmd()
        .args(["test", "--include-hidden"])
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("found undiscovered pending snapshots"),
        "{}",
        stderr
    );
}

#[test]
fn test_snapshot_kind_behavior() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_snapshot_kind")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshots() {
    insta::assert_snapshot!("new snapshot");
    insta::assert_snapshot!("existing snapshot");
}
"#
            .to_string(),
        )
        .add_file(
            "src/snapshots/test_snapshot_kind__existing.snap",
            r#"---
source: src/lib.rs
expression: "\"existing snapshot\""
snapshot_kind: text
---
existing snapshot
"#
            .to_string(),
        )
        .create_project();

    // Run the test with --accept to create the new snapshot
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify the new snapshot was created without snapshot_kind
    let new_snapshot = std::fs::read_to_string(
        test_project
            .workspace_dir
            .join("src/snapshots/test_snapshot_kind__snapshots.snap"),
    )
    .unwrap();

    assert!(!new_snapshot.contains("snapshot_kind:"));

    // Verify both snapshots work with --require-full-match
    let output = test_project
        .insta_cmd()
        .args(["test", "--require-full-match"])
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn test_ignored_snapshots() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_ignored_snapshots")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("Hello, world!", @"");
}
"#
            .to_string(),
        )
        .add_file(
            ".gitignore",
            r#"
src/
"#
            .to_string(),
        )
        .create_project();

    // We need to init a git repository in the project directory so it will be ignored
    let mut git_cmd = Command::new("git");
    git_cmd.current_dir(&test_project.workspace_dir);
    git_cmd.args(["init"]);
    git_cmd.output().unwrap();

    // Run test without --include-ignored flag
    let output = test_project
        .insta_cmd()
        // add the `--hidden` to check it's printing the correct warning
        .args(["test", "--include-hidden"])
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("found undiscovered pending snapshots")
            && stderr.contains("--include-ignored"),
        "{}",
        stderr
    );

    // Run test with --include-ignored flag
    let output = test_project
        .insta_cmd()
        .args(["test", "--include-ignored"])
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("found undiscovered pending snapshots"),
        "{}",
        stderr
    );
}
