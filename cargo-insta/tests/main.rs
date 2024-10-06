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
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::{env, fs::remove_dir_all};

use console::style;
use ignore::WalkBuilder;
use insta::assert_snapshot;
use itertools::Itertools;
use similar::udiff::unified_diff;
use tempfile::TempDir;

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
                format!("{} {}", style(&stderr_name).red(), line)
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
fn test_json_inline() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_json_inline"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features=["json", "redactions"] }
serde = { version = "1.0", features = ["derive"] }
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
use serde::Serialize;

#[derive(Serialize)]
struct User {
    id: u64,
    email: String,
}

#[test]
fn test_json_snapshot() {
    let user = User {
        id: 42,
        email: "john.doe@example.com".into(),
    };
    insta::assert_json_snapshot!(&user, {
        ".id" => "[user_id]",
    }, @"");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.diff("src/main.rs"), @r##"
    --- Original: src/main.rs
    +++ Updated: src/main.rs
    @@ -15,5 +15,10 @@
         };
         insta::assert_json_snapshot!(&user, {
             ".id" => "[user_id]",
    -    }, @"");
    +    }, @r#"
    +    {
    +      "id": "[user_id]",
    +      "email": "john.doe@example.com"
    +    }
    +    "#);
     }
    "##);
}

#[test]
fn test_yaml_inline() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_yaml_inline"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features=["yaml", "redactions"] }
serde = { version = "1.0", features = ["derive"] }
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
use serde::Serialize;

#[derive(Serialize)]
struct User {
    id: u64,
    email: String,
}

#[test]
fn test_yaml_snapshot() {
    let user = User {
        id: 42,
        email: "john.doe@example.com".into(),
    };
    insta::assert_yaml_snapshot!(&user, {
        ".id" => "[user_id]",
    }, @"");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.diff("src/main.rs"), @r###"
    --- Original: src/main.rs
    +++ Updated: src/main.rs
    @@ -15,5 +15,8 @@
         };
         insta::assert_yaml_snapshot!(&user, {
             ".id" => "[user_id]",
    -    }, @"");
    +    }, @r#"
    +    id: "[user_id]"
    +    email: john.doe@example.com
    +    "#);
     }
    "###);
}

#[test]
fn test_utf8_inline() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_utf8_inline"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
#[test]
fn test_non_basic_plane() {
    /* an offset here ❄️ */ insta::assert_snapshot!("a 😀oeu", @"");
}

#[test]
fn test_remove_existing_value() {
    insta::assert_snapshot!("this is the new value", @"this is the old value");
}

#[test]
fn test_remove_existing_value_multiline() {
    insta::assert_snapshot!(
        "this is the new value",
        @"this is\
        this is the old value\
        it really is"
    );
}

#[test]
fn test_trailing_comma_in_inline_snapshot() {
    insta::assert_snapshot!(
        "new value",
        @"old value",  // comma here
    );
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.diff("src/main.rs"), @r##"
    --- Original: src/main.rs
    +++ Updated: src/main.rs
    @@ -1,21 +1,19 @@
     
     #[test]
     fn test_non_basic_plane() {
    -    /* an offset here ❄️ */ insta::assert_snapshot!("a 😀oeu", @"");
    +    /* an offset here ❄️ */ insta::assert_snapshot!("a 😀oeu", @"a 😀oeu");
     }
     
     #[test]
     fn test_remove_existing_value() {
    -    insta::assert_snapshot!("this is the new value", @"this is the old value");
    +    insta::assert_snapshot!("this is the new value", @"this is the new value");
     }
     
     #[test]
     fn test_remove_existing_value_multiline() {
         insta::assert_snapshot!(
             "this is the new value",
    -        @"this is\
    -        this is the old value\
    -        it really is"
    +        @"this is the new value"
         );
     }
     
    @@ -23,6 +21,6 @@
     fn test_trailing_comma_in_inline_snapshot() {
         insta::assert_snapshot!(
             "new value",
    -        @"old value",  // comma here
    +        @"new value",  // comma here
         );
     }
    "##);
}

// Note that names need to be different to prevent the cache confusing them.
fn workspace_with_root_crate(name: String) -> TestFiles {
    TestFiles::new()
        .add_file(
            "Cargo.toml",
            format!(
                r#"
[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[workspace]
members = [
    "member",
]

[workspace.dependencies]
insta = {{path = '$PROJECT_PATH'}}

[dependencies]
insta = {{ workspace = true }}

"#
            )
            .to_string(),
        )
        .add_file(
            "member/Cargo.toml",
            format!(
                r#"
[package]
name = "{name}-member"
version = "0.0.0"
edition = "2021"

[dependencies]
insta = {{ workspace = true }}
"#
            )
            .to_string(),
        )
        .add_file(
            "member/src/lib.rs",
            r#"
#[test]
fn test_member() {
    insta::assert_debug_snapshot!(vec![1, 2, 3]);
}
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
fn main() {
    println!("Hello, world!");
}

#[test]
fn test_root() {
    insta::assert_debug_snapshot!(vec![1, 2, 3]);
}
"#
            .to_string(),
        )
}

/// Check that in a workspace with a default root crate, running `cargo insta
/// test --workspace --accept` will update snapshots in both the root crate and the
/// member crate.
#[test]
fn test_root_crate_workspace_accept() {
    let test_project =
        workspace_with_root_crate("root-crate-workspace-accept".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept", "--workspace"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r###"
    --- Original file tree
    +++ Updated file tree
    @@ -1,8 +1,13 @@
     
    +  Cargo.lock
       Cargo.toml
       member
         member/Cargo.toml
         member/src
           member/src/lib.rs
    +      member/src/snapshots
    +        member/src/snapshots/root_crate_workspace_accept_member__member.snap
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/root_crate_workspace_accept__root.snap
    "###     );
}

/// Check that in a workspace with a default root crate, running `cargo insta
/// test --workspace` will correctly report the number of pending snapshots
#[test]
fn test_root_crate_workspace() {
    let test_project =
        workspace_with_root_crate("root-crate-workspace".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        // Need to disable colors to assert the output below
        .args(["test", "--workspace", "--color=never"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // 1.39 had a bug where it would claim there were 3 snapshots here
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("info: 2 snapshots to review"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Check that in a workspace with a default root crate, running `cargo insta
/// test --accept` will only update snapshots in the root crate
#[test]
fn test_root_crate_no_all() {
    let test_project = workspace_with_root_crate("root-crate-no-all".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r###"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,5 @@
     
    +  Cargo.lock
       Cargo.toml
       member
         member/Cargo.toml
    @@ -6,3 +7,5 @@
           member/src/lib.rs
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/root_crate_no_all__root.snap
    "###     );
}

fn workspace_with_virtual_manifest(name: String) -> TestFiles {
    TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[workspace]
members = [
    "member-1",
    "member-2",
]

[workspace.dependencies]
insta = {path = '$PROJECT_PATH'}
"#
            .to_string()
            .to_string(),
        )
        .add_file(
            "member-1/Cargo.toml",
            format!(
                r#"
[package]
name = "{name}-member-1"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = {{ workspace = true }}
"#
            )
            .to_string(),
        )
        .add_file(
            "member-1/src/lib.rs",
            r#"
#[test]
fn test_member_1() {
    insta::assert_debug_snapshot!(vec![1, 2, 3]);
}
"#
            .to_string(),
        )
        .add_file(
            "member-2/Cargo.toml",
            format!(
                r#"
[package]
name = "{name}-member-2"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = {{ workspace = true }}
"#
            )
            .to_string(),
        )
        .add_file(
            "member-2/src/lib.rs",
            r#"
#[test]
fn test_member_2() {
    insta::assert_debug_snapshot!(vec![4, 5, 6]);
}
"#
            .to_string(),
        )
}

/// Check that in a workspace with a virtual manifest, running `cargo insta test
/// --workspace --accept` updates snapshots in all member crates.
#[test]
fn test_virtual_manifest_all() {
    let test_project =
        workspace_with_virtual_manifest("virtual-manifest-all".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept", "--workspace"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r###"
    --- Original file tree
    +++ Updated file tree
    @@ -1,10 +1,15 @@
     
    +  Cargo.lock
       Cargo.toml
       member-1
         member-1/Cargo.toml
         member-1/src
           member-1/src/lib.rs
    +      member-1/src/snapshots
    +        member-1/src/snapshots/virtual_manifest_all_member_1__member_1.snap
       member-2
         member-2/Cargo.toml
         member-2/src
           member-2/src/lib.rs
    +      member-2/src/snapshots
    +        member-2/src/snapshots/virtual_manifest_all_member_2__member_2.snap
    "###     );
}

/// Check that in a workspace with a virtual manifest, running `cargo insta test
/// --accept` updates snapshots in all member crates.
#[test]
fn test_virtual_manifest_default() {
    let test_project =
        workspace_with_virtual_manifest("virtual-manifest-default".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r###"
    --- Original file tree
    +++ Updated file tree
    @@ -1,10 +1,15 @@
     
    +  Cargo.lock
       Cargo.toml
       member-1
         member-1/Cargo.toml
         member-1/src
           member-1/src/lib.rs
    +      member-1/src/snapshots
    +        member-1/src/snapshots/virtual_manifest_default_member_1__member_1.snap
       member-2
         member-2/Cargo.toml
         member-2/src
           member-2/src/lib.rs
    +      member-2/src/snapshots
    +        member-2/src/snapshots/virtual_manifest_default_member_2__member_2.snap
    "###     );
}

/// Check that in a workspace with a virtual manifest, running `cargo insta test
/// -p <crate>` will only update snapshots in that crate.
#[test]
fn test_virtual_manifest_single_crate() {
    let test_project =
        workspace_with_virtual_manifest("virtual-manifest-single".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept", "-p", "virtual-manifest-single-member-1"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r###"
    --- Original file tree
    +++ Updated file tree
    @@ -1,9 +1,12 @@
     
    +  Cargo.lock
       Cargo.toml
       member-1
         member-1/Cargo.toml
         member-1/src
           member-1/src/lib.rs
    +      member-1/src/snapshots
    +        member-1/src/snapshots/virtual_manifest_single_member_1__member_1.snap
       member-2
         member-2/Cargo.toml
         member-2/src
    "###     );
}

/// Test the old format of inline YAML snapshots with a leading `---` still passes
#[test]
fn test_old_yaml_format() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "old-yaml-format"
version = "0.1.0"
edition = "2021"

[lib]
doctest = false

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["yaml"] }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_old_yaml_format() {
    insta::assert_yaml_snapshot!("foo", @r####"
    ---
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
        .args(["test", "--accept", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
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
    let output_current = test_current_insta
        .insta_cmd()
        .args(["test", "--accept", "--force-update-snapshots"])
        .output()
        .unwrap();

    assert!(&output_current.status.success());

    // Test with insta 1.40.0
    let output_1_40_0 = test_insta_1_40_0
        .insta_cmd()
        .args(["test", "--accept", "--force-update-snapshots"])
        .output()
        .unwrap();

    assert!(&output_1_40_0.status.success());

    // Check that both versions updated the snapshot correctly
    assert_snapshot!(test_current_insta.diff("src/snapshots/test_force_update_current__force_update.snap"), @r#"
    --- Original: src/snapshots/test_force_update_current__force_update.snap
    +++ Updated: src/snapshots/test_force_update_current__force_update.snap
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
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "force-update-inline-linebreaks"
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
        .args([
            "test",
            "--force-update-snapshots",
            "--accept",
            "--",
            "--nocapture",
        ])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // When #563 merges, or #630 is resolved, this will change the snapshot. I
    // also think it's possible to have it work sooner, but have iterated quite
    // a few times trying to get this to work, and then finding something else
    // without test coverage didn't work; so not sure it's a great investment of
    // time.
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");

    // assert_snapshot!(test_project.diff("src/lib.rs"), @r#####"
    // --- Original: src/lib.rs
    // +++ Updated: src/lib.rs
    // @@ -1,8 +1,5 @@

    //  #[test]
    //  fn test_linebreaks() {
    // -    insta::assert_snapshot!("foo", @r####"
    // -    foo
    // -
    // -    "####);
    // +    insta::assert_snapshot!("foo", @"foo");
    //  }
    // "#####);
}

#[test]
fn test_force_update_inline_snapshot_hashes() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "force-update-inline-hashes"
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
        .args([
            "test",
            "--force-update-snapshots",
            "--accept",
            "--",
            "--nocapture",
        ])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // TODO: we would like to update the number of hashes, but that's not easy
    // given the reasons at https://github.com/mitsuhiko/insta/pull/573. So this
    // result asserts the current state rather than the desired state.
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
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
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "exact-match-inline"
version = "0.1.0"
edition = "2021"

[lib]
doctest = false

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
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
fn test_hashtag_escape_in_inline_snapshot() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_hashtag_escape"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#####"
#[test]
fn test_hashtag_escape() {
    insta::assert_snapshot!(r###"Value with
    "## hashtags\n"###, @"");
}
"#####
                .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.diff("src/main.rs"), @r####"
    --- Original: src/main.rs
    +++ Updated: src/main.rs
    @@ -2,5 +2,8 @@
     #[test]
     fn test_hashtag_escape() {
         insta::assert_snapshot!(r###"Value with
    -    "## hashtags\n"###, @"");
    +    "## hashtags\n"###, @r###"
    +    Value with
    +        "## hashtags\n
    +    "###);
     }
    "####);
}

#[test]
fn test_snapshot_name_clash() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "snapshot_name_clash_test"
version = "0.1.0"
edition = "2021"

[lib]
doctest = false

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    #[test]
    fn test_foo_always_missing() {
        assert_debug_snapshot!(42);
    }

    #[test]
    fn foo_always_missing() {
        assert_debug_snapshot!(42);
    }
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
    assert!(error_output.contains("Insta snapshot name clash detected between 'foo_always_missing' and 'test_foo_always_missing' in 'snapshot_name_clash_test::tests'. Rename one function."));
}

/// A pending binary snapshot should have a binary file with the passed extension alongside it.
#[test]
fn test_binary_pending() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_binary_pending"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!(".txt", b"test".to_vec());
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project.insta_cmd().args(["test"]).output().unwrap();

    assert!(!&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/test_binary_pending__binary_snapshot.snap.new
    +      src/snapshots/test_binary_pending__binary_snapshot.snap.new.txt
    ");
}

/// An accepted binary snapshot should have a binary file with the passed extension alongside it.
#[test]
fn test_binary_accept() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_binary_accept"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!(".txt", b"test".to_vec());
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/test_binary_accept__binary_snapshot.snap
    +      src/snapshots/test_binary_accept__binary_snapshot.snap.txt
    ");
}

/// Changing the extension passed to the `assert_binary_snapshot` macro should create a new pending
/// snapshot with a binary file with the new extension alongside it and once approved the old binary
/// file with the old extension should be deleted.
#[test]
fn test_binary_change_extension() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_binary_change_extension"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!(".txt", b"test".to_vec());
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    test_project.update_file(
        "src/main.rs",
        r#"
#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!(".json", b"test".to_vec());
}
"#
        .to_string(),
    );

    let output = test_project.insta_cmd().args(["test"]).output().unwrap();

    assert!(!&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,10 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap.new
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap.new.json
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap.txt
    ");

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap.json
    ");
}

/// An assert with a pending binary snapshot should have both the metadata file and the binary file
/// deleted when the assert is removed and the tests are re-run.
#[test]
fn test_binary_pending_snapshot_removal() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_binary_pending_snapshot_removal"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!(".txt", b"test".to_vec());
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project.insta_cmd().args(["test"]).output().unwrap();

    assert!(!&output.status.success());

    test_project.update_file("src/main.rs", "".to_string());

    let output = test_project.insta_cmd().args(["test"]).output().unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,6 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/main.rs
    +    src/snapshots
    ");
}

/// Replacing a text snapshot with binary one should work and simply replace the text snapshot file
/// with the new metadata file and a new binary snapshot file alongside it.
#[test]
fn test_change_text_to_binary() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_change_text_to_binary"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
#[test]
fn test() {
    insta::assert_snapshot!("test");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/test_change_text_to_binary__test.snap
    ");

    test_project.update_file(
        "src/main.rs",
        r#"
#[test]
fn test() {
    insta::assert_binary_snapshot!(".txt", b"test".to_vec());
}
"#
        .to_string(),
    );

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/test_change_text_to_binary__test.snap
    +      src/snapshots/test_change_text_to_binary__test.snap.txt
    ");
}

/// When changing a snapshot from a binary to a text snapshot the previous binary file should be
/// gone after having approved the the binary snapshot.
#[test]
fn test_change_binary_to_text() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_change_binary_to_text"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
#[test]
fn test() {
    insta::assert_binary_snapshot!("some_name.json", b"{}".to_vec());
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/test_change_binary_to_text__some_name.snap
    +      src/snapshots/test_change_binary_to_text__some_name.snap.json
    ");

    test_project.update_file(
        "src/main.rs",
        r#"
#[test]
fn test() {
    insta::assert_snapshot!("some_name", "test");
}
"#
        .to_string(),
    );

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/test_change_binary_to_text__some_name.snap
    ");
}

// Can't get the test binary discovery to work, don't have a windows machine to
// hand, others are welcome to fix it. (No specific reason to think that insta
// doesn't work on windows, just that the test doesn't work.)
#[cfg(not(target_os = "windows"))]
#[test]
fn test_insta_workspace_root() {
    // This function locates the compiled test binary in the target directory.
    // It's necessary because the exact filename of the test binary includes a hash
    // that we can't predict, so we need to search for it.
    fn find_test_binary(dir: &Path) -> PathBuf {
        dir.join("target/debug/deps")
            .read_dir()
            .unwrap()
            .filter_map(Result::ok)
            .find(|entry| {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_str().unwrap_or("");
                // We're looking for a file that:
                file_name_str.starts_with("insta_workspace_root_test-") // Matches our test name
                    && !file_name_str.contains('.') // Doesn't have an extension (it's the executable, not a metadata file)
                    && entry.metadata().map(|m| m.is_file()).unwrap_or(false) // Is a file, not a directory
            })
            .map(|entry| entry.path())
            .expect("Failed to find test binary")
    }

    fn run_test_binary(
        binary_path: &Path,
        current_dir: &Path,
        env: Option<(&str, &str)>,
    ) -> std::process::Output {
        let mut cmd = Command::new(binary_path);
        TestProject::clean_env(&mut cmd);
        cmd.current_dir(current_dir);
        if let Some((key, value)) = env {
            cmd.env(key, value);
        }
        cmd.output().unwrap()
    }

    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
    [package]
    name = "insta_workspace_root_test"
    version = "0.1.0"
    edition = "2021"

    [dependencies]
    insta = { path = '$PROJECT_PATH' }
    "#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
    #[cfg(test)]
    mod tests {
        use insta::assert_snapshot;

        #[test]
        fn test_snapshot() {
            assert_snapshot!("Hello, world!");
        }
    }
    "#
            .to_string(),
        )
        .create_project();

    let mut cargo_cmd = Command::new("cargo");
    TestProject::clean_env(&mut cargo_cmd);
    let output = cargo_cmd
        .args(["test", "--no-run"])
        .current_dir(&test_project.workspace_dir)
        .output()
        .unwrap();
    assert!(&output.status.success());

    let test_binary_path = find_test_binary(&test_project.workspace_dir);

    // Run the test without snapshot (should fail)
    assert!(
        !&run_test_binary(&test_binary_path, &test_project.workspace_dir, None,)
            .status
            .success()
    );

    // Create the snapshot
    assert!(&run_test_binary(
        &test_binary_path,
        &test_project.workspace_dir,
        Some(("INSTA_UPDATE", "always")),
    )
    .status
    .success());

    // Verify snapshot creation
    assert!(test_project.workspace_dir.join("src/snapshots").exists());
    assert!(test_project
        .workspace_dir
        .join("src/snapshots/insta_workspace_root_test__tests__snapshot.snap")
        .exists());

    // Move the workspace
    let moved_workspace = {
        let moved_workspace = PathBuf::from("/tmp/cargo-insta-test-moved");
        remove_dir_all(&moved_workspace).ok();
        fs::create_dir(&moved_workspace).unwrap();
        fs::rename(&test_project.workspace_dir, &moved_workspace).unwrap();
        moved_workspace
    };
    let moved_binary_path = find_test_binary(&moved_workspace);

    // Run test in moved workspace without INSTA_WORKSPACE_ROOT (should fail)
    assert!(
        !&run_test_binary(&moved_binary_path, &moved_workspace, None)
            .status
            .success()
    );

    // Run test in moved workspace with INSTA_WORKSPACE_ROOT (should pass)
    assert!(&run_test_binary(
        &moved_binary_path,
        &moved_workspace,
        Some(("INSTA_WORKSPACE_ROOT", moved_workspace.to_str().unwrap())),
    )
    .status
    .success());
}

#[test]
fn test_external_test_path() {
    let test_project = TestFiles::new()
        .add_file(
            "proj/Cargo.toml",
            r#"
[package]
name = "external_test_path"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }

[[test]]
name = "tlib"
path = "../tests/lib.rs"
"#
            .to_string(),
        )
        .add_file(
            "proj/src/lib.rs",
            r#"
pub fn hello() -> String {
    "Hello, world!".to_string()
}
"#
            .to_string(),
        )
        .add_file(
            "tests/lib.rs",
            r#"
use external_test_path::hello;

#[test]
fn test_hello() {
    insta::assert_snapshot!(hello());
}
"#
            .to_string(),
        )
        .create_project();

    // Change to the proj directory for running cargo commands
    let proj_dir = test_project.workspace_dir.join("proj");

    // Initially, the test should fail
    let output = test_project
        .insta_cmd()
        .current_dir(&proj_dir)
        .args(["test", "--"])
        .output()
        .unwrap();

    assert!(!&output.status.success());

    // Verify that the snapshot was created in the correct location
    assert_snapshot!(TestProject::current_file_tree(&test_project.workspace_dir), @r"
    proj
      proj/Cargo.lock
      proj/Cargo.toml
      proj/src
        proj/src/lib.rs
    tests
      tests/lib.rs
      tests/snapshots
        tests/snapshots/tlib__hello.snap.new
    ");

    // Run cargo insta accept
    let output = test_project
        .insta_cmd()
        .current_dir(&proj_dir)
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // Verify that the snapshot was created in the correct location
    assert_snapshot!(TestProject::current_file_tree(&test_project.workspace_dir), @r"
    proj
      proj/Cargo.lock
      proj/Cargo.toml
      proj/src
        proj/src/lib.rs
    tests
      tests/lib.rs
      tests/snapshots
        tests/snapshots/tlib__hello.snap
    ");

    // Run the test again, it should pass now
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-insta"))
        .current_dir(&proj_dir)
        .args(["test"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    let snapshot_path = test_project
        .workspace_dir
        .join("tests/snapshots/tlib__hello.snap");
    assert_snapshot!(fs::read_to_string(snapshot_path).unwrap(), @r#"
    ---
    source: "../tests/lib.rs"
    expression: hello()
    snapshot_kind: text
    ---
    Hello, world!
    "#);
}

#[test]
fn test_unreferenced_delete() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_unreferenced_delete"
version = "0.1.0"
edition = "2021"

[lib]
doctest = false

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_snapshot() {
        insta::assert_snapshot!("Hello, world!");
    }
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
        .join("src/snapshots/test_unreferenced_delete__tests__unused_snapshot.snap");
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
    +      src/snapshots/test_unreferenced_delete__tests__snapshot.snap
    +      src/snapshots/test_unreferenced_delete__tests__unused_snapshot.snap
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
    +      src/snapshots/test_unreferenced_delete__tests__snapshot.snap
    ");
}

#[test]
fn test_hidden_snapshots() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_hidden_snapshots"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_snapshot() {
        insta::assert_snapshot!("Hello, world!");
    }
}
"#
            .to_string(),
        )
        .add_file(
            "src/snapshots/test_hidden_snapshots__tests__snapshot.snap",
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
fn test_ignored_snapshots() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_ignored_snapshots"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
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

#[test]
fn test_binary_unreferenced_delete() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_binary_unreferenced_delete"
version = "0.1.0"
edition = "2021"

[lib]
doctest = false

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_snapshot() {
        insta::assert_binary_snapshot!(".txt", b"abcd".to_vec());
    }
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

    test_project.update_file("src/lib.rs", "".to_string());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_binary_unreferenced_delete__tests__snapshot.snap
    +      src/snapshots/test_binary_unreferenced_delete__tests__snapshot.snap.txt
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
    @@ -1,4 +1,6 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    ");
}
