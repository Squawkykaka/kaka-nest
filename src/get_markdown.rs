use std::{
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{Ok, Result};

pub fn get_blogs() -> Result<Vec<PathBuf>> {
    let path = Path::new("./assets/blog");

    let files: Vec<PathBuf> = visit_dir(path)?
        .into_iter()
        .filter(|file| file.extension().unwrap_or_default() == "md")
        .collect();

    Ok(files)
}

fn visit_dir(dir: &Path) -> Result<Vec<PathBuf>> {
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
