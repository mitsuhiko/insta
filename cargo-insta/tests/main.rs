use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ignore::WalkBuilder;
use insta::assert_snapshot;
use similar::udiff::unified_diff;
use tempfile::TempDir;

struct TestProject {
    files: HashMap<PathBuf, String>,
    /// Temporary directory where the project is created
    temp_dir: TempDir,
    /// Path of this repo, so we can have it as a dependency in the test project
    project_path: Option<PathBuf>,
    /// File tree at start of test
    file_tree: Option<String>,
}

fn workspace_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn target_dir() -> PathBuf {
    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_path().join("target"))
        .join("test-projects");
    fs::create_dir_all(&target_dir).unwrap();
    target_dir
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "Tests failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

impl TestProject {
    fn new() -> Self {
        Self {
            files: HashMap::new(),
            temp_dir: TempDir::new().unwrap(),
            project_path: None,
            file_tree: None,
        }
    }

    fn add_file<P: AsRef<Path>>(mut self, path: P, content: String) -> Self {
        self.files.insert(path.as_ref().to_path_buf(), content);
        self
    }

    fn create(mut self) -> Self {
        let project_path = self.temp_dir.path();
        let insta_path = workspace_path().join("insta");

        // Create files and replace $PROJECT_PATH in all files
        for (path, content) in &self.files {
            let full_path = project_path.join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let replaced_content = content.replace("$PROJECT_PATH", insta_path.to_str().unwrap());
            fs::write(full_path, replaced_content).unwrap();
        }

        self.project_path = Some(project_path.to_path_buf());
        self
    }

    fn cmd(&mut self) -> Command {
        self.file_tree = Some(self.current_file_tree());
        let project_path = self
            .project_path
            .as_ref()
            .expect("Project has not been created yet. Call create() first.");
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-insta"));
        command.current_dir(project_path);
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
        let file_path_buf = self.project_path.as_ref().unwrap().join(file_path);
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

    fn current_file_tree(&self) -> String {
        WalkBuilder::new(&self.temp_dir)
            .filter_entry(|e| e.path().file_name() != Some(std::ffi::OsStr::new("target")))
            .build()
            .filter_map(|e| e.ok())
            .map(|entry| {
                let path = entry
                    .path()
                    .strip_prefix(&self.temp_dir)
                    .unwrap_or(entry.path());
                format!("{}{}", "  ".repeat(entry.depth()), path.display())
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn file_tree_diff(&self) -> String {
        unified_diff(
            similar::Algorithm::Patience,
            &self.file_tree.clone().unwrap(),
            self.current_file_tree().as_ref(),
            3,
            Some(("Original file tree", "Updated file tree")),
        )
    }
}

#[test]
fn test_json_inline() {
    let mut test_project = TestProject::new()
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
        .create();

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
    let mut test_project = TestProject::new()
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
        .create();

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
    let mut test_project = TestProject::new()
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
        .create();

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

// TODO: This panics and will be fixed by #531 (and the snapshot requires
// updating; the result is not what we want)
#[ignore]
#[test]
fn test_nested_crate() {
    let mut test_project = TestProject::new()
        .add_file(
            "Cargo.toml",
            r#"
[workspace]
members = [
    "crates/member-crate",
]

[workspace.dependencies]
insta = {path = '$PROJECT_PATH'}


[package]
name = "nested"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { workspace = true }

"#
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
        .create();

    let output = test_project
        .cmd()
        .args(["test", "--accept", "--workspace"])
        .output()
        .unwrap();

    assert_success(&output);

    assert_snapshot!(test_project.file_tree_diff(), @r#"
    --- Original file tree
    +++ Updated file tree
    @@ -5,5 +5,8 @@
           crates/member-crate/Cargo.toml
           crates/member-crate/src
             crates/member-crate/src/lib.rs
    +  Cargo.lock
       src
    +    src/snapshots
    +      src/snapshots/nested__root.snap
         src/main.rs
    \ No newline at end of file
    "#     );
}
