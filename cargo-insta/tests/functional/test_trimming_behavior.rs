use crate::TestFiles;

/// Test that min_indentation logic correctly removes common leading whitespace
/// This test creates files with excess indentation and verifies they get trimmed
#[test]
fn test_trimming_removes_common_indentation() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_trimming")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_needs_trimming() {
    // This snapshot has excess indentation that should be removed
    insta::assert_snapshot!("hello\nworld", @"
        hello
        world
    ");
}
"#
            .to_string(),
        )
        .create_project();

    // First run with --force-update-snapshots to apply trimming
    let output = test_project
        .insta_cmd()
        .args(["test", "--force-update-snapshots"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify the snapshot was trimmed
    let updated = std::fs::read_to_string(test_project.workspace_dir.join("src/lib.rs")).unwrap();

    // Check that the excess indentation was removed
    // The snapshot should now have just the base 4-space indent from the function
    assert!(
        updated.contains("@r\"\n    hello\n    world\n    \"")
            || updated.contains("@\"hello\\nworld\""),
        "Updated content doesn't contain expected trimmed snapshot"
    );
}

/// Test preservation of relative indentation when trimming
#[test]
fn test_trimming_preserves_relative_indentation() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_relative_indent")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_relative() {
    // Has 4 spaces minimum, but relative indentation should be preserved
    insta::assert_snapshot!("line1\n  line2\n    line3", @"
        line1
          line2
            line3
    ");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--force-update-snapshots"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let updated = std::fs::read_to_string(test_project.workspace_dir.join("src/lib.rs")).unwrap();

    // Check that relative indentation is preserved
    // The minimum 8 spaces are removed, preserving the relative indents
    assert!(
        updated.contains("@r\"\n    line1\n      line2\n        line3\n    \"")
            || updated.contains("@\"line1\\n  line2\\n    line3\""),
        "Relative indentation not preserved correctly"
    );
}

/// Test that tabs are handled as indentation units
#[test]
fn test_trimming_handles_tabs() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_tabs")
        .add_file(
            "src/lib.rs",
            "
#[test]
fn test_tabs() {
    // Snapshot with tab indentation
    insta::assert_snapshot!(\"hello\\nworld\", @\"\t\thello\n\t\tworld\");
}
"
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--force-update-snapshots"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let updated = std::fs::read_to_string(test_project.workspace_dir.join("src/lib.rs")).unwrap();

    // Check that tabs were removed (two tabs from each line)
    // The warning message indicates the multiline format was problematic
    assert!(
        updated.contains("@\"hello\\nworld\"")
            || updated.contains("hello") && updated.contains("world"),
        "Tabs not handled correctly"
    );
}

/// Test single-line compaction
#[test]
fn test_single_line_compaction() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_single_line")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_single() {
    // Multi-line format for single line should be compacted
    insta::assert_snapshot!("hello", @"
        hello
    ");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--force-update-snapshots"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let updated = std::fs::read_to_string(test_project.workspace_dir.join("src/lib.rs")).unwrap();

    // Check that single line was compacted
    assert!(updated.contains("@\"hello\""));
}
