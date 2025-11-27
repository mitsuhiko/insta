use insta::assert_snapshot;
use std::process::Stdio;

use crate::TestFiles;

// Tests for backwards compatibility with older snapshot formats.
//
// These tests verify that upgrading insta doesn't cause existing valid
// snapshots to fail. Legacy formats should still match via matches_legacy.
//
// See: https://github.com/mitsuhiko/insta/pull/819#issuecomment-3583709431

/// Test that single-line content stored in multiline format still passes.
///
/// This reproduces the issue reported at:
/// https://github.com/jj-vcs/jj/commit/2f0132a765518a8df705fd00e10dcc05862c3799
#[test]
fn test_no_reformat_single_line_in_multiline_format() {
    let test_project = TestFiles::new()
        .add_cargo_toml("no_reformat_multiline")
        .add_file(
            "src/lib.rs",
            // This is the old format - single-line content in multiline format
            // (as seen in jj before the upgrade)
            r#####"
#[test]
fn test_single_line_in_multiline() {
    insta::assert_snapshot!(get_status(), @r"
    Unconflicted Mode(FILE) 0839b2e9412b ctime=0:0 mtime=0:0 size=0 flags=0 file1.txt
    ");
}

fn get_status() -> &'static str {
    "Unconflicted Mode(FILE) 0839b2e9412b ctime=0:0 mtime=0:0 size=0 flags=0 file1.txt"
}
"#####
                .to_string(),
        )
        .create_project();

    // Run tests with --check - should pass (legacy format still matches)
    // If this fails, it means insta incorrectly rejects the legacy format
    let output = test_project
        .insta_cmd()
        .args(["test", "--check", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "Test should pass with --check (legacy format matches). Stderr: {stderr}"
    );

    // Should show legacy format warning
    let combined = format!("{stderr}\n{stdout}");
    assert!(
        combined.contains("existing value is in a legacy format"),
        "Should show legacy format warning. Output: {combined}"
    );

    // The file should NOT be modified (tests don't modify files)
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
}

/// Similar test with the UU file example from jj
#[test]
fn test_no_reformat_raw_string_multiline() {
    let test_project = TestFiles::new()
        .add_cargo_toml("no_reformat_raw_string")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_uu_file() {
    insta::assert_snapshot!(get_output(), @r#"
    UU file
    "#);
}

fn get_output() -> &'static str {
    "UU file"
}
"#####
                .to_string(),
        )
        .create_project();

    // Run tests with --check - should pass (legacy format still matches)
    let output = test_project
        .insta_cmd()
        .args(["test", "--check", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should pass with --check (legacy format matches). Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The file should NOT be modified (tests don't modify files)
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
}

/// Test that --force-update-snapshots DOES reformat to canonical form
#[test]
fn test_force_update_does_reformat() {
    let test_project = TestFiles::new()
        .add_cargo_toml("force_update_reformats")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_single_line_in_multiline() {
    insta::assert_snapshot!(get_status(), @r"
    single line content
    ");
}

fn get_status() -> &'static str {
    "single line content"
}
"#####
                .to_string(),
        )
        .create_project();

    // Run tests WITH --force-update-snapshots
    let output = test_project
        .insta_cmd()
        .args(["test", "--force-update-snapshots", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // With --force-update-snapshots, the file SHOULD be reformatted to canonical form
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -1,9 +1,7 @@
     
     #[test]
     fn test_single_line_in_multiline() {
    -    insta::assert_snapshot!(get_status(), @r"
    -    single line content
    -    ");
    +    insta::assert_snapshot!(get_status(), @"single line content");
     }
     
     fn get_status() -> &'static str {
    "#);
}

/// Test that actual multiline content in legacy format works correctly.
/// normalize_inline handles multiline correctly, so matches_latest succeeds
/// and there's no legacy fallback needed (no warning shown).
#[test]
fn test_no_reformat_true_multiline_legacy() {
    let test_project = TestFiles::new()
        .add_cargo_toml("true_multiline_legacy")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_multiline() {
    insta::assert_snapshot!(get_output(), @r"
    line1
    line2
    ");
}

fn get_output() -> &'static str {
    "line1\nline2"
}
"#####
                .to_string(),
        )
        .create_project();

    // Run tests with --check - should pass without needing any changes
    let output = test_project
        .insta_cmd()
        .args(["test", "--check", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should pass with --check. Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Note: No legacy warning expected here because normalize_inline correctly
    // handles true multiline content. The legacy warning only appears when
    // matches_latest fails but matches_legacy passes.

    // The file should NOT be modified (tests don't modify files)
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
}

/// Test multiline content with relative indentation is preserved.
/// The first line has extra indentation that should be kept.
#[test]
fn test_multiline_with_relative_indentation() {
    let test_project = TestFiles::new()
        .add_cargo_toml("relative_indent")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_indented_code() {
    insta::assert_snapshot!(get_output(), @r"
    if condition:
        return value
    ");
}

fn get_output() -> &'static str {
    "if condition:\n    return value"
}
"#####
                .to_string(),
        )
        .create_project();

    // Run tests with --check - should pass
    let output = test_project
        .insta_cmd()
        .args(["test", "--check", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should pass - relative indentation preserved. Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The file should NOT be modified (tests don't modify files)
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
}

/// Test that intentional leading whitespace in snapshots doesn't get stripped.
/// This is an edge case where someone INTENTIONALLY has leading spaces in their snapshot.
/// The test should FAIL because the actual value doesn't have those spaces.
#[test]
fn test_intentional_leading_spaces_should_fail() {
    let test_project = TestFiles::new()
        .add_cargo_toml("intentional_spaces")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_with_leading_spaces() {
    // The snapshot has intentional leading spaces, but the actual value doesn't
    insta::assert_snapshot!(get_content(), @"    content with leading spaces");
}

fn get_content() -> &'static str {
    "content with leading spaces"
}
"#####
                .to_string(),
        )
        .create_project();

    // Run tests with --check - should FAIL because values don't match
    let output = test_project
        .insta_cmd()
        .args(["test", "--check", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // This should FAIL - the intentional leading spaces should NOT be stripped
    assert!(
        !output.status.success(),
        "Test should FAIL when snapshot has intentional leading spaces that don't match actual. \
         If this passes, it means matches_legacy is too permissive."
    );
}

/// Test that content with trailing whitespace line works correctly.
/// Even though the heuristic triggers (has \n, 1 line after trim), the content
/// has no leading whitespace so trim_start() is a no-op.
#[test]
fn test_trailing_whitespace_line() {
    let test_project = TestFiles::new()
        .add_cargo_toml("trailing_ws")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_trailing() {
    insta::assert_snapshot!(get_content(), @r"
content
    ");
}

fn get_content() -> &'static str {
    "content"
}
"#####
                .to_string(),
        )
        .create_project();

    // Run tests with --check - should pass
    let output = test_project
        .insta_cmd()
        .args(["test", "--check", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Content with trailing whitespace line should pass. Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The file should NOT be modified
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
}

/// Test that empty snapshots in multiline format work correctly.
/// This is an edge case where the snapshot is just whitespace/newlines.
#[test]
fn test_empty_snapshot_in_multiline_format() {
    let test_project = TestFiles::new()
        .add_cargo_toml("empty_multiline")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_empty() {
    insta::assert_snapshot!(get_empty(), @r"
    ");
}

fn get_empty() -> &'static str {
    ""
}
"#####
                .to_string(),
        )
        .create_project();

    // Run tests with --check - should pass
    let output = test_project
        .insta_cmd()
        .args(["test", "--check", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Empty snapshot should pass. Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The file should NOT be modified (tests don't modify files)
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
}
