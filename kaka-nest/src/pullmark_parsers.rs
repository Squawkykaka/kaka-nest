use std::str::FromStr;

use handlebars::RenderError;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Parser, Tag, TagEnd};
use serde::Serialize;
use syntastica::{Processor, renderer::HtmlRenderer};
use syntastica_parsers::{Lang, LanguageSetImpl};
use tracing::debug;

use crate::{HANDLEBARS, TL_PROCESSOR};

#[derive(Serialize)]
struct CodeBlock {
    lang: String,
    contents: String,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum BlockquoteTypes {
    Question,
    // Aha,
}

#[derive(Serialize)]
struct BlockQuote<'a> {
    blockquote_type: Option<BlockquoteTypes>,
    contents: &'a str,
}

/// Gets every codeblock in a pullmark parser and adds syntax highlighting to the html
// ...existing code...
pub(crate) fn highlight_codeblocks<'a, I>(
    parser: I,
    processer: &'a mut Processor<'static, LanguageSetImpl>,
) -> impl Iterator<Item = Event<'a>> + 'a
where
    I: Iterator<Item = Event<'a>> + 'a,
{
    struct HighlightCodeblocks<'a, I: Iterator<Item = Event<'a>>> {
        inner: I,
        in_codeblock: bool,
        code_lang: Option<String>,
        code_buffer: String,
        processer: &'a mut Processor<'static, LanguageSetImpl>,
    }

    impl<'a, I: Iterator<Item = Event<'a>>> Iterator for HighlightCodeblocks<'a, I> {
        type Item = Event<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            while let Some(event) = self.inner.next() {
                match event {
                    Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                        self.in_codeblock = true;
                        self.code_lang = Some(lang.to_string());
                        self.code_buffer.clear();
                        continue;
                    }
                    Event::Text(text) if self.in_codeblock => {
                        self.code_buffer.push_str(&text);
                        continue;
                    }
                    Event::End(TagEnd::CodeBlock) if self.in_codeblock => {
                        self.in_codeblock = false;

                        let highlighted_code = if let Some(lang) = self.code_lang.as_deref() {
                            if let Ok(syntax) = Lang::from_str(lang) {
                                let processed =
                                    match self.processer.process(&self.code_buffer, syntax).ok() {
                                        Some(o) => o,
                                        None => {
                                            eprintln!("Highlighting code failed");
                                            std::process::exit(0);
                                        }
                                    };

                                let highlighted = syntastica::render(
                                    &processed,
                                    &mut HtmlRenderer,
                                    syntastica_themes::one::dark(),
                                );

                                // If Handlebar render is expensive, consider a simple format! here instead.
                                format_codeblock_html(&highlighted, Some(lang)).unwrap()
                            } else {
                                format_codeblock_html(&self.code_buffer.to_string(), None).unwrap()
                            }
                        } else {
                            format_codeblock_html(&self.code_buffer, None).unwrap()
                        };

                        return Some(Event::Html(highlighted_code.into()));
                    }
                    other => return Some(other),
                }
            }
            None
        }
    }

    HighlightCodeblocks {
        inner: parser,
        in_codeblock: false,
        code_lang: None,
        code_buffer: String::with_capacity(1024 * 4),
        processer,
    }
}

// ...existing code...
/// Processes blockquotes with the format:
///
/// ```md
/// > [!question]
/// > What is the meaning of life?
/// ```
pub(crate) fn format_blockquotes<'a>(
    parser: impl Iterator<Item = Event<'a>>,
) -> impl Iterator<Item = Event<'a>> {
    struct FormatBlockquotes<'a, I: Iterator<Item = Event<'a>>> {
        inner: I,
        in_blockquote: bool,
        blockquote_buffer: String,
    }

    impl<'a, I: Iterator<Item = Event<'a>>> Iterator for FormatBlockquotes<'a, I> {
        type Item = Event<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            while let Some(event) = self.inner.next() {
                match event {
                    Event::Start(Tag::BlockQuote(_)) => {
                        self.in_blockquote = true;
                        self.blockquote_buffer.clear();
                        continue;
                    }
                    Event::Text(text) if self.in_blockquote => {
                        self.blockquote_buffer.push_str(&text);
                        self.blockquote_buffer.push('\n'); // preserve line breaks
                        continue;
                    }
                    Event::End(TagEnd::BlockQuote(_)) if self.in_blockquote => {
                        self.in_blockquote = false;

                        if let Some((blockquote_string, rest)) =
                            parse_marker(&self.blockquote_buffer)
                        {
                            let blockquote_type = match blockquote_string.as_str() {
                                "question" => Some(BlockquoteTypes::Question),
                                // Add more types as needed
                                _ => None,
                            };

                            let rendered_contents = HANDLEBARS
                                .render(
                                    "blockquote",
                                    &BlockQuote {
                                        blockquote_type,
                                        contents: &rest,
                                    },
                                )
                                .expect("Failed to render blockquote");
                            return Some(Event::Html(rendered_contents.into()));
                        } else {
                            debug!("You forgot to add a type to blockquote");
                            continue;
                        };
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
        blockquote_buffer: String::with_capacity(4 * 1024),
    }
}

fn parse_marker(input: &String) -> Option<(String, String)> {
    let mut lines = input.lines();

    // Expect: line 1 == "[", line 2 starts with "!", line 3 == "]"
    if lines.next()? != "[" {
        return None;
    }

    let marker_line = lines.next()?;
    if !marker_line.starts_with('!') {
        return None;
    }
    let marker = marker_line.strip_prefix('!')?.to_string();

    if lines.next()? != "]" {
        return None;
    }

    // Collect the rest of the lines as the "value"
    let value = lines.collect::<Vec<_>>().join("\n");

    Some((marker, value))
}

fn format_codeblock_html(input: &str, lang: Option<&str>) -> Result<String, RenderError> {
    HANDLEBARS.render(
        "codeblock",
        &CodeBlock {
            lang: lang.unwrap_or(" ").to_string(),
            contents: input.to_string(),
        },
    )
}
#[cfg(test)]
mod tests {
    use pulldown_cmark::{Options, Parser};

    use crate::{
        TL_PROCESSOR,
        pullmark_parsers::{format_codeblock_html, highlight_codeblocks, parse_marker},
    };

    #[test]
    fn test_parser_marker() {
        assert_eq!(
            parse_marker(&"[\n!test\n]\nThis is super neat\n".to_string()),
            Some(("test".to_string(), "This is super neat".to_string()))
        );
    }

    #[test]
    fn test_valid_codeblock() {
        let test_input = "
```rust
fn bob() {
    println!(\"Hello Wolrd\");
}
```
        ";
        let test_output = "<div class=\"codeblock-wrapper\">\n  <p class=\"lang-label\">rust</p>\n\n  <div class=\"codeblock\">\n    <code style=\"display: block\">\n      <pre><span style=\"color:rgb(198,120,221);\">fn</span>&nbsp;<span style=\"color:rgb(97,175,239);\">bob</span><span style=\"color:rgb(132,139,152);\">(</span><span style=\"color:rgb(132,139,152);\">)</span>&nbsp;<span style=\"color:rgb(132,139,152);\">{</span><br>&nbsp;&nbsp;&nbsp;&nbsp;<span style=\"color:rgb(86,182,194);\">println</span><span style=\"color:rgb(86,182,194);\">!</span><span style=\"color:rgb(132,139,152);\">(</span><span style=\"color:rgb(152,195,121);\">\"Hello&nbsp;Wolrd\"</span><span style=\"color:rgb(132,139,152);\">)</span><span style=\"color:rgb(132,139,152);\">;</span><br><span style=\"color:rgb(132,139,152);\">}</span><br></pre>\n    </code>\n  </div>\n</div>\n";

        let mut pullmark_options = Options::empty();
        pullmark_options.insert(Options::ENABLE_WIKILINKS);
        pullmark_options.insert(Options::ENABLE_STRIKETHROUGH);
        pullmark_options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        pullmark_options.insert(Options::ENABLE_TASKLISTS);

        let html_output = TL_PROCESSOR.with_borrow_mut(|processer| {
            let parser = Parser::new_ext(test_input, pullmark_options);
            let parser = highlight_codeblocks(parser, processer);

            let mut html_output = String::new();
            pulldown_cmark::html::push_html(&mut html_output, parser);

            html_output
        });

        assert_eq!(html_output, test_output)
    }

    #[test]
    fn test_no_specified_lang_codeblock() {
        let test_input = "
```
fn bob() {
    println!(\"Hello Wolrd\");
}
```
        ";
        let test_output = "<div class=\"codeblock-wrapper\">\n  <p class=\"lang-label\"></p>\n\n  <div class=\"codeblock\">\n    <code style=\"display: block\">\n      <pre>fn bob() {\n    println!(\"Hello Wolrd\");\n}\n</pre>\n    </code>\n  </div>\n</div>\n";

        let mut pullmark_options = Options::empty();
        pullmark_options.insert(Options::ENABLE_WIKILINKS);
        pullmark_options.insert(Options::ENABLE_STRIKETHROUGH);
        pullmark_options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        pullmark_options.insert(Options::ENABLE_TASKLISTS);

        let html_output = TL_PROCESSOR.with_borrow_mut(|processer| {
            let parser = Parser::new_ext(test_input, pullmark_options);
            let parser = highlight_codeblocks(parser, processer);

            let mut html_output = String::new();
            pulldown_cmark::html::push_html(&mut html_output, parser);

            html_output
        });

        assert_eq!(html_output, test_output)
    }
}
