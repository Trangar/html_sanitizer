#![deny(missing_docs)]

//! This crate is a simple HTML sanitizer, build on top of [html5ever](https://crates.io/crates/html5ever)
//! 
//! With this crate, you can determine for every HTML tag what you want to sanitize. This is done by the [Tag](struct.Tag.html) struct that gets passed for every HTML tag.
//! 
//! ```no_run
//! use std::fs::File;
//! use html_sanitizer::TagParser;
//! 
//! fn main() {
//!     let mut file = File::open("your_html_document.html").unwrap();
//!     let mut tag_parser = TagParser::new(&mut file);
//!     let result = tag_parser.walk(|tag| {
//!         if tag.name == "html" || tag.name == "body" {
//!             // ignore <html> and <body> tags, but still parse their children
//!             tag.ignore_self();
//!         } else if tag.name == "head" || tag.name == "script" || tag.name == "style" {
//!             // Ignore <head>, <script> and <style> tags, and all their children
//!             tag.ignore_self_and_contents();
//!         } else if tag.name == "a" {
//!             // Allow specific attributes
//!             tag.allow_attribute(String::from("href"));
//!         } else if tag.name == "img" {
//!             // Completely rewrite tags and their children
//!             tag.rewrite_as(String::from("<b>Images not allowed</b>"));
//!         } else {
//!             // Allow specific attributes
//!             tag.allow_attribute(String::from("style"));
//!         }
//!     });
//!     // result contains a string of your sanitized HTML
//! }
//! ```

extern crate html5ever;

use html5ever::driver::ParseOpts;
use html5ever::parse_document;
use html5ever::rcdom::{Handle, NodeData, RcDom};
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use std::io;
use std::borrow::Cow;

/// Create a tag parser.
/// 
/// The tag parser is responsible for parsing the tag and walking through the nodes in the HTML document.
/// 
/// ```rust
/// # use html_sanitizer::TagParser;
/// # let mut input = std::io::BufReader::<&[u8]>::new(&[]);
/// let mut parser = TagParser::new(&mut input);
/// parser.walk(|tag| {
///     // Do something with `tag` here
/// });
/// ```
pub struct TagParser {
    dom: RcDom,
}

impl TagParser {
    /// Create a new tagparser with any Read source.
    /// 
    /// Suggested read targets are `std::fs::File` and `std::io::BufReader`.
    pub fn new<A>(input: &mut A) -> Result<Self, Vec<Cow<'static, str>>>
    where
        A: io::Read + Sized,
    {
        let opts = ParseOpts {
            tree_builder: TreeBuilderOpts {
                drop_doctype: true,
                scripting_enabled: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let dom = parse_document(RcDom::default(), opts)
            .from_utf8()
            .read_from(input)
            .unwrap();
        if !dom.errors.is_empty() {
            Err(dom.errors)
        } else {
            Ok(TagParser { dom })
        }
    }

    fn internal_walk<F>(handle: &Handle, callback: &F) -> String
    where
        F: Fn(&mut Tag),
    {
        let mut output = String::new();

        if let NodeData::Element { name, attrs, .. } = &handle.data {
            let name = &name.local;
            let attrs = attrs.borrow();
            let mut attributes = Vec::<(&str, &str)>::new();
            for attr in attrs.iter() {
                attributes.push((
                    &attr.name.local,
                    &attr.value
                ));
            }
            let mut tag = Tag::from_name_and_attrs(name, &attributes);
            callback(&mut tag);

            if tag.ignore_self && tag.ignore_contents {
                return output;
            }
            if let Some(rewrite) = tag.rewrite {
                return rewrite;
            }
            if !tag.ignore_self {
                output += "<";
                output += name;

                for attr in tag.attrs.iter() {
                    if tag.allowed_attributes.iter().any(|a| a == attr.0) {
                        output += " ";
                        output += attr.0;
                        output += "=\"";
                        output += attr.1;
                        output += "\"";
                    }
                }
                output += ">";
            }

            if !tag.ignore_contents {
                for child in handle.children.borrow().iter() {
                    output += &TagParser::internal_walk(child, callback);
                }
            }
            if !tag.ignore_self {
                output += "</";
                output += name;
                output += ">";
            }
        } else {
            match &handle.data {
                NodeData::Document => {}
                NodeData::Doctype { .. } => {}
                NodeData::Text { contents } => output += (&contents.borrow()).trim(),
                NodeData::Comment { .. } => {},
                NodeData::Element { .. } => unreachable!(),
                NodeData::ProcessingInstruction { target, contents } => println!(
                    "Unknown enum tag: NodeData::ProcessingInstruction {{ {:?} {:?} }}",
                    target, contents
                ),
            }
            for child in handle.children.borrow().iter() {
                output += &TagParser::internal_walk(child, callback);
            }
        }
        output
    }

    /// Recursively walk through all the HTML nodes, calling `callback` for each tag.
    pub fn walk<F>(&mut self, callback: F) -> String
    where
        F: Fn(&mut Tag),
    {
        TagParser::internal_walk(&self.dom.document, &callback)
    }
}

/// Represents a single HTML node. You can read the `name` and `attrs` properties to figure out what tag you're sanitizing.
/// 
/// By default all html nodes will be printed, but attributes will be stripped from a tag unless they are added with `allow_attribute` and `allow_attributes`.
pub struct Tag<'a> {
    /// The name of the HTML tag, e.g. 'div', 'img', etc.
    pub name: &'a str,

    /// The attributes of the HTML tag, e.g. ('style', 'width: 100%').
    pub attrs: &'a [(&'a str, &'a str)],
    rewrite: Option<String>,
    allowed_attributes: Vec<String>,
    ignore_self: bool,
    ignore_contents: bool,
}

impl<'a> Tag<'a> {
    fn from_name_and_attrs(name: &'a str, attrs: &'a [(&'a str, &'a str)]) -> Tag<'a> {
        Tag {
            name,
            attrs,
            rewrite: None,
            allowed_attributes: Vec::new(),
            ignore_self: false,
            ignore_contents: false,
        }
    }

    /// Allow the given attribute. This attribute does not have to exist in the `attrs` tag.
    /// 
    /// When this HTML node gets printed, this attribute will also get printed.
    pub fn allow_attribute(&mut self, attr: String) {
        self.allowed_attributes.push(attr);
    }

    /// Allow the given attributes. These attributes do not have to exist in the `attrs` tag.
    /// 
    /// When this HTML node gets printed, these attributes will also get printed.
    pub fn allow_attributes(&mut self, attr: &[String]) {
        self.allowed_attributes.reserve(attr.len());
        for attr in attr {
            self.allowed_attributes.push(attr.clone());
        }
    }

    /// Ignore this tag. This means that the HTML Node will not be printed in the output. In addition, all the child nodes and text content will also not be printed.
    pub fn ignore_self_and_contents(&mut self){
        self.ignore_self = true;
        self.ignore_contents = true;
    }

    /// Ignore this tag. This means that the HTML Node will not be printed in the output. All child nodes and text content will be printed.
    pub fn ignore_self(&mut self){
        self.ignore_self = true;
    }

    /// Completely rewrite this tag and all it's children, replacing them by the custom string that is passed to this function.
    pub fn rewrite_as(&mut self, new_contents: String) {
        self.rewrite = Some(new_contents);
    }
}

