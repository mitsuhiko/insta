use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use dircpy::CopyBuilder;
use insta::{assert_snapshot, Settings};
use walkdir::WalkDir;

fn main() {
    // copy new tests over
    fs::remove_dir_all("tests").ok();
    CopyBuilder::new("test-input", "tests")
        .overwrite(true)
        .run()
        .unwrap();

    // delete old build artifacts
    Command::new("cargo")
        .arg("clean")
        .arg("--package=integration-tests")
        .status()
        .unwrap();

    // make sure cargo-insta is built
    Command::new("cargo")
        .arg("build")
        .current_dir("..")
        .status()
        .unwrap();

    // run tests and accept snapshots
    let Output {
        status,
        stdout,
        stderr,
    } = Command::new("../target/debug/cargo-insta")
        .arg("test")
        .arg("--accept")
        .arg("--no-ignore")
        .output()
        .unwrap();
    use std::io::Write as _;
    std::io::stdout().write_all(&stdout).unwrap();
    std::io::stderr().write_all(&stderr).unwrap();
    assert!(status.success());
    let stdout = std::str::from_utf8(stdout.as_slice()).unwrap();
    let stderr = std::str::from_utf8(stderr.as_slice()).unwrap();
    assert!(stdout.contains("insta review finished"));
    assert!(stdout.contains("accepted"));
    assert!(stderr.contains("Compiling"));
    assert!(stderr.contains("integration-tests"));

    // use insta itself to assert snapshots
    for entry in WalkDir::new("test-input") {
        let entry = entry.unwrap();
        let filename = entry
            .path()
            .strip_prefix("test-input/")
            .unwrap()
            .to_str()
            .unwrap();
        if let Some(snapshot) = filename.strip_suffix(".rs") {
            let gen_file = Path::new("tests").join(filename);
            let mut settings = Settings::clone_current();
            settings.set_input_file(&gen_file);
            settings.bind(|| {
                assert_snapshot!(snapshot, &fs::read_to_string(gen_file).unwrap());
            });
        }
    }
}
