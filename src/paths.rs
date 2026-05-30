//! Static-path expansion: turning a dynamic route into the concrete pages it
//! stands for, by *interpreting the page tree*.
//!
//! A file-based route is a path in the page tree; a `[param]` segment is a
//! non-terminal. To expand it the build reads the page's own data bindings:
//! a node `<?data from="dir/$param.ext"?>` says "`$param` ranges over the files
//! of `dir`". So the output tree is the page tree crossed with the data tree —
//! no extra routing config, and the rule is generic over the parameter name,
//! directory, and file type.
//!
//! ```text
//! expand : Params × PageTree × Base → [Binding]
//! Binding = Map Param Value          -- one concrete page
//! ```
//!
//! A static route (no params) expands to a single empty binding, so the build
//! treats static and dynamic pages uniformly. Multiple params take the Cartesian
//! product of their collections, which covers nested routes like
//! `[user]/posts/[post]`.

use crate::template::Node;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// A concrete instantiation of a route: the values bound to its `[param]`s.
pub type Binding = BTreeMap<String, String>;

/// Enumerate the concrete pages a route expands to, relative to project `base`.
///
/// Static routes (`params` empty) yield one empty binding. A parameter with no
/// discoverable collection contributes no values, so its route expands to
/// nothing — an unbound dynamic page generates no output rather than a broken one.
pub fn expand(params: &[String], page: &[Node], base: &Path) -> Vec<Binding> {
    let mut bindings = vec![Binding::new()];
    for param in params {
        let values = match collection_dir(page, param) {
            Some(dir) => slugs_in(&base.join(dir)),
            None => Vec::new(),
        };
        // Cartesian extension: pair every binding so far with every value.
        bindings = bindings
            .iter()
            .flat_map(|base_binding| {
                values.iter().map(move |value| {
                    let mut next = base_binding.clone();
                    next.insert(param.clone(), value.clone());
                    next
                })
            })
            .collect();
    }
    bindings
}

/// The collection directory a parameter ranges over: the prefix of the first
/// `<?data from="…$param…"?>` source up to the `$param` reference.
fn collection_dir(nodes: &[Node], param: &str) -> Option<String> {
    let marker = format!("${}", param);
    find_data_from(nodes, &marker).map(|from| {
        let prefix = from.split(&marker).next().unwrap_or("");
        let dir = prefix.trim_end_matches('/');
        if dir.is_empty() { ".".to_string() } else { dir.to_string() }
    })
}

/// First `<?data?>` source attribute containing `marker`, searched depth-first.
fn find_data_from(nodes: &[Node], marker: &str) -> Option<String> {
    for node in nodes {
        match node {
            Node::VoidElement { name, attrs } if name == "data" => {
                if let Some(from) = attrs.get("from") {
                    if from.contains(marker) {
                        return Some(from.clone());
                    }
                }
            }
            Node::Element { children, .. } => {
                if let Some(found) = find_data_from(children, marker) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// The slugs of a collection directory, sharing the precedence and ordering
/// rules of `<?data from="dir"?>` (mdx > md > json > toml, hidden files skipped).
fn slugs_in(dir: &Path) -> Vec<String> {
    match crate::features::load_data_dir(dir) {
        Ok(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.get("slug")?.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

/// The `[param]` names of a page path, in order (`blog/[slug].hrml` → `["slug"]`).
pub fn route_params(template_path: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut rest = template_path;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else { break };
        let name = after[..close].trim_start_matches("...");
        params.push(name.to_string());
        rest = &after[close + 1..];
    }
    params
}

/// The output path for a rendered route: `pages/` and the extension dropped,
/// each `[param]` substituted with its bound value, `.html` appended
/// (`pages/blog/[slug].hrml` + {slug: intro} → `blog/intro.html`).
pub fn output_path(template_path: &str, binding: &Binding) -> String {
    let mut stem = template_path
        .strip_prefix("pages/")
        .unwrap_or(template_path)
        .trim_end_matches(".hrml")
        .trim_end_matches(".trml")
        .to_string();
    for (param, value) in binding {
        stem = stem
            .replace(&format!("[...{}]", param), value)
            .replace(&format!("[{}]", param), value);
    }
    format!("{}.html", stem)
}
