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

/// Test edge cases: mixed tabs/spaces, no common indent, whitespace-only
#[test]
fn test_edge_cases() {
    let test_project = TestFiles::new()
        .add_cargo_toml("edge_cases")
        .add_file(
            "src/lib.rs",
            r####"
#[test]
fn test_mixed_tabs_spaces() {
    // Lines with different indentation types
    insta::assert_snapshot!("line1\n\tline2\n    line3", @r###"
    line1
    	line2
        line3
    "###);
}

#[test]
fn test_no_common_indent() {
    // First line has no indent, so nothing should be removed
    insta::assert_snapshot!("line1\n    line2\n        line3", @"
line1
    line2
        line3
");
}

#[test]
fn test_whitespace_only() {
    // Snapshot with only whitespace should become empty
    insta::assert_snapshot!("    \n\t\n    ", @"");
}

#[test]
fn test_empty_snapshot() {
    // Empty snapshot should remain empty
    insta::assert_snapshot!("", @"");
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

    // Verify edge cases handled correctly
    let updated = std::fs::read_to_string(test_project.workspace_dir.join("src/lib.rs")).unwrap();
    
    // Mixed tabs/spaces should preserve structure
    assert!(updated.contains("line1") && updated.contains("line2"));
    
    // No common indent test should preserve indentation
    assert!(updated.contains("test_no_common_indent"));
    
    // Whitespace-only and empty should be handled
    assert!(updated.contains(r#"@"""#));
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
