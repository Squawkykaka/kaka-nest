---
date: 2025-08-16
published: true
read_mins: 5
tags:
  - "#rust"
  - "#blog-software"
description: This post describes how i added automatic syntax highlighting to my blog, and the future stuff i will add
---

I'm making my own blog system, which allows me to write my posts in [Obsidian](https://obsidian.md) and then it transforms them into html documents. I am writing it is [Rust](https://www.rust-lang.org/), the best language of all time. Currently its barely functional and missing a lot of basic features like serving the posts, here are some features im planning on adding:

---

## Urgent

- [x] fix huge performance regression with a lot of codeblocks

## Shiny

- [ ] replace handlebars with tera, and improve the templating
  - [ ] add helper functions
- [ ] improve error handling
  - [ ] add warning for invalid posts, and languages without tree-sitter parsers
- [?] A theme to my website, making it look better than just html
- [ ] Adding a sql database, pushing new posts and only parsing new ones
- [x] Little "mini avatars", which can show different expressions
- [x] tags list, where you can see what posts have what tags
- [ ] A server where it uploads to something like cloudflare pages so people can view my site.
- [ ] A searching system and tagging system, so people can find posts based on tags.
- [ ] Live reloading the websites html, whenever changes happen
- [ ] Add a way to generate a new post easily, and admin options when inside the website.
- [x] Use a better font, like [Iosevka](https://github.com/be5invis/Iosevka)
- [ ] add a config file, so other people can use easily.
- [ ] improve how i add modifications as the current implementation is quite bad

This list will grow over time, but i currently just implemented syntax highlighting using the crate [syntastica](https://crates.io/crates/syntastica) which allows you to easily parse and render code using [tree sitter](https://tree-sitter.github.io/tree-sitter/). to do this i needed to decrypt the inner working of [pulldown_cmark](https://crates.io/crates/pulldown_cmark), which converts markdown to html, and does the bulk of the work.

Pulldown works using _(to my understanding at least)_ on an event system, where each part of the markdown file is broken up and turned into an event, which then you can iterate over using iterators, like the following

```rust
let parser = parser.map(move |event| {
	match event {
		Event::Start(Tag::CodeBlock) => { /* Gets the start of a codeblock */ }
		// Just pass through every other event
		_ => event,
	}
});
```

> [!question]
> Thats what they thought... mwahahahaha

Yeah? Well it took me **2** hours to decyphyr the system to be able to insert the syntax highlighting.

This basic system expands with `Event::End`, so i ended up having to keep track of _where_ the code block started, trap all the text in-between until i reached the end event, and add syntax highlighting there.

```rust
let mut in_code_block = false;
// Stores all the text inside the codeblock
let mut code_buffer = String::new();

// Have to use 'move' for some reason
// its out of my understanding as im still learning
let parser = parser.map(move |event| {
	match event {
		Event::Start(Tag::CodeBlock) => {
			// Start tracking text
			in_code_block = true;
			// Clean the code block buffer,
			// since there might have been a code block before
			code_buffer.clear();
		}
		Event::Text(text) if in_code_block => {
			// Collect the text
			code_buffer.push_str(&text);
		}
		Event::End(TagEnd::CodeBlock) if in_code_block => {
			// Leaving code block, so stop tracking text
			in_code_block = false;

			// formatting code...
			// (cut...)
		}

		_ => event,
	}
});
```

The formatting code itself? that is an abomination, and i know theres probably a much better way of achieving it. **BUT THATS FOR FUTURE ME** so ill deal with that later™

The next feature im planning on adding is a tag list showing what posts have what tags

---

Adding tags was done by finding every mention of a tag in the metadata for markdown files and counting how often each tag was found and in what files, then making a new page for each tag

## Thinking about adding a database
Adding a database would mean i could query posts and image inside the html without having to manually find them in code, i could also speed up building the pages as i could cache compiled codeblocks and other files so that we wouldnt have to modify them every time.

Getting inspiration from [fasterthanli.me](https://fasterthanli.me) i could also query the database in tenplates to show latest posts and other data.
I can also using lol_html as it seems like it can do cool stuff.

Some problems are that i dont know how i can keep track of a specific versions changes and make sure that each version gets the files they need and reduce the amount of querying needed
i also still want the source of truth being my markdown files, not a database
im not skilled enough in rust and it might be a shiny object causing me to become demotivated with the project.

It would be awome thpugh, i can query posts and videos and tags inside the templates without having to do weird parsing and passing variables throughout the program
I could access the htmk transformed files and the non-modified ones with great ease

**Revision**
An input file is split up into the individual elements
Text, italics, codeblock, blockquote etc


Clock SVG by Dazzle UI

