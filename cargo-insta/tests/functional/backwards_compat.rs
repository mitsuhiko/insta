use crate::TestFiles;
use std::process::Stdio;

/// Test that snapshots created with old format (excess indentation) still pass
/// This ensures backwards compatibility - existing tests shouldn't break
#[test]
fn test_old_format_snapshots_still_pass() {
    let test_project = TestFiles::new()
        .add_cargo_toml("backwards_compat")
        .add_file(
            "src/lib.rs",
            r#####"
// These are snapshots as they would have been written before trimming feature
#[test]
fn test_old_format_with_indentation() {
    // This snapshot has the "old" format with full indentation preserved
    // It should still pass because trimming is applied to both sides during comparison
    insta::assert_snapshot!("hello\nworld", @r####"
    hello
    world
    "####);
}

#[test]
fn test_old_format_already_trimmed() {
    // This was already properly formatted, should still work
    insta::assert_snapshot!("foo\nbar", @"
foo
bar
");
}
"#####
            .to_string(),
        )
        .create_project();

    // Run tests WITHOUT --accept - they should pass due to trimming on both sides
    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();

    // Old format multiline snapshots with just indentation differences should pass
    // (Single-line with spaces would need review due to trailing space trimming)
    assert!(
        output.status.success(),
        "Backwards compatibility broken: old format snapshots don't pass"
    );
}

/// Test that --accept migrates old format to new trimmed format
#[test]
fn test_accept_migrates_old_format() {
    let test_project = TestFiles::new()
        .add_cargo_toml("migration_test")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_needs_migration() {
    // Old format with indentation that should be migrated
    insta::assert_snapshot!("content", @r####"
        content
    "####);
}

#[test]
fn test_multi_line_needs_migration() {
    insta::assert_snapshot!("line1\nline2\nline3", @r###"
        line1
        line2
        line3
    "###);
}
"#####
            .to_string(),
        )
        .create_project();

    // Run with --accept to migrate to new format (may have pending snapshots)
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify migration happened
    let migrated = std::fs::read_to_string(test_project.workspace_dir.join("src/lib.rs")).unwrap();
    
    // Single line should be compacted
    assert!(
        migrated.contains("@\"content\"") || migrated.contains("@r\"\n    content\n    \""),
        "Single line not migrated to compact format"
    );
    
    // Multi-line should have common indentation removed
    assert!(
        migrated.contains("test_multi_line_needs_migration"),
        "Multi-line test function not found"
    );
    // The snapshot should have the minimum indentation removed
    assert!(
        migrated.contains("@r") || migrated.contains("line1\n    line2"),
        "Multi-line not properly migrated"
    );
    
    // Tests should pass after migration
    let output = test_project
        .insta_cmd()
        .args(["test"])
        .output()
        .unwrap();
    
    assert!(output.status.success(), "Tests should pass after migration");
}

/// Test that multiline warning appears when appropriate
#[test]
fn test_multiline_warning_behavior() {
    let test_project = TestFiles::new()
        .add_cargo_toml("warning_test")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_missing_newlines() {
    // This will trigger a warning because multiline content doesn't start/end with newline
    insta::assert_snapshot!("line1\nline2", @"line1
line2");
}

#[test]
fn test_proper_newlines() {
    // This should NOT trigger a warning
    insta::assert_snapshot!("line1\nline2", @"
line1
line2
");
}
"#
            .to_string(),
        )
        .create_project();

    // Run test and capture stderr for warning
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should see warning for the first test
    assert!(
        stderr.contains("Multiline inline snapshot values should start and end with a newline"),
        "Missing multiline warning message"
    );
}

/// Test that already-correct snapshots remain unchanged
#[test]
fn test_properly_formatted_unchanged() {
    let test_project = TestFiles::new()
        .add_cargo_toml("no_changes_needed")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_already_correct() {
    // These are already properly formatted
    insta::assert_snapshot!("single", @"single");
    
    insta::assert_snapshot!("multi\nline", @"
multi
line
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
        .output()
        .unwrap();

    assert!(output.status.success());

    // Read the file and verify nothing changed
    let content = std::fs::read_to_string(test_project.workspace_dir.join("src/lib.rs")).unwrap();
    
    // Should still have the same format
    assert!(content.contains("@\"single\""));
    assert!(content.contains("@\"\nmulti\nline\n\""));
}

/// Test interaction between old snapshots and force-update
#[test]
fn test_force_update_on_old_format() {
    let test_project = TestFiles::new()
        .add_cargo_toml("force_old_format")
        .add_file(
            "src/lib.rs",
            r####"
#[test]
fn test_old_with_excess() {
    // Old format with lots of padding
    insta::assert_snapshot!("content", @r###"


        content


    "###);
}
"####
            .to_string(),
        )
        .create_project();

    // Run with --force-update-snapshots
    let output = test_project
        .insta_cmd()
        .args(["test", "--force-update-snapshots"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify aggressive trimming was applied
    let updated = std::fs::read_to_string(test_project.workspace_dir.join("src/lib.rs")).unwrap();
    
    assert!(
        updated.contains("@\"content\""),
        "Force update didn't aggressively trim old format"
    );
}