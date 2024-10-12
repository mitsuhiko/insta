use insta::assert_snapshot;

use crate::TestFiles;

#[test]
fn test_inline_pending_snapshot_deletion() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_pending_snapshot_deletion"
version = "0.1.0"
edition = "2021"

[lib]
doctest = false

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("Hello, world!", @"Hello!");
}
"#
            .to_string(),
        )
        .create_project();

    // Run the test to create a pending snapshot
    assert!(!test_project
        .insta_cmd()
        .args(["test", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    // Verify the file tree after creating the pending snapshot
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,5 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    ");

    // Modify the test to make it pass
    test_project.update_file(
        "src/lib.rs",
        r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("Hello, world!", @"Hello, world!");
}
"#
        .to_string(),
    );

    // Run the test again
    assert!(test_project
        .insta_cmd()
        .args(["test"])
        .output()
        .unwrap()
        .status
        .success());

    // Verify the file tree after running the passing test
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,5 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    ");
}

#[test]
fn test_file_snapshot_pending_deletion() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "test_file_snapshot_pending_deletion"
version = "0.1.0"
edition = "2021"

[lib]
doctest = false

[dependencies]
insta = { path = '$PROJECT_PATH' }
"#
            .to_string(),
        )
        .add_file(
            "src/lib.rs",
            r#"
use insta::assert_snapshot;

#[test]
fn test_file_snapshot() {
    assert_snapshot!("hello world");
}
"#
            .to_string(),
        )
        .create_project();

    // Run the test to create a snapshot
    assert!(test_project
        .insta_cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    // Verify the file tree
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_file_snapshot_pending_deletion__file_snapshot.snap
    ");

    // Modify the value & run the test to make it create a pending snapshot
    test_project.update_file(
        "src/lib.rs",
        r#"
use insta::assert_snapshot;

#[test]
fn test_file_snapshot() {
    assert_snapshot!("hello");
}
"#
        .to_string(),
    );
    assert!(!test_project
        .insta_cmd()
        .args(["test"])
        .output()
        .unwrap()
        .status
        .success());

    // We should have a pending snapshot
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_file_snapshot_pending_deletion__file_snapshot.snap
    +      src/snapshots/test_file_snapshot_pending_deletion__file_snapshot.snap.new
    ");

    // Modify the value back, run the test, which should remove the pending snapshot
    test_project.update_file(
        "src/lib.rs",
        r#"
use insta::assert_snapshot;

#[test]
fn test_file_snapshot() {
    assert_snapshot!("hello world");
}
"#
        .to_string(),
    );
    assert!(test_project
        .insta_cmd()
        .args(["test"])
        .output()
        .unwrap()
        .status
        .success());

    // Now there should be no pending snapshot
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_file_snapshot_pending_deletion__file_snapshot.snap
    ");
}
