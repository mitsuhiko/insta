use insta::assert_snapshot;

use crate::TestFiles;

/// Test that new snapshots get the closing --- marker
#[test]
fn test_new_snapshots_get_closing_marker() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_closing_marker")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("Hello, world!");
}
"#
            .to_string(),
        )
        .create_project();

    // Run test with --accept to create the new snapshot
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap()
        .status
        .success());

    // Read the created snapshot file
    let snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/test_closing_marker__snapshot.snap");
    let snapshot_content = std::fs::read_to_string(&snapshot_path).unwrap();

    // Verify it has the closing --- marker
    assert!(
        snapshot_content.ends_with("---\n"),
        "Snapshot should end with '---\\n', but got: {:?}",
        snapshot_content
    );

    // Also verify the full structure
    assert_snapshot!(snapshot_content, @r#"
    ---
    source: src/lib.rs
    expression: "\"Hello, world!\""
    ---
    Hello, world!
    ---
    "#);
}

/// Test that we can still read old snapshots without the closing marker
#[test]
fn test_reading_snapshots_without_closing_marker() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_no_closing_marker")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("snapshot", "Hello, world!");
}
"#
            .to_string(),
        )
        // Create an old-style snapshot without the closing ---
        .add_file(
            "src/snapshots/test_no_closing_marker__snapshot.snap",
            r#"---
source: src/lib.rs
expression: "\"Hello, world!\""
---
Hello, world!
"#
            .to_string(),
        )
        .create_project();

    // Run test - it should pass even without the closing marker
    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should pass with old-style snapshot. Output: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that force-update adds the closing marker to old snapshots
#[test]
fn test_force_update_adds_closing_marker() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_force_update_marker")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("force_update", "Hello, world!");
}
"#
            .to_string(),
        )
        // Create an old-style snapshot without closing marker
        .add_file(
            "src/snapshots/test_force_update_marker__force_update.snap",
            r#"
---
source: src/lib.rs
expression: 
---
Hello, world!


"#
            .to_string(),
        )
        .create_project();

    // Run with --force-update-snapshots
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept", "--force-update-snapshots"])
        .output()
        .unwrap()
        .status
        .success());

    // Check the updated snapshot
    assert_snapshot!(test_project.diff("src/snapshots/test_force_update_marker__force_update.snap"), @r#"
    --- Original: src/snapshots/test_force_update_marker__force_update.snap
    +++ Updated: src/snapshots/test_force_update_marker__force_update.snap
    @@ -1,8 +1,6 @@
    -
     ---
     source: src/lib.rs
    -expression: 
    +expression: "\"Hello, world!\""
     ---
     Hello, world!
    -
    -
    +---
    "#);
}

/// Test that updating a snapshot preserves the closing marker
#[test]
fn test_update_preserves_closing_marker() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_preserve_marker")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("updated", "New content!");
}
"#
            .to_string(),
        )
        // Create a modern snapshot with closing marker
        .add_file(
            "src/snapshots/test_preserve_marker__updated.snap",
            r#"---
source: src/lib.rs
expression: "\"Old content!\""
---
Old content!
---
"#
            .to_string(),
        )
        .create_project();

    // Run test with --accept to update the snapshot
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap()
        .status
        .success());

    // Check the updated snapshot still has the closing marker
    let updated_content = std::fs::read_to_string(
        test_project
            .workspace_dir
            .join("src/snapshots/test_preserve_marker__updated.snap"),
    )
    .unwrap();

    assert!(
        updated_content.ends_with("---\n"),
        "Updated snapshot should still end with '---\\n', but got: {:?}",
        updated_content
    );

    assert_snapshot!(updated_content, @r#"
    ---
    source: src/lib.rs
    expression: "\"New content!\""
    ---
    New content!
    ---
    "#);
}

/// Test that multi-line content with trailing newlines works correctly
#[test]
fn test_multiline_with_trailing_newlines() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_multiline_trailing")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("Line 1\nLine 2\nLine 3\n");
}
"#
            .to_string(),
        )
        .create_project();

    // Run test with --accept
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap()
        .status
        .success());

    // Check the snapshot
    let snapshot_content = std::fs::read_to_string(
        test_project
            .workspace_dir
            .join("src/snapshots/test_multiline_trailing__snapshot.snap"),
    )
    .unwrap();

    // The trailing newline from the content is stripped, then --- is added
    assert_snapshot!(snapshot_content, @r#"
    ---
    source: src/lib.rs
    expression: "\"Line 1\\nLine 2\\nLine 3\\n\""
    ---
    Line 1
    Line 2
    Line 3
    ---
    "#);
}

/// Test that empty snapshots work correctly with the closing marker
#[test]
fn test_empty_snapshot_with_marker() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_empty_marker")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("");
}
"#
            .to_string(),
        )
        .create_project();

    // Run test with --accept
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap()
        .status
        .success());

    // Check the snapshot
    let snapshot_content = std::fs::read_to_string(
        test_project
            .workspace_dir
            .join("src/snapshots/test_empty_marker__snapshot.snap"),
    )
    .unwrap();

    assert_snapshot!(snapshot_content, @r#"
    ---
    source: src/lib.rs
    expression: "\"\""
    ---

    ---
    "#);
}

/// Test that inline snapshots are not affected by the closing marker
#[test]
fn test_inline_snapshots_unaffected() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_inline_unaffected")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_inline() {
    insta::assert_snapshot!("Hello", @"");
}
"#
            .to_string(),
        )
        .create_project();

    // Run test with --accept
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap()
        .status
        .success());

    // Check that inline snapshot was updated correctly without any marker
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -1,5 +1,5 @@
     
     #[test]
     fn test_inline() {
    -    insta::assert_snapshot!("Hello", @"");
    +    insta::assert_snapshot!("Hello", @"Hello");
     }
    "#);
}

/// Test old snapshots with content ending in --- are preserved  
#[test]
fn test_old_snapshot_ending_with_dashes() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_old_with_dashes")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("snapshot", "Title\n---");
}
"#
            .to_string(),
        )
        // Create an OLD format snapshot (no closing marker) where content ends with ---
        .add_file(
            "src/snapshots/test_old_with_dashes__snapshot.snap",
            "---\nsource: src/lib.rs\nexpression: \"\\\"Title\\\\n---\\\"\"\n---\nTitle\n---"
                .to_string(),
        )
        .create_project();

    // Run test - it should pass with the old snapshot
    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should pass with old snapshot ending in ---. Output: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that content naturally ending with --- is preserved correctly
#[test]
fn test_content_naturally_ending_with_dashes() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_content_with_dashes")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_markdown_separator() {
    // This content naturally ends with --- (like a markdown horizontal rule)
    insta::assert_snapshot!("markdown", "Title\n---");
}

#[test]
fn test_yaml_separator() {
    // YAML documents often have --- as separators
    insta::assert_snapshot!("yaml", "---\nkey: value\n---");
}
"#
            .to_string(),
        )
        .create_project();

    // Create initial snapshots
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap()
        .status
        .success());

    // Read the created snapshots to verify content is preserved
    let markdown_snapshot = std::fs::read_to_string(
        test_project
            .workspace_dir
            .join("src/snapshots/test_content_with_dashes__markdown.snap"),
    )
    .unwrap();

    let yaml_snapshot = std::fs::read_to_string(
        test_project
            .workspace_dir
            .join("src/snapshots/test_content_with_dashes__yaml.snap"),
    )
    .unwrap();

    // Verify the content still has --- where it should
    assert!(
        markdown_snapshot.contains("Title\n---\n---"),
        "Markdown separator should be preserved. Got: {:?}",
        markdown_snapshot
    );

    assert!(
        yaml_snapshot.contains("---\nkey: value\n---\n---"),
        "YAML separators should be preserved. Got: {:?}",
        yaml_snapshot
    );

    // Now run tests again to ensure they still pass (roundtrip test)
    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    // The test should pass, proving content is correctly preserved
    assert!(
        output.status.success(),
        "Tests should pass on second run. Output: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Also run with --check to ensure no changes are detected
    let check_output = test_project
        .insta_cmd()
        .args(["test", "--check"])
        .output()
        .unwrap();

    assert!(
        check_output.status.success(),
        "No changes should be detected. Output: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );
}

/// Test edge case where old snapshots have YAML-style content with multiple ---
/// This verifies that matches_legacy handles complex content correctly
#[test]
fn test_edge_case_yaml_style_content() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_edge_yaml")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("test", "---\nkey: value\n---");
}
"#
            .to_string(),
        )
        // Old snapshot with YAML-like content
        .add_file(
            "src/snapshots/test_edge_yaml__test.snap",
            "---\nsource: src/lib.rs\nexpression: \"\\\"---\\\\nkey: value\\\\n---\\\"\"\n---\n---\nkey: value\n---\n"
                .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Test should pass. Output: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_edge_case_should_not_match() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_edge_mismatch")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("test", "Different content");
}
"#
            .to_string(),
        )
        // Snapshot with completely different content
        .add_file(
            "src/snapshots/test_edge_mismatch__test.snap",
            "---\nsource: src/lib.rs\nexpression: \n---\nOriginal content\n---\n".to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "Test should FAIL because content doesn't match. Output: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that content exactly "---" works correctly
#[test]
fn test_content_exactly_three_dashes() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_exact_dashes")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("exactly_dashes", "---");
}
"#
            .to_string(),
        )
        .create_project();

    // Run test with --accept to create snapshot
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap()
        .status
        .success());

    // Read the created snapshot
    let snapshot_content = std::fs::read_to_string(
        test_project
            .workspace_dir
            .join("src/snapshots/test_exact_dashes__exactly_dashes.snap"),
    )
    .unwrap();

    // Verify it has both the content "---" and the closing marker
    assert_snapshot!(snapshot_content, @r#"
    ---
    source: src/lib.rs
    expression: "\"---\""
    ---
    ---
    ---
    "#);
}

/// Test that debug snapshots get the closing marker
#[test] 
fn test_debug_snapshot_gets_marker() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_debug_marker")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_debug() {
    let data = vec![1, 2, 3];
    insta::assert_debug_snapshot!(data);
}
"#
            .to_string(),
        )
        .create_project();

    // Run tests with --accept
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap()
        .status
        .success());

    // Check debug snapshot has closing marker
    let debug_content = std::fs::read_to_string(
        test_project
            .workspace_dir
            .join("src/snapshots/test_debug_marker__debug.snap"),
    )
    .unwrap();
    assert!(
        debug_content.ends_with("---\n"),
        "Debug snapshot should end with closing marker"
    );
}

/// Test binary snapshots don't get the text closing marker
#[test]
fn test_binary_snapshots_no_text_marker() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_binary_no_marker")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_binary() {
    insta::assert_binary_snapshot!("test.png", vec![0x89, 0x50, 0x4E, 0x47]);
}
"#
            .to_string(),
        )
        .create_project();

    // Run test with --accept
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap()
        .status
        .success());

    // Check the metadata file (not the binary file)
    let snapshot_path = test_project
        .workspace_dir
        .join("src/snapshots/test_binary_no_marker__test.snap");
    let snapshot_content = std::fs::read_to_string(&snapshot_path)
        .unwrap_or_else(|e| panic!("Failed to read snapshot at {:?}: {}", snapshot_path, e));

    // Binary snapshots should only have metadata section, no content section
    // The file should have exactly two --- markers: opening and closing of metadata
    let parts: Vec<&str> = snapshot_content.split("---\n").collect();
    assert_eq!(
        parts.len(),
        3,
        "Binary snapshot should have exactly 2 --- markers (opening and closing metadata)"
    );
    assert_eq!(parts[0], "", "Should start with ---");
    assert!(parts[1].contains("extension:"), "Should have metadata");
    assert_eq!(parts[2], "", "Should end with --- and nothing after");
}
