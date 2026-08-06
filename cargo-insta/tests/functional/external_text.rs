use std::fs;
use std::process::Stdio;

use crate::{TestFiles, TestProject};

const PENDING_PATH: &str = "src/snapshots/external_text.snap.new";
const TARGET_PATH: &str = "generated/assets/output.txt";

fn external_text_project(name: &str, accepted: Option<&str>) -> TestProject {
    let files = TestFiles::new()
        .add_cargo_toml(name)
        .add_file("src/lib.rs", String::new())
        .add_file(
            PENDING_PATH,
            format!(
                "---\nsource: src/lib.rs\nassertion_line: 1\npath: {TARGET_PATH}\n---\nnew content\n"
            ),
        );

    match accepted {
        Some(content) => files.add_file(TARGET_PATH, content.to_string()),
        None => files,
    }
    .create_project()
}

#[test]
fn accept_external_text_proposal() {
    let test_project = external_text_project("accept_external_text_proposal", None);

    let output = test_project.insta_cmd().args(["accept"]).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(test_project.workspace_dir.join(TARGET_PATH)).unwrap(),
        "new content"
    );
    assert!(!test_project.workspace_dir.join(PENDING_PATH).exists());
}

#[test]
fn accept_summary_names_the_external_target() {
    let test_project = external_text_project("accept_summary_external_target", None);

    let output = test_project
        .insta_cmd()
        .args(["accept"])
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(TARGET_PATH), "{stdout}");
    assert!(!stdout.contains("src/lib.rs"), "{stdout}");
}

#[test]
fn an_unreadable_external_target_aborts_before_applying_snapshots() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_unreadable_target")
        .add_file("src/lib.rs", String::new())
        .add_file(
            "src/snapshots/broken.snap.new",
            "---\nsource: src/lib.rs\npath: generated/adir\n---\nnew content\n".to_string(),
        )
        .add_file(
            "src/snapshots/healthy.snap.new",
            "---\nsource: src/lib.rs\n---\nhealthy content\n".to_string(),
        )
        .create_project();
    // A directory where the external target is expected makes the target
    // unreadable without making the pending file itself malformed.
    fs::create_dir_all(test_project.workspace_dir.join("generated/adir")).unwrap();

    let output = test_project
        .insta_cmd()
        .args(["accept"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined_output = format!("{stdout}\n{stderr}");

    // The unreadable target is reported with the pending file that caused it.
    assert!(
        combined_output.contains("broken.snap.new"),
        "{combined_output}"
    );
    assert!(!output.status.success(), "{combined_output}");
    // Loading fails before any proposal is applied.
    assert!(!test_project
        .workspace_dir
        .join("src/snapshots/healthy.snap")
        .exists());
    assert!(test_project
        .workspace_dir
        .join("src/snapshots/healthy.snap.new")
        .exists());
    assert!(test_project
        .workspace_dir
        .join("src/snapshots/broken.snap.new")
        .exists());
}

#[test]
fn reject_external_text_proposal() {
    let test_project =
        external_text_project("reject_external_text_proposal", Some("accepted content"));

    let output = test_project
        .insta_cmd()
        .args(["reject", "--snapshot", TARGET_PATH])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(test_project.workspace_dir.join(TARGET_PATH)).unwrap(),
        "accepted content"
    );
    assert!(!test_project.workspace_dir.join(PENDING_PATH).exists());
}

#[test]
fn review_external_text_proposal_skips_it() {
    let test_project =
        external_text_project("review_external_text_proposal", Some("accepted content"));

    let output = test_project
        .insta_cmd()
        .args(["review", "--snapshot", TARGET_PATH])
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("accepted content"), "{stdout}");
    assert!(stdout.contains("new content"), "{stdout}");
    assert_eq!(
        fs::read_to_string(test_project.workspace_dir.join(TARGET_PATH)).unwrap(),
        "accepted content"
    );
    assert!(test_project.workspace_dir.join(PENDING_PATH).exists());
}
