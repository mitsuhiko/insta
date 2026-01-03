use std::fs;
use std::process::Stdio;

use crate::TestFiles;

#[cfg(unix)]
use crate::{target_dir, TestProject};
#[cfg(unix)]
use std::process::Command;

/// Test that INSTA_PENDING_DIR redirects pending snapshots to a separate directory
#[test]
fn test_pending_dir_file_snapshot() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_pending_dir")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("hello", "Hello, World!");
}
"#
            .to_string(),
        )
        .create_project();

    // Create a separate pending directory
    let pending_dir = test_project.workspace_dir.join("pending_output");
    fs::create_dir_all(&pending_dir).unwrap();

    // Run the test with INSTA_PENDING_DIR set
    let output = test_project
        .insta_cmd()
        .args(["test"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // The test should fail because the snapshot doesn't exist
    assert!(!output.status.success());

    // The .snap.new file should be in the pending directory, not the source tree
    let pending_snap = pending_dir.join("src/snapshots/test_pending_dir__hello.snap.new");
    assert!(
        pending_snap.exists(),
        "Expected pending snapshot at {:?}, but it doesn't exist. Pending dir contents: {:?}",
        pending_snap,
        fs::read_dir(&pending_dir)
            .ok()
            .map(|d| d.collect::<Vec<_>>())
    );

    // The .snap.new file should NOT be in the source tree
    let source_tree_snap = test_project
        .workspace_dir
        .join("src/snapshots/test_pending_dir__hello.snap.new");
    assert!(
        !source_tree_snap.exists(),
        "Pending snapshot should not be in source tree at {:?}",
        source_tree_snap
    );

    // Now run cargo insta accept with INSTA_PENDING_DIR set
    let output = test_project
        .insta_cmd()
        .args(["accept"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "cargo insta accept failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The accepted snapshot should be in the source tree
    let accepted_snap = test_project
        .workspace_dir
        .join("src/snapshots/test_pending_dir__hello.snap");
    assert!(
        accepted_snap.exists(),
        "Accepted snapshot should be in source tree at {:?}",
        accepted_snap
    );

    // The pending snapshot should be removed
    assert!(
        !pending_snap.exists(),
        "Pending snapshot should be removed after accept"
    );
}

/// Test that INSTA_PENDING_DIR works with inline snapshots
#[test]
fn test_pending_dir_inline_snapshot() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_pending_dir_inline")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_inline() {
    insta::assert_snapshot!("Hello, Inline!", @"");
}
"#
            .to_string(),
        )
        .create_project();

    // Create a separate pending directory
    let pending_dir = test_project.workspace_dir.join("pending_output");
    fs::create_dir_all(&pending_dir).unwrap();

    // Run the test with INSTA_PENDING_DIR set
    test_project
        .insta_cmd()
        .args(["test"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // With INSTA_FORCE_PASS=1 (set by cargo insta test), individual tests pass
    // but cargo-insta returns non-zero because there are snapshots to review.

    // The .pending-snap file should be in the pending directory
    let pending_snap = pending_dir.join("src/.lib.rs.pending-snap");
    assert!(
        pending_snap.exists(),
        "Expected pending inline snapshot at {:?}, but it doesn't exist",
        pending_snap
    );

    // The .pending-snap file should NOT be in the source tree
    let source_tree_snap = test_project.workspace_dir.join("src/.lib.rs.pending-snap");
    assert!(
        !source_tree_snap.exists(),
        "Pending inline snapshot should not be in source tree"
    );
}

/// Test that cargo insta reject works with INSTA_PENDING_DIR
#[test]
fn test_pending_dir_reject() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_pending_dir_reject")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("reject_me", "This should be rejected");
}
"#
            .to_string(),
        )
        .create_project();

    let pending_dir = test_project.workspace_dir.join("pending_output");
    fs::create_dir_all(&pending_dir).unwrap();

    // Run the test to create a pending snapshot
    test_project
        .insta_cmd()
        .args(["test"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    let pending_snap =
        pending_dir.join("src/snapshots/test_pending_dir_reject__reject_me.snap.new");
    assert!(
        pending_snap.exists(),
        "Pending snapshot should exist before reject"
    );

    // Now reject it
    let output = test_project
        .insta_cmd()
        .args(["reject"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "cargo insta reject failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The pending snapshot should be removed
    assert!(
        !pending_snap.exists(),
        "Pending snapshot should be removed after reject"
    );

    // No snapshot should exist in the source tree
    let source_snap = test_project
        .workspace_dir
        .join("src/snapshots/test_pending_dir_reject__reject_me.snap");
    assert!(
        !source_snap.exists(),
        "Rejected snapshot should not be in source tree"
    );
}

/// Test inline snapshot acceptance with INSTA_PENDING_DIR
#[test]
fn test_pending_dir_inline_snapshot_accept() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_pending_dir_inline_accept")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_inline() {
    insta::assert_snapshot!("Hello, Inline Accept!", @"");
}
"#
            .to_string(),
        )
        .create_project();

    let pending_dir = test_project.workspace_dir.join("pending_output");
    fs::create_dir_all(&pending_dir).unwrap();

    // Run the test to create a pending inline snapshot
    test_project
        .insta_cmd()
        .args(["test"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    let pending_snap = pending_dir.join("src/.lib.rs.pending-snap");
    assert!(
        pending_snap.exists(),
        "Pending inline snapshot should exist"
    );

    // Accept the snapshot
    let output = test_project
        .insta_cmd()
        .args(["accept"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "cargo insta accept failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The pending snapshot should be removed
    assert!(
        !pending_snap.exists(),
        "Pending inline snapshot should be removed after accept"
    );

    // The source file should now contain the accepted inline snapshot
    let source_content = fs::read_to_string(test_project.workspace_dir.join("src/lib.rs")).unwrap();
    assert!(
        source_content.contains(r#"@"Hello, Inline Accept!""#),
        "Source file should contain accepted inline snapshot literal, got: {}",
        source_content
    );
}

/// Test that pending directory is auto-created when it doesn't exist
#[test]
fn test_pending_dir_auto_created() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_pending_dir_auto")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("auto", "Auto-created dir");
}
"#
            .to_string(),
        )
        .create_project();

    // Don't create the pending directory - it should be auto-created
    let pending_dir = test_project.workspace_dir.join("auto_pending");
    assert!(
        !pending_dir.exists(),
        "Pending dir should not exist initially"
    );

    // Run the test with INSTA_PENDING_DIR set to non-existent directory
    test_project
        .insta_cmd()
        .args(["test"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // cargo-insta returns non-zero because there are snapshots to review

    // The pending directory should now exist with the snapshot
    let pending_snap = pending_dir.join("src/snapshots/test_pending_dir_auto__auto.snap.new");
    assert!(
        pending_snap.exists(),
        "Pending snapshot should exist in auto-created directory"
    );
}

/// Test updating an existing snapshot with INSTA_PENDING_DIR
#[test]
fn test_pending_dir_update_existing() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_pending_dir_update")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("update", "Original content");
}
"#
            .to_string(),
        )
        .create_project();

    let pending_dir = test_project.workspace_dir.join("pending_output");
    fs::create_dir_all(&pending_dir).unwrap();

    // First, create and accept the initial snapshot
    test_project
        .insta_cmd()
        .args(["test"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    // cargo-insta returns non-zero because there are snapshots to review

    let output = test_project
        .insta_cmd()
        .args(["accept"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "cargo insta accept failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let accepted_snap = test_project
        .workspace_dir
        .join("src/snapshots/test_pending_dir_update__update.snap");
    assert!(
        accepted_snap.exists(),
        "Initial snapshot should be accepted"
    );

    // Now update the test to produce different output
    fs::write(
        test_project.workspace_dir.join("src/lib.rs"),
        r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("update", "Updated content");
}
"#,
    )
    .unwrap();

    // Run tests again - should create a new pending snapshot
    let output = test_project
        .insta_cmd()
        .args(["test"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // Test should fail because snapshot changed
    assert!(!output.status.success());

    // The .snap.new should be in the pending directory
    let pending_snap = pending_dir.join("src/snapshots/test_pending_dir_update__update.snap.new");
    assert!(
        pending_snap.exists(),
        "Updated pending snapshot should be in pending directory"
    );

    // The .snap.new should NOT be in the source tree
    let source_tree_snap = test_project
        .workspace_dir
        .join("src/snapshots/test_pending_dir_update__update.snap.new");
    assert!(
        !source_tree_snap.exists(),
        "Updated pending snapshot should not be in source tree"
    );
}

/// Test --check mode with INSTA_PENDING_DIR
#[test]
fn test_pending_dir_check_mode() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_pending_dir_check")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("check", "Check mode test");
}
"#
            .to_string(),
        )
        .create_project();

    let pending_dir = test_project.workspace_dir.join("pending_output");
    fs::create_dir_all(&pending_dir).unwrap();

    // Run test to create pending snapshot
    test_project
        .insta_cmd()
        .args(["test"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    // cargo-insta returns non-zero because there are snapshots to review

    // Verify pending snapshot exists
    let pending_snap = pending_dir.join("src/snapshots/test_pending_dir_check__check.snap.new");
    assert!(pending_snap.exists(), "Pending snapshot should exist");

    // Run with --check - should fail because there are pending snapshots
    let output = test_project
        .insta_cmd()
        .args(["test", "--check"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "cargo insta test --check should fail with pending snapshots"
    );
}

/// Test with pending directory outside the workspace
#[test]
fn test_pending_dir_outside_workspace() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_pending_dir_outside")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("outside", "Outside workspace");
}
"#
            .to_string(),
        )
        .create_project();

    // Create pending directory completely outside the workspace
    let pending_dir = tempfile::tempdir().unwrap();
    let pending_path = pending_dir.path();

    // Run the test with INSTA_PENDING_DIR set to external directory
    let output = test_project
        .insta_cmd()
        .args(["test"])
        .env("INSTA_PENDING_DIR", pending_path)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(!output.status.success());

    // The .snap.new file should be in the external pending directory
    let pending_snap =
        pending_path.join("src/snapshots/test_pending_dir_outside__outside.snap.new");
    assert!(
        pending_snap.exists(),
        "Pending snapshot should be in external pending directory at {:?}",
        pending_snap
    );

    // Accept the snapshot
    let output = test_project
        .insta_cmd()
        .args(["accept"])
        .env("INSTA_PENDING_DIR", pending_path)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "cargo insta accept failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The accepted snapshot should be in the source tree
    let accepted_snap = test_project
        .workspace_dir
        .join("src/snapshots/test_pending_dir_outside__outside.snap");
    assert!(
        accepted_snap.exists(),
        "Accepted snapshot should be in source tree"
    );

    // The pending snapshot should be removed
    assert!(
        !pending_snap.exists(),
        "Pending snapshot should be removed after accept"
    );
}

/// Test that INSTA_PENDING_DIR properly rejects external test paths.
///
/// External test paths (e.g., `path = "../tests/lib.rs"` in Cargo.toml) are outside
/// the workspace root. Using them with INSTA_PENDING_DIR would cause snapshots to
/// escape the pending directory, so insta should fail with a clear error.
#[test]
fn test_pending_dir_rejects_external_test_path() {
    // Create a project structure with an external test path:
    // temp_dir/
    // ├── proj/           <- package root (has Cargo.toml)
    // │   └── src/
    // └── tests/          <- OUTSIDE package root
    //     └── lib.rs
    let test_project = TestFiles::new()
        .add_file(
            "proj/Cargo.toml",
            r#"
[package]
name = "external_test_pending"
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
    "Hello from external test!".to_string()
}
"#
            .to_string(),
        )
        .add_file(
            "tests/lib.rs",
            r#"
use external_test_pending::hello;

#[test]
fn test_hello() {
    insta::assert_snapshot!(hello());
}
"#
            .to_string(),
        )
        .create_project();

    let proj_dir = test_project.workspace_dir.join("proj");
    let pending_dir = test_project.workspace_dir.join("pending_output");
    fs::create_dir_all(&pending_dir).unwrap();

    // Run the test with INSTA_PENDING_DIR set
    let output = test_project
        .insta_cmd()
        .current_dir(&proj_dir)
        .args(["test", "--"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined_output = format!("{}{}", stdout, stderr);

    // The test should fail with a clear error about external test paths
    assert!(
        !output.status.success(),
        "Test should fail when using INSTA_PENDING_DIR with external test path"
    );

    assert!(
        combined_output.contains("escape") || combined_output.contains("INSTA_PENDING_DIR"),
        "Error message should mention the external path issue. Got:\nSTDOUT: {}\nSTDERR: {}",
        stdout,
        stderr
    );

    // The pending snapshot should NOT be in the source tree (no silent escape)
    let source_tree_snap = test_project
        .workspace_dir
        .join("tests/snapshots/tlib__hello.snap.new");
    assert!(
        !source_tree_snap.exists(),
        "Pending snapshot should NOT be written to source tree"
    );
}

/// Test INSTA_PENDING_DIR with symlinked workspace (simulates Bazel's execroot).
///
/// Bazel creates an "execroot" directory containing symlinks to the real source files.
/// When tests run from execroot, the workspace path contains symlinks, but file paths
/// may resolve differently. This test ensures `strip_prefix` works without following
/// symlinks (which would break the path matching).
///
/// Structure:
/// ```
/// temp_dir/
/// ├── real_src/           <- actual source files
/// │   ├── Cargo.toml
/// │   └── src/lib.rs
/// ├── execroot/           <- symlink to real_src (like Bazel)
/// └── pending/            <- where snapshots go
/// ```
#[test]
#[cfg(unix)] // Symlinks work differently on Windows
fn test_pending_dir_with_symlinks_like_bazel() {
    // Create the real source directory with test project files
    let test_project = TestFiles::new()
        .add_cargo_toml("test_bazel_symlink")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("bazel_test", "Hello from Bazel-like setup!");
}
"#
            .to_string(),
        )
        .create_project();

    let real_src = &test_project.workspace_dir;

    // Create a separate directory to hold our "execroot" symlink
    let outer_dir = tempfile::tempdir().unwrap();
    let execroot = outer_dir.path().join("execroot");

    // Create symlink: execroot -> real_src (this is what Bazel does)
    std::os::unix::fs::symlink(real_src, &execroot).unwrap();

    // Create pending directory outside both
    let pending_dir = outer_dir.path().join("pending");
    fs::create_dir_all(&pending_dir).unwrap();

    // Run cargo-insta from the execroot (symlinked directory)
    // This simulates how Bazel runs tests
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cargo-insta"));
    TestProject::clean_env(&mut cmd);
    cmd.current_dir(&execroot);
    cmd.env("CARGO_TARGET_DIR", target_dir());
    cmd.env("INSTA_PENDING_DIR", &pending_dir);
    cmd.args(["test"]);
    cmd.stderr(Stdio::piped());
    cmd.stdout(Stdio::piped());

    let output = cmd.output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The test should run (may fail because snapshot doesn't exist, that's fine)
    // The key is that it shouldn't panic with "snapshot path is outside workspace"
    assert!(
        !stderr.contains("outside workspace") && !stdout.contains("outside workspace"),
        "Should not fail with 'outside workspace' error when using symlinks.\n\
         This error would indicate strip_prefix is following symlinks.\n\
         STDOUT: {}\nSTDERR: {}",
        stdout,
        stderr
    );

    // The pending snapshot should be created in the pending directory
    let pending_snap = pending_dir.join("src/snapshots/test_bazel_symlink__bazel_test.snap.new");
    assert!(
        pending_snap.exists(),
        "Pending snapshot should be created in pending directory.\n\
         Expected: {:?}\n\
         Pending dir contents: {:?}\n\
         STDOUT: {}\nSTDERR: {}",
        pending_snap,
        fs::read_dir(&pending_dir)
            .ok()
            .map(|d| d.filter_map(|e| e.ok()).collect::<Vec<_>>()),
        stdout,
        stderr
    );

    // The snapshot should NOT be in the real source tree
    let real_src_snap = real_src.join("src/snapshots/test_bazel_symlink__bazel_test.snap.new");
    assert!(
        !real_src_snap.exists(),
        "Pending snapshot should NOT be in real source tree"
    );

    // The snapshot should NOT be in the execroot (symlinked) location
    let execroot_snap = execroot.join("src/snapshots/test_bazel_symlink__bazel_test.snap.new");
    assert!(
        !execroot_snap.exists(),
        "Pending snapshot should NOT be in execroot"
    );
}
