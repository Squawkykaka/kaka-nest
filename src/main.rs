use std::{fs, path::Path, sync::Mutex};

use handlebars::Handlebars;
use lazy_static::lazy_static;
use syntastica::Processor;
use syntastica_parsers::LanguageSetImpl;

use crate::get_markdown::visit_dir;

mod get_markdown;
mod pullmark_parsers;

lazy_static! {
    pub static ref HANDLEBARS: Handlebars<'static> = {
        let mut handlebars = handlebars::Handlebars::new();

        handlebars
            .register_template_file("blog", "./assets/templates/blog.html")
            .unwrap();
        handlebars
            .register_template_file("blockquote", "./assets/templates/blockquote.html")
            .unwrap();
        handlebars
            .register_template_file("codeblock", "./assets/templates/codeblock.html")
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

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let blogs: get_markdown::BlogList = get_markdown::get_blogs()?;

    // Replace silent error swallowing
    if Path::new("./output").exists() {
        fs::remove_dir_all("./output")?;
    } else {
        fs::create_dir_all("./output/posts")?;
    }
    // make output dirs
    fs::create_dir_all("./output/posts")?;
    fs::create_dir_all("./output/images")?;

    // Output all blogs.
    for blog in blogs.blogs {
        if !blog.metadata.published {
            continue;
        }

        println!("Rendering Blog {}: {}", blog.id, blog.metadata.title);

        let blog_html = blog.to_blog_html()?;

        fs::write(format!("./output/posts/{}.html", blog.id), blog_html)?;
    }
    // Copy over files
    fs::copy(
        "./assets/fonts/Iosevka-Regular.ttf",
        "./output/Iosevka-Regular.ttf",
    )?;

    let images = visit_dir(Path::new("./assets/images"))?;
    for image_path in images {
        let file_name = image_path.file_name().unwrap();
        fs::copy(
            &image_path,
            format!("./output/images/{}", file_name.to_str().unwrap()),
        )?;
    }

    // Output all tags

    // dbg!(blogs);
    Ok(())
}
