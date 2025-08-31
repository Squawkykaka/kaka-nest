use std::sync::LazyLock;

use handlebars::Handlebars;

pub static HANDLEBARS: LazyLock<Handlebars<'static>> = LazyLock::new(|| {
    let mut handlebars = handlebars::Handlebars::new();

    // Register partials
    handlebars
        .register_template_file("navbar", "./assets/templates/navbar.html")
        .unwrap();
    handlebars
        .register_template_file("styles", "./assets/templates/styles.html")
        .unwrap();

    // Register templates
    handlebars
        .register_template_file("blog", "./assets/templates/blog.html")
        .unwrap();
    handlebars
        .register_template_file("homepage", "./assets/templates/homepage.html")
        .unwrap();
    handlebars
        .register_template_file("blockquote", "./assets/templates/modules/blockquote.html")
        .unwrap();
    handlebars
        .register_template_file("codeblock", "./assets/templates/modules/codeblock.html")
        .unwrap();
    handlebars
        .register_template_file("tag_page", "./assets/templates/tag_page.html")
        .unwrap();

    handlebars
});
