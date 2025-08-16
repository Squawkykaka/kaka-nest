use std::{
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{Ok, Result};
use pulldown_cmark::{Options, Parser};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub(crate) struct Blog {
    pub id: u32,
    pub metadata: BlogMetadata,
    pub contents: String,
}
impl Blog {
    pub(crate) fn to_blog_html(&self, handlebars: &handlebars::Handlebars) -> Result<String> {
        let rendered_string = handlebars.render("blog", self)?;

        Ok(rendered_string)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct BlogMetadata {
    pub date: String,
    pub title: String,
    pub published: bool,
    pub tags: Vec<String>,
}

/// A struct containing all currently exisiting blogs & tags
#[derive(Default, Debug)]
pub(crate) struct BlogList {
    pub blogs: Vec<Blog>,
    pub existing_tags: Vec<String>,
}

pub fn get_blogs() -> Result<BlogList> {
    let mut blog_list = BlogList::default();
    let mut id: u32 = 0;

    let mut pullmark_options = Options::empty();
    pullmark_options.insert(Options::ENABLE_WIKILINKS);
    pullmark_options.insert(Options::ENABLE_STRIKETHROUGH);
    pullmark_options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

    let blog_paths = get_blog_paths()?;

    for blog_path in blog_paths {
        let blog_bytes = fs::read(&blog_path)?;
        let blog_contents = str::from_utf8(&blog_bytes)?;

        // Generate HTML
        let parser = Parser::new_ext(&blog_contents, pullmark_options);
        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);

        // get yaml options
        let blog_metadata_string = match blog_contents.split("---").nth(1) {
            Some(blog_metadata) => blog_metadata,
            None => {
                return Err(color_eyre::eyre::eyre!(
                    "You didnt include metadata for file: {}",
                    blog_path.to_str().unwrap()
                ));
            }
        };

        let blog_metadata: BlogMetadata = serde_yaml::from_str(blog_metadata_string)?;

        // Find better way not using clone
        blog_list
            .existing_tags
            .append(&mut blog_metadata.tags.clone());
        blog_list.blogs.push(Blog {
            id,
            metadata: blog_metadata,
            contents: html_output,
        });

        id += 1;
    }

    blog_list
        .existing_tags
        .sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
    blog_list
        .existing_tags
        .dedup_by(|a, b| a.eq_ignore_ascii_case(b));

    Ok(blog_list)
}

fn get_blog_paths() -> Result<Vec<PathBuf>> {
    let path = Path::new("./assets/blog");

    let files: Vec<PathBuf> = visit_dir(path)?
        .into_iter()
        .filter(|file| file.extension().unwrap_or_default() == "md")
        .collect();

    Ok(files)
}

fn visit_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let mut new_files = visit_dir(path.as_path())?;

            files.append(&mut new_files);
        } else {
            files.push(path);
        }
    }

    Ok(files)
}
