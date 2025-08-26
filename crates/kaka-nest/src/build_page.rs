use std::{
    collections::{HashMap, HashSet},
    fs::{self},
    path::{Path, PathBuf},
};

use fs_extra::{copy_items, dir::CopyOptions};
use lol_html::{HtmlRewriter, Settings, element};
use pulldown_cmark::{Options, Parser};
use pullmark_parsers::{TL_PROCESSOR, format_blockquotes, highlight_codeblocks};
use rss::{Category, ChannelBuilder, ItemBuilder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_utils::HANDLEBARS;
use slugify::slugify;
use tracing::{Level, debug, info, span, trace};

use crate::util::get_blog_paths;

#[derive(Debug, Serialize)]
pub struct Post {
    pub title: String,
    pub slug: String,
    pub metadata: PostMetadata,
    pub contents: String,
}
impl Post {
    pub(crate) fn to_rendered_html(&self) -> Result<String, Box<dyn std::error::Error>> {
        let rendered_string = HANDLEBARS.render("blog", self)?;

        // Replace local image links with /images/{{ image }}
        debug!("rewriting img links");
        let mut output = vec![];
        let mut rewriter = HtmlRewriter::new(
            Settings {
                element_content_handlers: vec![element!("img[src]", |el| {
                    let binding = el
                        .get_attribute("src")
                        .expect("Failed to get src attribute, this shoudlnt happen");
                    let img_name = binding.as_str();

                    if let Some(src) = el.get_attribute("src") {
                        let l = src.to_lowercase();

                        let is_absolute = l.starts_with("http://")
                            || l.starts_with("https://")
                            || l.starts_with("data:")
                            || src.starts_with('/')
                            || src.starts_with("//");

                        if !is_absolute {
                            let new_src = format!("/images/{img_name}");

                            el.set_attribute("src", &new_src)?;
                        }
                    }

                    Ok(())
                })],
                ..Settings::new()
            },
            |c: &[u8]| output.extend_from_slice(c),
        );

        rewriter.write(&rendered_string.into_bytes())?;
        rewriter.end()?;

        Ok(String::from_utf8(output)?)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostMetadata {
    pub date: String,
    pub published: bool,
    pub tags: Option<Vec<String>>,
    pub read_mins: u32,
    pub description: Option<String>,
}

/// A struct containing all currently exisiting blogs & tags
#[derive(Default, Debug)]
pub struct PostList {
    pub blogs: Vec<Post>,
    pub tags: HashMap<String, HashSet<String>>,
}

fn build_post_from_path(path: &Path) -> Result<Post, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let metadata = parse_front_matter(&content)?;
    let html = render_markdown_to_html(&content);

    let title = generate_title_from_path(path).ok_or("the file name is invalid")?;
    Ok(Post {
        title: title.into(), // or derive from metadata
        slug: slugify!(title),
        metadata,
        contents: html,
    })
}

fn generate_title_from_path(path: &Path) -> Option<&str> {
    let file_name = path.file_stem()?.to_str()?;

    Some(file_name)
}

fn render_markdown_to_html(content: &str) -> String {
    let span = span!(Level::INFO, "pullmark parsing");
    let _enter = span.enter();

    let mut pullmark_options = Options::empty();
    pullmark_options.insert(Options::ENABLE_WIKILINKS);
    pullmark_options.insert(Options::ENABLE_STRIKETHROUGH);
    pullmark_options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    pullmark_options.insert(Options::ENABLE_TASKLISTS);
    pullmark_options.insert(Options::ENABLE_TABLES);

    let html_output = TL_PROCESSOR.with_borrow_mut(|processer| {
        debug!("created parser");
        let parser = Parser::new_ext(content, pullmark_options);
        debug!("highlighting codeblocks");
        let parser = highlight_codeblocks(parser, processer);
        debug!("formatting blockquotes");
        let parser = format_blockquotes(parser);

        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);

        html_output
    });

    debug!("finished parsing into html");
    html_output
}

fn parse_front_matter(content: &str) -> Result<PostMetadata, Box<dyn std::error::Error>> {
    debug!("extracting metadata from file");

    let Some(blog_metadata_string) = content.split("---").nth(1) else {
        return Err("Didnt include metadata for file".into());
    };

    let mut blog_metadata: PostMetadata = serde_yaml::from_str(blog_metadata_string)?;

    // Remove '#' prefix from each tag if present
    if let Some(tags) = &mut blog_metadata.tags {
        for tag in tags.iter_mut() {
            if let Some(stripped) = tag.strip_prefix('#') {
                *tag = stripped.to_string();
            }
        }
    }

    Ok(blog_metadata)
}

fn build_blog_list(blog_paths: &[PathBuf]) -> Result<PostList, Box<dyn std::error::Error>> {
    let mut blog_list = PostList::default();

    for path in blog_paths {
        let blog = build_post_from_path(path)?;
        if let Some(tags) = &blog.metadata.tags {
            for tag in tags {
                blog_list
                    .tags
                    .entry(tag.clone())
                    .or_default()
                    .insert(blog.slug.clone());
            }
        }

        if !blog.metadata.published {
            continue;
        }

        blog_list.blogs.push(blog);
    }

    Ok(blog_list)
}

/// This function reads all input files from the operating systemm,
/// builds the blogs and Copys the images and static assets to output directory
///
/// # Errors
/// - Deleting output directory
/// - Copying assets
/// - Converting `OsStr` to string
/// - Reading input dir
pub fn create_blog_on_system() -> Result<(), Box<dyn std::error::Error>> {
    let blog_paths = get_blog_paths()?;
    let posts = build_blog_list(&blog_paths)?;

    trace!("Deleting output directory");
    if Path::new("./output").exists() {
        fs::remove_dir_all("./output")?;
    }
    // Create the base output directory first, as threads will try to write to it.
    fs::create_dir("./output")?;

    // --- Start of Copying ---
    info!("Copying static files and images");
    let static_src = "./assets/static";
    let images_src = "./assets/blog/images";
    let static_dest = "./output";
    let images_dest = "./output/images";

    // -- Copy Static Files --
    let mut options = CopyOptions::new();
    options.overwrite = true;
    let paths_to_copy: Vec<_> = fs::read_dir(static_src)?
        .filter_map(Result::ok) // Ignore any read errors for individual entries
        .map(|entry| entry.path())
        .collect();
    copy_items(&paths_to_copy, static_dest, &options)?;

    // -- Copy image files --
    fs::create_dir_all(images_dest)?;
    let paths_to_copy: Vec<_> = fs::read_dir(images_src)?
        .filter_map(Result::ok) // Ignore any read errors for individual entries
        .map(|entry| entry.path())
        .collect();
    copy_items(&paths_to_copy, images_dest, &options)?;
    info!("Finished copying blog images");

    // --- End of Copying ---
    info!("All file copying complete.");

    // Create remaining output directories
    fs::create_dir_all("./output/posts")?;
    fs::create_dir_all("./output/tags")?;

    // Output all generated files.
    {
        let span = span!(Level::INFO, "output generated files");
        let _enter = span.enter();

        for blog in &posts.blogs {
            let span = tracing::span!(Level::INFO, "render blog", blog = blog.title);
            let _enter = span.enter();

            info!("converting to html");
            let blog_html = blog.to_rendered_html()?;

            debug!("writing to filesytem");
            fs::create_dir(format!("./output/posts/{}", blog.slug))?;
            fs::write(
                format!("./output/posts/{}/index.html", blog.slug),
                blog_html,
            )?;
        }

        output_tags_to_fs(&posts)?;
        output_homepage_to_fs(&posts)?;
        output_rss_to_fs(&posts)?;
    }

    Ok(())
}

fn output_rss_to_fs(blogs: &PostList) -> Result<(), Box<dyn std::error::Error>> {
    let span = span!(Level::INFO, "output rss");
    let _enter = span.enter();

    info!("creating channel");
    let mut channel = ChannelBuilder::default()
        .title("Squawkykaka")
        .link("https://squawkykaka.com")
        .description("The RSS Feed for squawykaka.com")
        .build();

    // TODO fix all the .clones here
    for post in &blogs.blogs {
        let span = span!(Level::INFO, "post catagories", post = post.title);
        let _enter = span.enter();

        let catagories: Vec<Category> = match post.metadata.tags.as_ref() {
            Some(tags) => tags
                .iter()
                .map(|name| Category {
                    name: name.clone(),
                    domain: None,
                })
                .collect(),
            None => vec![],
        };

        let rss_post = ItemBuilder::default()
            .title(post.title.clone())
            .author(String::from("squawkykaka@gmail.com"))
            .categories(catagories)
            .pub_date(post.metadata.date.clone())
            .content(post.contents.clone())
            .link(format!(
                "https://squawkykaka.com/posts/{}/index.html",
                post.slug
            ))
            .build();

        info!("finished post");

        channel.items.push(rss_post);
    }

    debug!("writing rss to fs");
    fs::write("./output/index.xml", channel.to_string())?;

    Ok(())
    // todo!()
}

fn output_tags_to_fs(blogs: &PostList) -> Result<(), Box<dyn std::error::Error>> {
    let span = span!(Level::DEBUG, "output tags");
    let _enter = span.enter();

    info!("outputting tags");
    for (tag, blogs_with) in &blogs.tags {
        let span = span!(Level::DEBUG, "tag", name = tag);
        let _enter = span.enter();

        debug!("filtering posts");
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

        trace!("stripping # prefix");
        let stripped_tag = match tag.strip_prefix("#") {
            Some(tag) => tag,
            None => tag.as_str(),
        };

        debug!("writing to fs");
        fs::create_dir(format!("./output/tags/{stripped_tag}"))?;
        fs::write(format!("./output/tags/{stripped_tag}/index.html"), contents)?;
    }

    Ok(())
}

fn output_homepage_to_fs(blogs: &PostList) -> Result<(), Box<dyn std::error::Error>> {
    let span = span!(Level::DEBUG, "output homepage");
    let _enter = span.enter();
    info!("outputting homepage");

    // The homepage template expects an object with a `blogs` field
    let ctx = json!({ "blogs": blogs.blogs });
    let contents = HANDLEBARS.render("homepage", &ctx)?;

    fs::write("./output/index.html", contents)?;

    Ok(())
}
