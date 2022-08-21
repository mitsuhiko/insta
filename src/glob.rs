use std::env;
use std::path::Path;

use globset::{GlobBuilder, GlobMatcher};
use once_cell::sync::Lazy;
use walkdir::WalkDir;

use crate::settings::Settings;

pub fn glob_exec<F: FnMut(&Path)>(base: &Path, pattern: &str, mut f: F) {
    let glob = GlobBuilder::new(pattern)
        .case_insensitive(true)
        .literal_separator(true)
        .build()
        .unwrap()
        .compile_matcher();

    static GLOB_FILTER: Lazy<Option<GlobMatcher>> = Lazy::new(|| {
        let glob_filter = env::var("INSTA_GLOB_FILTER").ok()?;
        let glob = GlobBuilder::new(&glob_filter)
            .case_insensitive(true)
            .literal_separator(true)
            .build()
            .ok()?
            .compile_matcher();
        Some(glob)
    });

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
        if let Some(filter) = &*GLOB_FILTER {
            if !filter.is_match(stripped_path) {
                continue;
            }
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
