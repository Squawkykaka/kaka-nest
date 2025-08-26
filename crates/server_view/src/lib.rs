use actix_files as fs;
use actix_web::{App, HttpServer};

pub async fn start_file_server() -> Result<(), std::io::Error> {
    HttpServer::new(|| {
        App::new().service(fs::Files::new("/", "./output").index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}
