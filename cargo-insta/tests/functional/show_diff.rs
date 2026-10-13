use std::process::Stdio;

use crate::TestFiles;

fn lib_rs(value: &str) -> String {
    format!(
        r#"
#[test]
fn test_one() {{
    insta::assert_snapshot!("one", "{value}");
}}
"#
    )
}

/// `cargo insta show --diff` prints the diff between a pending snapshot and
/// the accepted one, whether the path points to the `.snap` or the
/// `.snap.new` file (GH-886).
#[test]
fn test_show_diff() {
    let test_project = TestFiles::new()
        .add_cargo_toml("show_diff")
        .add_file("src/lib.rs", lib_rs("first value"))
        .create_project();

    // Create and accept the snapshot, then change the value so that it's pending.
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());
    std::fs::write(
        test_project.workspace_dir.join("src/lib.rs"),
        lib_rs("second value"),
    )
    .unwrap();
    assert!(!&test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    for path in [
        "src/snapshots/show_diff__one.snap",
        "src/snapshots/show_diff__one.snap.new",
    ] {
        let output = test_project
            .insta_cmd()
            .args(["show", "--diff", path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{path}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(stdout.contains("-old snapshot"), "{path}: {stdout}");
        assert!(stdout.contains("+new results"), "{path}: {stdout}");
        assert!(stdout.contains("first value"), "{path}: {stdout}");
        assert!(stdout.contains("second value"), "{path}: {stdout}");
    }
}

/// Without a pending snapshot there's nothing to diff, so `show --diff` fails.
#[test]
fn test_show_diff_no_pending_snapshot() {
    let test_project = TestFiles::new()
        .add_cargo_toml("show_diff_no_pending")
        .add_file("src/lib.rs", lib_rs("value"))
        .create_project();

    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    let output = test_project
        .insta_cmd()
        .args([
            "show",
            "--diff",
            "src/snapshots/show_diff_no_pending__one.snap",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(!output.status.success());
    // cargo-insta prints errors to stdout.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("no pending snapshot found"), "{stdout}");
}

/// With `INSTA_PENDING_DIR`, the pending snapshot lives in that directory, and
/// `show --diff` given the `.snap` path still finds it.
#[test]
fn test_show_diff_pending_dir() {
    let test_project = TestFiles::new()
        .add_cargo_toml("show_diff_pending_dir")
        .add_file("src/lib.rs", lib_rs("first value"))
        .create_project();
    let pending_dir = test_project.workspace_dir.join("pending_output");
    std::fs::create_dir_all(&pending_dir).unwrap();

    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());
    std::fs::write(
        test_project.workspace_dir.join("src/lib.rs"),
        lib_rs("second value"),
    )
    .unwrap();
    assert!(!&test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .output()
        .unwrap()
        .status
        .success());
    assert!(pending_dir
        .join("src/snapshots/show_diff_pending_dir__one.snap.new")
        .is_file());

    let output = test_project
        .insta_cmd()
        .args([
            "show",
            "--diff",
            "src/snapshots/show_diff_pending_dir__one.snap",
        ])
        .env("INSTA_PENDING_DIR", &pending_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("first value"), "{stdout}");
    assert!(stdout.contains("second value"), "{stdout}");
}
