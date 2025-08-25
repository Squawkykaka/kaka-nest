use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::Result;
use log::{debug, info};
use pulldown_cmark::{Options, Parser};
use rss::{Category, ChannelBuilder, Item, ItemBuilder, extension::Extension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    HANDLEBARS, TL_PROCESSOR, build_page,
    pullmark_parsers::{format_blockquotes, highlight_codeblocks},
    util::{get_blog_paths, visit_dir},
};

#[derive(Debug, Serialize)]
pub(crate) struct Blog {
    pub title: String,
    pub slug: String,
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
    pub published: bool,
    pub tags: Option<Vec<String>>,
    pub read_mins: u32,
    pub description: Option<String>,
}

/// A struct containing all currently exisiting blogs & tags
#[derive(Default, Debug)]
pub(crate) struct BlogList {
    pub blogs: Vec<Blog>,
    pub tags: HashMap<String, HashSet<String>>,
}

pub(crate) fn create_blogs_on_system() -> color_eyre::eyre::Result<()> {
    let blogs: build_page::BlogList = build_page::get_blogs()?;

    // Replace silent error swallowing
    if Path::new("./output").exists() {
        fs::remove_dir_all("./output")?;
    }

    // Copy static files
    copy_dir::copy_dir("./assets/static", "./output")?;

    // make output dirs
    fs::create_dir_all("./output/posts")?;
    fs::create_dir_all("./output/tags")?;

    // Output all blogs.
    for blog in &blogs.blogs {
        if !blog.metadata.published {
            continue;
        }

        info!("Converting blog {} to html", blog.title);
        let blog_html = blog.to_blog_html()?;

        fs::write(format!("./output/posts/{}.html", blog.slug), blog_html)?;
    }

    output_tags_to_fs(&blogs)?;
    output_homepage_to_fs(&blogs)?;
    output_rss_to_fs(&blogs)?;

    Ok(())
}

fn output_rss_to_fs(blogs: &BlogList) -> Result<()> {
    let mut channel = ChannelBuilder::default()
        .title("Squawkykaka")
        .link("https://squawkykaka.com")
        .description("The RSS Feed for squawykaka.com")
        .build();

    // TODO fix all the .clones here
    for post in &blogs.blogs {
        let catagories = {
            let mut catagories = vec![];

            match post.metadata.tags.clone() {
                Some(tags) => {
                    for tag in tags {
                        catagories.push(Category {
                            name: tag,
                            domain: None,
                        });
                    }

                    catagories
                }
                None => {
                    catagories.push(Category {
                        name: "no_catagory".into(),
                        domain: None,
                    });

                    catagories
                }
            }
        };

        let rss_post = ItemBuilder::default()
            .title(post.title.clone())
            .author(String::from("squawkykaka@gmail.com"))
            .categories(catagories)
            .pub_date(post.metadata.date.clone())
            .content(post.contents.clone())
            .link(format!("https://squawkykaka.com/posts/{}.html", post.slug))
            .build();

        channel.items.push(rss_post);
    }

    fs::write(format!("./output/index.xml",), channel.to_string())?;

    Ok(())
    // todo!()
}

fn output_tags_to_fs(blogs: &BlogList) -> Result<()> {
    for (tag, blogs_with) in &blogs.tags {
        let posts: Vec<_> = blogs
            .blogs
            .iter()
            .filter(|blog| blogs_with.contains(&blog.slug))
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

fn output_homepage_to_fs(blogs: &BlogList) -> Result<()> {
    // The homepage template expects an object with a `blogs` field
    let ctx = json!({ "blogs": blogs.blogs });
    let contents = HANDLEBARS.render("homepage", &ctx)?;

    fs::write("./output/home.html", contents)?;

    Ok(())
}

pub fn get_blogs() -> Result<BlogList> {
    let mut blog_list = BlogList::default();

    let blog_paths = get_blog_paths()?;

    for blog_path in blog_paths {
        render_blog(blog_path, &mut blog_list)?;
    }

    Ok(blog_list)
}

fn render_blog(blog_path: PathBuf, blog_list: &mut BlogList) -> Result<()> {
    let blog_bytes = fs::read(&blog_path)?;
    let blog_contents = std::str::from_utf8(&blog_bytes)?;

    // Generate HTML
    let html_output = render_html_page_from_markdown(&blog_contents);

    // get metadata options
    let blog_metadata_string = match blog_contents.split("---").nth(1) {
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

    // TODO Chaneg to maybe add a date suffix maybe from file creation date?
    let blog_title = blog_path
        .file_name()
        .expect("Patth returned is ..")
        .to_string_lossy()
        .strip_suffix(".md")
        .expect("Path should have .md extension when making blog title")
        .to_string();
    let blog_slug = blog_title.replace(" ", "-").to_ascii_lowercase();

    let blog = Blog {
        title: blog_title.clone(),
        slug: blog_slug.clone(),
        metadata: blog_metadata,
        contents: html_output,
    };

    // Insert blog into list
    // FIXME change to not clone each time, probably using &str
    if let Some(tags) = &blog.metadata.tags {
        for tag in tags {
            blog_list
                .tags
                .entry(tag.to_string())
                .or_default()
                .insert(blog_slug.clone());
        }
    }

    dbg!(&blog);

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
