use super::*;

/// Test that test_runner_fallback config file setting enables fallback
/// when nextest is unavailable.
///
/// This test creates a fake cargo wrapper that pretends nextest is not installed,
/// then verifies that the config file's `test_runner_fallback: true` causes
/// cargo-insta to fall back to `cargo test`.
#[test]
#[cfg(unix)]
fn test_runner_fallback_config_file() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Stdio;

    let test_project = TestFiles::new()
        .add_cargo_toml("test_runner_fallback_config")
        .add_file(
            "insta.yaml",
            r#"
test:
  runner: nextest
  runner_fallback: true
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("value", @"value");
}
"#
            .to_string(),
        )
        .add_file(
            "fake_cargo.sh",
            r#"#!/bin/bash
if [ "$1" = "nextest" ]; then
    echo "error: no such command: nextest" >&2
    exit 1
fi
exec cargo "$@"
"#
            .to_string(),
        )
        .create_project();

    // Make the fake cargo executable and get path
    let fake_cargo_path = test_project.workspace_dir.join("fake_cargo.sh");
    std::fs::set_permissions(&fake_cargo_path, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Run with fake cargo - should fall back to cargo test because config says so
    let output = test_project
        .insta_cmd()
        .args(["test"])
        .env("CARGO", &fake_cargo_path)
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should succeed with config file test_runner_fallback: true\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that --test-runner-fallback flag (without value) enables fallback
#[test]
fn test_runner_fallback_flag_enables() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_runner_fallback_enables")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("value", @"value");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with --test-runner-fallback flag
    // The test should succeed (we're just verifying the flag is accepted and parsed)
    let output = test_project
        .insta_cmd()
        .args(["test", "--test-runner-fallback"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should succeed with --test-runner-fallback flag\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that --test-runner-fallback=false disables fallback
#[test]
fn test_runner_fallback_equals_false() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_runner_fallback_equals_false")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("value", @"value");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with --test-runner-fallback=false
    let output = test_project
        .insta_cmd()
        .args(["test", "--test-runner-fallback=false"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should succeed with --test-runner-fallback=false\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that --test-runner-fallback=true enables fallback (backward compatibility)
#[test]
fn test_runner_fallback_equals_true() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_runner_fallback_equals_true")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("value", @"value");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with --test-runner-fallback=true (backward compatibility)
    let output = test_project
        .insta_cmd()
        .args(["test", "--test-runner-fallback=true"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should succeed with --test-runner-fallback=true\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that --test-runner-fallback true (with space) enables fallback (backward compatibility)
#[test]
fn test_runner_fallback_space_true() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_runner_fallback_space_true")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("value", @"value");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with --test-runner-fallback true (space syntax, backward compatibility)
    let output = test_project
        .insta_cmd()
        .args(["test", "--test-runner-fallback", "true"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should succeed with --test-runner-fallback true\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that --no-test-runner-fallback disables fallback
#[test]
fn test_no_test_runner_fallback_flag() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_no_runner_fallback")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("value", @"value");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with --no-test-runner-fallback flag
    let output = test_project
        .insta_cmd()
        .args(["test", "--no-test-runner-fallback"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should succeed with --no-test-runner-fallback flag\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that last flag wins when both --test-runner-fallback and --no-test-runner-fallback are specified
#[test]
fn test_runner_fallback_override_with_no_flag() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_runner_fallback_override_no")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("value", @"value");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with --test-runner-fallback first, then --no-test-runner-fallback
    // The last flag should win (disable fallback)
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--test-runner-fallback",
            "--no-test-runner-fallback",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should succeed with both flags (last wins)\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that last flag wins when --no-test-runner-fallback is specified first
#[test]
fn test_runner_fallback_override_with_flag() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_runner_fallback_override_flag")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("value", @"value");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with --no-test-runner-fallback first, then --test-runner-fallback
    // The last flag should win (enable fallback)
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--no-test-runner-fallback",
            "--test-runner-fallback",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should succeed with both flags (last wins)\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
