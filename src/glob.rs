use std::env;
use std::path::Path;

use globset::{GlobBuilder, GlobMatcher};
use once_cell::sync::Lazy;
use walkdir::WalkDir;

use crate::settings::Settings;

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

        settings.set_input_file(&path);
        settings.set_snapshot_suffix(path.file_name().unwrap().to_str().unwrap());

        settings.bind(|| {
            f(path);
        });
    }

    if !glob_found_matches && !settings.allow_empty_glob() {
        panic!("the glob! macro did not match any files.");
    }
}
