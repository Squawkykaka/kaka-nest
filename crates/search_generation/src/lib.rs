use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use tempdir::TempDir;

/// This function compiles a search index into wasm and outputs it to the output directory.
///
/// # Errors
/// - Creating temp dir
/// - Copying wasm crate files
/// - Running wasm-pack
/// - Copying output
pub fn compile_wasm_crate(index: Vec<u8>, path: &Path) -> io::Result<()> {
    let temp_dir = TempDir::new("kaka-nest-wasm")?;
    let pkg_path = temp_dir.path().join("pkg");

    create_wasm_crate(temp_dir.path(), index)?;

    // Run wasm-pack
    Command::new("wasm-pack")
        .arg("build")
        .arg("--target")
        .arg("web")
        .current_dir(temp_dir.path())
        .stdout(Stdio::inherit())
        .output()?;

    // Copy generated wasm
    fs::copy(pkg_path.join("wasm_search.js"), path.join("wasm_search.js"))?;
    fs::copy(
        pkg_path.join("wasm_search_bg.wasm"),
        path.join("wasm_search_bg.wasm"),
    )?;

    Ok(())
}

fn create_wasm_crate(path: &Path, index: Vec<u8>) -> io::Result<()> {
    let src_dir = path.join("src");
    fs::create_dir_all(&src_dir)?;

    let cargo_toml_path = path.join("Cargo.toml");
    let cargo_lock_path = path.join("Cargo.lock");
    let cargo_lib_path = path.join("src/lib.rs");
    let compiled_index_path = path.join("src/index");

    fs::write(
        cargo_toml_path,
        include_str!("../../../wasm-search-crate/Cargo.toml"),
    )?;
    fs::write(
        cargo_lock_path,
        include_str!("../../../wasm-search-crate/Cargo.lock"),
    )?;
    fs::write(
        cargo_lib_path,
        include_str!("../../../wasm-search-crate/src/lib.rs"),
    )?;
    fs::write(compiled_index_path, index)?;

    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
