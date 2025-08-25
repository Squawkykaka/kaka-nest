use std::{
    collections::{HashMap, HashSet},
    fs::{self},
    path::{Path, PathBuf},
};

use color_eyre::eyre::Result;
use lol_html::{HtmlRewriter, Settings, element};
use pulldown_cmark::{Options, Parser};
use rss::{Category, ChannelBuilder, ItemBuilder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{Level, debug, info, span, trace};

use crate::{
    HANDLEBARS, TL_PROCESSOR, build_page,
    pullmark_parsers::{format_blockquotes, highlight_codeblocks},
    util::get_blog_paths,
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

pub fn create_blogs_on_system() -> color_eyre::eyre::Result<()> {
    let blogs: build_page::BlogList = build_page::get_blogs()?;

    // Replace silent error swallowing
    trace!("Deleting output directory");
    if Path::new("./output").exists() {
        fs::remove_dir_all("./output")?;
    }

    // Copy static files
    info!("Copying static files");
    copy_dir::copy_dir("./assets/static", "./output")?;

    // Copy blog images
    {
        let span = span!(Level::INFO, "copy blog images");
        let _enter = span.enter();

        for image in fs::read_dir("./assets/blog/images")? {
            match image {
                Ok(image) => {
                    if image.file_type()?.is_file() {
                        // dbg!(image);
                        debug!(file = %image.path().display(), "copying image");
                        let ostr_filename = image.file_name();
                        let image_name = ostr_filename
                            .to_str()
                            .expect("Failed to convert OsStr to string");

                        fs::copy(image.path(), format!("./output/images/{image_name}"))?;
                    }
                }
                Err(e) => {
                    tracing::error!(?e);
                }
            }
        }
    }

    // make output dirs
    fs::create_dir_all("./output/posts")?;
    fs::create_dir_all("./output/tags")?;

    // Output all blogs.
    {
        let span = span!(Level::INFO, "output blogs");
        let _enter = span.enter();

        for blog in &blogs.blogs {
            if !blog.metadata.published {
                continue;
            }

            let span = tracing::span!(Level::INFO, "render blog", blog = blog.title);
            let _enter = span.enter();

            info!("converting to html");
            let blog_html = blog.to_blog_html()?;

            debug!("writing to filesytem");
            fs::write(format!("./output/posts/{}.html", blog.slug), blog_html)?;
        }

        output_tags_to_fs(&blogs)?;
        output_homepage_to_fs(&blogs)?;
        output_rss_to_fs(&blogs)?;
    }

    Ok(())
}

fn output_rss_to_fs(blogs: &BlogList) -> Result<()> {
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

        let catagories = {
            let mut catagories = vec![];

            if let Some(tags) = post.metadata.tags.clone() {
                for tag in tags {
                    debug!(tag = tag, "new catagory");
                    catagories.push(Category {
                        name: tag,
                        domain: None,
                    });
                }

                catagories
            } else {
                debug!("No catagorys found");
                catagories.push(Category {
                    name: "no_catagory".into(),
                    domain: None,
                });

                catagories
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

        info!("finished post");

        channel.items.push(rss_post);
    }

    debug!("writing rss to fs");
    fs::write("./output/index.xml", channel.to_string())?;

    Ok(())
    // todo!()
}

fn output_tags_to_fs(blogs: &BlogList) -> Result<()> {
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
        fs::write(format!("./output/tags/{stripped_tag}.html"), contents)?;
    }

    Ok(())
}

fn output_homepage_to_fs(blogs: &BlogList) -> Result<()> {
    let span = span!(Level::DEBUG, "output homepage");
    let _enter = span.enter();
    info!("outputting homepage");

    // The homepage template expects an object with a `blogs` field
    let ctx = json!({ "blogs": blogs.blogs });
    let contents = HANDLEBARS.render("homepage", &ctx)?;

    fs::write("./output/home.html", contents)?;

    Ok(())
}

pub fn get_blogs() -> Result<BlogList> {
    let mut blog_list = BlogList::default();

    info!("getting blogs");
    let blog_paths = get_blog_paths()?;

    for blog_path in blog_paths {
        render_blog(&blog_path, &mut blog_list)?;
    }

    Ok(blog_list)
}

fn render_blog(blog_path: &PathBuf, blog_list: &mut BlogList) -> Result<()> {
    let span = span!(Level::DEBUG, "render post", post = %blog_path.as_path().display());
    let _enter = span.enter();

    trace!("reading post from fs");
    let blog_bytes = fs::read(blog_path)?;
    let blog_contents = std::str::from_utf8(&blog_bytes)?;

    // Generate HTML
    let html_output = render_html_page_from_markdown(blog_contents);

    // get metadata options
    let blog_metadata = {
        debug!("extracting metadata from file");
        let Some(blog_metadata_string) = blog_contents.split("---").nth(1) else {
            return Err(color_eyre::eyre::eyre!("Didnt include metadata for file"));
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

        blog_metadata
    };

    // TODO Chaneg to maybe add a date suffix maybe from file creation date?
    debug!("getting post title");
    let blog_title = blog_path
        .file_name()
        .expect("Path returned is ..")
        .to_str()
        .unwrap()
        .strip_suffix(".md")
        .expect("Path should have .md extension when making blog title");

    let blog_slug = blog_title.replace(' ', "-").to_ascii_lowercase();

    let blog = Blog {
        title: String::from(blog_title),
        slug: blog_slug.clone(),
        metadata: blog_metadata,
        contents: html_output,
    };

    // Insert blog into list
    // FIXME change to not clone each time, probably using &str

    if let Some(tags) = &blog.metadata.tags {
        let span = span!(Level::DEBUG, "insert tags");
        let _enter = span.enter();

        for tag in tags {
            debug!(tag = tag, "found tag");
            blog_list
                .tags
                .entry(tag.to_string())
                .or_default()
                .insert(blog_slug.clone());
        }
    }

    blog_list.blogs.push(blog);

    Ok(())
}

fn render_html_page_from_markdown(input: &str) -> String {
    let span = span!(Level::INFO, "pullmark parsing");
    let _enter = span.enter();

    let mut pullmark_options = Options::empty();
    pullmark_options.insert(Options::ENABLE_WIKILINKS);
    pullmark_options.insert(Options::ENABLE_STRIKETHROUGH);
    pullmark_options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    pullmark_options.insert(Options::ENABLE_TASKLISTS);

    let html_output = TL_PROCESSOR.with_borrow_mut(|processer| {
        info!("created parser");
        let parser = Parser::new_ext(input, pullmark_options);
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
