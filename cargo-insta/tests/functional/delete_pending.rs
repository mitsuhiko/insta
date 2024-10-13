use insta::assert_snapshot;

use crate::TestFiles;

/// `--unreferenced=delete` should delete pending snapshots
#[test]
fn delete_unreferenced() {
    let test_project = TestFiles::new()
        .add_cargo_toml("delete_unreferenced")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_snapshot!("Hello, inline!", @"Hello!");
}

#[test]
fn test_snapshot_file() {
    insta::assert_snapshot!("Hello, world!");
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

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
    +    src/.lib.rs.pending-snap
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/delete_unreferenced__snapshot_file.snap.new
    ");

    // Now remove the tests; the pending snapshots should be deleted when
    // passing `--unreferenced=delete`
    test_project.update_file("src/lib.rs", "".to_string());

    assert!(&test_project
        .insta_cmd()
        .args(["test", "--unreferenced=delete", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,6 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    ");
}

#[test]
fn test_pending_snapshot_deletion() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_combined_snapshot_deletion")
        .add_file(
            "src/lib.rs",
            r#"
use insta::assert_snapshot;

#[test]
fn test_inline_snapshot() {
    insta::assert_snapshot!("Hello, world!", @"Hello!");
}

#[test]
fn test_file_snapshot() {
    assert_snapshot!("hello world");
}
"#
            .to_string(),
        )
        .create_project();

    // Run the test with `--accept` to create correct snapshots
    assert!(test_project
        .insta_cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    // Verify the file tree has a `.snap` file
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_combined_snapshot_deletion__file_snapshot.snap
    ");

    // Modify the tests to make them fail
    test_project.update_file(
        "src/lib.rs",
        r#"
use insta::assert_snapshot;

#[test]
fn test_inline_snapshot() {
    insta::assert_snapshot!("Hello WORLD!", @"Hello, world!");
}

#[test]
fn test_file_snapshot() {
    assert_snapshot!("hello WORLD");
}
"#
        .to_string(),
    );

    // Run the tests to create pending snapshots
    assert!(!test_project
        .insta_cmd()
        .args(["test"])
        .output()
        .unwrap()
        .status
        .success());

    // Verify pending snapshots exist
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,9 @@
     
    +  Cargo.lock
       Cargo.toml
       src
    +    src/.lib.rs.pending-snap
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_combined_snapshot_deletion__file_snapshot.snap
    +      src/snapshots/test_combined_snapshot_deletion__file_snapshot.snap.new
    ");

    // Run `cargo insta reject` to delete pending snapshots
    assert!(test_project
        .insta_cmd()
        .args(["reject"])
        .output()
        .unwrap()
        .status
        .success());

    // Pending snapshots should be deleted
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_combined_snapshot_deletion__file_snapshot.snap
    ");

    // Run the tests again to create pending snapshots
    assert!(!test_project
        .insta_cmd()
        .args(["test"])
        .output()
        .unwrap()
        .status
        .success());

    // They should be back...
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,9 @@
     
    +  Cargo.lock
       Cargo.toml
       src
    +    src/.lib.rs.pending-snap
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_combined_snapshot_deletion__file_snapshot.snap
    +      src/snapshots/test_combined_snapshot_deletion__file_snapshot.snap.new
    ");

    // Modify the test back so they pass
    test_project.update_file(
        "src/lib.rs",
        r#"
use insta::assert_snapshot;

#[test]
fn test_inline_snapshot() {
    insta::assert_snapshot!("Hello, world!", @"Hello, world!");
}

#[test]
fn test_file_snapshot() {
    assert_snapshot!("hello world");
}
"#
        .to_string(),
    );

    // Run the tests with `--unreferenced=delete` to delete pending snapshots
    assert!(test_project
        .insta_cmd()
        .args(["test", "--unreferenced=delete"])
        .output()
        .unwrap()
        .status
        .success());

    // Verify the pending snapshots are deleted
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_combined_snapshot_deletion__file_snapshot.snap
    ");
}
