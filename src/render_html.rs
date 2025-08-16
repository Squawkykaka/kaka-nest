use color_eyre::eyre::Result;

use crate::get_markdown::Blog;

impl Blog {
    pub(crate) fn to_blog_html(&self, handlebars: &handlebars::Handlebars) -> Result<String> {
        // let html_template = fs::read_to_string()?;

        let rendered_string = handlebars.render("blog", self)?;

        Ok(rendered_string)
    }
}
