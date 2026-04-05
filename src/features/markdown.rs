use pulldown_cmark::{html::push_html, Options, Parser as MdParser};
use serde_json::Value;

pub fn render_markdown(source: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = MdParser::new_ext(source, options);
    let mut html = String::new();
    push_html(&mut html, parser);
    html
}

pub fn render_markdown_with_frontmatter(source: &str) -> (Value, String) {
    let (frontmatter, body) = split_frontmatter(source);
    let meta = parse_frontmatter(frontmatter);
    (meta, render_markdown(body))
}

fn split_frontmatter(source: &str) -> (&str, &str) {
    if !source.starts_with("---\n") {
        return ("", source);
    }

    let rest = &source[4..];
    if let Some(end) = rest.find("\n---\n") {
        let fm = &rest[..end];
        let body = &rest[end + 5..];
        return (fm, body);
    }

    ("", source)
}

fn parse_frontmatter(frontmatter: &str) -> Value {
    if frontmatter.trim().is_empty() {
        return Value::Null;
    }

    match frontmatter.parse::<toml::Value>() {
        Ok(v) => serde_json::to_value(v).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}
