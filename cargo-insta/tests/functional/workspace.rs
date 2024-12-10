use std::{fs, process::Stdio};

use insta::assert_snapshot;

use crate::{TestFiles, TestProject};

// Note that names need to be different to prevent the cache confusing them.
fn workspace_with_root_crate(name: String) -> TestFiles {
    TestFiles::new()
        .add_file(
            "Cargo.toml",
            format!(
                r#"
[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[workspace]
members = [
    "member",
]

[workspace.dependencies]
insta = {{path = '$PROJECT_PATH'}}

[dependencies]
insta = {{ workspace = true }}

"#
            )
            .to_string(),
        )
        .add_file(
            "member/Cargo.toml",
            format!(
                r#"
[package]
name = "{name}-member"
version = "0.0.0"
edition = "2021"

[dependencies]
insta = {{ workspace = true }}
"#
            )
            .to_string(),
        )
        .add_file(
            "member/src/lib.rs",
            r#"
#[test]
fn test_member() {
    insta::assert_debug_snapshot!(vec![1, 2, 3]);
}
"#
            .to_string(),
        )
        .add_file(
            "src/main.rs",
            r#"
fn main() {
    println!("Hello, world!");
}

#[test]
fn test_root() {
    insta::assert_debug_snapshot!(vec![1, 2, 3]);
}
"#
            .to_string(),
        )
}

/// Check that in a workspace with a default root crate, running `cargo insta
/// test --workspace --accept` will update snapshots in both the root crate and the
/// member crate.
#[test]
fn test_root_crate_workspace_accept() {
    let test_project =
        workspace_with_root_crate("root-crate-workspace-accept".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept", "--workspace"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r###"
    --- Original file tree
    +++ Updated file tree
    @@ -1,8 +1,13 @@
     
    +  Cargo.lock
       Cargo.toml
       member
         member/Cargo.toml
         member/src
           member/src/lib.rs
    +      member/src/snapshots
    +        member/src/snapshots/root_crate_workspace_accept_member__member.snap
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/root_crate_workspace_accept__root.snap
    "###     );
}

/// Check that in a workspace with a default root crate, running `cargo insta
/// test --workspace` will correctly report the number of pending snapshots
#[test]
fn test_root_crate_workspace() {
    let test_project =
        workspace_with_root_crate("root-crate-workspace".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        // Need to disable colors to assert the output below
        .args(["test", "--workspace", "--color=never"])
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // 1.39 had a bug where it would claim there were 3 snapshots here
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("info: 2 snapshots to review"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Check that in a workspace with a default root crate, running `cargo insta
/// test --accept` will only update snapshots in the root crate
#[test]
fn test_root_crate_no_all() {
    let test_project = workspace_with_root_crate("root-crate-no-all".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r###"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,5 @@
     
    +  Cargo.lock
       Cargo.toml
       member
         member/Cargo.toml
    @@ -6,3 +7,5 @@
           member/src/lib.rs
       src
         src/main.rs
    +    src/snapshots
    +      src/snapshots/root_crate_no_all__root.snap
    "###     );
}

fn workspace_with_virtual_manifest(name: String) -> TestFiles {
    TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[workspace]
members = [
    "member-1",
    "member-2",
]

[workspace.dependencies]
insta = {path = '$PROJECT_PATH'}
"#
            .to_string()
            .to_string(),
        )
        .add_file(
            "member-1/Cargo.toml",
            format!(
                r#"
[package]
name = "{name}-member-1"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = {{ workspace = true }}
"#
            )
            .to_string(),
        )
        .add_file(
            "member-1/src/lib.rs",
            r#"
#[test]
fn test_member_1() {
    insta::assert_debug_snapshot!(vec![1, 2, 3]);
}
"#
            .to_string(),
        )
        .add_file(
            "member-2/Cargo.toml",
            format!(
                r#"
[package]
name = "{name}-member-2"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = {{ workspace = true }}
"#
            )
            .to_string(),
        )
        .add_file(
            "member-2/src/lib.rs",
            r#"
#[test]
fn test_member_2() {
    insta::assert_debug_snapshot!(vec![4, 5, 6]);
}
"#
            .to_string(),
        )
}

/// Check that in a workspace with a virtual manifest, running `cargo insta test
/// --workspace --accept` updates snapshots in all member crates.
#[test]
fn test_virtual_manifest_all() {
    let test_project =
        workspace_with_virtual_manifest("virtual-manifest-all".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept", "--workspace"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r###"
    --- Original file tree
    +++ Updated file tree
    @@ -1,10 +1,15 @@
     
    +  Cargo.lock
       Cargo.toml
       member-1
         member-1/Cargo.toml
         member-1/src
           member-1/src/lib.rs
    +      member-1/src/snapshots
    +        member-1/src/snapshots/virtual_manifest_all_member_1__member_1.snap
       member-2
         member-2/Cargo.toml
         member-2/src
           member-2/src/lib.rs
    +      member-2/src/snapshots
    +        member-2/src/snapshots/virtual_manifest_all_member_2__member_2.snap
    "###     );
}

/// Check that in a workspace with a virtual manifest, running `cargo insta test
/// --accept` updates snapshots in all member crates.
#[test]
fn test_virtual_manifest_default() {
    let test_project =
        workspace_with_virtual_manifest("virtual-manifest-default".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r###"
    --- Original file tree
    +++ Updated file tree
    @@ -1,10 +1,15 @@
     
    +  Cargo.lock
       Cargo.toml
       member-1
         member-1/Cargo.toml
         member-1/src
           member-1/src/lib.rs
    +      member-1/src/snapshots
    +        member-1/src/snapshots/virtual_manifest_default_member_1__member_1.snap
       member-2
         member-2/Cargo.toml
         member-2/src
           member-2/src/lib.rs
    +      member-2/src/snapshots
    +        member-2/src/snapshots/virtual_manifest_default_member_2__member_2.snap
    "###     );
}

/// Check that in a workspace with a virtual manifest, running `cargo insta test
/// -p <crate>` will only update snapshots in that crate.
#[test]
fn test_virtual_manifest_single_crate() {
    let test_project =
        workspace_with_virtual_manifest("virtual-manifest-single".to_string()).create_project();

    let output = test_project
        .insta_cmd()
        .args(["test", "--accept", "-p", "virtual-manifest-single-member-1"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    assert_snapshot!(test_project.file_tree_diff(), @r###"
    --- Original file tree
    +++ Updated file tree
    @@ -1,9 +1,12 @@
     
    +  Cargo.lock
       Cargo.toml
       member-1
         member-1/Cargo.toml
         member-1/src
           member-1/src/lib.rs
    +      member-1/src/snapshots
    +        member-1/src/snapshots/virtual_manifest_single_member_1__member_1.snap
       member-2
         member-2/Cargo.toml
         member-2/src
    "###     );
}

// Can't get the test binary discovery to work on Windows, don't have a windows
// machine to hand, others are welcome to fix it. (No specific reason to think
// that insta doesn't work on windows, just that the test doesn't work.)
#[cfg(not(target_os = "windows"))]
#[test]
fn test_insta_workspace_root() {
    use std::{
        fs::{self, remove_dir_all},
        path::{Path, PathBuf},
        process::Command,
    };

    use crate::TestProject;

    // This function locates the compiled test binary in the target directory.
    // It's necessary because the exact filename of the test binary includes a hash
    // that we can't predict, so we need to search for it.
    fn find_test_binary(dir: &Path) -> PathBuf {
        dir.join("target/debug/deps")
            .read_dir()
            .unwrap()
            .filter_map(Result::ok)
            .find(|entry| {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_str().unwrap_or("");
                // We're looking for a file that:
                file_name_str.starts_with("insta_workspace_root_test-") // Matches our test name
                    && !file_name_str.contains('.') // Doesn't have an extension (it's the executable, not a metadata file)
                    && entry.metadata().map(|m| m.is_file()).unwrap_or(false) // Is a file, not a directory
            })
            .map(|entry| entry.path())
            .expect("Failed to find test binary")
    }

    fn run_test_binary(
        binary_path: &Path,
        current_dir: &Path,
        env: Option<(&str, &str)>,
    ) -> std::process::Output {
        let mut cmd = Command::new(binary_path);
        TestProject::clean_env(&mut cmd);
        cmd.current_dir(current_dir);
        if let Some((key, value)) = env {
            cmd.env(key, value);
        }
        cmd.output().unwrap()
    }

    let test_project = TestFiles::new()
        .add_cargo_toml("insta_workspace_root_test")
        .add_file(
            "src/lib.rs",
            r#"
use insta::assert_snapshot;

#[test]
fn test_snapshot() {
    assert_snapshot!("Hello, world!");
}
    "#
            .to_string(),
        )
        .create_project();

    let mut cargo_cmd = Command::new("cargo");
    TestProject::clean_env(&mut cargo_cmd);
    let output = cargo_cmd
        .args(["test", "--no-run"])
        .current_dir(&test_project.workspace_dir)
        .output()
        .unwrap();
    assert!(&output.status.success());

    let test_binary_path = find_test_binary(&test_project.workspace_dir);

    // Run the test without snapshot (should fail)
    assert!(
        !&run_test_binary(&test_binary_path, &test_project.workspace_dir, None,)
            .status
            .success()
    );

    // Create the snapshot
    assert!(&run_test_binary(
        &test_binary_path,
        &test_project.workspace_dir,
        Some(("INSTA_UPDATE", "always")),
    )
    .status
    .success());

    // Verify snapshot creation
    assert!(test_project.workspace_dir.join("src/snapshots").exists());
    assert!(test_project
        .workspace_dir
        .join("src/snapshots/insta_workspace_root_test__snapshot.snap")
        .exists());

    // Move the workspace
    let moved_workspace = {
        let moved_workspace = PathBuf::from("/tmp/cargo-insta-test-moved");
        remove_dir_all(&moved_workspace).ok();
        fs::create_dir(&moved_workspace).unwrap();
        fs::rename(&test_project.workspace_dir, &moved_workspace).unwrap();
        moved_workspace
    };
    let moved_binary_path = find_test_binary(&moved_workspace);

    // Run test in moved workspace without INSTA_WORKSPACE_ROOT (should fail)
    assert!(
        !&run_test_binary(&moved_binary_path, &moved_workspace, None)
            .status
            .success()
    );

    // Run test in moved workspace with INSTA_WORKSPACE_ROOT (should pass)
    assert!(&run_test_binary(
        &moved_binary_path,
        &moved_workspace,
        Some(("INSTA_WORKSPACE_ROOT", moved_workspace.to_str().unwrap())),
    )
    .status
    .success());
}

/// A cargo target that references a file outside of the project's directory
/// should still work
#[test]
fn test_external_test_path() {
    let test_project = TestFiles::new()
        .add_file(
            "proj/Cargo.toml",
            r#"
[package]
name = "external_test_path"
version = "0.1.0"
edition = "2021"

[dependencies]
insta = { path = '$PROJECT_PATH' }

[[test]]
name = "tlib"
path = "../tests/lib.rs"
"#
            .to_string(),
        )
        .add_file(
            "proj/src/lib.rs",
            r#"
pub fn hello() -> String {
    "Hello, world!".to_string()
}
"#
            .to_string(),
        )
        .add_file(
            "tests/lib.rs",
            r#"
use external_test_path::hello;

#[test]
fn test_hello() {
    insta::assert_snapshot!(hello());
}
"#
            .to_string(),
        )
        .create_project();

    // Change to the proj directory for running cargo commands
    let proj_dir = test_project.workspace_dir.join("proj");

    // Initially, the test should fail
    let output = test_project
        .insta_cmd()
        .current_dir(&proj_dir)
        .args(["test", "--"])
        .output()
        .unwrap();

    assert!(!&output.status.success());

    // Verify that the snapshot was created in the correct location
    assert_snapshot!(TestProject::current_file_tree(&test_project.workspace_dir), @r"
    proj
      proj/Cargo.lock
      proj/Cargo.toml
      proj/src
        proj/src/lib.rs
    tests
      tests/lib.rs
      tests/snapshots
        tests/snapshots/tlib__hello.snap.new
    ");

    // Run cargo insta accept
    let output = test_project
        .insta_cmd()
        .current_dir(&proj_dir)
        .args(["test", "--accept"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    // Verify that the snapshot was created in the correct location
    assert_snapshot!(TestProject::current_file_tree(&test_project.workspace_dir), @r"
    proj
      proj/Cargo.lock
      proj/Cargo.toml
      proj/src
        proj/src/lib.rs
    tests
      tests/lib.rs
      tests/snapshots
        tests/snapshots/tlib__hello.snap
    ");

    // Run the test again, it should pass now
    let output = test_project
        .insta_cmd()
        .current_dir(&proj_dir)
        .args(["test"])
        .output()
        .unwrap();

    assert!(&output.status.success());

    let snapshot_path = test_project
        .workspace_dir
        .join("tests/snapshots/tlib__hello.snap");
    assert_snapshot!(fs::read_to_string(snapshot_path).unwrap(), @r#"
    ---
    source: "../tests/lib.rs"
    expression: hello()
    ---
    Hello, world!
    "#);
}

/// Check that `--workspace-root` points `cargo-insta` at another path
#[test]
fn test_workspace_root_option() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "workspace_root_test"
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
pub fn hello() -> String {
    "Hello from workspace root!".to_string()
}

#[test]
fn test_hello() {
    insta::assert_snapshot!(hello());
}

#[test]
fn test_inline() {
    insta::assert_snapshot!("This is an inline snapshot", @"");
}
"#
            .to_string(),
        )
        .create_project();

    // Run the test with --workspace-root option
    let output = test_project
        .insta_cmd()
        .current_dir(std::env::current_dir().unwrap()) // Run from the current directory
        .args([
            "test",
            "--accept",
            "--workspace-root",
            test_project.workspace_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify inline snapshot
    assert_snapshot!(test_project.diff("src/lib.rs"), @r#"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -10,5 +10,5 @@
     
     #[test]
     fn test_inline() {
    -    insta::assert_snapshot!("This is an inline snapshot", @"");
    +    insta::assert_snapshot!("This is an inline snapshot", @"This is an inline snapshot");
     }
    "#);

    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/workspace_root_test__hello.snap
    ");
}

/// Check that `--manifest` points `cargo-insta` at another path
#[test]
fn test_manifest_option() {
    let test_project = TestFiles::new()
        .add_file(
            "Cargo.toml",
            r#"
[package]
name = "manifest_path_test"
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
pub fn greeting() -> String {
    "Greetings from manifest path!".to_string()
}

#[test]
fn test_greeting() {
    insta::assert_snapshot!(greeting());
}

#[test]
fn test_inline() {
    insta::assert_snapshot!("This is an inline snapshot for manifest path test", @"");
}
"#
            .to_string(),
        )
        .create_project();

    // Run the test with --manifest-path option
    let output = test_project
        .insta_cmd()
        .current_dir(std::env::current_dir().unwrap()) // Run from the current directory
        .args([
            "test",
            "--accept",
            "--manifest-path",
            test_project
                .workspace_dir
                .join("Cargo.toml")
                .to_str()
                .unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify inline snapshot
    assert_snapshot!(test_project.diff("src/lib.rs"), @r##"
    --- Original: src/lib.rs
    +++ Updated: src/lib.rs
    @@ -10,5 +10,5 @@
     
     #[test]
     fn test_inline() {
    -    insta::assert_snapshot!("This is an inline snapshot for manifest path test", @"");
    +    insta::assert_snapshot!("This is an inline snapshot for manifest path test", @"This is an inline snapshot for manifest path test");
     }
    "##);
    assert_snapshot!(test_project.file_tree_diff(), @r"
    --- Original file tree
    +++ Updated file tree
    @@ -1,4 +1,7 @@
     
    +  Cargo.lock
       Cargo.toml
       src
         src/lib.rs
    +    src/snapshots
    +      src/snapshots/manifest_path_test__greeting.snap
    ");
}
