use super::*;

/// A failing `glob!` says which pattern it searched for and where.
#[test]
fn test_glob_error_messages() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_glob_errors"
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
fn test_invalid_pattern() {
    insta::glob!("data/[", |_| {});
}

#[test]
fn test_missing_base_dir() {
    insta::glob!("missing", "*.txt", |_| {});
}

#[test]
fn test_no_match() {
    insta::glob!("src/data/*.txt", |_| {});
}
"#
            .to_string(),
        )
        .add_file("src/data/apple.txt", "apple".to_string())
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--", "--test-threads=1"])
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    assert!(!output.status.success());

    // Keep the panic messages from the failures section, without the panic
    // locations, and replace the temporary project path.
    let project = test_project
        .workspace_dir
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let messages = stdout
        .lines()
        .skip_while(|line| *line != "failures:")
        .skip(1)
        .take_while(|line| *line != "failures:")
        .filter(|line| {
            !line.is_empty() && !line.starts_with("thread '") && !line.starts_with("note: ")
        })
        .map(|line| match line.find(project) {
            Some(idx) if line.starts_with("base directory: ") => format!(
                "base directory: [PROJECT]{}",
                line[idx + project.len()..].replace('\\', "/")
            ),
            _ => line.to_string(),
        })
        .join("\n");

    assert_snapshot!(messages, @"
    ---- test_invalid_pattern stdout ----
    the glob! macro got an invalid pattern 'data/[': unclosed character class; missing ']'
    ---- test_missing_base_dir stdout ----
    the glob! macro did not match any files.
    pattern: *.txt
    base directory: [PROJECT]/src/missing (does not exist)
    The pattern is relative to the base directory, which defaults to the directory of the file calling glob!.
    ---- test_no_match stdout ----
    the glob! macro did not match any files.
    pattern: src/data/*.txt
    base directory: [PROJECT]/src
    The pattern is relative to the base directory, which defaults to the directory of the file calling glob!.
    ");
}
