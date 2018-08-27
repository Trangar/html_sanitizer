This crate is a simple HTML sanitizer, build on top of [html5ever](https://crates.io/crates/html5ever)

With this crate, you can determine for every HTML tag what you want to sanitize. This is done by the [Tag](struct.Tag.html) struct that gets passed for every HTML tag.

```rust
use std::fs::File;
use html_sanitizer::TagParser;

fn main() {
    let mut file = File::open("your_html_document.html").unwrap();
    let mut tag_parser = TagParser::new(&mut file);
    let result = tag_parser.walk(|tag| {
        if tag.name == "html" || tag.name == "body" {
            // ignore <html> and <body> tags, but still parse their children
            tag.ignore_self();
        } else if tag.name == "head" || tag.name == "script" || tag.name == "style" {
            // Ignore <head>, <script> and <style> tags, and all their children
            tag.ignore_self_and_contents();
        } else if tag.name == "a" {
            // Allow specific attributes
            tag.allow_attribute(String::from("href"));
        } else if tag.name == "img" {
            // Completely rewrite tags and their children
            tag.rewrite_as(String::from("<b>Images not allowed</b>"));
        } else {
            // Allow specific attributes
            tag.allow_attribute(String::from("style"));
        }
    });
    // result contains a string of your sanitized HTML
}
```