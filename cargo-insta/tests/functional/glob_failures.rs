use std::collections::BTreeMap;

use super::*;

/// Runs the tests of the project with `cargo insta test --check` and returns
/// the result of each test, then the panic messages and the glob failure lines
/// of each failed test.
fn run_tests(test_project: &TestProject, args: &[&str], envs: &[(&str, &str)]) -> String {
    let output = test_project
        .insta_cmd()
        .args(["test", "--check", "--"])
        .args(args)
        .env("RUST_BACKTRACE", "0")
        .envs(envs.iter().copied())
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results: Vec<&str> = stdout
        .lines()
        .filter(|line| {
            line.starts_with("test ")
                && (line.ends_with(" ... ok") || line.ends_with(" ... FAILED"))
        })
        .collect();
    results.sort_unstable();

    // Tests that run in parallel finish in any order, so the output of the
    // failed tests is sorted by test name.
    let mut failures: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut test = None;
    let mut after_panic = false;
    for line in stdout.lines() {
        if let Some(name) = line
            .strip_prefix("---- ")
            .and_then(|line| line.strip_suffix(" stdout ----"))
        {
            test = Some(name);
        } else if let Some(test) = test {
            if after_panic || line.starts_with("snapshot assertion") {
                failures.entry(test).or_default().push(line);
            }
        }
        after_panic = line.starts_with("thread '") && line.contains(" panicked at ");
    }

    let mut summary: Vec<String> = results.into_iter().map(str::to_string).collect();
    for (test, lines) in failures {
        summary.push(format!("---- {test} stdout ----"));
        summary.extend(lines.into_iter().map(str::to_string));
    }
    summary.join("\n")
}

fn glob_project(name: &str, lib: &str) -> TestProject {
    TestFiles::new()
        .add_file(
            "Cargo.toml",
            format!(
                r#"
[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
doctest = false

[dependencies]
insta = {{ path = '$PROJECT_PATH', features = ["glob"] }}
"#
            ),
        )
        .add_file("src/lib.rs", lib.to_string())
        .add_file("src/data/a.txt", "a".to_string())
        .create_project()
}

/// A snapshot failure in another test that runs at the same time as a
/// `glob!` fails that test, and is not counted by the `glob!`.
#[test]
fn test_glob_counts_failures_of_its_own_thread() {
    let test_project = glob_project(
        "test_glob_counts_failures_of_its_own_thread",
        r#"
use std::sync::Barrier;

static GLOB_STARTED: Barrier = Barrier::new(2);
static ASSERTED: Barrier = Barrier::new(2);

#[test]
fn test_glob() {
    insta::glob!("data/*.txt", |_| {
        GLOB_STARTED.wait();
        ASSERTED.wait();
        insta::assert_snapshot!("value", @"other value");
    });
}

#[test]
fn test_outside_glob() {
    GLOB_STARTED.wait();
    let result = std::panic::catch_unwind(|| {
        insta::assert_snapshot!("value", @"other value");
    });
    ASSERTED.wait();
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}
"#,
    );

    assert_snapshot!(run_tests(&test_project, &["--test-threads=2"], &[]), @"
    test test_glob ... FAILED
    test test_outside_glob ... FAILED
    ---- test_glob stdout ----
    snapshot assertion from glob for 'glob' failed in line 12
    glob! resulted in 1 snapshot assertion failure
    ---- test_outside_glob stdout ----
    snapshot assertion for 'outside_glob' failed in line 20
    ");
}

/// A panic in a `glob!` closure does not make later snapshot failures in the
/// same thread pass, and nested globs keep their counts.
#[test]
fn test_glob_closure_panic() {
    let test_project = glob_project(
        "test_glob_closure_panic",
        r#"
#[test]
fn test_a_failing_snapshot_after_caught_glob_panic() {
    let result = std::panic::catch_unwind(|| {
        insta::glob!("data/*.txt", |_| panic!("closure panicked"));
    });
    assert!(result.is_err());
    insta::assert_snapshot!("value", @"other value");
}

#[test]
fn test_b_nested_globs() {
    insta::glob!("data/*.txt", |_| {
        let result = std::panic::catch_unwind(|| {
            insta::glob!("data/*.txt", |_| {
                insta::assert_snapshot!("value", @"other value");
            });
        });
        assert!(result.is_err(), "the inner glob did not fail");
        insta::assert_snapshot!("value", @"other value");
    });
}
"#,
    );

    assert_snapshot!(run_tests(&test_project, &["--test-threads=1"], &[]), @"
    test test_a_failing_snapshot_after_caught_glob_panic ... FAILED
    test test_b_nested_globs ... FAILED
    ---- test_a_failing_snapshot_after_caught_glob_panic stdout ----
    closure panicked
    snapshot assertion for 'a_failing_snapshot_after_caught_glob_panic' failed in line 8
    ---- test_b_nested_globs stdout ----
    snapshot assertion from glob for 'b_nested_globs' failed in line 16
    glob! resulted in 1 snapshot assertion failure
    snapshot assertion from glob for 'b_nested_globs-2' failed in line 20
    glob! resulted in 1 snapshot assertion failure
    ");
}

/// With `INSTA_GLOB_FAIL_FAST=1`, a failure in one `glob!` does not break
/// later globs and snapshot assertions.
#[test]
fn test_glob_fail_fast() {
    let test_project = glob_project(
        "test_glob_fail_fast",
        r#"
#[test]
fn test_a_glob_fails() {
    insta::glob!("data/*.txt", |_| {
        insta::assert_snapshot!("value", @"other value");
    });
}

#[test]
fn test_b_glob_passes() {
    insta::glob!("data/*.txt", |_| {});
}

#[test]
fn test_c_failing_snapshot() {
    insta::assert_snapshot!("value", @"other value");
}
"#,
    );

    let results = run_tests(
        &test_project,
        &["--test-threads=1"],
        &[("INSTA_GLOB_FAIL_FAST", "1")],
    );
    assert_snapshot!(results, @"
    test test_a_glob_fails ... FAILED
    test test_b_glob_passes ... ok
    test test_c_failing_snapshot ... FAILED
    ---- test_a_glob_fails stdout ----
    snapshot assertion from glob for 'a_glob_fails' failed in line 5
    ---- test_c_failing_snapshot stdout ----
    snapshot assertion for 'c_failing_snapshot' failed in line 16
    ");
}
