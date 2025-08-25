use std::{
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::Result;

pub(crate) fn get_blog_paths() -> Result<Vec<PathBuf>> {
    // TODO, move into an option in the config file
    let files = visit_dir(Path::new("./assets/blog"))?
        .into_iter()
        .filter(|file| {
            file.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "md")
        })
        .collect();
    Ok(files)
}

pub(crate) fn visit_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let mut new_files = visit_dir(path.as_path())?;

            files.append(&mut new_files);
        } else {
            files.push(path);
        }
    }

    Ok(files)
}
