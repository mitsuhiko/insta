use std::env;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use globset::{GlobBuilder, GlobMatcher};
use walkdir::WalkDir;

use crate::settings::Settings;
use crate::utils::style;

pub(crate) struct GlobCollector {
    pub(crate) fail_fast: bool,
    pub(crate) failed: usize,
    pub(crate) show_insta_hint: bool,
}

// the glob stack holds failure count + an indication if cargo insta review
// should be run.
lazy_static::lazy_static! {
    pub(crate) static ref GLOB_STACK: Mutex<Vec<GlobCollector>> = Mutex::default();
}

lazy_static::lazy_static! {
    static ref GLOB_FILTER: Vec<GlobMatcher> = {
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
    };
}

pub fn glob_exec<F: FnMut(&Path)>(base: &Path, pattern: &str, mut f: F) {
    let glob = GlobBuilder::new(pattern)
        .case_insensitive(true)
        .literal_separator(true)
        .build()
        .unwrap()
        .compile_matcher();

    let walker = WalkDir::new(base).follow_links(true);
    let mut glob_found_matches = false;
    let mut settings = Settings::clone_current();

    GLOB_STACK.lock().unwrap().push(GlobCollector {
        failed: 0,
        show_insta_hint: false,
        fail_fast: std::env::var("INSTA_GLOB_FAIL_FAST").as_deref() == Ok("1"),
    });

    // step 1: collect all matching files
    let mut matching_files = vec![];
    for file in walker {
        let file = file.unwrap();
        let path = file.path();
        let stripped_path = path.strip_prefix(base).unwrap_or(path);
        if !glob.is_match(stripped_path) {
            continue;
        }

        glob_found_matches = true;

        // if there is a glob filter, skip if it does not match this path
        if !GLOB_FILTER.is_empty() && !GLOB_FILTER.iter().any(|x| x.is_match(stripped_path)) {
            eprintln!("Skipping {} due to glob filter", stripped_path.display());
            continue;
        }

        matching_files.push(path.to_path_buf());
    }

    // step 2: sort, determine common prefix and run assertions
    matching_files.sort();
    let common_prefix = find_common_prefix(&matching_files);
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
    if !glob_found_matches && !settings.allow_empty_glob() {
        panic!("the glob! macro did not match any files.");
    }

    if top.failed > 0 {
        if top.show_insta_hint {
            println!(
                "{hint}",
                hint = style("To update snapshots run `cargo insta review`").dim(),
            );
        }
        if top.failed > 1 {
            println!(
                "{hint}",
                hint = style("To enable fast failing for glob! export INSTA_GLOB_FAIL_FAST=1 as environment variable.").dim()
            );
        }
        panic!(
            "glob! resulted in {} snapshot assertion failure{}s",
            top.failed,
            if top.failed == 1 { "" } else { "s" },
        );
    }
}

fn find_common_prefix(sorted_paths: &[PathBuf]) -> Option<&Path> {
    let first = sorted_paths.first()?;
    let last = sorted_paths.last()?;
    let mut prefix_len = 0;
    for (a, b) in first.components().zip(last.components()) {
        if a == b {
            prefix_len += 1;
        } else {
            break;
        }
    }

    if prefix_len == 0 {
        None
    } else {
        let mut components = first.components();
        for _ in 0..first.components().count() - prefix_len {
            components.next_back();
        }
        Some(components.as_path())
    }
}
