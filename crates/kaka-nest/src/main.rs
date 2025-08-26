#![warn(clippy::pedantic)]

use clap::{Parser, Subcommand};
use kaka_nest::build_page::create_blog_on_system;
use server_view::start_file_server;
use tracing::info;

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
    }

    Ok(())
}
