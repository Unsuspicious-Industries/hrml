use std::collections::BTreeMap;

use crate::template::{error::TemplateResult, Node};

use super::ParseTree;

pub struct TParser;

impl ParseTree for TParser {
    fn parse(&self, source: &str, template_path: Option<&str>) -> TemplateResult<Vec<Node>> {
        parse_toml_style(source, template_path)
    }
}

#[derive(Debug, Clone)]
struct Line {
    indent: usize,
    name: String,
    attrs: BTreeMap<String, String>,
    text: Option<String>,
}

fn parse_toml_style(source: &str, _template_path: Option<&str>) -> TemplateResult<Vec<Node>> {
    let lines = tokenize(source)?;
    if lines.is_empty() {
        return Ok(Vec::new());
    }

    let mut stack: Vec<Tree> = Vec::new();
    let root = Tree {
        name: "__root__".to_string(),
        attrs: BTreeMap::new(),
        text: None,
        children: Vec::new(),
    };
    stack.push(root);

    for line in &lines {
        let depth = line.indent / 2;

        while stack.len() > depth + 1 {
            let done = stack.pop().unwrap();
            let parent = stack.last_mut().unwrap();
            parent.children.push(TreeChild::Element(done.into_node()));
        }

        if line.name.is_empty() {
            if let Some(ref text) = line.text {
                let parent = stack.last_mut().unwrap();
                parent.children.push(TreeChild::Text(text.clone()));
            }
            continue;
        }

        let tree = Tree {
            name: line.name.clone(),
            attrs: line.attrs.clone(),
            text: line.text.clone(),
            children: Vec::new(),
        };
        stack.push(tree);
    }

    while stack.len() > 1 {
        let done = stack.pop().unwrap();
        let parent = stack.last_mut().unwrap();
        parent.children.push(TreeChild::Element(done.into_node()));
    }

    let root = stack.pop().unwrap();
    flatten(root.children)
}

#[derive(Debug)]
struct Tree {
    name: String,
    attrs: BTreeMap<String, String>,
    text: Option<String>,
    children: Vec<TreeChild>,
}

#[derive(Debug)]
enum TreeChild {
    Element(Node),
    Text(String),
}

impl Tree {
    fn into_node(self) -> Node {
        let mut children: Vec<Node> = Vec::new();
        for child in self.children {
            match child {
                TreeChild::Element(n) => children.push(n),
                TreeChild::Text(t) => children.push(Node::Text(t)),
            }
        }
        if let Some(t) = self.text {
            children.push(Node::Text(t));
        }
        if children.is_empty() {
            Node::VoidElement {
                name: self.name,
                attrs: self.attrs,
            }
        } else {
            Node::Element {
                name: self.name,
                attrs: self.attrs,
                children,
            }
        }
    }
}

fn flatten(children: Vec<TreeChild>) -> TemplateResult<Vec<Node>> {
    let mut out = Vec::new();
    for child in children {
        match child {
            TreeChild::Element(n) => out.push(n),
            TreeChild::Text(t) => out.push(Node::Text(t)),
        }
    }
    Ok(out)
}

fn tokenize(source: &str) -> TemplateResult<Vec<Line>> {
    let mut lines = Vec::new();

    for raw in source.lines() {
        let trimmed = raw.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let indent = raw.len() - trimmed.len();

        if let Some(tag) = trimmed.strip_prefix('[') {
            let rest = tag.strip_suffix(']').unwrap_or(tag);
            let content = rest.trim();
            let (name, attrs) = split_name_attrs(content);

            lines.push(Line {
                indent,
                name,
                attrs,
                text: None,
            });
        } else if trimmed.contains('=') && !trimmed.starts_with('=') {
            if let Some((key, rest)) = split_key_value(trimmed) {
                let key = key.trim();
                let value = parse_value(rest.trim());
                if let Some(last) = lines.last_mut() {
                    last.attrs.insert(key.to_string(), value);
                }
            } else {
                // Line has = but not in key=value format → treat as text
                let text = unescape_text(trimmed);
                if let Some(last) = lines.last_mut() {
                    last.text = match last.text.take() {
                        Some(existing) => Some(format!("{existing}\n{text}")),
                        None => Some(text),
                    };
                }
            }
        } else {
            let text = unescape_text(trimmed);
            if let Some(last) = lines.last_mut() {
                last.text = match last.text.take() {
                    Some(existing) => Some(format!("{existing}\n{text}")),
                    None => Some(text),
                };
            } else {
                lines.push(Line {
                    indent,
                    name: String::new(),
                    attrs: BTreeMap::new(),
                    text: Some(text),
                });
            }
        }
    }

    Ok(lines)
}

fn parse_value(raw: &str) -> String {
    let trimmed = raw.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        let inner = &trimmed[1..trimmed.len() - 1];
        inner.to_string()
    } else {
        trimmed.to_string()
    }
}

fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    let mut skip_next = false;
    for (byte_pos, ch) in trimmed.char_indices() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if ch == '\\' {
            skip_next = true;
            continue;
        }
        if ch == '=' {
            let key = trimmed[..byte_pos].trim_end();
            let value = trimmed[byte_pos + 1..].trim();
            return if key.is_empty() { None } else { Some((key, value)) };
        }
    }
    None
}

fn unescape_text(s: &str) -> String {
    s.replace("\\=", "=")
}

fn split_name_attrs(content: &str) -> (String, BTreeMap<String, String>) {
    let mut attrs = BTreeMap::new();
    let mut chars = content.chars().peekable();
    let mut name = String::new();
    let mut in_quote = false;
    let mut quote_char = '"';

    while let Some(&c) = chars.peek() {
        if in_quote {
            if c == quote_char {
                in_quote = false;
            }
            name.push(c);
            chars.next();
            continue;
        }
        if c == '"' || c == '\'' {
            quote_char = c;
            in_quote = true;
            name.push(c);
            chars.next();
            continue;
        }
        if c.is_whitespace() {
            chars.next();
            break;
        }
        name.push(c);
        chars.next();
    }

    name = name.trim().to_string();
    let remaining: String = chars.collect();
    let remaining = remaining.trim();
    if !remaining.is_empty() {
        let mut pos = 0;
        let chars: Vec<char> = remaining.chars().collect();
        while pos < chars.len() {
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }
            if pos >= chars.len() {
                break;
            }
            let mut key = String::new();
            while pos < chars.len() && !chars[pos].is_whitespace() && chars[pos] != '=' {
                key.push(chars[pos]);
                pos += 1;
            }
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }
            if pos < chars.len() && chars[pos] == '=' {
                pos += 1;
                while pos < chars.len() && chars[pos].is_whitespace() {
                    pos += 1;
                }
                let val = if pos < chars.len() && (chars[pos] == '"' || chars[pos] == '\'') {
                    let q = chars[pos];
                    pos += 1;
                    let mut v = String::new();
                    while pos < chars.len() && chars[pos] != q {
                        v.push(chars[pos]);
                        pos += 1;
                    }
                    pos += 1;
                    v
                } else {
                    let mut v = String::new();
                    while pos < chars.len() && !chars[pos].is_whitespace() {
                        v.push(chars[pos]);
                        pos += 1;
                    }
                    v
                };
                attrs.insert(key, val);
            }
        }
    }

    (name, attrs)
}
