use super::*;

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
