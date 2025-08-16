use std::fs;

mod get_markdown;

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    let blogs: get_markdown::BlogList = get_markdown::get_blogs()?;
    let mut handlebars = handlebars::Handlebars::new();
    handlebars.register_template_file("blog", "./assets/templates/blog.html")?;

    // Remove otuput dir
    match fs::remove_dir_all("./output") {
        Ok(_) => {}
        Err(_) => {}
    };
    // make output dir
    fs::create_dir_all("./output/posts")?;

    dbg!(blogs.existing_tags);

    // Output all blogs.
    for blog in blogs.blogs {
        println!("Rendering Blog {}: {}", blog.id, blog.metadata.title);

        if !blog.metadata.published {
            continue;
        }

        let blog_html = blog.to_blog_html(&handlebars)?;

        fs::write(format!("./output/posts/{}.html", blog.id), blog_html)?;
    }

    // Output all tags

    // dbg!(blogs);
    Ok(())
}
