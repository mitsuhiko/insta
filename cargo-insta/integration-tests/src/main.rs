use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

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
    Command::new("../target/debug/cargo-insta")
        .arg("test")
        .arg("--accept")
        .arg("--no-ignore")
        .status()
        .unwrap();

    // use insta itself to assert snapshots
    for entry in WalkDir::new("test-input") {
        let entry = entry.unwrap();
        let filename = entry
            .path()
            .strip_prefix("test-input/")
            .unwrap()
            .to_str()
            .unwrap();
        if filename.ends_with(".rs") {
            let gen_file = Path::new("tests").join(filename);
            let mut settings = Settings::clone_current();
            settings.set_input_file(&gen_file);
            let snapshot = &filename[..filename.len() - 3];
            settings.bind(|| {
                assert_snapshot!(snapshot, &fs::read_to_string(gen_file).unwrap());
            });
        }
    }
}
