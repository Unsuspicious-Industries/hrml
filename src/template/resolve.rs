//! Load expansion and slot/block substitution — the one resolution pass.
//!
//! Resolution is pure tree-rewriting on [`Node`], independent of where files
//! come from. The only thing that varies between the disk-backed [`Engine`] and
//! the in-memory [`Project`] is the `fetch` function that turns a file path into
//! its parsed nodes; everything downstream is identical.
//!
//! Two rewrites make up the pass:
//!
//! * **block injection** — `inject_blocks` substitutes each `<?slot id=k?>` with
//!   `blocks[k]`. Component definitions are *substitution boundaries*: they own
//!   their own slots (filled later by `<?use?>`), so the rewrite never descends
//!   into a `<?component?>` subtree. This is exactly capture-avoiding
//!   substitution — a component is a binder for its slot names.
//!
//! * **load expansion** — `resolve_loads` replaces each `<?load file=f?>` with
//!   the resolved nodes of `f`, injecting the enclosing scope's blocks into the
//!   loaded subtree's slots (`inject_blocks(resolve(f), blocks)`).
//!
//! [`Engine`]: super::Engine
//! [`Project`]: super::super::project::Project

use super::ast::Node;
use super::error::{TemplateError, TemplateErrorPhase, TemplateResult};
use std::collections::BTreeMap;

/// Pure data-pipeline directives (Kleisli steps over the context): they set a
/// context variable and emit nothing. These are the only directives hoisted
/// ahead of expanded loads, so the data they bind is ready before any injected
/// layout content reads it. Output-producing directives (`<?get?>`, …) keep
/// their position.
const PREAMBLE_DIRECTIVES: &[&str] = &["data", "sort", "filter", "slice"];

/// A source of parsed template nodes, keyed by file path.
///
/// Disk-backed engines read and parse the file; in-memory projects look it up
/// in their file map. Cycle detection is the resolver's concern, not the
/// fetcher's.
pub type Fetch<'a> = dyn Fn(&str) -> TemplateResult<Vec<Node>> + 'a;

/// Wrap a page in the configured default layout when it declares no `<?load?>`
/// of its own. A page then needs only its `<?block?>` fills (and any preamble
/// directives); the engine prepends the auto-imports and the layout load, after
/// which ordinary load resolution injects the page's blocks into the layout's
/// slots. A page that loads anything explicitly is left untouched — it is
/// taken to be managing its own document.
pub fn with_default_layout(nodes: &[Node], layout: Option<&str>, imports: &[String]) -> Vec<Node> {
    let Some(layout) = layout else {
        return nodes.to_vec();
    };
    let declares_load = nodes
        .iter()
        .any(|n| matches!(n, Node::VoidElement { name, .. } if name == "load"));
    if declares_load {
        return nodes.to_vec();
    }

    let mut wrapped: Vec<Node> = imports.iter().map(|f| load_node(f)).collect();
    wrapped.push(load_node(layout));
    wrapped.extend(nodes.iter().cloned());
    wrapped
}

/// Synthesise a `<?load file="…"?>` node.
fn load_node(file: &str) -> Node {
    let mut attrs = BTreeMap::new();
    attrs.insert("file".to_string(), file.to_string());
    Node::VoidElement {
        name: "load".to_string(),
        attrs,
    }
}

/// Substitute `<?slot id=k?>` placeholders with `blocks[k]`.
///
/// `<?component?>` subtrees are left untouched — they are substitution
/// boundaries that own their slots until instantiated by `<?use?>`.
pub fn inject_blocks(nodes: Vec<Node>, blocks: &BTreeMap<String, Vec<Node>>) -> Vec<Node> {
    let mut result = Vec::new();
    for node in nodes {
        match node {
            // Component definitions own their slots; never inject page blocks into them.
            Node::Element { ref name, .. } if name == "component" => result.push(node),

            Node::Element {
                name,
                attrs,
                children,
            } if name == "slot" => {
                if let Some(filled) = attrs.get("id").and_then(|id| blocks.get(id)) {
                    result.extend(filled.clone());
                } else {
                    // No matching block: keep the slot so its default content renders.
                    result.push(Node::Element {
                        name,
                        attrs,
                        children: inject_blocks(children, blocks),
                    });
                }
            }
            Node::VoidElement { name, attrs } if name == "slot" => {
                if let Some(filled) = attrs.get("id").and_then(|id| blocks.get(id)) {
                    result.extend(filled.clone());
                } else {
                    result.push(Node::VoidElement { name, attrs });
                }
            }
            Node::Element {
                name,
                attrs,
                children,
            } => result.push(Node::Element {
                name,
                attrs,
                children: inject_blocks(children, blocks),
            }),
            other => result.push(other),
        }
    }
    result
}

/// Collect every `<?component?>` definition reachable in `nodes`. This is the
/// shared basis of component auto-discovery: a component library is just the
/// union of these definitions across the configured component files, prepended
/// to a page as an implicit prelude (definitions emit nothing, so they never
/// affect output ordering).
pub fn collect_components(nodes: &[Node], out: &mut Vec<Node>) {
    for node in nodes {
        if let Node::Element { name, children, .. } = node {
            if name == "component" {
                out.push(node.clone());
            } else {
                collect_components(children, out);
            }
        }
    }
}

/// Collect the top-level `<?block slot=k?>` children of `nodes`, keyed by slot.
pub fn extract_blocks(nodes: &[Node]) -> BTreeMap<String, Vec<Node>> {
    let mut blocks = BTreeMap::new();
    for node in nodes {
        if let Node::Element {
            name,
            attrs,
            children,
        } = node
        {
            if name == "block" {
                if let Some(slot) = attrs.get("slot") {
                    blocks.insert(slot.clone(), children.clone());
                }
            }
        }
    }
    blocks
}

/// Every `<?load file=...?>` target reachable in `nodes` (recursively).
pub fn extract_load_targets(nodes: &[Node]) -> Vec<String> {
    let mut targets = Vec::new();
    for node in nodes {
        match node {
            Node::VoidElement { name, attrs } if name == "load" => {
                if let Some(file) = attrs.get("file") {
                    targets.push(file.clone());
                }
            }
            Node::Element { children, .. } => targets.extend(extract_load_targets(children)),
            _ => {}
        }
    }
    targets
}

/// Expand every `<?load?>` in `nodes` by fetching and resolving the target file,
/// injecting the enclosing scope's `<?block?>`s into the loaded subtree's slots.
///
/// When `hoist` is true (top-level page only) the scope's context-setting void
/// directives (`<?data?>`, `<?sort?>`, `<?bind?>`, …) are moved ahead of the
/// expanded loads, so they run before any injected block content that reads the
/// variables they produce. Loaded sub-files use `hoist = false` to preserve
/// their structural order.
pub fn resolve_loads(
    nodes: &[Node],
    fetch: &Fetch,
    visited: &mut Vec<String>,
    hoist: bool,
) -> TemplateResult<Vec<Node>> {
    // A file's *top-level* `<?block?>`s fill the slots of the files it loads.
    let blocks = extract_blocks(nodes);
    let mut preamble = Vec::new();
    let mut body = Vec::new();

    for node in nodes {
        match node {
            Node::VoidElement { name, attrs } if name == "load" => {
                let loaded = fetch_loaded(fetch, attrs, visited)?;
                body.extend(inject_blocks(loaded, &blocks));
            }
            Node::Element { name, .. } if name == "block" => {
                // Top-level block: extracted into `blocks`; the marker produces nothing.
            }
            Node::VoidElement { name, .. }
                if hoist && PREAMBLE_DIRECTIVES.contains(&name.as_str()) =>
            {
                preamble.push(node.clone())
            }
            Node::Element {
                name,
                attrs,
                children,
            } => body.push(Node::Element {
                name: name.clone(),
                attrs: attrs.clone(),
                // Descend transparently: nested `<?block?>`s here are component-use
                // arguments (`<?use?>…<?block?>`), not layout fills, so preserve them.
                children: expand_loads(children, fetch, visited, &blocks)?,
            }),
            other => body.push(other.clone()),
        }
    }

    preamble.extend(body);
    Ok(preamble)
}

/// Expand `<?load?>`s inside a subtree while preserving its structure verbatim —
/// in particular `<?block?>` markers, which inside an element are component-use
/// arguments rather than layout fills. `scope_blocks` fills any nested load.
fn expand_loads(
    nodes: &[Node],
    fetch: &Fetch,
    visited: &mut Vec<String>,
    scope_blocks: &BTreeMap<String, Vec<Node>>,
) -> TemplateResult<Vec<Node>> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            Node::VoidElement { name, attrs } if name == "load" => {
                let loaded = fetch_loaded(fetch, attrs, visited)?;
                out.extend(inject_blocks(loaded, scope_blocks));
            }
            Node::Element {
                name,
                attrs,
                children,
            } => out.push(Node::Element {
                name: name.clone(),
                attrs: attrs.clone(),
                children: expand_loads(children, fetch, visited, scope_blocks)?,
            }),
            other => out.push(other.clone()),
        }
    }
    Ok(out)
}

/// Fetch and fully resolve a `<?load file=…?>` target, with cycle detection.
fn fetch_loaded(
    fetch: &Fetch,
    attrs: &BTreeMap<String, String>,
    visited: &mut Vec<String>,
) -> TemplateResult<Vec<Node>> {
    let file = attrs.get("file").ok_or_else(|| {
        TemplateError::code(
            TemplateErrorPhase::Resolve,
            "Load missing 'file' attribute".to_string(),
        )
    })?;

    if visited.iter().any(|v| v == file) {
        return Err(TemplateError::code(
            TemplateErrorPhase::Resolve,
            format!(
                "Circular template dependency detected: {} -> {}",
                visited.join(" -> "),
                file
            ),
        )
        .with_template_path(file)
        .with_directive("load"));
    }

    let loaded = fetch(file)?;
    visited.push(file.clone());
    let resolved = resolve_loads(&loaded, fetch, visited, false);
    visited.pop();
    resolved
}
