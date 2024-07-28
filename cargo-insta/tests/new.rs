use insta::assert_snapshot;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

// TODO:
// - pull out the common parts — setting up the test
// - how to handle compilation? We want each test to be independent, but we
//   don't want to compile `insta` for each test. Maybe we can compile it once
//   and copy the `target` directory for each test?

#[test]
fn test_json_inline_integration() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Get the absolute path to the current insta crate
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cargo_insta_path = PathBuf::from(manifest_dir).canonicalize().unwrap();
    let insta_path = cargo_insta_path.parent().unwrap().join("insta");

    // Create Cargo.toml
    let cargo_toml = format!(
        r#"
[package]
name = "test_json_inline"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = {{ path = "{}", features=["json", "redactions"] }}
serde = {{ version = "1.0", features = ["derive"] }}
"#,
        insta_path.to_str().unwrap()
    );
    fs::write(project_path.join("Cargo.toml"), cargo_toml).unwrap();

    fs::create_dir(project_path.join("src")).unwrap();

    let main_rs = r#"
use serde::Serialize;

#[derive(Serialize)]
struct User {
    id: u64,
    email: String,
}

#[test]
fn test_json_snapshot() {
    let user = User {
        id: 42,
        email: "john.doe@example.com".into(),
    };
    insta::assert_json_snapshot!(&user, {
        ".id" => "[user_id]",
    }, @"");
}

#[test]
fn test_json_snapshot_trailing_comma() {
    let user = User {
        id: 42,
        email: "john.doe@example.com".into(),
    };
    insta::assert_compact_json_snapshot!(
        &user,
        @"",
    );
}

#[test]
fn test_json_snapshot_trailing_comma_redaction() {
    let user = User {
        id: 42,
        email: "john.doe@example.com".into(),
    };
    insta::assert_json_snapshot!(
        &user,
        {
            ".id" => "[user_id]",
        },
        @"",
    );
}
"#;

    let dest = project_path.join("src").join("main.rs");
    fs::write(dest.clone(), main_rs).unwrap();

    // Run cargo-insta test
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-insta"))
        .current_dir(project_path)
        .arg("test")
        .arg("--accept")
        .output()
        .expect("Failed to execute command");

    // Check if the tests passed
    assert!(
        output.status.success(),
        "Tests failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(dest).unwrap();
    assert_snapshot!(contents, @r#####"

    use serde::Serialize;

    #[derive(Serialize)]
    struct User {
        id: u64,
        email: String,
    }

    #[test]
    fn test_json_snapshot() {
        let user = User {
            id: 42,
            email: "john.doe@example.com".into(),
        };
        insta::assert_json_snapshot!(&user, {
            ".id" => "[user_id]",
        }, @r###"
        {
          "id": "[user_id]",
          "email": "john.doe@example.com"
        }
        "###);
    }

    #[test]
    fn test_json_snapshot_trailing_comma() {
        let user = User {
            id: 42,
            email: "john.doe@example.com".into(),
        };
        insta::assert_compact_json_snapshot!(
            &user,
            @r###"{"id": 42, "email": "john.doe@example.com"}"###,
        );
    }

    #[test]
    fn test_json_snapshot_trailing_comma_redaction() {
        let user = User {
            id: 42,
            email: "john.doe@example.com".into(),
        };
        insta::assert_json_snapshot!(
            &user,
            {
                ".id" => "[user_id]",
            },
            @r###"
        {
          "id": "[user_id]",
          "email": "john.doe@example.com"
        }
        "###,
        );
    }
    "#####);
}

// // Verify snapshots were created
// let snapshot_dir = project_path
//     .parent()
//     .unwrap()
//     .join("src")
//     .join("snapshots")
//     .to_owned()
//     .as_path().;
// assert!(
//     snapshot_dir.exists(),
//     "Snapshot directory was not created. Project structure:\n\n{}",
//     list_directory_contents(&project_path.to_path_buf())
// );

// // Additional checks for snapshot files
// let snapshot_files = fs::read_dir(&snapshot_dir)
//     .expect("Failed to read snapshot directory")
//     .filter_map(|entry| entry.ok())
//     .map(|entry| entry.file_name().to_string_lossy().into_owned())
//     .collect::<Vec<_>>();

// assert!(
//     !snapshot_files.is_empty(),
//     "No snapshot files were created in the snapshot directory. Directory contents:\n\n{}",
//     list_directory_contents(&snapshot_dir)
// );
// }
// use ignore::WalkBuilder;
// fn list_directory_contents(path: &PathBuf) -> String {
//     // TODO: could use `find_snapshots` but would need to reorganize the crate a
//     // bit given visibility
//     WalkBuilder::new(path)
//         // .into_iter()
//         // .filter_entry(|e| e.ok())
//         .filter_entry(|e| e.path().file_name() != Some(std::ffi::OsStr::new("target")))
//         .build()
//         .filter_map(|e| e.ok())
//         .map(|entry| {
//             let path = entry.path().strip_prefix(path).unwrap_or(entry.path());
//             format!("{}{}", "  ".repeat(entry.depth()), path.display())
//         })
//         .collect::<Vec<_>>()
//         .join("\n")
// }
