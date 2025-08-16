use std::fs;

mod get_markdown;
mod render_html;

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    let blogs = get_markdown::get_blogs()?;
    let mut handlebars = handlebars::Handlebars::new();
    handlebars.register_template_file("blog", "./assets/templates/blog.html")?;

    // make output if it doesnt exist;
    match fs::create_dir_all("./output/posts") {
        Ok(_) => {}
        Err(_) => {}
    };

    for blog in blogs {
        let blog_html = blog.to_blog_html(&handlebars)?;

        fs::write(
            format!("./output/posts/{}.html", blog.metadata.title),
            blog_html,
        )?;
    }

    // dbg!(blogs);
    Ok(())
}
