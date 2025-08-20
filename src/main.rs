use clap::{Parser, Subcommand};
use handlebars::Handlebars;
use lazy_static::lazy_static;
use log::info;
use syntastica::Processor;
use syntastica_parsers::LanguageSetImpl;

use crate::build_page::create_blogs_on_system;

mod build_page;
mod pullmark_parsers;
mod util;

lazy_static! {
    pub static ref HANDLEBARS: Handlebars<'static> = {
        let mut handlebars = handlebars::Handlebars::new();

        // Register partials
        handlebars
            .register_template_file("navbar","./assets/templates/navbar.html")
            .unwrap();
        handlebars
            .register_template_file("styles","./assets/templates/styles.html")
            .unwrap();

        // Register templates
        handlebars
            .register_template_file("blog", "./assets/templates/blog.html")
            .unwrap();
        handlebars
            .register_template_file("blockquote", "./assets/templates/modules/blockquote.html")
            .unwrap();
        handlebars
            .register_template_file("codeblock", "./assets/templates/modules/codeblock.html")
            .unwrap();
        handlebars
            .register_template_file("tag_page", "./assets/templates/tag_page.html")
            .unwrap();

        handlebars
    };
    pub static ref LEAKED_LANGSET: &'static LanguageSetImpl =
        Box::leak(Box::new(LanguageSetImpl::new()));
}

thread_local! {
    pub static TL_PROCESSOR: std::cell::RefCell<Processor<'static, LanguageSetImpl>> =
        std::cell::RefCell::new(Processor::new(*LEAKED_LANGSET));
}

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
}

fn main() -> color_eyre::eyre::Result<()> {
    env_logger::init();
    color_eyre::install()?;

    let args = Cli::parse();

    match args.command.unwrap_or({
        info!("No command specified, assuming \"build\"");

        Commands::Build
    }) {
        Commands::Build {} => {
            create_blogs_on_system()?;
        }
    }

    Ok(())
}
