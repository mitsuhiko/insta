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
/// tests but with different commands. (We could try to enforce different names,
/// or give up using a consistent target directory for a cache, but it would
/// slow down repeatedly running the tests locally.)
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ignore::WalkBuilder;
use insta::assert_snapshot;
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
        command.current_dir(self.workspace_dir.as_path());
        // Use the same target directory as other tests, consistent across test
        // run. This makes the compilation much faster (though do some tests
        // tread on the toes of others? We could have a different cache for each
        // project if so...)
        command.env("CARGO_TARGET_DIR", target_dir());
        // Turn off CI flag so that cargo insta test behaves as we expect
        // under normal operation
        command.env("CI", "0");
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
            .map(|entry| {
                let path = entry
                    .path()
                    .strip_prefix(workspace_dir)
                    .unwrap_or(entry.path());
                format!("{}{}", "  ".repeat(entry.depth()), path.display())
            })
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

    assert_snapshot!(test_project.diff("src/main.rs"), @r##"
    --- Original: src/main.rs
    +++ Updated: src/main.rs
    @@ -15,5 +15,9 @@
         };
         insta::assert_yaml_snapshot!(&user, {
             ".id" => "[user_id]",
    -    }, @"");
    +    }, @r#"
    +    ---
    +    id: "[user_id]"
    +    email: john.doe@example.com
    +    "#);
     }
    "##);
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
    "crates/member-crate",
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
            "crates/member-crate/Cargo.toml",
            r#"
[package]
name = "member-crate"
version = "0.0.0"
edition = "2021"

[dependencies]
insta = { workspace = true }
"#
            .to_string(),
        )
        .add_file(
            "crates/member-crate/src/lib.rs",
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
/// test --workspace` will update snapsnots in both the root crate and the
/// member crate.
#[test]
fn test_root_crate_all() {
    let test_project = workspace_with_root_crate("root-crate-all".to_string()).create_project();

    let output = test_project
        .cmd()
        .args(["test", "--accept", "--workspace"])
        .output()
        .unwrap();

    assert_success(&output);

    assert_snapshot!(test_project.file_tree_diff(), @r###"
    --- Original file tree
    +++ Updated file tree
    @@ -4,6 +4,11 @@
         crates/member-crate
           crates/member-crate/Cargo.toml
           crates/member-crate/src
    +        crates/member-crate/src/snapshots
    +          crates/member-crate/src/snapshots/member_crate__member.snap
             crates/member-crate/src/lib.rs
    +  Cargo.lock
       src
    +    src/snapshots
    +      src/snapshots/root_crate_all__root.snap
         src/main.rs
    \ No newline at end of file
    "###     );
}

/// Check that in a workspace with a default root crate, running `cargo insta
/// test` will only update snapsnots in the root crate
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
    @@ -5,5 +5,8 @@
           crates/member-crate/Cargo.toml
           crates/member-crate/src
             crates/member-crate/src/lib.rs
    +  Cargo.lock
       src
    +    src/snapshots
    +      src/snapshots/root_crate_no_all__root.snap
         src/main.rs
    \ No newline at end of file
    "###     );
}
