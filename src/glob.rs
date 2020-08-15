use std::path::Path;

use globwalk::{FileType, GlobWalkerBuilder};

use crate::settings::Settings;

pub fn glob_exec<F: FnMut(&Path)>(base: &Path, pattern: &str, mut f: F) -> usize {
    let walker = GlobWalkerBuilder::new(base, pattern)
        .case_insensitive(true)
        .file_type(FileType::FILE)
        .build()
        .unwrap();

    let mut count = 0;
    for file in walker {
        count += 1;
        let file = file.unwrap();
        let path = file.path();

        let mut settings = Settings::clone_current();
        settings.set_input_file(&path);
        settings.set_snapshot_suffix(path.file_name().unwrap().to_str().unwrap());

        settings.bind(|| {
            f(path);
        });
    }

    count
}
