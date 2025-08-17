use std::str::FromStr;

use handlebars::Handlebars;
use html_escape::encode_text;
use lazy_static::lazy_static;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use syntastica::{Processor, language_set, renderer::HtmlRenderer};
use syntastica_parsers::{Lang, LanguageSetImpl};

use crate::{HANDLEBARS, SYNTAX_PROCESSER};

#[derive(Serialize)]
struct CodeBlock {
    lang: String,
    contents: String,
}

/// Gets every codeblock in a pullmark parser and adds syntax highlighting to the html
pub(crate) fn highlight_codeblocks(parser: Parser<'_>) -> impl Iterator<Item = Event<'_>> {
    struct HighlightBlockquotes<'a, I: Iterator<Item = Event<'a>>> {
        inner: I,
        in_codeblock: bool,
        code_lang: Option<CowStr<'a>>,
        code_buffer: String,
    }

    impl<'a, I: Iterator<Item = Event<'a>>> Iterator for HighlightBlockquotes<'a, I> {
        type Item = Event<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            while let Some(event) = self.inner.next() {
                match event {
                    Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                        self.in_codeblock = true;
                        self.code_lang = Some(lang);
                        self.code_buffer.clear();

                        return Some(Event::Html("".into()));
                    }
                    Event::Text(text) if self.in_codeblock => {
                        // Collect code lines
                        self.code_buffer.push_str(&text);
                        return Some(Event::Html("".into()));
                    }
                    Event::End(TagEnd::CodeBlock) if self.in_codeblock => {
                        self.in_codeblock = false;

                        let highlighted_code = if let Some(lang) = self.code_lang.as_deref() {
                            if let Ok(syntax) = Lang::from_str(lang) {
                                let highlighted_code = SYNTAX_PROCESSER
                                    .lock()
                                    .expect("Failed to lock the syntax processer")
                                    .process(&self.code_buffer, syntax)
                                    .unwrap();

                                let highlighted_code = syntastica::render(
                                    &highlighted_code,
                                    &mut HtmlRenderer,
                                    syntastica_themes::one::dark(),
                                );

                                let rendered_html = HANDLEBARS
                                    .render(
                                        "codeblock",
                                        &CodeBlock {
                                            lang: lang.to_string(),
                                            contents: highlighted_code,
                                        },
                                    )
                                    .expect("Failed to render html codeblock");

                                rendered_html
                            } else {
                                // If language parsing fails, just escape the code
                                html_escape::encode_text(&self.code_buffer).to_string()
                            }
                        } else {
                            // If no language is specified, just escape the code
                            html_escape::encode_text(&self.code_buffer).to_string()
                        };

                        return Some(Event::Html(highlighted_code.into()));
                    }

                    _ => return Some(event),
                }
            }
            None
        }
    }

    HighlightBlockquotes {
        inner: parser,
        in_codeblock: false,
        code_buffer: String::new(),
        code_lang: None,
    }
}

pub(crate) fn format_codeblocks<'a>(
    parser: impl Iterator<Item = Event<'a>>,
) -> impl Iterator<Item = Event<'a>> {
    struct FormatBlockquotes<'a, I: Iterator<Item = Event<'a>>> {
        inner: I,
        in_blockquote: bool,
        code_buffer: Vec<String>,
    }

    impl<'a, I: Iterator<Item = Event<'a>>> Iterator for FormatBlockquotes<'a, I> {
        type Item = Event<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            while let Some(event) = self.inner.next() {
                match event {
                    Event::Start(Tag::BlockQuote(_)) => {
                        self.in_blockquote = true;
                        self.code_buffer.clear();
                        return Some(Event::Html("".into()));
                    }
                    Event::Text(ref text) if self.in_blockquote => {
                        self.code_buffer.push(text.to_string());
                        return Some(Event::Html("".into()));
                    }
                    Event::End(TagEnd::BlockQuote(_)) if self.in_blockquote => {
                        self.in_blockquote = false;
                        // Example: parse blockquote type from first line
                        let blockquote_type = if let Some(first) = self.code_buffer.get(1) {
                            let without_first_char = &first[1..]; // skips the first character

                            match without_first_char {
                                "question" => BlockquoteTypes::Question,
                                // Add more types as needed
                                _ => panic!("Unknown blockquote type"),
                            }
                        } else {
                            panic!("Empty blockquote");
                        };
                        let contents = self.code_buffer[1..].join("\n");
                        let rendered_contents = HANDLEBARS
                            .render(
                                "blockquote",
                                &BlockQuote {
                                    blockquote_type,
                                    contents,
                                },
                            )
                            .expect("Failed to render blockquote");
                        return Some(Event::Html(rendered_contents.into()));
                    }
                    _ => return Some(event),
                }
            }
            None
        }
    }

    FormatBlockquotes {
        inner: parser,
        in_blockquote: false,
        code_buffer: Vec::new(),
    }
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
