use std::env;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use globset::{GlobBuilder, GlobMatcher};
use once_cell::sync::Lazy;
use walkdir::WalkDir;

use crate::env::get_tool_config;
use crate::settings::Settings;
use crate::utils::style;

pub(crate) struct GlobCollector {
    pub(crate) fail_fast: bool,
    pub(crate) failed: usize,
    pub(crate) show_insta_hint: bool,
}

/// the glob stack holds failure count and an indication if `cargo insta review`
/// should be run.
pub(crate) static GLOB_STACK: Lazy<Mutex<Vec<GlobCollector>>> = Lazy::new(Mutex::default);

static GLOB_FILTER: Lazy<Vec<GlobMatcher>> = Lazy::new(|| {
    env::var("INSTA_GLOB_FILTER")
        .unwrap_or_default()
        .split(';')
        .filter(|x| !x.is_empty())
        .filter_map(|filter| {
            GlobBuilder::new(filter)
                .case_insensitive(true)
                .build()
                .ok()
                .map(|x| x.compile_matcher())
        })
        .collect()
});

pub fn glob_exec<F: FnMut(&Path)>(workspace_dir: &Path, base: &Path, pattern: &str, mut f: F) {
    // Check if the pattern contains parent directory traversal (../)
    if pattern.contains("../") || pattern.starts_with("..") {
        panic!("Parent directory traversal is not supported in glob patterns. Use the three-argument form of glob! with an explicit base directory instead.");
    }

    // If settings.allow_empty_glob() == true and `base` doesn't exist, skip
    // everything. This is necessary as `base` is user-controlled via `glob!/3`
    // and may not exist.
    let mut settings = Settings::clone_current();

    if settings.allow_empty_glob() && !base.exists() {
        return;
    }

    let glob = match GlobBuilder::new(pattern)
        .case_insensitive(true)
        .literal_separator(true)
        .build()
    {
        Ok(glob) => glob.compile_matcher(),
        Err(err) => panic!(
            "the glob! macro got an invalid pattern '{pattern}': {}",
            err.kind()
        ),
    };

    if !base.exists() {
        panic_no_match(base, pattern);
    }

    let walker = WalkDir::new(base).follow_links(true);
    let mut glob_found_matches = false;

    // step 1: collect all matching files
    let mut all_matching_files = vec![];
    let mut filtered_files = vec![];
    for file in walker {
        let file = match file {
            Ok(file) => file,
            Err(err) => panic!(
                "the glob! macro failed while walking {}: {err}",
                base.display()
            ),
        };
        let path = file.path();
        let stripped_path = path.strip_prefix(base).unwrap_or(path);
        if !glob.is_match(stripped_path) {
            continue;
        }

        glob_found_matches = true;
        all_matching_files.push(path.to_path_buf());

        // if there is a glob filter, skip if it does not match this path
        if !GLOB_FILTER.is_empty() && !GLOB_FILTER.iter().any(|x| x.is_match(stripped_path)) {
            eprintln!("Skipping {} due to glob filter", stripped_path.display());
            continue;
        }

        filtered_files.push(path.to_path_buf());
    }

    if !glob_found_matches && !settings.allow_empty_glob() {
        panic_no_match(base, pattern);
    }

    // step 2: sort, determine common prefix and run assertions
    all_matching_files.sort();
    filtered_files.sort();

    // Use the common prefix from ALL matching files, not just filtered ones
    // This preserves the original snapshot naming when filtering
    let common_prefix = find_common_prefix(&all_matching_files);
    let matching_files = filtered_files;

    GLOB_STACK.lock().unwrap().push(GlobCollector {
        failed: 0,
        show_insta_hint: false,
        fail_fast: get_tool_config(workspace_dir).glob_fail_fast(),
    });

    for path in &matching_files {
        settings.set_input_file(path);

        // if there is a common prefix, use that stirp down the input file.  That way we
        // can ensure that a glob like inputs/*/*.txt with a/file.txt and b/file.txt
        // does not create two identical snapshot suffixes.  Instead of file.txt for both
        // it would end up as a/file.txt and b/file.txt.
        let snapshot_suffix = if let Some(prefix) = common_prefix {
            path.strip_prefix(prefix).unwrap().as_os_str()
        } else {
            path.file_name().unwrap()
        };

        settings.set_snapshot_suffix(snapshot_suffix.to_str().unwrap());
        settings.bind(|| {
            f(path);
        });
    }

    let top = GLOB_STACK.lock().unwrap().pop().unwrap();

    if top.failed > 0 {
        if top.show_insta_hint {
            println!(
                "{hint}",
                hint = style(
                    "To update snapshots run `cargo insta review` or set `INSTA_UPDATE=always`"
                )
                .dim(),
            );
        }
        if top.failed > 1 {
            println!(
                "{hint}",
                hint = style("To enable fast failing for glob! export INSTA_GLOB_FAIL_FAST=1 as environment variable.").dim()
            );
        }
        panic!(
            "glob! resulted in {} snapshot assertion failure{}",
            top.failed,
            if top.failed == 1 { "" } else { "s" },
        );
    }
}

fn panic_no_match(base: &Path, pattern: &str) -> ! {
    let mut base_dir = base.display().to_string();
    if !base.exists() {
        base_dir.push_str(" (does not exist)");
    }
    panic!(
        "the glob! macro did not match any files.\n\
         pattern: {pattern}\n\
         base directory: {base_dir}\n\
         The pattern is relative to the base directory, which defaults to the \
         directory of the file calling glob!."
    );
}

fn find_common_prefix(sorted_paths: &[PathBuf]) -> Option<&Path> {
    let first = sorted_paths.first()?;
    let last = sorted_paths.last()?;
    let prefix_len = first
        .components()
        .zip(last.components())
        .take_while(|(a, b)| a == b)
        .count();

    if prefix_len == 0 {
        None
    } else {
        let mut prefix = first.components();
        for _ in 0..first.components().count() - prefix_len {
            prefix.next_back();
        }
        Some(prefix.as_path())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn failed_glob_leaves_no_stack_entry() {
        let tests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
        // A broken symlink makes the walk fail.
        #[cfg(unix)]
        let broken_link_dir = {
            let dir = tempfile::tempdir().unwrap();
            std::os::unix::fs::symlink(dir.path().join("missing"), dir.path().join("broken.txt"))
                .unwrap();
            dir
        };
        for (base, pattern) in [
            (tests_dir.join("missing-dir"), "*.txt"),
            (tests_dir.clone(), "nonexistent"),
            (tests_dir.clone(), "inputs/["),
            #[cfg(unix)]
            (broken_link_dir.path().to_path_buf(), "*.txt"),
        ] {
            let result = std::panic::catch_unwind(|| glob_exec(&tests_dir, &base, pattern, |_| {}));
            assert!(result.is_err(), "{pattern} in {}", base.display());
        }
        assert_eq!(GLOB_STACK.lock().unwrap().len(), 0);
    }
}
