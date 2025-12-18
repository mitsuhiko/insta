use crate::TestFiles;
use std::process::Stdio;

/// This test verifies that tests with missing snapshots should fail when
/// using --unreferenced=auto, and unreferenced snapshots should be preserved in this case
#[test]
fn test_unreferenced_auto_with_missing_snapshots() {
    // Create a test project with a test that has no snapshot yet
    let test_project = TestFiles::new()
        .add_cargo_toml("test_unreferenced_missing")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_with_missing_snapshot() {
    insta::assert_snapshot!("This has no snapshot yet");
}
"#
            .to_string(),
        )
        .create_project();

    // Manually add an unreferenced snapshot
    let unreferenced_snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/test_unreferenced_missing__unused_snapshot.snap");
    std::fs::create_dir_all(unreferenced_snapshot_path.parent().unwrap()).unwrap();
    std::fs::write(
        &unreferenced_snapshot_path,
        r#"---
    source: src/lib.rs
    expression: "\"Unused snapshot\""
    ---
    Unused snapshot
    "#,
    )
    .unwrap();

    // Run test, which should fail since the snapshot is missing
    // initially without --unreferenced=auto
    assert!(!test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    // ...now with --unreferenced=auto
    let output = test_project
        .insta_cmd()
        .args(["test", "--unreferenced=auto", "--", "--nocapture"])
        .output()
        .unwrap();

    // Verify that the test run failed
    assert!(
        !output.status.success(),
        "The test run should have failed due to missing snapshot"
    );

    // verify the unreferenced snapshot was not deleted when a test fails due to
    // missing snapshots
    assert!(
        std::path::Path::new(&unreferenced_snapshot_path).exists(),
        "The unreferenced snapshot should not be deleted when tests fail due to missing snapshots"
    );
}

/// This test verifies that inline tests with missing snapshots should fail when
/// using --unreferenced=auto, and unreferenced snapshots should be preserved in this case
#[test]
fn test_unreferenced_auto_with_missing_inline_snapshots() {
    // Create a test project with a test that has no snapshot yet
    let test_project = TestFiles::new()
        .add_cargo_toml("test_unreferenced_missing_inline")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_with_missing_inline_snapshot() {
    insta::assert_snapshot!("This has no inline snapshot yet", @"");
}
"#
            .to_string(),
        )
        .create_project();

    // Manually add an unreferenced snapshot
    let unreferenced_snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/test_unreferenced_missing_inline__unused_snapshot.snap");
    std::fs::create_dir_all(unreferenced_snapshot_path.parent().unwrap()).unwrap();
    std::fs::write(
        &unreferenced_snapshot_path,
        r#"---
    source: src/lib.rs
    expression: "\"Unused snapshot\""
    ---
    Unused snapshot
    "#,
    )
    .unwrap();

    // Run test, which should fail since the inline snapshot is incorrect
    // initially without --unreferenced=auto
    assert!(!test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    // ...now with --unreferenced=auto
    let output = test_project
        .insta_cmd()
        .args(["test", "--unreferenced=auto", "--", "--nocapture"])
        .output()
        .unwrap();

    // Verify that the test run failed
    assert!(
        !output.status.success(),
        "The test run should have failed due to incorrect inline snapshot"
    );

    // verify the unreferenced snapshot was not deleted when a test fails due to
    // missing snapshots
    assert!(
        std::path::Path::new(&unreferenced_snapshot_path).exists(),
        "The unreferenced snapshot should not be deleted when tests fail due to missing snapshots"
    );
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

    insta::assert_snapshot!(test_project.file_tree_diff(), @"
    --- Original file tree
    +++ Updated file tree
    @@ -1,3 +1,7 @@
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
    insta::assert_snapshot!(test_project.file_tree_diff(), @"
    --- Original file tree
    +++ Updated file tree
    @@ -1,3 +1,6 @@
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_unreferenced_delete__snapshot.snap
    ");
}

#[test]
fn test_unreferenced_config_reject() {
    // This test verifies that the `test.unreferenced: reject` setting in insta.yaml
    // is respected when no command-line argument is provided.
    //
    // Specifically, it tests the fix for issue #757, which ensures that:
    // 1. Config file settings are properly applied when not overridden by command-line flags
    // 2. Error handling for unreferenced snapshots properly updates the success flag
    let test_project = TestFiles::new()
        .add_cargo_toml("test_unreferenced_config_reject")
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

    // Run tests to create snapshots first (without the config file)
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Now add the config file after snapshot is created
    test_project.update_file(
        "insta.yaml",
        r#"
test:
  unreferenced: reject
"#
        .to_string(),
    );

    // Manually add an unreferenced snapshot
    let unreferenced_snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/test_unreferenced_config_reject__unused_snapshot.snap");
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

    // Verify files exist
    let snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/test_unreferenced_config_reject__snapshot.snap");
    let unreferenced_path = test_project
        .workspace_dir
        .join("src/snapshots/test_unreferenced_config_reject__unused_snapshot.snap");

    assert!(snapshot_path.exists(), "Normal snapshot file should exist");
    assert!(
        unreferenced_path.exists(),
        "Unreferenced snapshot file should exist"
    );

    // First verify explicitly passing --unreferenced=reject does fail correctly
    let output = test_project
        .insta_cmd()
        .args(["test", "--unreferenced=reject", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // The test should fail with explicit flag
    assert!(
        !output.status.success(),
        "Command should fail with explicit --unreferenced=reject flag"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("encountered unreferenced snapshots"),
        "Expected error message about unreferenced snapshots, got: {stderr}"
    );

    // Now run without flags - this should also fail due to the config file setting
    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // The command should fail because of the config file setting
    assert!(
        !output.status.success(),
        "Command should fail when config file has test.unreferenced: reject"
    );

    // Verify the error message mentions unreferenced snapshots
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("encountered unreferenced snapshots"),
        "Expected error message about unreferenced snapshots, got: {stderr}"
    );

    // Run with --unreferenced=delete to clean up
    let output = test_project
        .insta_cmd()
        .args(["test", "--unreferenced=delete", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify the unreferenced snapshot was deleted
    assert!(
        snapshot_path.exists(),
        "Normal snapshot file should still exist"
    );
    assert!(
        !unreferenced_path.exists(),
        "Unreferenced snapshot file should have been deleted"
    );
}

/// Test that non-insta snapshot files (e.g., vitest, jest) are not flagged as
/// unreferenced insta snapshots. This addresses issue #845 where projects using
/// both insta and vitest would have vitest's .snap files incorrectly flagged.
#[test]
fn test_unreferenced_ignores_non_insta_snapshots() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_non_insta_snap")
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

    // Run tests to create the insta snapshot
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Add a vitest-format snapshot file (non-insta)
    let vitest_snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/example.test.js.snap");
    std::fs::write(
        &vitest_snapshot_path,
        r#"// Vitest Snapshot v1, https://vitest.dev/guide/snapshot.html

exports[`example test 1`] = `"Hello from vitest"`;
"#,
    )
    .unwrap();

    // Add a jest-format snapshot file (non-insta)
    let jest_snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/another.test.js.snap");
    std::fs::write(
        &jest_snapshot_path,
        r#"// Jest Snapshot v1, https://goo.gl/fbAQLP

exports[`another test 1`] = `"Hello from jest"`;
"#,
    )
    .unwrap();

    // Run with --unreferenced=delete
    let output = test_project
        .insta_cmd()
        .args(["test", "--unreferenced=delete", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());

    // The vitest and jest snapshots should NOT be deleted
    assert!(
        vitest_snapshot_path.exists(),
        "Vitest snapshot should be preserved (not an insta snapshot)"
    );
    assert!(
        jest_snapshot_path.exists(),
        "Jest snapshot should be preserved (not an insta snapshot)"
    );

    // The insta snapshot should still exist
    let insta_snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/test_non_insta_snap__snapshot.snap");
    assert!(
        insta_snapshot_path.exists(),
        "Insta snapshot should still exist"
    );

    // Verify the output mentions "no unreferenced snapshots found"
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no unreferenced snapshots found"),
        "Should report no unreferenced snapshots since non-insta files are ignored. Got: {stderr}"
    );
}

/// Test that --unreferenced=reject does not fail on non-insta snapshot files.
#[test]
fn test_unreferenced_reject_ignores_non_insta_snapshots() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_reject_non_insta")
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

    // Run tests to create the insta snapshot
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Add a vitest-format snapshot file (non-insta)
    let vitest_snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/component.test.ts.snap");
    std::fs::write(
        &vitest_snapshot_path,
        r#"// Vitest Snapshot v1, https://vitest.dev/guide/snapshot.html

exports[`Component renders correctly 1`] = `
<div>
  <span>Hello</span>
</div>
`;
"#,
    )
    .unwrap();

    // Run with --unreferenced=reject - should NOT fail because vitest file is ignored
    let output = test_project
        .insta_cmd()
        .args(["test", "--unreferenced=reject", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // Should succeed - no insta snapshots are unreferenced
    assert!(
        output.status.success(),
        "Should succeed because vitest snapshots are not considered insta snapshots"
    );

    // Vitest snapshot should still exist
    assert!(
        vitest_snapshot_path.exists(),
        "Vitest snapshot should be preserved"
    );
}
