use insta::assert_snapshot;

use crate::TestFiles;

/// A pending binary snapshot should have a binary file with the passed extension alongside it.
#[test]
fn test_binary_pending() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_binary_pending")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!(".txt", b"test".to_vec());
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project.insta_cmd().args(["test"]).output().unwrap();

    assert!(!&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_binary_pending__binary_snapshot.snap.new
    +      src/snapshots/test_binary_pending__binary_snapshot.snap.new.txt
    ");
}

/// An accepted binary snapshot should have a binary file with the passed extension alongside it.
#[test]
fn test_binary_accept() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_binary_accept")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!(".txt", b"test".to_vec());
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_binary_accept__binary_snapshot.snap
    +      src/snapshots/test_binary_accept__binary_snapshot.snap.txt
    ");
}

/// Changing the extension passed to the `assert_binary_snapshot` macro should create a new pending
/// snapshot with a binary file with the new extension alongside it and once approved the old binary
/// file with the old extension should be deleted.
#[test]
fn test_binary_change_extension() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_binary_change_extension")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!(".txt", b"test".to_vec());
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    test_project.update_file(
        "src/lib.rs",
        r#"
#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!(".json", b"test".to_vec());
}
"#
        .to_string(),
    );

    let output = test_project.insta_cmd().args(["test"]).output().unwrap();

    assert!(!&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,10 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap.new
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap.new.json
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap.txt
    ");

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap
    +      src/snapshots/test_binary_change_extension__binary_snapshot.snap.json
    ");
}

/// An assert with a pending binary snapshot should have both the metadata file and the binary file
/// deleted when the assert is removed and the tests are re-run.
#[test]
fn test_binary_pending_snapshot_removal() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_binary_pending_snapshot_removal")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_binary_snapshot() {
    insta::assert_binary_snapshot!(".txt", b"test".to_vec());
}
"#
            .to_string(),
        )
        .create_project();

    // create the snapshot
    assert!(&test_project
        .insta_cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .output()
        .unwrap()
        .status
        .success());

    test_project.update_file("src/lib.rs", "".to_string());

    assert!(&test_project
        .insta_cmd()
        .args(["test", "--unreferenced=delete"])
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

/// Replacing a text snapshot with binary one should work and simply replace the text snapshot file
/// with the new metadata file and a new binary snapshot file alongside it.
#[test]
fn test_change_text_to_binary() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_change_text_to_binary")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test() {
    insta::assert_snapshot!("test");
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_change_text_to_binary__test.snap
    ");

    test_project.update_file(
        "src/lib.rs",
        r#"
#[test]
fn test() {
    insta::assert_binary_snapshot!(".txt", b"test".to_vec());
}
"#
        .to_string(),
    );

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_change_text_to_binary__test.snap
    +      src/snapshots/test_change_text_to_binary__test.snap.txt
    ");
}

/// When changing a snapshot from a binary to a text snapshot the previous binary file should be
/// gone after having approved the the binary snapshot.
#[test]
fn test_change_binary_to_text() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_change_binary_to_text")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test() {
    insta::assert_binary_snapshot!("some_name.json", b"{}".to_vec());
}
"#
            .to_string(),
        )
        .create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_change_binary_to_text__some_name.snap
    +      src/snapshots/test_change_binary_to_text__some_name.snap.json
    ");

    test_project.update_file(
        "src/lib.rs",
        r#"
#[test]
fn test() {
    insta::assert_snapshot!("some_name", "test");
}
"#
        .to_string(),
    );

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_change_binary_to_text__some_name.snap
    ");
}

#[test]
fn test_binary_unreferenced_delete() {
    let test_project = TestFiles::new()
        .add_cargo_toml("test_binary_unreferenced_delete")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_snapshot() {
    insta::assert_binary_snapshot!(".txt", b"abcd".to_vec());
}
"#
            .to_string(),
        )
        .create_project();

    // Run tests to create snapshots
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    test_project.update_file("src/lib.rs", "".to_string());

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,8 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/test_binary_unreferenced_delete__snapshot.snap
    +      src/snapshots/test_binary_unreferenced_delete__snapshot.snap.txt
    ");

    // Run cargo insta test with --unreferenced=delete
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--unreferenced=delete",
            "--accept",
            "--",
            "--nocapture",
        ])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // We should now see the unreferenced snapshot deleted
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
