use std::process::Stdio;

use crate::TestFiles;

/// Test that nextest with doctests shows a warning
#[test]
fn test_nextest_doctest_warning() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_nextest_doctest_warning")
        .add_file(
            "src/lib.rs",
            r#"
/// This is a function with a doctest
///
/// ```
/// assert_eq!(test_nextest_doctest_warning::add(2, 2), 4);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_simple() {
    insta::assert_snapshot!("test_value", @"test_value");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with nextest and capture stderr to check for warning
    let output = test_project
        .insta_cmd()
        .args(["test", "--test-runner", "nextest", "--accept"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "warning: insta won't run a separate doctest process when using nextest in the future"
        ),
        "Expected warning message not found in stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Pass `--disable-nextest-doctest` to update to this behavior now and silence this warning"),
        "Expected flag suggestion not found in stderr:\n{stderr}"
    );
}

/// Test that nextest with --disable-nextest-doctest flag doesn't show warning
#[test]
fn test_nextest_doctest_flag_no_warning() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_nextest_doctest_flag")
        .add_file(
            "src/lib.rs",
            r#"
/// This is a function with a doctest
///
/// ```
/// assert_eq!(test_nextest_doctest_flag::add(2, 2), 4);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_simple() {
    insta::assert_snapshot!("test_value", @"test_value");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with nextest and the flag, capture stderr to verify no warning
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--test-runner",
            "nextest",
            "--disable-nextest-doctest",
            "--accept",
        ])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("warning: insta won't run a separate doctest process"),
        "Warning message should not appear when flag is used:\n{stderr}"
    );
}

/// Test that no warning appears when there are no doctests
#[test]
fn test_nextest_no_doctests_no_warning() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_nextest_no_doctests")
        .add_file(
            "src/lib.rs",
            r#"
// No doctests here
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_simple() {
    insta::assert_snapshot!("test_value", @"test_value");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with nextest when there are no doctests
    let output = test_project
        .insta_cmd()
        .args(["test", "--test-runner", "nextest", "--accept"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("warning: insta won't run a separate doctest process"),
        "Warning should not appear when there are no doctests:\n{stderr}"
    );
}

/// Test that cargo-test doesn't show the warning even with doctests
#[test]
fn test_cargo_test_no_warning() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_cargo_test_no_warning")
        .add_file(
            "src/lib.rs",
            r#"
/// This is a function with a doctest
///
/// ```
/// assert_eq!(test_cargo_test_no_warning::add(2, 2), 4);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_simple() {
    insta::assert_snapshot!("test_value", @"test_value");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with cargo-test (should not show warning)
    let output = test_project
        .insta_cmd()
        .args(["test", "--test-runner", "cargo-test", "--accept"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("warning: insta won't run a separate doctest process"),
        "Warning should not appear with cargo-test runner:\n{stderr}"
    );
}
