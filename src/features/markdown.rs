use pulldown_cmark::{html::push_html, Options, Parser as MdParser};
use serde_json::{Map, Number, Value};

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
    let body_start = if source.starts_with("---\r\n") {
        5
    } else if source.starts_with("---\n") {
        4
    } else {
        return ("", source);
    };

    let rest = &source[body_start..];

    for marker in ["\n---\n", "\r\n---\r\n"] {
        if let Some(end) = rest.find(marker) {
            let fm = &rest[..end];
            let body = &rest[end + marker.len()..];
            return (fm, body);
        }
    }

    ("", source)
}

fn parse_frontmatter(frontmatter: &str) -> Value {
    if frontmatter.trim().is_empty() {
        return Value::Null;
    }

    match frontmatter.parse::<toml::Value>() {
        Ok(v) => serde_json::to_value(v).unwrap_or(Value::Null),
        Err(_) => parse_yamlish_frontmatter(frontmatter),
    }
}

fn parse_yamlish_frontmatter(frontmatter: &str) -> Value {
    let mut out = Map::new();
    let mut active_list_key: Option<String> = None;

    for raw_line in frontmatter.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if let Some(item) = trimmed.strip_prefix("- ") {
            let Some(key) = &active_list_key else {
                continue;
            };

            let entry = out
                .entry(key.clone())
                .or_insert_with(|| Value::Array(Vec::new()));
            if let Value::Array(values) = entry {
                values.push(parse_scalar_value(item.trim()));
            }
            continue;
        }

        active_list_key = None;

        let Some((key, raw_value)) = trimmed.split_once(':') else {
            continue;
        };

        let key = key.trim().to_string();
        let value = raw_value.trim();
        if value.is_empty() {
            out.insert(key.clone(), Value::Array(Vec::new()));
            active_list_key = Some(key);
            continue;
        }

        if value.starts_with('[') && value.ends_with(']') {
            let inner = &value[1..value.len() - 1];
            let values = inner
                .split(',')
                .map(|item| item.trim())
                .filter(|item| !item.is_empty())
                .map(parse_scalar_value)
                .collect();
            out.insert(key, Value::Array(values));
            continue;
        }

        out.insert(key, parse_scalar_value(value));
    }

    Value::Object(out)
}

fn parse_scalar_value(value: &str) -> Value {
    let unquoted = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value)
        .trim();

    if let Ok(boolean) = unquoted.parse::<bool>() {
        return Value::Bool(boolean);
    }

    if let Ok(integer) = unquoted.parse::<i64>() {
        return Value::Number(integer.into());
    }

    if let Ok(float) = unquoted.parse::<f64>() {
        if let Some(number) = Number::from_f64(float) {
            return Value::Number(number);
        }
    }

    Value::String(unquoted.to_string())
}
