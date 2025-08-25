use ignore::{DirEntry, Walk};
use tracing::error;

async fn discover_assets() {
    for file in Walk::new("./assets") {
        match file {
            Ok(file) => insert_file(file).await,
            Err(e) => {
                error!("File error: {}", e)
            }
        }
    }
}

async fn insert_file(file: DirEntry) {}
