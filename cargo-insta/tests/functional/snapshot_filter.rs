use insta::assert_snapshot;

use crate::TestFiles;

/// `cargo insta accept --snapshot <name>` should accept only the matching
/// snapshot, and a bare file name (rather than the full workspace-relative
/// path that's shown by `pending-snapshots`) is enough. Regression test for
/// GH-902, which reported that the filter only accepted hard-to-produce
/// absolute paths (`\\?\C:\...` on Windows).
#[test]
fn test_snapshot_filter_partial_name() {
    let test_project = TestFiles::new()
        .add_cargo_toml("snapshot_filter_partial_name")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_one() {
    insta::assert_snapshot!("one", "first value");
}

#[test]
fn test_two() {
    insta::assert_snapshot!("two", "second value");
}
"#
            .to_string(),
        )
        .create_project();

    // The first run leaves two pending snapshots.
    assert!(!&test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,3 +1,7 @@
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/snapshot_filter_partial_name__one.snap.new
    +      src/snapshots/snapshot_filter_partial_name__two.snap.new
    ");

    // Accept just one of them, addressed by a bare file name.
    assert!(&test_project
        .insta_cmd()
        .args([
            "accept",
            "--snapshot",
            "snapshot_filter_partial_name__one.snap",
        ])
        .output()
        .unwrap()
        .status
        .success());

    // `one` is now an accepted snapshot; `two` is still pending.
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,3 +1,7 @@
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/snapshot_filter_partial_name__one.snap
    +      src/snapshots/snapshot_filter_partial_name__two.snap.new
    ");
}

/// `cargo insta pending-snapshots` lists workspace-relative keys with `/`
/// separators that can be fed straight back to `--snapshot`.
#[test]
fn test_pending_snapshots_keys() {
    let test_project = TestFiles::new()
        .add_cargo_toml("pending_snapshots_keys")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snap() {
    insta::assert_snapshot!("snap", "a value");
}
"#
            .to_string(),
        )
        .create_project();

    assert!(!&test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    let output = test_project
        .insta_cmd()
        .args(["pending-snapshots"])
        .stdout(std::process::Stdio::piped())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The key is workspace-relative and uses `/` separators on every platform,
    // so it can be passed straight back to `--snapshot`.
    assert!(
        stdout.contains("src/snapshots/pending_snapshots_keys__snap.snap"),
        "{stdout}"
    );
    assert!(
        !stdout.contains('\\'),
        "keys should use `/` separators: {stdout}"
    );
}
