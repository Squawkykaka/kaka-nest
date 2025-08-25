use handlebars::Handlebars;
use std::sync::LazyLock;
use syntastica::Processor;
use syntastica_parsers::LanguageSetImpl;

pub mod build_page;
pub mod pullmark_parsers;
pub mod util;

// lazy_static! {
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

pub static LEAKED_LANGSET: LazyLock<&'static LanguageSetImpl> =
    LazyLock::new(|| Box::leak(Box::new(LanguageSetImpl::new())));
// }

thread_local! {
    pub static TL_PROCESSOR: std::cell::RefCell<Processor<'static, LanguageSetImpl>> =
        std::cell::RefCell::new(Processor::new(*LEAKED_LANGSET));
}
