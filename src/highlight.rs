use std::str::FromStr;

use html_escape::encode_text;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use syntastica::renderer::HtmlRenderer;
use syntastica_parsers::{Lang, LanguageSetImpl};

/// Gets every codeblock in a pullmark parser and adds syntax highlighting to the html
pub(crate) fn highlight_codeblocks(parser: Parser<'_>) -> impl Iterator<Item = Event<'_>> {
    let mut in_code_block = false;
    let mut code_lang: Option<String> = None;
    let mut code_buffer = String::new();

    let parser = parser.map(move |event| {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                // Entering a code block
                in_code_block = true;
                code_lang = Some(lang.to_string());
                code_buffer.clear();
                Event::Html("".into()) // suppress original
            }
            Event::Text(text) if in_code_block => {
                // Collect code lines
                code_buffer.push_str(&text);
                Event::Html("".into()) // suppress original
            }
            Event::End(TagEnd::CodeBlock) if in_code_block => {
                // Leaving code block: highlight
                in_code_block = false;
                let highlighted = if let Some(lang) = &code_lang {
                    // Add replacments for different langs, e.g. js to javascript
                    let lang = match lang.as_str() {
                        "js" => "javascript",
                        "rs" => "rust",
                        _ => lang,
                    };

                    if let Ok(syntax) = Lang::from_str(lang) {
                        let highlighted_code = syntastica::highlight(
                            &code_buffer,
                            syntax,
                            &LanguageSetImpl::new(),
                            &mut HtmlRenderer,
                            syntastica_themes::one::dark(),
                        )
                        .expect("Failed to process code block");

                        format!(
                            "<div><p>{}</p><pre><code>{}</code></pre></div>",
                            lang, highlighted_code
                        )
                    } else {
                        // Fallback: plain pre/code
                        format!("<pre><code>{}</code></pre>", encode_text(&code_buffer))
                    }
                } else {
                    // No language given
                    format!("<pre><code>{}</code></pre>", encode_text(&code_buffer))
                };

                Event::Html(highlighted.into())
            }

            _ => event,
        }
    });
    parser
}
