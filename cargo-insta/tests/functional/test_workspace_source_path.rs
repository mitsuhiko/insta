use crate::TestFiles;
use std::fs;

/// Test for issue #777: Insta "source" in snapshot is full absolute path when workspace is not parent
#[test]
fn test_workspace_source_path_issue_777() {
    // Create a workspace structure where project is not a child of workspace
    // This reproduces the exact issue from #777
    let test_project = TestFiles::new()
        .add_file(
            "workspace/Cargo.toml",
            r#"
[workspace]
resolver = "2"
members = ["../project1"]
"#
            .to_string(),
        )
        .add_file(
            "project1/Cargo.toml",
            r#"
[package]
name = "project1"
version = "0.1.0"
edition = "2021"

workspace = "../workspace"

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["yaml"] }
"#
            .to_string(),
        )
        .add_file(
            "project1/src/lib.rs",
            r#"
#[test]
fn test_something() {
    insta::assert_yaml_snapshot!(vec![1, 2, 3]);
}
"#
            .to_string(),
        )
        .create_project();

    // Run test to create snapshot from within project1 directory
    // This should trigger the issue where source path becomes absolute
    let output = test_project
        .insta_cmd()
        .current_dir(test_project.workspace_dir.join("project1"))
        // Set workspace root to the actual workspace directory
        .env(
            "INSTA_WORKSPACE_ROOT",
            test_project.workspace_dir.join("workspace"),
        )
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Read the generated snapshot
    let snapshot_path = test_project
        .workspace_dir
        .join("project1/src/snapshots/project1__something.snap");

    let snapshot_content = fs::read_to_string(&snapshot_path).unwrap();

    // Parse the snapshot to check the source field
    let source_line = snapshot_content
        .lines()
        .find(|line| line.starts_with("source:"))
        .expect("source line not found");

    let source_path = source_line
        .strip_prefix("source: ")
        .expect("invalid source line")
        .trim()
        .trim_matches('"');

    // The source path should be relative and start with ../ (since workspace and project are siblings)
    assert!(
        source_path.starts_with("../"),
        "Source path should be relative starting with '../', but got: {}",
        source_path
    );

    // The path should be exactly ../project1/src/lib.rs
    assert_eq!(
        source_path, "../project1/src/lib.rs",
        "Expected simplified relative path"
    );
}

/// Test that the fix works with a more complex workspace structure
#[test]
fn test_workspace_source_path_complex() {
    // Create a complex workspace structure
    let test_project = TestFiles::new()
        .add_file(
            "code/workspace/Cargo.toml",
            r#"
[workspace]
resolver = "2"
members = ["../../projects/app1", "../../projects/app2"]
"#
            .to_string(),
        )
        .add_file(
            "projects/app1/Cargo.toml",
            r#"
[package]
name = "app1"
version = "0.1.0"
edition = "2021"

workspace = "../../code/workspace"

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["yaml"] }
"#
            .to_string(),
        )
        .add_file(
            "projects/app1/src/lib.rs",
            r#"
#[test]
fn test_app1() {
    insta::assert_yaml_snapshot!(vec!["app1"]);
}
"#
            .to_string(),
        )
        .add_file(
            "projects/app2/Cargo.toml",
            r#"
[package]
name = "app2"
version = "0.1.0"
edition = "2021"

workspace = "../../code/workspace"

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["yaml"] }
"#
            .to_string(),
        )
        .add_file(
            "projects/app2/src/lib.rs",
            r#"
#[test]
fn test_app2() {
    insta::assert_yaml_snapshot!(vec!["app2"]);
}
"#
            .to_string(),
        )
        .create_project();

    // Run tests for both projects
    let output1 = test_project
        .insta_cmd()
        .current_dir(test_project.workspace_dir.join("projects/app1"))
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(output1.status.success());

    let output2 = test_project
        .insta_cmd()
        .current_dir(test_project.workspace_dir.join("projects/app2"))
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(output2.status.success());

    // Check both snapshots
    let snapshot1_path = test_project
        .workspace_dir
        .join("projects/app1/src/snapshots/app1__app1.snap");
    let snapshot1_content = fs::read_to_string(&snapshot1_path).unwrap();

    let snapshot2_path = test_project
        .workspace_dir
        .join("projects/app2/src/snapshots/app2__app2.snap");
    let snapshot2_content = fs::read_to_string(&snapshot2_path).unwrap();

    // Neither snapshot should contain absolute paths
    assert!(
        !snapshot1_content.contains(&test_project.workspace_dir.to_string_lossy().to_string()),
        "App1 snapshot contains absolute path"
    );
    assert!(
        !snapshot2_content.contains(&test_project.workspace_dir.to_string_lossy().to_string()),
        "App2 snapshot contains absolute path"
    );

    // Both should have relative paths
    assert!(
        snapshot1_content.contains("source: \"../../projects/app1/src/lib.rs\""),
        "App1 snapshot source is not the expected relative path. Got:\n{}",
        snapshot1_content
    );
    assert!(
        snapshot2_content.contains("source: \"../../projects/app2/src/lib.rs\""),
        "App2 snapshot source is not the expected relative path. Got:\n{}",
        snapshot2_content
    );
}
