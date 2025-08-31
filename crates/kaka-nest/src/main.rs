#![warn(clippy::pedantic)]

use std::{fs, io};

use clap::{Parser, Subcommand};
use kaka_nest::{
    build_page::{build_blog_list, create_blog_on_system},
    util::get_blog_paths,
};
use server_view::start_file_server;
use tinysearch::TinySearch;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Clone, Debug)]
enum Commands {
    Build,
    Serve,
    Search { query: Vec<String> },
}

#[actix_web::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args = Cli::parse();

    match args.command.unwrap_or(Commands::Build) {
        Commands::Build => {
            create_blog_on_system().unwrap();
        }
        Commands::Serve => start_file_server().await?,
        Commands::Search { query } => {
            let blog_paths = get_blog_paths()?;
            let posts = build_blog_list(&blog_paths).unwrap();

            let tinyesearch = TinySearch::new();
            let index = tinyesearch.build_index(&posts.blogs).unwrap();

            let results = tinyesearch.search(&index, &query.join(" "), 5);

            for result in results {
                println!("Found: {} at {}", result.0, result.1);
            }
        }
    }

    Ok(())
}
