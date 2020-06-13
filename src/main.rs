use html_sanitizer::TagParser;
use std::fs::File;

fn main() {
    let input_file = "C:/Users/Victor/Documents/spain october 2018/transavia flight ticket.html";
    let mut file = File::open(input_file).unwrap();
    match TagParser::new(&mut file) {
        Ok(mut tag_parser) => {
            let result = tag_parser.walk(|tag| {
                if tag.name == "html" || tag.name == "body" {
                    tag.ignore_self();
                } else if tag.name == "head" || tag.name == "script" || tag.name == "style" {
                    tag.ignore_self_and_contents();
                } else if tag.name == "a" {
                    tag.allow_attribute(String::from("href"));
                } else if tag.name == "img" {
                    if let Some(url) = tag.attrs.iter().find(|(key, _)| key == &"src").map(|(_, url)| url) {
                        let name = match url.rfind('/')  {
                            Some(i) => &url[i+1..],
                            None => "Load image",
                        };
                        tag.rewrite_as(format!("<a href=\"{0}\" onclick=\"return replace_url_by_image(this);\" title=\"{0}\">{1}</a>", url, name));
                    }
                } else {
                    tag.allow_attribute(String::from("style"));
                }
            });
            println!("{}", result);
        }
        Err(errors) => println!("Errors: {:?}", errors),
    };
}
