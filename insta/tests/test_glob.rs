#![cfg(all(feature = "glob", feature = "json"))]

mod glob_submodule;

#[test]
fn test_basic_globbing() {
    insta::glob!("inputs/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_json_snapshot!(&contents);
    });
}

#[test]
fn test_basic_globbing_nested() {
    insta::glob!("inputs-nested/*/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(&contents);
    });
}

#[test]
fn test_globs_follow_links() {
    insta::glob!("link-to-inputs/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_json_snapshot!(&contents);
    });
}

#[test]
#[should_panic(expected = "the glob! macro did not match any files.")]
fn test_empty_glob_fails() {
    insta::glob!("nonexistent", |_| {
        // nothing
    });
}

#[test]
fn test_empty_glob_reports_pattern_and_base() {
    let err = std::panic::catch_unwind(|| {
        insta::glob!("nonexistent", |_| {});
    })
    .unwrap_err();
    let msg = match err.downcast::<String>() {
        Ok(msg) => *msg,
        Err(err) => err.downcast_ref::<&str>().unwrap().to_string(),
    };
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .canonicalize()
        .unwrap();
    assert!(msg.contains("\npattern: nonexistent\n"), "{msg}");
    assert!(
        msg.contains(&format!("\nbase directory: {}\n", base.display())),
        "{msg}"
    );
}

#[test]
#[should_panic(expected = "missing-dir (does not exist)")]
fn test_missing_base_dir_fails() {
    insta::glob!("missing-dir", "*.txt", |_| {});
}

#[test]
#[should_panic(expected = "the glob! macro got an invalid pattern 'inputs/['")]
fn test_invalid_glob_pattern_fails() {
    insta::glob!("inputs/[", |_| {});
}

#[cfg(unix)]
#[test]
#[should_panic(expected = "the glob! macro failed while walking")]
fn test_glob_walk_error_fails() {
    let dir = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(dir.path().join("missing"), dir.path().join("broken.txt")).unwrap();
    insta::glob!(dir.path(), "*.txt", |_| {});
}

#[test]
#[should_panic(expected = "Parent directory traversal is not supported in glob patterns")]
fn test_parent_dir_glob_fails_with_helpful_message() {
    insta::glob!("../**/*.rs", |_| {
        // This should fail with a helpful error message about parent directory traversal
    });
}
