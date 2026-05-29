use std::collections::BTreeMap;

/// The single template tree type.
///
/// `Node` is an inductive sum-of-products (Pierce, *TAPL* §11) — the W-type
/// `W(s : Shape). Branch(s)` whose shape index is `{ Text, Element, VoidElement,
/// Load }`. Attribute maps and block maps are ordered (`BTreeMap`) so that
/// rendering is a deterministic function of the tree: equal trees always render
/// to byte-identical output, which makes builds reproducible and content hashes
/// stable.
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
    /// A deferred `<?load?>`: a hole in the tree to be filled by the resolver
    /// with the named file's nodes, with `blocks` substituted into its slots.
    Load {
        file: String,
        blocks: BTreeMap<String, Vec<Node>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateAst {
    pub nodes: Vec<Node>,
}
