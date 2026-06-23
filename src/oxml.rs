use std::collections::BTreeMap;
use std::ops::{Add, AddAssign};

// ============================================================================
// OXML - Oxidized Markup Language
//
// A closed algebraic system for constructing valid HTML in Rust.
//
// Design invariants:
//   1. Void elements are represented as a distinct type and cannot have children.
//   2. Content elements always render with matching open/close tags.
//   3. Composition (cat) of valid nodes always yields a valid node.
//   4. All attribute maps are ordered (BTreeMap) for deterministic output.
//   5. The language is closed under all public operations.
// ============================================================================

// --- Element classification ---

/// Classification of HTML elements by their content model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElemKind {
    /// Void elements that must not have children or closing tags.
    Void,
    /// Elements that can contain child nodes.
    Content,
}

impl ElemKind {
    pub const fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }
}

/// Typed HTML element tag with known classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElemTag {
    pub name: &'static str,
    pub kind: ElemKind,
}

impl ElemTag {
    pub const fn void(name: &'static str) -> Self {
        Self {
            name,
            kind: ElemKind::Void,
        }
    }

    pub const fn content(name: &'static str) -> Self {
        Self {
            name,
            kind: ElemKind::Content,
        }
    }

    pub const fn is_void(&self) -> bool {
        matches!(self.kind, ElemKind::Void)
    }
}

// --- Canonical tag registry ---

pub mod tags {
    use super::ElemTag;

    // Document structure
    pub const HTML: ElemTag = ElemTag::content("html");
    pub const HEAD: ElemTag = ElemTag::content("head");
    pub const BODY: ElemTag = ElemTag::content("body");
    pub const TITLE: ElemTag = ElemTag::content("title");
    pub const STYLE: ElemTag = ElemTag::content("style");
    pub const SCRIPT: ElemTag = ElemTag::content("script");

    // Sectioning
    pub const MAIN: ElemTag = ElemTag::content("main");
    pub const SECTION: ElemTag = ElemTag::content("section");
    pub const ARTICLE: ElemTag = ElemTag::content("article");
    pub const NAV: ElemTag = ElemTag::content("nav");
    pub const ASIDE: ElemTag = ElemTag::content("aside");
    pub const HEADER: ElemTag = ElemTag::content("header");
    pub const FOOTER: ElemTag = ElemTag::content("footer");

    // Grouping
    pub const DIV: ElemTag = ElemTag::content("div");
    pub const P: ElemTag = ElemTag::content("p");
    pub const PRE: ElemTag = ElemTag::content("pre");
    pub const BLOCKQUOTE: ElemTag = ElemTag::content("blockquote");
    pub const FIGURE: ElemTag = ElemTag::content("figure");
    pub const FIGCAPTION: ElemTag = ElemTag::content("figcaption");
    pub const UL: ElemTag = ElemTag::content("ul");
    pub const OL: ElemTag = ElemTag::content("ol");
    pub const LI: ElemTag = ElemTag::content("li");
    pub const DL: ElemTag = ElemTag::content("dl");
    pub const DT: ElemTag = ElemTag::content("dt");
    pub const DD: ElemTag = ElemTag::content("dd");

    // Text
    pub const H1: ElemTag = ElemTag::content("h1");
    pub const H2: ElemTag = ElemTag::content("h2");
    pub const H3: ElemTag = ElemTag::content("h3");
    pub const H4: ElemTag = ElemTag::content("h4");
    pub const H5: ElemTag = ElemTag::content("h5");
    pub const H6: ElemTag = ElemTag::content("h6");
    pub const SPAN: ElemTag = ElemTag::content("span");
    pub const A: ElemTag = ElemTag::content("a");
    pub const STRONG: ElemTag = ElemTag::content("strong");
    pub const EM: ElemTag = ElemTag::content("em");
    pub const CODE: ElemTag = ElemTag::content("code");
    pub const BR: ElemTag = ElemTag::void("br");
    pub const HR: ElemTag = ElemTag::void("hr");

    // Inline
    pub const IMG: ElemTag = ElemTag::void("img");
    pub const IFRAME: ElemTag = ElemTag::content("iframe");
    pub const CANVAS: ElemTag = ElemTag::content("canvas");

    // Form
    pub const FORM: ElemTag = ElemTag::content("form");
    pub const INPUT: ElemTag = ElemTag::void("input");
    pub const BUTTON: ElemTag = ElemTag::content("button");
    pub const TEXTAREA: ElemTag = ElemTag::content("textarea");
    pub const SELECT: ElemTag = ElemTag::content("select");
    pub const OPTION: ElemTag = ElemTag::content("option");
    pub const LABEL: ElemTag = ElemTag::content("label");

    // Table
    pub const TABLE: ElemTag = ElemTag::content("table");
    pub const THEAD: ElemTag = ElemTag::content("thead");
    pub const TBODY: ElemTag = ElemTag::content("tbody");
    pub const TFOOT: ElemTag = ElemTag::content("tfoot");
    pub const TR: ElemTag = ElemTag::content("tr");
    pub const TH: ElemTag = ElemTag::content("th");
    pub const TD: ElemTag = ElemTag::content("td");

    // Head / metadata (void)
    pub const META: ElemTag = ElemTag::void("meta");
    pub const LINK: ElemTag = ElemTag::void("link");
    pub const BASE: ElemTag = ElemTag::void("base");
}

// --- Core node algebra ---

/// An OXML node. The type system guarantees that:
///   - Void elements never carry children.
///   - Content elements always render with matching open/close tags.
///   - Composition of valid nodes is always valid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ONode {
    /// Identity element for cat. Renders to empty string.
    Empty,
    /// Escaped text node.
    Text(String),
    /// Raw HTML fragment (use with care).
    Raw(String),
    /// A typed content element with children.
    Content(OContent),
    /// A typed void element (no children possible).
    Void(OVoid),
    /// Ordered sequence of nodes.
    Cat(Vec<ONode>),
}

/// A content element: tag + attrs + children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OContent {
    pub tag: ElemTag,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<ONode>,
}

/// A void element: tag + attrs. No children by construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OVoid {
    pub tag: ElemTag,
    pub attrs: BTreeMap<String, String>,
}

// --- Builders ---

/// Builder for content elements. Consumed on `.build()`.
pub struct ContentBuilder {
    tag: ElemTag,
    attrs: BTreeMap<String, String>,
    children: Vec<ONode>,
}

/// Builder for void elements. Consumed on `.build()`.
pub struct VoidBuilder {
    tag: ElemTag,
    attrs: BTreeMap<String, String>,
}

impl ContentBuilder {
    pub fn new(tag: ElemTag) -> Self {
        debug_assert!(!tag.is_void(), "ContentBuilder requires a non-void tag");
        Self {
            tag,
            attrs: BTreeMap::new(),
            children: Vec::new(),
        }
    }

    pub fn attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.insert(key.into(), value.into());
        self
    }

    pub fn attr_if(self, cond: bool, key: impl Into<String>, value: impl Into<String>) -> Self {
        if cond {
            self.attr(key, value)
        } else {
            self
        }
    }

    pub fn class(self, value: impl Into<String>) -> Self {
        self.attr("class", value)
    }

    pub fn id(self, value: impl Into<String>) -> Self {
        self.attr("id", value)
    }

    pub fn data(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attr(format!("data-{}", key.into()), value)
    }

    pub fn child(mut self, node: ONode) -> Self {
        self.children.push(node);
        self
    }

    pub fn text(self, value: impl Into<String>) -> Self {
        self.child(ONode::text(value))
    }

    pub fn raw(self, value: impl Into<String>) -> Self {
        self.child(ONode::raw(value))
    }

    pub fn children(mut self, nodes: Vec<ONode>) -> Self {
        self.children.extend(nodes);
        self
    }

    pub fn build(self) -> ONode {
        ONode::Content(OContent {
            tag: self.tag,
            attrs: self.attrs,
            children: self.children,
        })
    }
}

impl VoidBuilder {
    pub fn new(tag: ElemTag) -> Self {
        debug_assert!(tag.is_void(), "VoidBuilder requires a void tag");
        Self {
            tag,
            attrs: BTreeMap::new(),
        }
    }

    pub fn attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.insert(key.into(), value.into());
        self
    }

    pub fn attr_if(self, cond: bool, key: impl Into<String>, value: impl Into<String>) -> Self {
        if cond {
            self.attr(key, value)
        } else {
            self
        }
    }

    pub fn build(self) -> ONode {
        ONode::Void(OVoid {
            tag: self.tag,
            attrs: self.attrs,
        })
    }
}

// --- ONode constructors and algebra ---

impl ONode {
    pub fn empty() -> Self {
        Self::Empty
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn raw(value: impl Into<String>) -> Self {
        Self::Raw(value.into())
    }

    pub fn cat(nodes: Vec<ONode>) -> Self {
        Self::Cat(flatten_vec(nodes))
    }

    pub fn content(tag: ElemTag) -> ContentBuilder {
        ContentBuilder::new(tag)
    }

    pub fn void(tag: ElemTag) -> VoidBuilder {
        VoidBuilder::new(tag)
    }

    /// Cat (concatenation): append two nodes into a flat sequence.
    /// This is the monoidal append for ONode.
    pub fn cat2(self, rhs: ONode) -> Self {
        match (self, rhs) {
            (ONode::Empty, b) => b,
            (a, ONode::Empty) => a,
            (ONode::Cat(mut a), ONode::Cat(b)) => {
                a.extend(b);
                ONode::Cat(a)
            }
            (ONode::Cat(mut a), b) => {
                a.push(b);
                ONode::Cat(a)
            }
            (a, ONode::Cat(mut b)) => {
                let mut out = Vec::with_capacity(1 + b.len());
                out.push(a);
                out.append(&mut b);
                ONode::Cat(out)
            }
            (a, b) => ONode::Cat(vec![a, b]),
        }
    }

    /// Wrap this node inside a new content element.
    pub fn wrap(self, tag: ElemTag) -> ONode {
        ONode::content(tag).child(self).build()
    }

    /// Insert a node at a given index in a Cat sequence.
    pub fn insert_at(self, index: usize, node: ONode) -> ONode {
        match self {
            ONode::Cat(mut nodes) => {
                let idx = index.min(nodes.len());
                nodes.insert(idx, node);
                ONode::Cat(nodes)
            }
            other => {
                if index == 0 {
                    ONode::Cat(vec![node, other])
                } else {
                    ONode::Cat(vec![other, node])
                }
            }
        }
    }

    /// Map over all text nodes in the tree.
    pub fn map_text(self, f: impl Fn(&str) -> String + Copy) -> ONode {
        match self {
            ONode::Text(t) => ONode::Text(f(&t)),
            ONode::Raw(t) => ONode::Raw(t),
            ONode::Empty => ONode::Empty,
            ONode::Content(c) => ONode::Content(OContent {
                tag: c.tag,
                attrs: c.attrs,
                children: c.children.into_iter().map(|n| n.map_text(f)).collect(),
            }),
            ONode::Void(v) => ONode::Void(v),
            ONode::Cat(nodes) => ONode::Cat(nodes.into_iter().map(|n| n.map_text(f)).collect()),
        }
    }

    /// Filter out Empty nodes from Cat sequences.
    pub fn compact(self) -> ONode {
        match self {
            ONode::Cat(nodes) => {
                let compacted: Vec<ONode> = nodes
                    .into_iter()
                    .map(|n| n.compact())
                    .filter(|n| !matches!(n, ONode::Empty))
                    .collect();
                match compacted.len() {
                    0 => ONode::Empty,
                    1 => compacted.into_iter().next().unwrap(),
                    _ => ONode::Cat(compacted),
                }
            }
            ONode::Content(c) => ONode::Content(OContent {
                tag: c.tag,
                attrs: c.attrs,
                children: c.children.into_iter().map(|n| n.compact()).collect(),
            }),
            other => other,
        }
    }

    /// Render to an HTML string.
    pub fn render(&self) -> String {
        match self {
            ONode::Empty => String::new(),
            ONode::Text(t) => escape_html(t),
            ONode::Raw(r) => r.clone(),
            ONode::Content(c) => c.render(),
            ONode::Void(v) => v.render(),
            ONode::Cat(nodes) => nodes
                .iter()
                .map(|n| n.render())
                .collect::<Vec<_>>()
                .join(""),
        }
    }

    /// Render with indentation for debugging.
    pub fn render_pretty(&self) -> String {
        self.render_indent(0)
    }

    fn render_indent(&self, depth: usize) -> String {
        let pad = "  ".repeat(depth);
        match self {
            ONode::Empty => String::new(),
            ONode::Text(t) => format!("{}{}", pad, escape_html(t)),
            ONode::Raw(r) => format!("{}{}", pad, r),
            ONode::Content(c) => c.render_indent(depth),
            ONode::Void(v) => format!("{}{}", pad, v.render()),
            ONode::Cat(nodes) => nodes
                .iter()
                .map(|n| n.render_indent(depth))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

// --- Monoid instance ---

impl Add for ONode {
    type Output = ONode;
    fn add(self, rhs: ONode) -> Self::Output {
        self.cat2(rhs)
    }
}

impl AddAssign for ONode {
    fn add_assign(&mut self, rhs: ONode) {
        let lhs = std::mem::replace(self, ONode::Empty);
        *self = lhs.cat2(rhs);
    }
}

/// Flatten a Vec<ONode>, collapsing nested Cat nodes.
fn flatten_vec(nodes: Vec<ONode>) -> Vec<ONode> {
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            ONode::Cat(inner) => out.extend(flatten_vec(inner)),
            other => out.push(other),
        }
    }
    out
}

// --- Element rendering ---

impl OContent {
    pub fn render(&self) -> String {
        let attrs = render_attrs(&self.attrs);
        let children = self
            .children
            .iter()
            .map(|n| n.render())
            .collect::<Vec<_>>()
            .join("");
        format!(
            "<{}{}>{}</{}>",
            self.tag.name, attrs, children, self.tag.name
        )
    }

    fn render_indent(&self, depth: usize) -> String {
        let pad = "  ".repeat(depth);
        let attrs = render_attrs(&self.attrs);
        if self.children.is_empty() {
            return format!("{}<{}{}></{}>", pad, self.tag.name, attrs, self.tag.name);
        }
        let inner = self
            .children
            .iter()
            .map(|n| n.render_indent(depth + 1))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{}<{}{}>\n{}\n{}</{}>",
            pad, self.tag.name, attrs, inner, pad, self.tag.name
        )
    }
}

impl OVoid {
    pub fn render(&self) -> String {
        let attrs = render_attrs(&self.attrs);
        format!("<{}{}>", self.tag.name, attrs)
    }
}

fn render_attrs(attrs: &BTreeMap<String, String>) -> String {
    attrs
        .iter()
        .map(|(k, v)| format!(" {}=\"{}\"", k, escape_attr(v)))
        .collect::<String>()
}

pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn escape_attr(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// --- Typed convenience constructors ---

pub fn post_button(text: &str, url: &str, target: &str) -> String {
    button()
        .attr("class", "btn btn-primary")
        .attr("data-post", url)
        .attr("data-target", target)
        .attr("data-swap", "innerHTML")
        .text(text)
        .build()
        .render()
}

pub fn get_link(text: &str, url: &str, target: &str) -> String {
    a().attr("href", "#")
        .attr("data-get", url)
        .attr("data-target", target)
        .attr("data-swap", "innerHTML")
        .text(text)
        .build()
        .render()
}

pub fn list(items: Vec<String>) -> String {
    ul().children(
        items
            .into_iter()
            .map(|item| li().child(ONode::text(item)).build())
            .collect(),
    )
    .build()
    .render()
}

/// Build a full HTML document from head and body nodes.
pub fn doc(head: ONode, body: ONode) -> String {
    let html = ONode::content(tags::HTML)
        .attr("lang", "en")
        .children(vec![
            ONode::content(tags::HEAD).child(head).build(),
            ONode::content(tags::BODY).child(body).build(),
        ])
        .build()
        .render();
    format!("<!DOCTYPE html>{}", html)
}

// --- Common element shorthands ---

pub fn div() -> ContentBuilder {
    ContentBuilder::new(tags::DIV)
}
pub fn span() -> ContentBuilder {
    ContentBuilder::new(tags::SPAN)
}
pub fn p() -> ContentBuilder {
    ContentBuilder::new(tags::P)
}
pub fn h1() -> ContentBuilder {
    ContentBuilder::new(tags::H1)
}
pub fn h2() -> ContentBuilder {
    ContentBuilder::new(tags::H2)
}
pub fn h3() -> ContentBuilder {
    ContentBuilder::new(tags::H3)
}
pub fn h4() -> ContentBuilder {
    ContentBuilder::new(tags::H4)
}
pub fn h5() -> ContentBuilder {
    ContentBuilder::new(tags::H5)
}
pub fn h6() -> ContentBuilder {
    ContentBuilder::new(tags::H6)
}
pub fn section() -> ContentBuilder {
    ContentBuilder::new(tags::SECTION)
}
pub fn article() -> ContentBuilder {
    ContentBuilder::new(tags::ARTICLE)
}
pub fn nav() -> ContentBuilder {
    ContentBuilder::new(tags::NAV)
}
pub fn header() -> ContentBuilder {
    ContentBuilder::new(tags::HEADER)
}
pub fn footer() -> ContentBuilder {
    ContentBuilder::new(tags::FOOTER)
}
pub fn main() -> ContentBuilder {
    ContentBuilder::new(tags::MAIN)
}
pub fn a() -> ContentBuilder {
    ContentBuilder::new(tags::A)
}
pub fn button() -> ContentBuilder {
    ContentBuilder::new(tags::BUTTON)
}
pub fn form() -> ContentBuilder {
    ContentBuilder::new(tags::FORM)
}
pub fn ul() -> ContentBuilder {
    ContentBuilder::new(tags::UL)
}
pub fn ol() -> ContentBuilder {
    ContentBuilder::new(tags::OL)
}
pub fn li() -> ContentBuilder {
    ContentBuilder::new(tags::LI)
}
pub fn table() -> ContentBuilder {
    ContentBuilder::new(tags::TABLE)
}
pub fn tr() -> ContentBuilder {
    ContentBuilder::new(tags::TR)
}
pub fn th() -> ContentBuilder {
    ContentBuilder::new(tags::TH)
}
pub fn td() -> ContentBuilder {
    ContentBuilder::new(tags::TD)
}
pub fn title() -> ContentBuilder {
    ContentBuilder::new(tags::TITLE)
}
pub fn style() -> ContentBuilder {
    ContentBuilder::new(tags::STYLE)
}
pub fn script() -> ContentBuilder {
    ContentBuilder::new(tags::SCRIPT)
}
pub fn meta() -> VoidBuilder {
    VoidBuilder::new(tags::META)
}
pub fn link() -> VoidBuilder {
    VoidBuilder::new(tags::LINK)
}
pub fn img() -> VoidBuilder {
    VoidBuilder::new(tags::IMG)
}
pub fn br() -> VoidBuilder {
    VoidBuilder::new(tags::BR)
}
pub fn hr() -> VoidBuilder {
    VoidBuilder::new(tags::HR)
}
pub fn input() -> VoidBuilder {
    VoidBuilder::new(tags::INPUT)
}
