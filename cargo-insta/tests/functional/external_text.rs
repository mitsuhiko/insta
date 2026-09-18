use std::fs;
use std::process::{Command, Stdio};

use crate::{target_dir, TestFiles, TestProject};

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

fn cargo_test_command(test_project: &TestProject) -> Command {
    let mut command = Command::new(env!("CARGO"));
    TestProject::clean_env(&mut command);
    command.current_dir(&test_project.workspace_dir);
    command.env("CARGO_TARGET_DIR", target_dir());
    command
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

#[test]
fn direct_update_external_text_snapshot() {
    let test_project = TestFiles::new()
        .add_cargo_toml("direct_update_external_text_snapshot")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("../generated/output.txt", "generated\r\ncontent \t\r\n\r\n");
}
"#
            .to_string(),
        )
        .create_project();

    let output = cargo_test_command(&test_project)
        .env("INSTA_UPDATE", "always")
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(test_project.workspace_dir.join("generated/output.txt")).unwrap(),
        "generated\ncontent"
    );
}

#[test]
fn dynamic_external_text_paths_have_distinct_proposals() {
    let test_project = TestFiles::new()
        .add_cargo_toml("dynamic_external_text_paths")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshots() {
    for directory in ["first", "second"] {
        let path = format!("../generated/{directory}/output.txt");
        insta::assert_file_snapshot!(&path, format!("content for {directory}"));
    }
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(!output.status.success());

    let mut pending_files = fs::read_dir(test_project.workspace_dir.join("src/snapshots"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    pending_files.sort();
    assert_eq!(pending_files.len(), 2, "{pending_files:?}");
    assert!(pending_files.iter().all(|name| {
        name.starts_with("dynamic_external_text_paths__output.txt@") && name.ends_with(".snap.new")
    }));

    let output = test_project.insta_cmd().args(["accept"]).output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(
            test_project
                .workspace_dir
                .join("generated/first/output.txt")
        )
        .unwrap(),
        "content for first"
    );
    assert_eq!(
        fs::read_to_string(
            test_project
                .workspace_dir
                .join("generated/second/output.txt")
        )
        .unwrap(),
        "content for second"
    );
}

#[test]
fn unchanged_external_text_snapshot_passes_full_match() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_full_match")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("../generated/output.txt", "accepted content");
}
"#
            .to_string(),
        )
        .add_file("generated/output.txt", "accepted content".to_string())
        .create_project();

    let output = cargo_test_command(&test_project)
        .env("INSTA_REQUIRE_FULL_MATCH", "1")
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn external_text_snapshot_invokes_full_match_comparator() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_custom_full_match")
        .add_file(
            "src/lib.rs",
            r#"
#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use insta::{Comparator, Snapshot};

    static MATCHES_FULLY_CALLED: AtomicBool = AtomicBool::new(false);

    struct TrackingComparator;

    impl Comparator for TrackingComparator {
        fn matches(&self, _reference: &Snapshot, _test: &Snapshot) -> bool {
            true
        }

        fn matches_fully(&self, reference: &Snapshot, test: &Snapshot) -> bool {
            assert_eq!(reference.metadata(), test.metadata());
            MATCHES_FULLY_CALLED.store(true, Ordering::SeqCst);
            true
        }

        fn dyn_clone(&self) -> Box<dyn Comparator> {
            Box::new(TrackingComparator)
        }
    }

    #[test]
    fn external_text_snapshot() {
        insta::with_settings!({comparator => Box::new(TrackingComparator)}, {
            insta::assert_file_snapshot!("../generated/output.txt", "accepted content");
        });
        assert!(MATCHES_FULLY_CALLED.load(Ordering::SeqCst));
    }
}
"#
            .to_string(),
        )
        .add_file("generated/output.txt", "accepted content".to_string())
        .create_project();

    let output = cargo_test_command(&test_project)
        .env("INSTA_REQUIRE_FULL_MATCH", "1")
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn allow_duplicates_accepts_matching_external_text_snapshots() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_allow_duplicates")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::allow_duplicates! {
        for _ in 0..2 {
            insta::assert_file_snapshot!("../generated/output.txt", "accepted content");
        }
    }
}
"#
            .to_string(),
        )
        .add_file("generated/output.txt", "accepted content".to_string())
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--check", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn allow_duplicates_rejects_diverging_external_text_snapshots() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_allow_duplicates_mismatch")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::allow_duplicates! {
        for i in 0..2 {
            insta::assert_file_snapshot!(
                "../generated/output.txt",
                format!("accepted content {i}")
            );
        }
    }
}
"#
            .to_string(),
        )
        .add_file("generated/output.txt", "accepted content 0".to_string())
        .create_project();

    let output = cargo_test_command(&test_project)
        .env("INSTA_UPDATE", "always")
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("does not match previous snapshot in allow-duplicates block"),
        "{stderr}"
    );
}

#[test]
fn conflicting_external_text_assertions_are_caller_responsibility() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_conflicting_targets")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("../generated/output.txt", "first content");
    insta::assert_file_snapshot!("../generated/output.txt", "second content");
}
"#
            .to_string(),
        )
        .create_project();

    let output = cargo_test_command(&test_project)
        .env("INSTA_UPDATE", "always")
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(test_project.workspace_dir.join("generated/output.txt")).unwrap(),
        "second content"
    );
}

#[test]
fn passing_external_text_snapshot_cleans_proposal_and_is_not_unreferenced() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_cleanup")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("snapshots/golden.txt", "changed content");
}
"#
            .to_string(),
        )
        .add_file("src/snapshots/golden.txt", "accepted content".to_string())
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let pending_path = fs::read_dir(test_project.workspace_dir.join("src/snapshots"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().and_then(|extension| extension.to_str()) == Some("new"))
        .unwrap();

    test_project.update_file(
        "src/lib.rs",
        r#"
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("snapshots/golden.txt", "accepted content");
}
"#
        .to_string(),
    );
    let output = test_project
        .insta_cmd()
        .args(["test", "--unreferenced=reject", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!pending_path.exists());
    assert_eq!(
        fs::read_to_string(test_project.workspace_dir.join("src/snapshots/golden.txt")).unwrap(),
        "accepted content"
    );
}

#[test]
fn moving_the_assertion_cleans_up_the_earlier_proposal() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_moved_assertion")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("snapshots/golden.txt", "changed content");
}
"#
            .to_string(),
        )
        .add_file("src/snapshots/golden.txt", "accepted content".to_string())
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(!output.status.success());

    // Fix the value and shift the assertion onto a different line. The proposal
    // from the first run has to be cleaned up even though the assertion moved.
    test_project.update_file(
        "src/lib.rs",
        r#"
// a comment that shifts the assertion down
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("snapshots/golden.txt", "accepted content");
}
"#
        .to_string(),
    );
    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let pending: Vec<_> = fs::read_dir(test_project.workspace_dir.join("src/snapshots"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".new"))
        .collect();
    assert!(pending.is_empty(), "{pending:?}");

    // Accepting must not resurrect the superseded proposal.
    let output = test_project.insta_cmd().args(["accept"]).output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(test_project.workspace_dir.join("src/snapshots/golden.txt")).unwrap(),
        "accepted content"
    );
}

#[cfg(unix)]
#[test]
fn an_unwritable_external_target_names_the_path() {
    use std::os::unix::fs::PermissionsExt;

    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_unwritable_target")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("../generated/output.txt", "generated content");
}
"#
            .to_string(),
        )
        .create_project();
    let generated = test_project.workspace_dir.join("generated");
    fs::create_dir_all(&generated).unwrap();
    fs::set_permissions(&generated, fs::Permissions::from_mode(0o555)).unwrap();

    let output = cargo_test_command(&test_project)
        .env("INSTA_UPDATE", "always")
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    fs::set_permissions(&generated, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("failed to write external snapshot"),
        "{stderr}"
    );
    assert!(stderr.contains("generated/output.txt"), "{stderr}");
}

#[cfg(unix)]
#[test]
fn direct_update_follows_symlink_and_preserves_permissions() {
    use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};

    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_symlink")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("../generated/link.txt", "updated content");
}
"#
            .to_string(),
        )
        .add_file("generated/target.txt", "accepted content".to_string())
        .create_project();
    let target_path = test_project.workspace_dir.join("generated/target.txt");
    let link_path = test_project.workspace_dir.join("generated/link.txt");
    fs::set_permissions(&target_path, fs::Permissions::from_mode(0o744)).unwrap();
    symlink("target.txt", &link_path).unwrap();

    let output = cargo_test_command(&test_project)
        .env("INSTA_UPDATE", "always")
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fs::symlink_metadata(&link_path)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(fs::read_to_string(&target_path).unwrap(), "updated content");
    assert_eq!(fs::metadata(target_path).unwrap().mode() & 0o777, 0o744);
}

#[test]
fn external_text_proposal_uses_workspace_relative_target() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_workspace_path")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("../generated/output.txt", "proposed\r\ncontent \t\r\n");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!test_project
        .workspace_dir
        .join("generated/output.txt")
        .exists());

    let pending_path = fs::read_dir(test_project.workspace_dir.join("src/snapshots"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let pending = fs::read_to_string(&pending_path).unwrap();
    assert!(
        pending.contains("\npath: generated/output.txt\n"),
        "{pending}"
    );

    let output = test_project.insta_cmd().args(["accept"]).output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(test_project.workspace_dir.join("generated/output.txt")).unwrap(),
        "proposed\ncontent"
    );
    assert!(!pending_path.exists());
}

#[test]
fn external_text_proposal_uses_pending_directory() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_pending_directory")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("../generated/output.txt", "proposed content");
}
"#
            .to_string(),
        )
        .create_project();
    let pending_dir = test_project.workspace_dir.join("pending");

    let output = test_project
        .insta_cmd()
        .env("INSTA_PENDING_DIR", &pending_dir)
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(!output.status.success());

    let pending_path = fs::read_dir(pending_dir.join("src/snapshots"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(pending_path.extension().unwrap(), "new");

    let output = test_project
        .insta_cmd()
        .env("INSTA_PENDING_DIR", &pending_dir)
        .args(["accept"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(test_project.workspace_dir.join("generated/output.txt")).unwrap(),
        "proposed content"
    );
    assert!(!pending_path.exists());
}

#[test]
fn external_text_snapshot_can_target_absolute_path_outside_workspace() {
    let external_dir = tempfile::tempdir().unwrap();
    let target_path = external_dir.path().join("nested/output.txt");
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_absolute_path")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    let path = std::env::var_os("EXTERNAL_SNAPSHOT_PATH").unwrap();
    insta::assert_file_snapshot!(path, "proposed content");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .env("EXTERNAL_SNAPSHOT_PATH", &target_path)
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!target_path.exists());

    let pending_path = fs::read_dir(test_project.workspace_dir.join("src/snapshots"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let pending = fs::read_to_string(&pending_path).unwrap();
    let recorded = pending
        .lines()
        .find_map(|line| line.strip_prefix("path: "))
        .unwrap_or_else(|| panic!("{pending}"))
        .trim_matches('"');

    let output = test_project.insta_cmd().args(["accept"]).output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(&target_path).unwrap(),
        "proposed content"
    );
    assert!(!pending_path.exists());

    // The recorded path is resolved against the workspace root, so it has to
    // name the target no matter whether it came out relative or absolute. A
    // target on another volume cannot be expressed relatively at all.
    assert_eq!(
        fs::canonicalize(test_project.workspace_dir.join(recorded)).unwrap(),
        fs::canonicalize(&target_path).unwrap()
    );
}

#[test]
fn external_text_snapshot_works_in_doctest() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "external_text_doctest"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
/// ```
/// insta::assert_file_snapshot!("../generated/doctest.txt", "doctest content");
/// ```
pub fn documented_function() {}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!test_project
        .workspace_dir
        .join("generated/doctest.txt")
        .exists());

    let pending_path = fs::read_dir(test_project.workspace_dir.join("src/snapshots"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(
        pending_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("doctest_lib_rs__doctest.txt@"),
        "{}",
        pending_path.display()
    );

    let output = test_project.insta_cmd().args(["accept"]).output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(test_project.workspace_dir.join("generated/doctest.txt")).unwrap(),
        "doctest content"
    );

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[cfg(unix)]
#[test]
fn non_utf8_external_text_snapshot_is_rejected() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_non_utf8_content")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    insta::assert_file_snapshot!("../generated/output.bin", "generated content");
}
"#
            .to_string(),
        )
        .create_project();
    let target_path = test_project.workspace_dir.join("generated/output.bin");
    fs::create_dir_all(target_path.parent().unwrap()).unwrap();
    fs::write(&target_path, [0xff]).unwrap();

    let output = cargo_test_command(&test_project)
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("external text snapshot is not valid UTF-8"),
        "{stderr}"
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_external_snapshot_path_fails_in_every_update_mode() {
    let test_project = TestFiles::new()
        .add_cargo_toml("external_text_non_utf8_path")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn external_text_snapshot() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let path = std::path::PathBuf::from("../generated")
        .join(OsString::from_vec(vec![b'o', b'u', b't', 0xff]));
    insta::assert_file_snapshot!(path, "generated content");
}
"#
            .to_string(),
        )
        .create_project();

    for update_mode in ["auto", "always", "new", "unseen", "no", "force"] {
        let output = cargo_test_command(&test_project)
            .env("INSTA_UPDATE", update_mode)
            .args(["test", "--", "--nocapture"])
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "INSTA_UPDATE={update_mode}");
        assert!(
            stderr.contains("external snapshot path is not valid UTF-8"),
            "INSTA_UPDATE={update_mode}\n{stderr}"
        );
    }
}
