use std::path::Path;

use globset::GlobBuilder;
use walkdir::WalkDir;

use crate::settings::Settings;

pub fn glob_exec<F: FnMut(&Path)>(base: &Path, pattern: &str, mut f: F) {
    let glob = GlobBuilder::new(pattern)
        .case_insensitive(true)
        .literal_separator(true)
        .build()
        .unwrap()
        .compile_matcher();

    let walker = WalkDir::new(base).follow_links(true);

    for file in walker {
        let file = file.unwrap();
        let path = file.path();
        if !glob.is_match(path) {
            continue;
        }

        let mut settings = Settings::clone_current();
        settings.set_input_file(&path);
        settings.set_snapshot_suffix(path.file_name().unwrap().to_str().unwrap());

        settings.bind(|| {
            f(path);
        });
    }
}
