use crate::util::get_blog_paths;
use fs_extra::{copy_items, dir::CopyOptions};
use lol_html::{HtmlRewriter, Settings, element};
use pulldown_cmark::{Options, Parser};
use pullmark_parsers::{TL_PROCESSOR, format_blockquotes, highlight_codeblocks};
use rss::{Category, ChannelBuilder, ItemBuilder};
use search_generation::compile_wasm_crate;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_utils::HANDLEBARS;
use slugify::slugify;
use std::{
    collections::{HashMap, HashSet},
    fs::{self},
    path::{Path, PathBuf},
};
use tempdir::TempDir;
use tinysearch::{Post as PostTrait, TinySearch};
use tracing::{Level, debug, info, span, trace};

#[derive(Debug, Serialize, Clone)]
pub struct BlogPost {
    pub title: String,
    pub url: String,
    pub slug: String,
    pub metadata: PostMetadata,
    pub contents: String,
    pub rendered_contents: String,
}

impl PostTrait for BlogPost {
    fn title(&self) -> &str {
        &self.title
    }

    fn url(&self) -> &str {
        &self.url
    }

    fn body(&self) -> Option<&str> {
        Some(&self.contents)
    }

    fn meta(&self) -> HashMap<String, String> {
        let mut meta = HashMap::new();
        if let Some(tags) = &self.metadata.tags {
            // Join all tags into a single string so they are all indexed
            // under the "tags" key (plural).
            meta.insert("tags".to_string(), tags.join(" "));
        }
        meta
    }
}

impl BlogPost {
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

    fn from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let metadata = PostMetadata::from_str(&content)?;
        let html = render_markdown_to_html(&content);

        let title = generate_title_from_path(path).ok_or("the file name is invalid")?;
        let slug = slugify!(title);
        Ok(Self {
            title: title.into(), // or derive from metadata
            url: format!("/posts/{}", &slug),
            slug,
            metadata,
            contents: content,
            rendered_contents: html,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostMetadata {
    pub date: String,
    pub published: bool,
    pub tags: Option<Vec<String>>,
    pub read_mins: u32,
    pub description: Option<String>,
}

impl PostMetadata {
    fn from_str(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
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
}

/// A struct containing all currently exisiting blogs & tags
#[derive(Debug)]
pub struct PostList {
    pub blogs: Vec<BlogPost>,
    pub tags: HashMap<String, HashSet<String>>,

    tempdir: TempDir,
}

impl PostList {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let tempdir = TempDir::new("kaka-nest")?;

        Ok(Self {
            tempdir,
            blogs: vec![],
            tags: HashMap::new(),
        })
    }
    pub fn from_paths(blog_paths: &[PathBuf]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut blog_list = PostList::new()?;

        for path in blog_paths {
            let blog = BlogPost::from_path(path)?;
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

    pub fn output_to_system(build_index: bool) -> Result<(), Box<dyn std::error::Error>> {
        let blog_paths = get_blog_paths()?;
        let posts = PostList::from_paths(&blog_paths)?;

        trace!("Deleting output directory");
        if Path::new("./output").exists() {
            fs::remove_dir_all("./output")?;
        }
        // Create the base output directory first, as threads will try to write to it.
        fs::create_dir("./output")?;

        // --- Start of Copying ---
        info!("Copying static files and images");
        let static_src = Path::new("./assets/static");
        let images_src = Path::new("./assets/blog/images");
        let static_dest = posts.tempdir.path();
        let images_dest = static_dest.join("images");

        let mut options = CopyOptions::new();
        options.overwrite = true;

        copy_dir_contents(static_src, static_dest, &options)?;

        fs::create_dir_all(&images_dest)?; // Ensure destination exists
        copy_dir_contents(images_src, &images_dest, &options)?;

        info!("Finished copying static files and images");

        // Create remaining output directories
        fs::create_dir_all(static_dest.join("posts"))?;
        fs::create_dir_all(static_dest.join("tags"))?;
        fs::create_dir_all(static_dest.join("wasm-pkg"))?;

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
                fs::create_dir(static_dest.join("posts").join(&blog.slug))?;
                fs::write(
                    static_dest
                        .join("posts")
                        .join(&blog.slug)
                        .join("index.html"),
                    blog_html,
                )?;
            }

            posts.output_tags()?;
            posts.output_homepage()?;
            posts.output_rss()?;
        }

        // --- Create search index ---
        if build_index {
            info!("Creating search index");
            let index = TinySearch::new().build_and_serialize_index(&posts.blogs)?;
            compile_wasm_crate(index, static_dest.join("wasm-pkg").as_path())?;
        }

        // Copy all items from tempdir to output using copy_items
        let tempdir_items: Vec<_> = fs::read_dir(posts.tempdir.path())?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        copy_items(&tempdir_items, "./output", &options)?;

        Ok(())
    }

    fn output_tags(&self) -> Result<(), Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "output tags");
        let _enter = span.enter();

        info!("outputting tags");

        for (tag, blogs_with) in &self.tags {
            let span = span!(Level::DEBUG, "tag", name = tag);
            let _enter = span.enter();

            debug!("filtering posts");
            let posts: Vec<_> = self
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
            fs::create_dir(self.tempdir.path().join("tags").join(stripped_tag))?;
            fs::write(
                self.tempdir
                    .path()
                    .join("tags")
                    .join(stripped_tag)
                    .join("index.html"),
                contents,
            )?;
        }

        Ok(())
    }

    fn output_rss(&self) -> Result<(), Box<dyn std::error::Error>> {
        let span = span!(Level::INFO, "output rss");
        let _enter = span.enter();

        info!("creating channel");
        let mut channel = ChannelBuilder::default()
            .title("Squawkykaka")
            .link("https://squawkykaka.com")
            .description("The RSS Feed for squawykaka.com")
            .build();

        // TODO fix all the .clones here
        for post in &self.blogs {
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
                .content(match post.metadata.description.clone() {
                    Some(d) => d,
                    None => "No description set.".to_string(),
                })
                .link(format!(
                    "https://squawkykaka.com/posts/{}/index.html",
                    post.slug
                ))
                .build();

            info!("finished post");

            channel.items.push(rss_post);
        }

        debug!("writing rss to fs");
        fs::write(self.tempdir.path().join("index.xml"), channel.to_string())?;

        Ok(())
    }

    fn output_homepage(&self) -> Result<(), Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "output homepage");
        let _enter = span.enter();
        info!("outputting homepage");

        // The homepage template expects an object with a `blogs` field
        let ctx = json!({ "blogs": self.blogs });
        let contents = HANDLEBARS.render("homepage", &ctx)?;

        fs::write(self.tempdir.path().join("index.html"), contents)?;

        Ok(())
    }
}

// General utility functions

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

fn copy_dir_contents(
    src: &Path,
    dest: &Path,
    options: &CopyOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    if !src.exists() {
        return Ok(()); // Or return an error if the source dir is required
    }
    let paths_to_copy: Vec<_> = fs::read_dir(src)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    fs_extra::copy_items(&paths_to_copy, dest, options)?;
    Ok(())
}
