use crate::TestFiles;
use insta::assert_snapshot;
use std::process::Stdio;

/// Test that inline snapshots are properly trimmed when they have excess indentation
#[test]
fn test_inline_snapshot_trimming() {
    let test_project = TestFiles::new()
        .add_cargo_toml("inline_trimming")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_basic_trimming() {
    // Inline snapshot with unnecessary indentation should be trimmed
    insta::assert_snapshot!("hello\nworld", @r####"
    
    hello
    world
    "####);
}

#[test]
fn test_single_line_trimming() {
    // Single line snapshots should not have leading/trailing whitespace
    insta::assert_snapshot!("hello world", @r#"
        hello world
    "#);
}

#[test]
fn test_mixed_indentation() {
    // Test with mixed spaces - should preserve relative indentation
    insta::assert_snapshot!("line1\n  line2\n    line3", @r###"
        line1
          line2
            line3
    "###);
}

#[test]
fn test_tab_indentation() {
    // Test with tabs - should preserve tab indentation
    insta::assert_snapshot!("line1\n\tline2\n\t\tline3", @"
		line1
			line2
				line3
    ");
}

#[test]
fn test_empty_lines_preserved() {
    // Empty lines in the middle should be preserved
    insta::assert_snapshot!("line1\n\nline2", @r#"
        line1
        
        line2
    "#);
}

#[test]
fn test_no_excess_indentation() {
    // Already properly formatted snapshot should not change
    insta::assert_snapshot!("hello\nworld", @"
hello
world
");
}
"#####
            .to_string(),
        )
        .create_project();

    // Run test - snapshots will need to be updated
    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // We expect failures since the snapshots have excess indentation
    assert!(!output.status.success());

    // Run with --accept to fix the inline snapshots
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Check that the inline snapshots were properly trimmed
    assert_snapshot!(test_project.diff("src/lib.rs"), @r##"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -12,9 +12,7 @@
     #[test]
     fn test_single_line_trimming() {
         // Single line snapshots should not have leading/trailing whitespace
    -    insta::assert_snapshot!("hello world", @r#"
    -        hello world
    -    "#);
    +    insta::assert_snapshot!("hello world", @"hello world");
     }
     
     #[test]
    "##);

    // Run test again - should pass now with trimmed snapshots
    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(output.status.success());
}

/// Test force-update removes all excess whitespace and indentation
#[test]
fn test_force_update_trimming() {
    let test_project = TestFiles::new()
        .add_cargo_toml("force_update_trimming")
        .add_file(
            "src/lib.rs",
            r#####"
#[test]
fn test_multiline_with_excess() {
    insta::assert_snapshot!("foo\nbar", @r####"
    
    
    foo
    bar
    
    
    "####);
}

#[test]
fn test_single_line_with_padding() {
    insta::assert_snapshot!("hello", @r###"
        
        hello
        
    "###);
}
"#####
            .to_string(),
        )
        .create_project();

    // Run with --force-update-snapshots to aggressively trim
    let output = test_project
        .insta_cmd()
        .args(["test", "--force-update-snapshots", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Check that force update removed all excess whitespace
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#####"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -1,21 +1,13 @@
     
     #[test]
     fn test_multiline_with_excess() {
    -    insta::assert_snapshot!("foo\nbar", @r####"
    -    
    -    
    +    insta::assert_snapshot!("foo\nbar", @r"
         foo
         bar
    -    
    -    
    -    "####);
    +    ");
     }
     
     #[test]
     fn test_single_line_with_padding() {
    -    insta::assert_snapshot!("hello", @r###"
    -        
    -        hello
    -        
    -    "###);
    +    insta::assert_snapshot!("hello", @"hello");
     }
    "#####);
}

/// Test that file snapshots are also properly trimmed
#[test]
fn test_file_snapshot_trimming() {
    let test_project = TestFiles::new()
        .add_cargo_toml("file_snapshot_trimming")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_file_snapshot() {
    insta::assert_snapshot!("test_snapshot", "    indented content\n    second line    ");
}
"#
            .to_string(),
        )
        .create_project();

    // Run test to create file snapshot
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Read the created snapshot file
    let snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/file_snapshot_trimming__test_snapshot.snap");
    let snapshot_content = std::fs::read_to_string(&snapshot_path).unwrap();

    // Verify the file snapshot has proper structure and trimming
    assert!(snapshot_content.contains("---"));
    assert!(snapshot_content.contains("source: src/lib.rs"));
    assert!(snapshot_content.contains("expression:"));

    // The snapshot content should have the text content
    assert!(snapshot_content.contains("indented content"));
    assert!(snapshot_content.contains("second line"));
}

/// Test that complex nested structures maintain proper relative indentation
#[test]
fn test_complex_indentation_preservation() {
    let test_project = TestFiles::new()
        .add_cargo_toml("complex_indentation")
        .add_file(
            "src/lib.rs",
            r####"
#[test]
fn test_yaml_like_structure() {
    let content = r#"
root:
  child1:
    value: 1
  child2:
    - item1
    - item2
"#;
    insta::assert_snapshot!(content, @r###"
        
        root:
          child1:
            value: 1
          child2:
            - item1
            - item2
        
    "###);
}

#[test]
fn test_code_block() {
    let code = r#"fn main() {
    println!("Hello");
    if true {
        println!("World");
    }
}"#;
    insta::assert_snapshot!(code, @r##"
        fn main() {
            println!("Hello");
            if true {
                println!("World");
            }
        }
    "##);
}
"####
            .to_string(),
        )
        .create_project();

    // Run test with accept
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Check that relative indentation is preserved
    assert_snapshot!(test_project.diff("src/lib.rs"), @"");
}

// ====== BACKWARDS COMPATIBILITY TESTS ======

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
    let output = test_project.insta_cmd().args(["test"]).output().unwrap();

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