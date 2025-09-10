use crate::TestFiles;
use std::process::Stdio;

/// # Inline Snapshot Leading Newline Tests
///
/// These tests verify the new behavior where multiline inline snapshots
/// must start with a newline after the opening delimiter.
///
/// ## New Behavior:
/// 1. Multiline snapshots should start with a newline after the delimiter
/// 2. If they don't, a warning is issued
/// 3. The leading newline is stripped during processing
/// 4. Single-line snapshots are unaffected
///
/// ## Backwards Compatibility:
/// - Old snapshots with excess indentation still work (trimming already existed)
/// - No warnings are issued for indentation (only for missing newlines)
#[test]
fn test_warning_only_for_missing_newline() {
    // Test 1: Missing leading newline - SHOULD WARN
    let test_project = TestFiles::new()
        .add_cargo_toml("missing_newline")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_missing() {
    insta::assert_snapshot!("line1\nline2", @"line1
line2");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Multiline inline snapshot values should start and end with a newline"),
        "Should warn for missing leading newline"
    );
    assert!(
        stderr.contains("The existing value's first line is `line1`"),
        "Warning should show the problematic line"
    );

    // Test 2: Proper leading newline - SHOULD NOT WARN
    let test_project = TestFiles::new()
        .add_cargo_toml("proper_newline")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_proper() {
    insta::assert_snapshot!("line1\nline2", @"
line1
line2
");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Multiline inline snapshot values should start and end with a newline"),
        "Should NOT warn when leading newline is present. Got: {stderr}"
    );

    // Test 3: Single-line - SHOULD NOT WARN
    let test_project = TestFiles::new()
        .add_cargo_toml("single_line")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_single() {
    insta::assert_snapshot!("single", @"single");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Multiline inline snapshot values should start and end with a newline"),
        "Should NOT warn for single-line snapshots. Got: {stderr}"
    );
}

/// Test that leading newlines are properly handled (stripped from multiline)
#[test]
fn test_leading_newline_processing() {
    let test_project = TestFiles::new()
        .add_cargo_toml("newline_processing")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_multiline_with_leading_newline() {
    // The leading newline should be stripped during processing
    let value = "content";
    insta::assert_snapshot!(value, @"
content
");
}

#[test]
fn test_multiline_with_indentation() {
    // Leading newline + indentation trimming (pre-existing feature)
    let value = "line1\nline2";
    insta::assert_snapshot!(value, @"
        line1
        line2
    ");
}
"#
            .to_string(),
        )
        .create_project();

    // Tests should pass
    let output = test_project.insta_cmd().args(["test"]).output().unwrap();
    assert!(
        output.status.success(),
        "Tests should pass with proper newline handling"
    );
}

/// Test backwards compatibility - old format multiline without leading newline still works
#[test]
fn test_backwards_compatibility() {
    let test_project = TestFiles::new()
        .add_cargo_toml("backwards_compat")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_old_format_multiline() {
    // Old format without leading newline - should still pass but with warning
    insta::assert_snapshot!("hello\nworld", @"hello
world");
}
"#
            .to_string(),
        )
        .create_project();

    // Run test - should pass despite old format
    let output = test_project
        .insta_cmd()
        .args(["test"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should warn about missing leading newline
    assert!(
        stderr.contains("Multiline inline snapshot values should start and end with a newline"),
        "Should warn about missing leading newline in old format"
    );

    // But should still pass (backwards compatibility)
    assert!(
        output.status.success(),
        "Old format should still pass with warning"
    );
}

/// Test that no warnings are issued for excess indentation (only for missing newlines)
#[test]
fn test_no_indentation_warnings() {
    let test_project = TestFiles::new()
        .add_cargo_toml("indentation_test")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_excess_indentation() {
    // Has leading newline but lots of indentation - should NOT warn
    insta::assert_snapshot!("content", @"
            content
        ");
}

#[test]
fn test_multiline_excess_indentation() {
    // Multiline with proper leading newline but excess indentation - should NOT warn
    insta::assert_snapshot!("line1\nline2", @"
            line1
            line2
        ");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with --accept
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should pass without warnings
    assert!(output.status.success(), "Tests should pass");

    // Verify NO warnings about missing newlines (they have proper newlines)
    assert!(
        !stderr.contains("Multiline inline snapshot values should start and end with a newline"),
        "Should NOT warn about excess indentation (only missing newlines trigger warnings)"
    );
}

/// Test edge cases for single-line vs multiline detection
#[test]
fn test_single_vs_multiline_detection() {
    let test_project = TestFiles::new()
        .add_cargo_toml("line_detection")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_single_line_with_escaped_n() {
    // Contains literal backslash-n, not a newline - single line
    insta::assert_snapshot!("has\\\\n", @"has\\n");
}

#[test]
fn test_actual_multiline() {
    // Actual multiline content - should require leading newline
    insta::assert_snapshot!("line1\nline2", @"
line1
line2
");
}

#[test]
fn test_empty_string() {
    // Empty strings are single-line
    insta::assert_snapshot!("", @"");
}

#[test]
fn test_whitespace_only() {
    // Whitespace-only with trimming
    insta::assert_snapshot!("   ", @"   ");
}
"#
            .to_string(),
        )
        .create_project();

    // Run with --accept in case any snapshots need updating
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "All line detection tests should pass"
    );
}

/// Test that warnings persist across runs until fixed
#[test]
fn test_warning_persistence() {
    let test_project = TestFiles::new()
        .add_cargo_toml("warning_persist")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_needs_newline() {
    // Missing leading newline
    insta::assert_snapshot!("line1\nline2", @"line1
line2");
}
"#
            .to_string(),
        )
        .create_project();

    // First run should warn
    let output = test_project
        .insta_cmd()
        .args(["test"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Multiline inline snapshot values should start and end with a newline"),
        "Should warn on first run"
    );
    assert!(output.status.success());

    // Second run should still warn (warning persists until fixed)
    let output = test_project
        .insta_cmd()
        .args(["test"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Multiline inline snapshot values should start and end with a newline"),
        "Should continue warning until format is fixed"
    );
    assert!(output.status.success());
}
