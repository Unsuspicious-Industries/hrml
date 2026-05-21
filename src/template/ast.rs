use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateAst {
    pub nodes: Vec<AstNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstNode {
    Text(String),
    Element {
        name: String,
        attrs: BTreeMap<String, String>,
        children: Vec<AstNode>,
    },
    VoidElement {
        name: String,
        attrs: BTreeMap<String, String>,
    },
    Load {
        file: String,
        blocks: BTreeMap<String, Vec<AstNode>>,
    },
}
