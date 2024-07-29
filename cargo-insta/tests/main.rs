// TODO:
// - How to handle compilation? We want each test to be independent, but we
//   don't want to compile insta for each test. Maybe we can compile it once
//   and copy the `target` directory for each test?

use insta::assert_snapshot;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

struct TestProject {
    files: HashMap<PathBuf, String>,
    temp_dir: TempDir,
    project_path: Option<PathBuf>,
}

impl TestProject {
    fn new() -> Self {
        Self {
            files: HashMap::new(),
            temp_dir: TempDir::new().unwrap(),
            project_path: None,
        }
    }

    fn add_file<P: AsRef<Path>>(mut self, path: P, content: String) -> Self {
        let relative_path = path.as_ref().strip_prefix("/").unwrap_or(path.as_ref());
        self.files.insert(relative_path.to_path_buf(), content);
        self
    }

    fn create(&mut self) -> &PathBuf {
        let project_path = self.temp_dir.path();

        // Get the absolute path to the current insta crate
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cargo_insta_path = PathBuf::from(manifest_dir).canonicalize().unwrap();
        let insta_path = cargo_insta_path.parent().unwrap().join("insta");

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
        self.project_path.as_ref().unwrap()
    }

    fn cmd(&self) -> Command {
        let project_path = self
            .project_path
            .as_ref()
            .expect("Project has not been created yet. Call create() first.");
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-insta"));
        command.current_dir(project_path);
        command
    }
}

#[test]
fn test_json_inline() {
    let mut project = TestProject::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_json_inline"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = "$PROJECT_PATH", features=["json", "redactions"] }
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
        );

    project.create();

    let output = project
        .cmd()
        .args(["test", "--accept"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Tests failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let contents =
        fs::read_to_string(project.project_path.as_ref().unwrap().join("src/main.rs")).unwrap();
    assert_snapshot!(contents, @r#####"

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
        }, @r###"
        {
          "id": "[user_id]",
          "email": "john.doe@example.com"
        }
        "###);
    }
    "#####);
}

#[test]
fn test_yaml_inline() {
    let mut project = TestProject::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_yaml_inline"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = "$PROJECT_PATH", features=["yaml", "redactions"] }
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
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
        );

    project.create();

    let output = project
        .cmd()
        .args(["test", "--accept"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Tests failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let contents =
        fs::read_to_string(project.project_path.as_ref().unwrap().join("src/main.rs")).unwrap();
    assert_snapshot!(contents, @r#####"

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
        }, @r###"
        ---
        id: "[user_id]"
        email: john.doe@example.com
        "###);
    }
    "#####);
}

#[test]
fn test_utf8_inline() {
    let mut project = TestProject::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_utf8_inline"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = "$PROJECT_PATH" }
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
#[test]
#[rustfmt::skip]
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
        );

    project.create();

    let output = project
        .cmd()
        .args(["test", "--accept"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Tests failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let contents =
        fs::read_to_string(project.project_path.as_ref().unwrap().join("src/main.rs")).unwrap();
    assert_snapshot!(contents, @r###"
    #[test]
    #[rustfmt::skip]
    fn test_non_basic_plane() {
        /* an offset here ❄️ */ insta::assert_snapshot!("a 😀oeu", @"a 😀oeu");
    }

    #[test]
    fn test_remove_existing_value() {
        insta::assert_snapshot!("this is the new value", @"this is the new value");
    }

    #[test]
    fn test_remove_existing_value_multiline() {
        insta::assert_snapshot!(
            "this is the new value",
            @"this is the new value"
        );
    }

    #[test]
    fn test_trailing_comma_in_inline_snapshot() {
        insta::assert_snapshot!(
            "new value",
            @"new value",  // comma here
        );
    }
    "###);
}
