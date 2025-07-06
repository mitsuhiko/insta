use super::*;

/// Test that INSTA_GLOB_FILTER preserves snapshot file names
///
/// When using INSTA_GLOB_FILTER to filter glob matches to a single file,
/// the snapshot name should maintain its file suffix (e.g., "test_name@file.txt.snap")
/// by using the common prefix from all matches, not just filtered ones.
#[test]
fn test_glob_filter_preserves_snapshot_names() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_glob_filter"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH', features = ["glob"] }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_glob_snapshots() {
    insta::glob!("data/*.txt", |path| {
        let content = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(content);
    });
}
"#
            .to_string(),
        )
        .add_file("src/data/apple.txt", "apple".to_string())
        .add_file("src/data/banana.txt", "banana".to_string())
        .add_file("src/data/cherry.txt", "cherry".to_string())
        .create_project();

    // Run without filter to create initial snapshots
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Check snapshot names
    let snapshot_dir = test_project.workspace_dir.join("src/snapshots");
    let get_snapshot_list = || -> String {
        let mut names: Vec<_> = std::fs::read_dir(&snapshot_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| name.ends_with(".snap"))
            .collect();
        names.sort();
        names.join("\n")
    };

    assert_snapshot!(get_snapshot_list(), @r###"
    test_glob_filter__glob_snapshots@apple.txt.snap
    test_glob_filter__glob_snapshots@banana.txt.snap
    test_glob_filter__glob_snapshots@cherry.txt.snap
    "###);

    // Clean and run with INSTA_GLOB_FILTER to filter to single file
    std::fs::remove_dir_all(&snapshot_dir).unwrap();
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .env("INSTA_GLOB_FILTER", "**/apple.txt")
        .output()
        .unwrap();
    assert!(output.status.success());

    // When filtered to one file, the snapshot name should keep the file suffix
    assert_snapshot!(get_snapshot_list(), @"test_glob_filter__glob_snapshots@apple.txt.snap");

    // Verify the fix works with any pattern that matches one file
    std::fs::remove_dir_all(&snapshot_dir).unwrap();
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .env("INSTA_GLOB_FILTER", "*pple*")
        .output()
        .unwrap();
    assert!(output.status.success());

    assert_snapshot!(get_snapshot_list(), @"test_glob_filter__glob_snapshots@apple.txt.snap");
}
