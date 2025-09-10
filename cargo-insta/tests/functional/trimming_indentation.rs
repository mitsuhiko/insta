use crate::TestFiles;
use insta::assert_snapshot;

/// Test that the min_indentation logic correctly identifies and removes
/// the common leading whitespace from inline snapshots
#[test]
fn test_min_indentation_removal() {
    let test_project = TestFiles::new()
        .add_cargo_toml("min_indentation_test")
        .add_file(
            "src/lib.rs",
            "
#[test]
fn test_common_4_space_indent() {
    // All lines have at least 4 spaces - those should be removed
    insta::assert_snapshot!(\"foo\\nbar\\nbaz\", @r#\"
    foo
    bar
    baz
    \"#);
}

#[test]
fn test_common_8_space_indent() {
    // All lines have 8 spaces - all 8 should be removed
    insta::assert_snapshot!(\"foo\\nbar\", @r#\"
        foo
        bar
    \"#);
}

#[test]
fn test_mixed_indent_preserves_relative() {
    // Lines have 4, 6, 8 spaces - should remove 4 from all, preserving relative
    insta::assert_snapshot!(\"foo\\n  bar\\n    baz\", @r#\"
    foo
      bar
        baz
    \"#);
}

#[test]
fn test_empty_lines_ignored_for_min() {
    // Empty lines should not affect min indentation calculation
    insta::assert_snapshot!(\"foo\\n\\nbar\", @r#\"
    foo
    
    bar
    \"#);
}

#[test]
fn test_no_common_indent() {
    // One line has no indent, so nothing should be removed
    insta::assert_snapshot!(\"foo\\n  bar\\nbaz\", @r#\"
foo
  bar
baz
    \"#);
}
"
            .to_string(),
        )
        .create_project();

    // Run tests - they should fail because snapshots need updating
    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    
    assert!(!output.status.success());

    // Accept to apply trimming
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();
    
    assert!(output.status.success());

    // Verify the trimming was applied correctly
    assert_snapshot!(test_project.diff("src/lib.rs"), @r###"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -2,10 +2,10 @@
     #[test]
     fn test_common_4_space_indent() {
         // All lines have at least 4 spaces - those should be removed
    -    insta::assert_snapshot!("foo\nbar\nbaz", @r#"
    -    foo
    -    bar
    -    baz
    +    insta::assert_snapshot!("foo\nbar\nbaz", @"
    +foo
    +bar
    +baz
         "#);
     }
     
    @@ -13,8 +13,8 @@
     fn test_common_8_space_indent() {
         // All lines have 8 spaces - all 8 should be removed
         insta::assert_snapshot!("foo\nbar", @r#"
    -        foo
    -        bar
    +foo
    +bar
         "#);
     }
     
    @@ -22,9 +22,9 @@
     fn test_mixed_indent_preserves_relative() {
         // Lines have 4, 6, 8 spaces - should remove 4 from all, preserving relative
         insta::assert_snapshot!("foo\n  bar\n    baz", @r#"
    -    foo
    -      bar
    -        baz
    +foo
    +  bar
    +    baz
         "#);
     }
     
    @@ -32,9 +32,9 @@
     fn test_empty_lines_ignored_for_min() {
         // Empty lines should not affect min indentation calculation
         insta::assert_snapshot!("foo\n\nbar", @r#"
    -    foo
    -    
    -    bar
    +foo
    +
    +bar
         "#);
     }
     
    "###);
}

/// Test that tabs are handled correctly as indentation units
#[test]
fn test_tab_indentation_handling() {
    let test_project = TestFiles::new()
        .add_cargo_toml("tab_indentation_test")
        .add_file(
            "src/lib.rs",
            "
#[test]
fn test_common_tab_indent() {
    // All lines have one tab - it should be removed
    insta::assert_snapshot!(\"foo\\nbar\", @\"
\tfoo
\tbar
    \");
}

#[test]
fn test_mixed_tabs_preserves_relative() {
    // Lines have 1, 2 tabs - should remove 1 from all
    insta::assert_snapshot!(\"foo\\n\\tbar\", @\"
\tfoo
\t\tbar
    \");
}

#[test]
fn test_mixed_spaces_and_tabs() {
    // Mixed spaces and tabs - finds common prefix
    insta::assert_snapshot!(\"foo\\nbar\", @\"
 \tfoo
 \tbar
    \");
}
"
            .to_string(),
        )
        .create_project();

    // Run and accept
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();
    
    assert!(output.status.success());

    // Verify tab handling
    assert_snapshot!(test_project.diff("src/lib.rs"), @r###"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -3,8 +3,8 @@
     fn test_common_tab_indent() {
         // All lines have one tab - it should be removed
         insta::assert_snapshot!("foo\nbar", @"
    -	foo
    -	bar
    +foo
    +bar
         ");
     }
     
    @@ -12,8 +12,8 @@
     fn test_mixed_tabs_preserves_relative() {
         // Lines have 1, 2 tabs - should remove 1 from all
         insta::assert_snapshot!("foo\n\tbar", @"
    -	foo
    -		bar
    +foo
    +	bar
         ");
     }
     
    @@ -21,8 +21,8 @@
     fn test_mixed_spaces_and_tabs() {
         // Mixed spaces and tabs - finds common prefix
         insta::assert_snapshot!("foo\nbar", @"
    - 	foo
    - 	bar
    +foo
    +bar
         ");
     }
    "###);
}

/// Test that single-line snapshots are trimmed appropriately
#[test]
fn test_single_line_trimming() {
    let test_project = TestFiles::new()
        .add_cargo_toml("single_line_test")
        .add_file(
            "src/lib.rs",
            "
#[test]
fn test_single_line_with_spaces() {
    // Single line with leading/trailing spaces should be trimmed
    insta::assert_snapshot!(\"hello\", @\"    hello    \");
}

#[test]
fn test_single_line_multiline_string() {
    // Single line in multiline format should be compacted
    insta::assert_snapshot!(\"hello\", @r#\"
    hello
    \"#);
}
"
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();
    
    assert!(output.status.success());

    // Verify single line handling
    assert_snapshot!(test_project.diff("src/lib.rs"), @r###"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -2,14 +2,12 @@
     #[test]
     fn test_single_line_with_spaces() {
         // Single line with leading/trailing spaces should be trimmed
    -    insta::assert_snapshot!("hello", @"    hello    ");
    +    insta::assert_snapshot!("hello", @"hello");
     }
     
     #[test]
     fn test_single_line_multiline_string() {
         // Single line in multiline format should be compacted
    -    insta::assert_snapshot!("hello", @r#"
    -    hello
    -    "#);
    +    insta::assert_snapshot!("hello", @"hello");
     }
    "###);
}

/// Test edge cases around empty content and whitespace-only lines
#[test]
fn test_empty_and_whitespace_edge_cases() {
    let test_project = TestFiles::new()
        .add_cargo_toml("edge_cases_test")
        .add_file(
            "src/lib.rs",
            "
#[test]
fn test_empty_snapshot() {
    // Empty snapshot should remain empty
    insta::assert_snapshot!(\"\", @\"\");
}

#[test]
fn test_whitespace_only_lines() {
    // Lines with only whitespace should be treated as empty for min indent
    insta::assert_snapshot!(\"foo\\n    \\nbar\", @r#\"
    foo
        
    bar
    \"#);
}

#[test]
fn test_all_empty_lines() {
    // All empty lines should result in empty snapshot
    insta::assert_snapshot!(\"\\n\\n\", @r#\"
    
    
    \"#);
}
"
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();
    
    assert!(output.status.success());

    // Check edge case handling
    let updated = std::fs::read_to_string(
        test_project.workspace_dir.join("src/lib.rs")
    ).unwrap();
    
    // Empty snapshot should stay empty
    assert!(updated.contains(r#"insta::assert_snapshot!("", @"");"#));
    
    // Whitespace-only lines should have whitespace removed but lines preserved
    assert!(updated.contains("foo\\n\\nbar"));
}

/// Test that force-update applies more aggressive trimming
#[test]
fn test_force_update_aggressive_trimming() {
    let test_project = TestFiles::new()
        .add_cargo_toml("force_update_test")
        .add_file(
            "src/lib.rs",
            "
#[test]
fn test_force_removes_all_excess() {
    // Force update should minimize to most compact form
    insta::assert_snapshot!(\"foo\", @r#\"

        foo

    \"#);
}

#[test]  
fn test_force_compacts_multiline() {
    // Even multiline content gets maximum trimming
    insta::assert_snapshot!(\"foo\\nbar\", @r#\"
    
        foo
        bar
    
    \"#);
}
"
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

    // Verify aggressive trimming
    assert_snapshot!(test_project.diff("src/lib.rs"), @r###"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -2,11 +2,7 @@
     #[test]
     fn test_force_removes_all_excess() {
         // Force update should minimize to most compact form
    -    insta::assert_snapshot!("foo", @r#"
    -
    -        foo
    -
    -    "#);
    +    insta::assert_snapshot!("foo", @"foo");
     }
     
     #[test]  
    @@ -14,10 +10,8 @@
         // Even multiline content gets maximum trimming
         insta::assert_snapshot!("foo\nbar", @r#"
         
    -        foo
    -        bar
    -    
    -    "#);
    +foo
    +bar
    +"#);
     }
    "###);
}