use crate::template::parser::hparser::HTML_TAG_PREFIX;
use crate::template::{Node, Engine};

pub fn to_trml(source: &str) -> Result<String, String> {
    let ast = Engine::parse_source(source).map_err(|e| e.to_string())?;
    Ok(serialize_ast(&ast.nodes, 0))
}

fn display_name(name: &str) -> &str {
    name.strip_prefix(HTML_TAG_PREFIX).unwrap_or(name)
}

fn serialize_ast(nodes: &[Node], indent: usize) -> String {
    let mut out = String::new();
    let p = "  ".repeat(indent);
    let pi = "  ".repeat(indent + 1);

    for node in nodes {
        match node {
            Node::Text(text) => {
                let t = text.trim();
                if t.is_empty() {
                    continue;
                }
                for line in t.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let escaped = line.replace('=', "\\=");
                    out.push_str(&format!("{p}{}\n", escaped));
                }
            }
            Node::VoidElement { name, attrs } => {
                out.push_str(&format!("{p}[{}]\n", display_name(name)));
                for (k, v) in attrs {
                    out.push_str(&format!("{pi}{k} = {:?}\n", v));
                }
            }
            Node::Element {
                name,
                attrs,
                children,
            } => {
                out.push_str(&format!("{p}[{}]\n", display_name(name)));
                for (k, v) in attrs {
                    out.push_str(&format!("{pi}{k} = {:?}\n", v));
                }
                out.push_str(&serialize_ast(children, indent + 1));
            }
            Node::Load { file, blocks } => {
                out.push_str(&format!("{p}[load]\n"));
                out.push_str(&format!("{pi}file = {:?}\n", file));
                for (slot, block_nodes) in blocks {
                    out.push_str(&format!("{p}[block]\n"));
                    out.push_str(&format!("{pi}slot = {:?}\n", slot));
                    out.push_str(&serialize_ast(block_nodes, indent + 1));
                }
            }
        }
    }
    out
}

