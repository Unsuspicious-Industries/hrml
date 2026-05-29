use std::collections::BTreeMap;

/// The single template tree type.
///
/// `Node` is an inductive sum-of-products (Pierce, *TAPL* §11) — the W-type
/// `W(s : Shape). Branch(s)` whose shape index is `{ Text, Element, VoidElement }`.
/// Attribute maps are ordered (`BTreeMap`) so that rendering is a deterministic
/// function of the tree: equal trees always render to byte-identical output,
/// which makes builds reproducible and content hashes stable.
///
/// `<?load?>` is just a `VoidElement { name: "load" }` in the parsed tree;
/// [`resolve::resolve_loads`](super::resolve::resolve_loads) expands it away
/// before rendering, so there is no dedicated `Load` variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Text(String),
    Element {
        name: String,
        attrs: BTreeMap<String, String>,
        children: Vec<Node>,
    },
    VoidElement {
        name: String,
        attrs: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateAst {
    pub nodes: Vec<Node>,
}
