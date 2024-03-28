use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use insta::{assert_snapshot, Settings};
use walkdir::WalkDir;

struct OnDrop<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        let Self(f) = self;
        f.take().unwrap()();
    }
}

#[test]
fn main() {
    const NO_RECURSION: &str = "CARGO_INSTA_INTEGRATION_TESTS_NO_RECURSION";

    if env::var_os(NO_RECURSION).is_some() {
        return;
    }

    // copy new tests over

    // late-bind files as they're copied to ensure cargo doesn't try to run
    // these tests on the next invocation
    let copied = std::cell::RefCell::new(Vec::new());
    let _on_drop = OnDrop(Some(|| {
        let copied = copied.borrow();
        let copied = copied
            .iter()
            .filter_map(|copied| fs::remove_file(copied).err().map(|err| (copied, err)))
            .collect::<Vec<_>>();
        assert!(copied.is_empty(), "{:?}", copied);
    }));

    const SRC: &str = "tests/test-input";
    const DST: &str = "tests";
    for entry in WalkDir::new(SRC) {
        let entry = entry.unwrap();
        let source = entry.path();
        if source.is_dir() {
            continue;
        }
        let relative_source = source.strip_prefix(SRC).unwrap();
        let destination = Path::new(DST).join(relative_source);
        fs::copy(source, &destination).unwrap();
        copied.borrow_mut().push(destination);
    }

    // run tests and accept snapshots
    let Output {
        status,
        stdout,
        stderr,
    } = Command::new(env!("CARGO_BIN_EXE_cargo-insta"))
        .env(NO_RECURSION, "this value doesn't matter")
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
    assert!(stdout.contains("insta review finished"));
    assert!(stdout.contains("accepted"));

    // use insta itself to assert snapshots
    for entry in WalkDir::new(SRC) {
        let entry = entry.unwrap();
        let filename = entry.path().strip_prefix(SRC).unwrap().to_str().unwrap();
        if let Some(snapshot) = filename.strip_suffix(".rs") {
            let gen_file = Path::new(DST).join(filename);
            let mut settings = Settings::clone_current();
            settings.set_input_file(&gen_file);
            settings.bind(|| {
                assert_snapshot!(snapshot, &fs::read_to_string(gen_file).unwrap());
            });
        }
    }
}
