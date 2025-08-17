use std::str::FromStr;

use handlebars::Handlebars;
use html_escape::encode_text;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use syntastica::renderer::HtmlRenderer;
use syntastica_parsers::{Lang, LanguageSetImpl};

use crate::HANDLEBARS;

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

pub(crate) fn format_codeblocks<'a>(
    parser: impl Iterator<Item = Event<'a>>,
) -> impl Iterator<Item = Event<'a>> {
    let mut in_blockquote = false;
    let mut code_buffer: Vec<String> = Vec::new();

    let parser = parser.map(move |event| {
        return match event {
            Event::Start(Tag::BlockQuote(_)) => {
                in_blockquote = true;

                Event::Html("".into())
            }
            Event::Text(text) if in_blockquote => {
                // FIXME chanege to maybe not use clone
                code_buffer.push(text.clone().to_string());

                // event
                Event::Html("".into())
            }
            Event::End(TagEnd::BlockQuote(_)) if in_blockquote => {
                // Leaving code block: highlight
                in_blockquote = false;

                let parsed_codeblock = parse_blockquotes(&mut code_buffer, &HANDLEBARS);

                Event::Html(parsed_codeblock.into())
            }
            _ => event,
        };
    });

    parser
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum BlockquoteTypes {
    // Confused,
    Question,
    // Aha,
}

#[derive(Serialize)]
struct BlockQuote {
    blockquote_type: BlockquoteTypes,
    contents: String,
}

fn parse_blockquotes(input: &mut Vec<String>, handlebars: &Handlebars) -> String {
    let (blockquote_string, contents) = input.split_at_mut(3);

    // Get the type of blockquote
    let blockquote_string = blockquote_string[1].split_off(1);

    let blockquote_type = match blockquote_string.as_str() {
        // "confused" => BlockquoteTypes::Confused,
        "question" => BlockquoteTypes::Question,
        _ => panic!("Unknown blockquote type"),
    };
    // Handlebars load the template and place stuff in
    let rendered_contents = handlebars
        .render(
            "blockquote",
            &BlockQuote {
                blockquote_type,
                contents: contents.join("\n"),
            },
        )
        .expect("Failed to render blockquote");

    rendered_contents

    // Return

    // .to_string()
}
