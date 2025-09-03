use super::TestFiles;

fn check_nextest_installed() {
    if std::process::Command::new("cargo")
        .args(["nextest", "--version"])
        .output()
        .map(|output| !output.status.success())
        .unwrap_or(true)
    {
        panic!("cargo-nextest is required to run these tests. Install with: cargo install cargo-nextest");
    }
}

/// Test that additional separator works with nextest to pass arguments to both nextest and test binary
#[test]
fn test_nextest_additional_separator() {
    check_nextest_installed();
    let test_project = TestFiles::new()
        .add_cargo_toml("nextest_double_separator")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_success() {
    insta::assert_snapshot!("success", @"Hello, world!");
}

#[test] 
fn test_another() {
    insta::assert_snapshot!("another", @"Another test!");
}
"#
            .to_string(),
        )
        .create_project();

    // Test with additional separator - should pass nextest args correctly
    // Using --status-level none should suppress output
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--test-runner",
            "nextest",
            "--accept",
            "--",
            "--status-level",
            "none",
            "--",
        ])
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The deprecation warning should NOT appear with additional separator
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("In a future version"),
        "Deprecation warning should not appear with additional separator: {stderr}"
    );

    // With --status-level none, we should see minimal output
    assert!(
        !stderr.contains("PASS"),
        "PASS should not appear with --status-level none: {stderr}"
    );
}

/// Test that single separator with nextest shows deprecation warning
#[test]
fn test_nextest_single_separator_deprecation() {
    check_nextest_installed();
    let test_project = TestFiles::new()
        .add_cargo_toml("nextest_single_separator")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_simple() {
    insta::assert_snapshot!("simple", @"Test!");
}
"#
            .to_string(),
        )
        .create_project();

    // Test with single separator - should show deprecation warning
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--test-runner",
            "nextest",
            "--accept",
            "--",
            "--nocapture",
        ])
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    // It will fail because --nocapture is passed to test binary which doesn't understand it
    // but that's expected for backward compatibility

    let stderr = String::from_utf8_lossy(&output.stderr);

    // The deprecation warning SHOULD appear with single separator
    assert!(
        stderr.contains("The single `--` separator with nextest will change behavior"),
        "Deprecation warning should appear with single separator. Stderr: {stderr}"
    );
}

/// Test that cargo test (not nextest) still works with single separator and no warning
#[test]
fn test_cargo_test_single_separator_no_warning() {
    let test_project = TestFiles::new()
        .add_cargo_toml("cargo_test_single_separator")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_basic() {
    insta::assert_snapshot!("basic", @"Basic test!");
}
"#
            .to_string(),
        )
        .create_project();

    // Test with cargo test and single separator - should work normally with no warning
    let output = test_project
        .insta_cmd()
        .args(["test", "--accept", "--", "--nocapture"])
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // No deprecation warning should appear for cargo test
    assert!(
        !stderr.contains("In a future version"),
        "Deprecation warning should not appear with cargo test: {stderr}"
    );
}

/// Test that nextest with additional separator correctly passes status-level to nextest
#[test]
fn test_nextest_status_level_all() {
    check_nextest_installed();
    let test_project = TestFiles::new()
        .add_cargo_toml("nextest_status_level")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_visible() {
    insta::assert_snapshot!("visible", @"Should see this with status-level all!");
}
"#
            .to_string(),
        )
        .create_project();

    // Test with --status-level all to see output
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--test-runner",
            "nextest",
            "--accept",
            "--",
            "--status-level",
            "all",
            "--",
        ])
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // With --status-level all, we should see PASS in the output
    let combined_output = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        combined_output.contains("PASS"),
        "PASS should appear with --status-level all. Output: {combined_output}"
    );
}

/// Test backward compatibility: single -- with nextest passes args to test binaries
#[test]
fn test_nextest_single_separator_backward_compat() {
    check_nextest_installed();
    let test_project = TestFiles::new()
        .add_cargo_toml("nextest_backward_compat")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_specific() {
    insta::assert_snapshot!("specific", @"Specific test!");
}

#[test]
fn test_other() {
    insta::assert_snapshot!("other", @"Other test!");
}
"#
            .to_string(),
        )
        .create_project();

    // Test with single separator - args should go to test binaries for backward compat
    // Using a test name filter as the argument
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--test-runner",
            "nextest",
            "--accept",
            "--",
            "test_specific", // This should be passed to test binaries as a filter
        ])
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should see deprecation warning
    assert!(
        stderr.contains("The single `--` separator with nextest will change behavior"),
        "Deprecation warning should appear with single separator. Stderr: {stderr}"
    );

    // Check that only the filtered test ran (backward compat behavior)
    // The test filter should have been passed to the test binary
    let combined_output = format!("{}{}", String::from_utf8_lossy(&output.stdout), stderr);

    // Should have run 1 test, skipped 1 (filtered by test binary)
    assert!(
        combined_output.contains("1 test run") && combined_output.contains("1 passed"),
        "Should run only the filtered test for backward compat. Output: {combined_output}"
    );
}

/// Test that with double --, args before second -- go to nextest
#[test]
fn test_nextest_double_separator_args_to_nextest() {
    check_nextest_installed();
    let test_project = TestFiles::new()
        .add_cargo_toml("nextest_args_routing")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_one() {
    insta::assert_snapshot!("one", @"Test one!");
}

#[test]
fn test_two() {
    insta::assert_snapshot!("two", @"Test two!");
}

#[test]
fn test_three() {
    insta::assert_snapshot!("three", @"Test three!");
}
"#
            .to_string(),
        )
        .create_project();

    // Test with double separator - args before second -- should go to nextest
    // Using --max-fail=1 which is a nextest-specific argument that stops after 1 failure
    // We'll force a failure to test this works
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--test-runner",
            "nextest",
            "--",
            "--max-fail=1", // This is a nextest arg, should stop after 1 failure
            "--",
        ])
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .unwrap();

    // Should fail because snapshots aren't accepted
    assert!(
        !output.status.success(),
        "Should fail due to unaccepted snapshots"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined_output = format!("{}{}", String::from_utf8_lossy(&output.stdout), stderr);

    // Should NOT see deprecation warning with double separator
    assert!(
        !stderr.contains("The single `--` separator with nextest will change behavior"),
        "Deprecation warning should NOT appear with double separator. Stderr: {stderr}"
    );

    // Verify --max-fail=1 worked by checking that nextest stopped after 1 failure
    // With --max-fail=1, nextest should report "Canceling due to --max-fail=1"
    assert!(
        combined_output.contains("max-fail")
            || combined_output.contains("Canceling")
            || combined_output.contains("1 test failed"),
        "Should show nextest respected --max-fail=1. Output: {combined_output}"
    );
}

/// Test empty arguments after separator don't cause issues
#[test]
fn test_nextest_empty_args() {
    check_nextest_installed();
    let test_project = TestFiles::new()
        .add_cargo_toml("nextest_empty_args")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_empty() {
    insta::assert_snapshot!("empty", @"Empty args test!");
}
"#
            .to_string(),
        )
        .create_project();

    // Test with additional separator but no args - should work fine
    let output = test_project
        .insta_cmd()
        .args(["test", "--test-runner", "nextest", "--accept", "--", "--"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that test binary arguments are passed through nextest with additional separator
#[test]
fn test_nextest_test_binary_args_passed() {
    check_nextest_installed();
    let test_project = TestFiles::new()
        .add_cargo_toml("nextest_test_binary_args")
        .add_file(
            "src/lib.rs",
            r#"
#[test]
fn test_with_filter() {
    println!("This test runs with filter");
    insta::assert_snapshot!("filtered", @"Test with filter!");
}

#[test]
fn test_another() {
    println!("This test should not run");
    insta::assert_snapshot!("another", @"Should not see this!");
}
"#
            .to_string(),
        )
        .create_project();

    // Test with additional separator passing test filter to test binary
    // The filter "test_with_filter" should be passed to the test binary
    let output = test_project
        .insta_cmd()
        .args([
            "test",
            "--test-runner",
            "nextest",
            "--accept",
            "--",
            "--status-level",
            "all", // nextest arg to see output
            "--",
            "test_with_filter", // test binary arg (filter)
        ])
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let combined_output = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Should see the filtered test running (PASS line)
    assert!(
        combined_output.contains("PASS") && combined_output.contains("test_with_filter"),
        "Should see test_with_filter passing in output: {combined_output}"
    );

    // The other test should be skipped (filtered out by test binary arg)
    assert!(
        combined_output.contains("SKIP") && combined_output.contains("test_another"),
        "Should see test_another being skipped (filtered out): {combined_output}"
    );

    // Verify we ran 1 test and skipped 1
    assert!(
        combined_output.contains("1 test run: 1 passed, 1 skipped"),
        "Should show 1 test run and 1 skipped: {combined_output}"
    );
}
