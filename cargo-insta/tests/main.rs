/// Integration tests which allow creating a full repo, running `cargo-insta`
/// and then checking the output.
///
/// We can write more docs if that would be helpful. For the moment one thing to
/// be aware of: it seems the packages must have different names, or we'll see
/// interference between the tests.
///
/// (That seems to be because they all share the same `target` directory, which
/// cargo will confuse for each other if they share the same name. I haven't
/// worked out why — this is the case even if the files are the same between two
/// tests but with different commands — and those files exist in different
/// temporary workspace dirs. (We could try to enforce different names, or give
/// up using a consistent target directory for a cache, but it would slow down
/// repeatedly running the tests locally. To demonstrate the effect, name crates
/// the same...). This also causes issues when running the same tests
/// concurrently.
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ignore::WalkBuilder;
use insta::assert_snapshot;
use itertools::Itertools;
use similar::udiff::unified_diff;
use tempfile::TempDir;

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

/// Path of the insta crate in this repo, which we use as a dependency in the test project
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

fn assert_success(output: &std::process::Output) {
    // Print stderr. Cargo test hides this when tests are successful, but if a
    // test successfully exectues a command but then fails (e.g. on a snapshot),
    // we would otherwise lose any output from the command such as `dbg!`
    // statements.
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    eprint!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success(),
        "Tests failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
    fn cmd(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-insta"));
        // Remove environment variables so we don't inherit anything (such as
        // `INSTA_FORCE_PASS` or `CARGO_INSTA_*`) from a cargo-insta process
        // which runs this integration test.
        for (key, _) in env::vars() {
            if key.starts_with("CARGO_INSTA") || key.starts_with("INSTA") {
                command.env_remove(&key);
            }
        }
        // Turn off CI flag so that cargo insta test behaves as we expect
        // under normal operation
        command.env("CI", "0");
        // And any others that can affect the output
        command.env_remove("CARGO_TERM_COLOR");
        command.env_remove("CLICOLOR_FORCE");
        command.env_remove("RUSTDOCFLAGS");

        command.current_dir(self.workspace_dir.as_path());
        // Use the same target directory as other tests, consistent across test
        // run. This makes the compilation much faster (though do some tests
        // tread on the toes of others? We could have a different cache for each
        // project if so...)
        command.env("CARGO_TARGET_DIR", target_dir());

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
        .cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert_success(&output);

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
        .cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert_success(&output);

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
        .cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert_success(&output);

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
/// test --workspace --accept` will update snapsnots in both the root crate and the
/// member crate.
#[test]
fn test_root_crate_workspace_accept() {
    let test_project =
        workspace_with_root_crate("root-crate-workspace-accept".to_string()).create_project();

    let output = test_project
        .cmd()
        .args(["test", "--accept", "--workspace"])
        .output()
        .unwrap();

    assert_success(&output);

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
        .cmd()
        // Need to disable colors to assert the output below
        .args(["test", "--workspace", "--color=never"])
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
/// test --accept` will only update snapsnots in the root crate
#[test]
fn test_root_crate_no_all() {
    let test_project = workspace_with_root_crate("root-crate-no-all".to_string()).create_project();

    let output = test_project
        .cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert_success(&output);

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
        .cmd()
        .args(["test", "--accept", "--workspace"])
        .output()
        .unwrap();

    assert_success(&output);

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
        .cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert_success(&output);

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
        .cmd()
        .args(["test", "--accept", "-p", "virtual-manifest-single-member-1"])
        .output()
        .unwrap();

    assert_success(&output);

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
        .cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .output()
        .unwrap();

    assert_success(&output);

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
        .cmd()
        .args(["test", "--accept", "--force-update-snapshots"])
        .output()
        .unwrap();

    assert_success(&output_current);

    // Test with insta 1.40.0
    let output_1_40_0 = test_insta_1_40_0
        .cmd()
        .args(["test", "--accept", "--force-update-snapshots"])
        .output()
        .unwrap();

    assert_success(&output_1_40_0);

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
}

#[test]
fn test_force_update_inline_snapshot() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "force-update-inline"
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
        .cmd()
        .args([
            "test",
            "--force-update-snapshots",
            "--accept",
            "--",
            "--nocapture",
        ])
        .output()
        .unwrap();

    assert_success(&output);

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
            r#"
#[test]
fn test_hashtag_escape() {
    insta::assert_snapshot!("Value with #### hashtags\n", @"");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert_success(&output);

    assert_snapshot!(test_project.diff("src/main.rs"), @r######"
    --- Original: src/main.rs
    +++ Updated: src/main.rs
    @@ -1,5 +1,7 @@
     
     #[test]
     fn test_hashtag_escape() {
    -    insta::assert_snapshot!("Value with #### hashtags\n", @"");
    +    insta::assert_snapshot!("Value with #### hashtags\n", @r#####"
    +    Value with #### hashtags
    +    "#####);
     }
    "######);
}
