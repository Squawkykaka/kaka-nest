use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use color_eyre::eyre::Result;
use log::{debug, info};
use pulldown_cmark::{Options, Parser};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    HANDLEBARS, TL_PROCESSOR, build_page,
    pullmark_parsers::{format_blockquotes, highlight_codeblocks},
    util::{get_blog_paths, visit_dir},
};

#[derive(Debug, Serialize)]
pub(crate) struct Blog {
    pub id: u32,
    pub metadata: BlogMetadata,
    pub contents: String,
}
impl Blog {
    pub(crate) fn to_blog_html(&self) -> Result<String> {
        let rendered_string = HANDLEBARS.render("blog", self)?;

        Ok(rendered_string)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct BlogMetadata {
    pub date: String,
    pub title: String,
    pub published: bool,
    pub tags: Option<Vec<String>>,
    pub read_mins: u32,
}

/// A struct containing all currently exisiting blogs & tags
#[derive(Default, Debug)]
pub(crate) struct BlogList {
    pub blogs: Vec<Blog>,
    pub tags: HashMap<String, HashSet<u32>>,
}

pub(crate) fn create_blogs_on_system() -> color_eyre::eyre::Result<()> {
    let blogs: build_page::BlogList = build_page::get_blogs()?;

    // Replace silent error swallowing
    if Path::new("./output").exists() {
        fs::remove_dir_all("./output")?;
    } else {
        fs::create_dir_all("./output/posts")?;
    }
    // make output dirs
    fs::create_dir_all("./output/posts")?;
    fs::create_dir_all("./output/images")?;
    fs::create_dir_all("./output/tags")?;

    // Output all blogs.
    for blog in &blogs.blogs {
        if !blog.metadata.published {
            continue;
        }

        info!("Converting blog {} to html", blog.id);
        let blog_html = blog.to_blog_html()?;

        fs::write(format!("./output/posts/{}.html", blog.id), blog_html)?;
    }

    output_tags_to_fs(&blogs)?;

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

    Ok(())
}

fn output_tags_to_fs(blogs: &BlogList) -> Result<()> {
    for (tag, blogs_with) in &blogs.tags {
        let posts: Vec<_> = blogs
            .blogs
            .iter()
            .filter(|blog| blogs_with.contains(&blog.id))
            .collect();

        let json_tag = json!({
            "name": tag,
            "posts": posts
        });

        let contents = HANDLEBARS.render("tag_page", &json_tag)?;

        let stripped_tag = match tag.strip_prefix("#") {
            Some(tag) => tag,
            None => {
                debug!("Tag without # prefix ({})", tag);
                tag.as_str()
            }
        };

        fs::write(format!("./output/tags/{}.html", stripped_tag), contents)?;
    }

    Ok(())
}

pub fn get_blogs() -> Result<BlogList> {
    let mut blog_list = BlogList::default();

    let blog_paths = get_blog_paths()?;

    for (id, blog_path) in blog_paths.into_iter().enumerate() {
        debug!("Reading blog {}", id);
        let blog_bytes = fs::read(&blog_path)?;
        let blog_contents = std::str::from_utf8(&blog_bytes)?;

        render_blog(blog_contents, &mut blog_list, id as u32)?;

        debug!("Finished parsing blog {}", id);
    }

    Ok(blog_list)
}

fn render_blog(input: &str, blog_list: &mut BlogList, id: u32) -> Result<()> {
    // Generate HTML
    let html_output = render_html_page_from_markdown(input);

    // get metadata options
    let blog_metadata_string = match input.split("---").nth(1) {
        Some(blog_metadata) => blog_metadata,
        None => return Err(color_eyre::eyre::eyre!("Didnt include metadata for file")),
    };

    let mut blog_metadata: BlogMetadata = serde_yaml::from_str(blog_metadata_string)?;

    // Remove '#' prefix from each tag if present
    if let Some(tags) = &mut blog_metadata.tags {
        for tag in tags.iter_mut() {
            if let Some(stripped) = tag.strip_prefix('#') {
                *tag = stripped.to_string();
            }
        }
    }

    let blog = Blog {
        id,
        metadata: blog_metadata,
        contents: html_output,
    };

    // Insert blog into list
    if let Some(tags) = &blog.metadata.tags {
        for tag in tags {
            blog_list
                .tags
                .entry(tag.to_string())
                .or_default()
                .insert(id);
        }
    }

    blog_list.blogs.push(blog);

    Ok(())
}

fn render_html_page_from_markdown(input: &str) -> String {
    let mut pullmark_options = Options::empty();
    pullmark_options.insert(Options::ENABLE_WIKILINKS);
    pullmark_options.insert(Options::ENABLE_STRIKETHROUGH);
    pullmark_options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    pullmark_options.insert(Options::ENABLE_TASKLISTS);

    let html_output = TL_PROCESSOR.with_borrow_mut(|processer| {
        let parser = Parser::new_ext(input, pullmark_options);
        let parser = highlight_codeblocks(parser, processer);
        let parser = format_blockquotes(parser);

        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);

        html_output
    });

    html_output
}
