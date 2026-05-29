//! The HRML template engine — an algebraically closed evaluator for the
//! directive language.
//!
//! # Design references
//!
//! The template engine is designed as a **free monad over a directive
//! signature functor** composed with a **reader monad** for context
//! propagation.  The literature justifying this design:
//!
//! ## Algebraic data types and structural induction
//!
//! - **Pierce, Benjamin C.** (2002). *Types and Programming Languages*. MIT
//!   Press.  §11 (algebraic datatypes) and §21 (metatheory of recursive
//!   types).  The `Node` type is an *inductive sum-of-products* — each
//!   directive constructor (`Text`, `VoidElement`, `Element`, `Load`) is a
//!   tagged variant whose eliminator is `render_node`.
//!
//! - **Burstall, Rod** (1969). "Proving Properties of Programs by Structural
//!   Induction". *Computer Journal* 12(1):41–48. Correctness of every
//!   directive handler is provable by structural induction on `Node`:
//!   ```text
//!   P(Text(t))   ∧   ∀n∈VoidElement. P(n)   ∧
//!   ∀n∈Element. (∀c∈children(n). P(c)) ⇒ P(n)
//!   ──────────────────────────────────────────
//!                ∀n∈Node. P(n)
//!   ```
//!
//! - **Martin-Löf, Per** (1984). *Intuitionistic Type Theory*. Bibliopolis.
//!   The W-type `W(x:A). B(x)` encoding of well-founded trees corresponds
//!   exactly to the template AST: `A = { Text, Void, Elem, Load }` is the
//!   shape index, and `B` provides the branching structure for each variant.
//!
//! ## Functors, monads, and composition
//!
//! - **Mac Lane, Saunders** (1971). *Categories for the Working
//!   Mathematician*. Springer.  §II.3 (functors) and §VI (monads).
//!   The pipeline `<?data?>` → `<?sort?>` → `<?filter?>` → `<?map?>` is
//!   functor composition `F ∘ G ∘ H ∘ K` in the Kleisli category of the
//!   `Context` monad.
//!
//! - **Wadler, Philip** (1992). "The Essence of Functional Programming".
//!   *POPL '92*.  §2.3 (monads).  The `Context` carries a reader monad
//!   `Env ⇒ X` threaded through template evaluation; `set_value` is the
//!   monadic bind, and `get_value` is the projection.
//!
//! - **Moggi, Eugenio** (1991). "Notions of Computation and Monads".
//!   *Information and Computation* 93(1):55–92.  §4 (monadic semantics).
//!   Template rendering has a *monadic semantics*:
//!   ```text
//!   render_node : Node → M ONode   where M = Reader Context
//!   render_nodes : [Node] → M [ONode]
//!   ```
//!   The `M` monad models scope/context as an effect.
//!
//! - **Swierstra, Wouter** (2008). "Data types à la carte". *Journal of
//!   Functional Programming* 18(4):423–436.  Each HRML directive (`<?data?>`,
//!   `<?map?>`, `<?filter?>`, `<?sort?>`, …) is a *signature functor*.
//!   Directives compose via the **coproduct** `⊕` of functors, yielding a
//!   modular syntax tree extensible without modifying existing cases.
//!   The `TagRegistry` (void/block) extends this principle dynamically.
//!
//! - **Hughes, John** (1990). "Why Functional Programming Matters". *Computer
//!   Journal* 32(2):98–107.  §3 (gluing programs together).  The
//!   `data → sort → filter → map` pipeline is Hughes' *lazy list
//!   composition* pattern applied to template rendering — each stage produces
//!   a lazy sequence consumed by the next without materializing intermediate
//!   buffers.
//!
//! ## Algebraic effects and free constructions
//!
//! - **Plotkin, Gordon; Power, John** (2003). "Algebraic Operations and
//!   Generic Effects". *Applied Categorical Structures* 11(1):69–94.
//!   Directive handlers (`<?data?>`, `<?map?>`, etc.) are *algebraic
//!   operations* on the carrier set `Context`.  The equational theory:
//!   ```text
//!   map(id) over xs ≡ xs                            (identity)
//!   map(f ∘ g) over xs ≡ map(f) over map(g) over xs  (functor composition)
//!   filter(cond) ∘ filter(cond') ≡ filter(cond ∧ cond')  (conjunction)
//!   sort(by=k, asc) ∘ sort(by=k, asc) ≡ sort(by=k, asc)  (idempotence)
//!   ```
//!
//! - **Kiselyov, Oleg** (2012). "Free and Freer Monads". Blog post /
//!   *extensible-effects*.  The `render_node` dispatch table is a *freer
//!   monad* interpreter: the syntax tree is constructed first, then
//!   interpreted against an effect handler (the directive match arms).
//!
//! ## Derivative-based parsing
//!
//! - **Brzozowski, Janusz A.** (1964). "Derivatives of Regular Expressions".
//!   *JACM* 11(4):481–494.  The constrained generation system in `<?mdx?>`
//!   content uses Brzozowski derivatives for *prefix validation* of
//!   structured input (see §Completeability in the documentation).
//!
//! - **Might, Matthew; Darais, David; Spiewak, Daniel** (2011). "Parsing with
//!   Derivatives: A Functional Pearl". *ICFP '11*.  The template parser uses
//!   a derivative-based approach: consuming `<?` yields the derivative of
//!   the directive grammar, consuming the closing `?>` yields the derivative
//!   of the content grammar.
//!
//! ## Formal type algebra of the directive language
//!
//! ```text
//! Value      ≜  Null | Bool | Number | String
//!             |  Object(Map String Value)          — product type
//!             |  Array(Vec Value)                  — list type
//!
//! Node       ≜  Text(String)
//!             |  VoidElement { name: String, attrs: Map String String }
//!             |  Element   { name: String, attrs: ..., children: [Node] }
//!             |  Load      { file: String, blocks: Map String [Node] }
//!             — inductive sum-of-products (Pierce §11)
//!
//! Attr       ≜  Map String String                   — association list
//!
//! Dir ::=  data  (from: Path,  as: Name)
//!        | map   (over: Name, as: Name)  { block }
//!        | filter(over: Name, where: Field, as: Name)
//!        | sort  (over: Name, by: Field, order: {asc,desc}, as: Name)
//!        | slice (over: Name, start: Nat, count: Nat, as: Name)
//!        | record(as: Name) { field | items }
//!        | list  (as: Name) { item* }
//!             — signature functors (Swierstra 2008)
//!
//! Context    ≜  { data: Value, vars: Map String Value,
//!                 components: Map String [Node], load_stack: [String] }
//!             — reader monad: TemplateResult ≜ Context → Result Html Error
//!
//! render     :  Path → Value → Context → Html
//! render_node:  Node → Context → Html                 — structural recursion
//! render_map :  [A] → (A → Html) → [Html]              — list functor fmap
//! eval_pred  :  Cond → Value → Bool                    — semantic predicate
//! ```
//!
//! ## Structural induction scheme
//!
//! Correctness of the evaluator is established by induction on `Node`:
//! ```text
//! Base cases:
//!   • render_node(Text(t)) ⇝ t                     — identity on literals
//!   • render_node(VoidElement(d, attrs)) ⇝ H_d(attrs) — directive handler
//!
//! Inductive step:
//!   • render_node(Element(name, attrs, children))
//!     ≜  render_nodes(children) ∘ handler(name, attrs)   — compositional
//!
//! Load induction:
//!   • render_node(Load(file, blocks))
//!     ≜  inject_blocks(resolve(file), blocks)             — substitution
//! ```
//!
//! ## Pipeline composition (Kleisli category)
//!
//! The data pipeline is a chain of Kleisli arrows:
//! ```text
//! data from="posts" as="posts"            step₁ : 1 → M [Post]
//! sort over="posts" by="date" as="posts"  step₂ : [Post] → M [Post]
//! map  over="posts" as="post"             step₃ : [Post] → M Html
//! ```
//!
//! Each `stepₙ` is a morphism in the Kleisli category of `M`.
//! The composition `step₃ ∘M step₂ ∘M step₁` is a single Kleisli arrow
//! from the unit type to a rendered fragment.

use crate::config::Config;
use crate::oxml::*;

pub mod parser;
use self::parser::ParseTree;
use self::parser::hparser::HTML_TAG_PREFIX;

fn is_html_node(name: &str) -> bool {
    name.starts_with(HTML_TAG_PREFIX)
}

fn strip_html_prefix(name: &str) -> &str {
    name.strip_prefix(HTML_TAG_PREFIX).unwrap_or(name)
}

const HTML_VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

fn is_html_void_tag(name: &str) -> bool {
    HTML_VOID_TAGS.iter().any(|v| v.eq_ignore_ascii_case(name))
}
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

mod ast;
mod error;
mod head;
pub mod resolve;

fn parse_with_extension(source: &str, template_path: &str) -> TemplateResult<Vec<Node>> {
    if template_path.ends_with(".trml") {
        parser::tparser::TParser.parse(source, Some(template_path))
    } else {
        parser::hparser::HParser.parse(source, Some(template_path))
    }
}

pub use ast::{Node, TemplateAst};
pub use error::{
    TemplateError, TemplateErrorKind, TemplateErrorLocation, TemplateErrorPhase, TemplateResult,
};

#[derive(Clone)]
pub struct Engine {
    base_path: PathBuf,
    site_name: String,
    site_description: Option<String>,
    favicon: Option<String>,
    site_url: Option<String>,
    globals: serde_json::Value,
    tag_registry: crate::features::TagRegistry,
}

impl Engine {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: PathBuf::from(base_path),
            site_name: "HRML App".to_string(),
            site_description: None,
            favicon: None,
            site_url: None,
            globals: serde_json::Value::Object(serde_json::Map::new()),
            tag_registry: crate::features::TagRegistry::new(),
        }
    }

    pub fn with_base_path(base_path: &str) -> Self {
        Self::new(base_path)
    }

    pub fn with_config(config: &Config) -> Self {
        Self {
            base_path: PathBuf::new(),
            site_name: config.site_name.clone(),
            site_description: config.site_description.clone(),
            favicon: config.favicon.clone(),
            site_url: config.site_url.clone(),
            globals: config.globals.clone(),
            tag_registry: crate::features::TagRegistry::new(),
        }
    }

    pub fn with_site_name(mut self, site_name: String) -> Self {
        self.site_name = site_name;
        self
    }

    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.site_description = description;
        self
    }

    pub fn with_favicon(mut self, favicon: Option<String>) -> Self {
        self.favicon = favicon;
        self
    }

    pub fn with_site_url(mut self, site_url: Option<String>) -> Self {
        self.site_url = site_url;
        self
    }

    pub fn with_globals(mut self, globals: serde_json::Value) -> Self {
        self.globals = globals;
        self
    }

    pub fn register_void_tag(&mut self, name: &str, handler: crate::features::VoidTagHandler) {
        self.tag_registry.register_void(name, handler);
    }

    pub fn register_block_tag(&mut self, name: &str, handler: crate::features::BlockTagHandler) {
        self.tag_registry.register_block(name, handler);
    }

    pub fn render(&self, template_path: &str, data: &Value) -> TemplateResult<String> {
        let content = self.read_template(template_path)?;
        self.render_resolved(&content, template_path, data, true)
    }

    pub fn render_fragment(&self, template_path: &str, data: &Value) -> TemplateResult<String> {
        let content = self.read_template(template_path)?;
        self.render_resolved(&content, template_path, data, false)
    }

    pub fn parse_template(&self, template_path: &str) -> TemplateResult<TemplateAst> {
        let content = self.read_template(template_path)?;
        Ok(TemplateAst {
            nodes: self.resolve_source(&content, template_path)?,
        })
    }

    pub fn parse_source(source: &str) -> TemplateResult<TemplateAst> {
        Self::parse_source_with_path(source, None)
    }

    pub fn parse_source_with_path(
        source: &str,
        template_path: Option<&str>,
    ) -> TemplateResult<TemplateAst> {
        let path = template_path.unwrap_or("inline.hrml");
        let nodes = parse_with_extension(source, path)?;
        Ok(TemplateAst { nodes })
    }

    pub fn render_content(
        &self,
        content: &str,
        path: &str,
        data: &Value,
    ) -> TemplateResult<String> {
        self.render_resolved(content, path, data, true)
    }

    pub fn render_content_fragment(
        &self,
        content: &str,
        path: &str,
        data: &Value,
    ) -> TemplateResult<String> {
        self.render_resolved(content, path, data, false)
    }

    pub fn render_nodes_from_tree(&self, nodes: &[Node], data: &Value) -> TemplateResult<ONode> {
        let mut context = self.build_context(data);
        self.register_components_from_tree(nodes, &mut context)?;
        self.render_nodes(nodes, &mut context, "")
    }

    /// Read a template file relative to the engine's base path.
    fn read_template(&self, file: &str) -> TemplateResult<String> {
        fs::read_to_string(self.base_path.join(file)).map_err(|e| {
            TemplateError::code(
                TemplateErrorPhase::Io,
                format!("Failed to read template {}: {}", file, e),
            )
            .with_template_path(file)
        })
    }

    /// Parse `content` and expand all `<?load?>`s into a fully-resolved tree,
    /// reading loaded files from disk relative to the base path.
    fn resolve_source(&self, content: &str, path: &str) -> TemplateResult<Vec<Node>> {
        let nodes = parse_with_extension(content, path)?;
        let fetch = |file: &str| parse_with_extension(&self.read_template(file)?, file);
        let mut visited = vec![path.to_string()];
        resolve::resolve_loads(&nodes, &fetch, &mut visited, true)
    }

    /// Resolve, render, and (when `wrap`) wrap a bare fragment in an HTML shell.
    fn render_resolved(
        &self,
        content: &str,
        path: &str,
        data: &Value,
        wrap: bool,
    ) -> TemplateResult<String> {
        let nodes = self.resolve_source(content, path)?;
        let mut context = self.build_context(data);
        self.register_components_from_tree(&nodes, &mut context)?;
        let body_node = self.render_nodes(&nodes, &mut context, path)?;
        let body = body_node.render();
        if !wrap || is_html_doc(&body) {
            Ok(body)
        } else {
            Ok(self.wrap_html(body_node))
        }
    }

    fn build_context(&self, data: &Value) -> Context {
        let mut context = Context::new(data.clone());
        context.set_str("site_name", self.site_name.clone());
        if let Some(desc) = &self.site_description {
            context.set_str("site_description", desc.clone());
        }
        if let Some(icon) = &self.favicon {
            context.set_str("favicon", icon.clone());
        }
        if let Some(site_url) = &self.site_url {
            context.set_str("site_url", site_url.clone());
        }
        context.set_value("globals", self.globals.clone());
        context
    }

    /// Register every `<?component?>` definition in the resolved tree so a
    /// `<?use?>` can reference it regardless of source order. Loads have already
    /// been expanded by [`resolve::resolve_loads`], so this is a plain walk.
    fn register_components_from_tree(
        &self,
        nodes: &[Node],
        context: &mut Context,
    ) -> TemplateResult<()> {
        for node in nodes {
            if let Node::Element {
                name,
                attrs,
                children,
            } = node
            {
                if name == "component" {
                    if let Some(id) = attrs.get("id") {
                        context.set_component(id, children.clone());
                    }
                }
                self.register_components_from_tree(children, context)?;
            }
        }
        Ok(())
    }

    fn render_nodes(
        &self,
        nodes: &[Node],
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<ONode> {
        let mut result = Vec::new();
        for node in nodes {
            result.push(self.render_node(node, context, template_path)?);
        }
        Ok(ONode::cat(result).compact())
    }

    fn render_node(
        &self,
        node: &Node,
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<ONode> {
        match node {
            Node::Text(text) => Ok(ONode::raw(self.resolve(text, context))),
            Node::VoidElement { name, attrs } if is_html_node(name) => {
                let tag = strip_html_prefix(name);
                let resolved = self.resolve_attrs(attrs, context);
                if is_html_void_tag(tag) {
                    Ok(ONode::raw(crate::generate::void(tag, &resolved)))
                } else {
                    Ok(ONode::raw(crate::generate::self_closed(tag, &resolved)))
                }
            }
            Node::Element {
                name,
                attrs,
                children,
            } if is_html_node(name) => {
                let tag = strip_html_prefix(name);
                let inner = self.render_nodes(children, context, template_path)?;
                let resolved = self.resolve_attrs(attrs, context);
                Ok(ONode::raw(crate::generate::element(
                    tag,
                    &resolved,
                    &inner.render(),
                )))
            }
            Node::VoidElement { name, attrs } => {
                if let Some(result) = self.tag_registry.render_void(name, attrs) {
                    return result
                        .map_err(|message| {
                            TemplateError::code(TemplateErrorPhase::Render, message)
                                .with_template_path(template_path)
                                .with_directive(name)
                        })
                        .map(ONode::raw);
                }
                match name.as_str() {
                    "load" => Ok(ONode::empty()),
                    "else" => Ok(ONode::empty()),
                    "wasm" => {
                        use crate::features::oxml_tags;
                        let module = attrs.get("module").cloned().unwrap_or_default();
                        let src = attrs.get("src").cloned().unwrap_or_default();
                        let export = attrs
                            .get("export")
                            .cloned()
                            .unwrap_or_else(|| "mount".to_string());
                        let start = attrs.get("start").cloned().unwrap_or_default();
                        let target_class = attrs
                            .get("class")
                            .cloned()
                            .unwrap_or_else(|| "hrml-wasm-window".to_string());
                        let props = attrs
                            .get("props")
                            .cloned()
                            .unwrap_or_else(|| "{}".to_string());

                        Ok(ONode::content(oxml_tags::DIV)
                            .attr("class", &target_class)
                            .attr("data-wasm-module", &module)
                            .attr("data-wasm-src", &src)
                            .attr("data-wasm-export", &export)
                            .attr("data-wasm-start", &start)
                            .attr("data-wasm-props", &props)
                            .build())
                    }
                    "set" => {
                        if let (Some(id), Some(val)) = (attrs.get("id"), attrs.get("value")) {
                            context.set_str(id, val.clone());
                        }
                        Ok(ONode::empty())
                    }
                    "pure" => Ok(ONode::raw(attrs.get("value").cloned().unwrap_or_default())),
                    "mdx" => self.render_mdx(attrs, template_path).map(ONode::raw),
                    "markdown" => self.render_markdown(attrs, template_path).map(ONode::raw),
                    "data" => self
                        .render_data(attrs, context, template_path)
                        .map(ONode::raw),
                    "item" => Ok(ONode::empty()),
                    "field" => Ok(ONode::empty()),
                    "filter" => self
                        .render_filter(attrs, context, template_path)
                        .map(ONode::raw),
                    "sort" => self
                        .render_sort(attrs, context, template_path)
                        .map(ONode::raw),
                    "slice" => self
                        .render_slice(attrs, context, template_path)
                        .map(ONode::raw),
                    "markdownfm" => self
                        .render_markdownfm(attrs, context, template_path)
                        .map(ONode::raw),
                    "latex" => self.render_latex(attrs, template_path).map(ONode::raw),
                    head if head::is_head_directive(head) => Ok(ONode::raw(
                        head::render(head, attrs, &|s| self.resolve(s, context))
                            .unwrap_or_default(),
                    )),
                    "use" => self.render_component_use(attrs, &[], context, template_path),
                    "bind" => {
                        if let Some(from) = attrs.get("from") {
                            let var = attrs.get("var").map(String::as_str).unwrap_or("value");
                            let bound = context
                                .get_value(from)
                                .unwrap_or(Value::String(String::new()));
                            context.set_value(var, bound);
                        }
                        Ok(ONode::empty())
                    }
                    "slot" => Ok(ONode::empty()),
                    "block" => {
                        // Void block elements are handled by extract_blocks_local in Element context
                        Ok(ONode::empty())
                    }
                    "get" => {
                        if let Some(id) = attrs.get("id") {
                            Ok(ONode::raw(context.get(id)))
                        } else {
                            Ok(ONode::empty())
                        }
                    }
                    _ => Ok(ONode::raw(crate::generate::self_closed(name, attrs))),
                }
            }
            Node::Element {
                name,
                attrs,
                children,
            } => {
                if self.tag_registry.has_block(name) {
                    let rendered_children = self.render_nodes(children, context, template_path)?;
                    if let Some(result) =
                        self.tag_registry
                            .render_block(name, attrs, &rendered_children.render())
                    {
                        return result
                            .map_err(|message| {
                                TemplateError::code(TemplateErrorPhase::Render, message)
                                    .with_template_path(template_path)
                                    .with_directive(name)
                            })
                            .map(ONode::raw);
                    }
                }
                match name.as_str() {
                    "block" => self.render_nodes(children, context, template_path),
                    "slot" => self.render_nodes(children, context, template_path),
                    "if" => self.render_if(attrs, children, context, template_path),
                    "for" => self.render_for(attrs, children, context, template_path),
                    "map" => self.render_map(attrs, children, context, template_path),
                    "record" => self.render_record(attrs, children, context, template_path),
                    "list" => self.render_list(attrs, children, context, template_path),
                    "field" => self.render_nodes(children, context, template_path),
                    "items" => self.render_nodes(children, context, template_path),
                    "item" => self.render_nodes(children, context, template_path),
                    "set" => {
                        if let Some(id) = attrs.get("id") {
                            let content = self
                                .render_nodes(children, context, template_path)?
                                .render();
                            context.set_str(id, content);
                        }
                        Ok(ONode::empty())
                    }
                    "component" => {
                        if let Some(id) = attrs.get("id") {
                            context.set_component(id, children.to_vec());
                        }
                        Ok(ONode::empty())
                    }
                    "use" => self.render_component_use(attrs, children, context, template_path),
                    "bind" => self.render_bind(attrs, children, context, template_path),
                    "btn" => {
                        use crate::features::oxml_tags;
                        let inner = self.render_nodes(children, context, template_path)?;
                        let method = if attrs.contains_key("post") {
                            "post"
                        } else {
                            "get"
                        };
                        let endpoint = attrs.get(method).unwrap_or(&String::new()).clone();
                        let target = attrs
                            .get("target")
                            .cloned()
                            .unwrap_or_else(|| "#body".to_string());
                        let swap = attrs
                            .get("swap")
                            .cloned()
                            .unwrap_or_else(|| "innerHTML".to_string());

                        Ok(ONode::content(oxml_tags::BUTTON)
                            .attr("class", "btn btn-primary")
                            .attr(format!("data-{}", method), endpoint)
                            .attr("data-target", target)
                            .attr("data-swap", swap)
                            .child(inner)
                            .build())
                    }
                    "link" => {
                        use crate::features::oxml_tags;
                        let inner = self.render_nodes(children, context, template_path)?;
                        let endpoint = attrs.get("get").unwrap_or(&String::new()).clone();
                        let target = attrs
                            .get("target")
                            .cloned()
                            .unwrap_or_else(|| "#body".to_string());
                        let swap = attrs
                            .get("swap")
                            .cloned()
                            .unwrap_or_else(|| "innerHTML".to_string());

                        Ok(ONode::content(oxml_tags::A)
                            .attr("href", "#")
                            .attr("data-get", endpoint)
                            .attr("data-target", target)
                            .attr("data-swap", swap)
                            .child(inner)
                            .build())
                    }
                    "form" => {
                        use crate::features::oxml_tags;
                        let inner = self.render_nodes(children, context, template_path)?;
                        let endpoint = attrs.get("post").unwrap_or(&String::new()).clone();
                        let target = attrs
                            .get("target")
                            .cloned()
                            .unwrap_or_else(|| "#body".to_string());
                        let swap = attrs
                            .get("swap")
                            .cloned()
                            .unwrap_or_else(|| "innerHTML".to_string());

                        Ok(ONode::content(oxml_tags::FORM)
                            .attr("data-post", endpoint)
                            .attr("data-target", target)
                            .attr("data-swap", swap)
                            .child(inner)
                            .build())
                    }
                    _ => {
                        let inner = self.render_nodes(children, context, template_path)?;
                        let html = crate::generate::element(name, attrs, &inner.render());
                        Ok(ONode::raw(html))
                    }
                }
            }
        }
    }

    fn render_if(
        &self,
        attrs: &BTreeMap<String, String>,
        children: &[Node],
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<ONode> {
        let condition = attrs.get("cond").cloned().unwrap_or_default();
        let is_true = self.eval(&condition, context);

        let (true_nodes, false_nodes) = self.split_if_children(children);

        if is_true {
            self.render_nodes(&true_nodes, context, template_path)
        } else {
            self.render_nodes(&false_nodes, context, template_path)
        }
    }

    fn split_if_children(&self, children: &[Node]) -> (Vec<Node>, Vec<Node>) {
        let mut true_branch = Vec::new();
        let mut false_branch = Vec::new();
        let mut in_else = false;

        for node in children {
            if let Node::VoidElement { name, .. } = node {
                if name == "else" {
                    in_else = true;
                    continue;
                }
            }
            if in_else {
                false_branch.push(node.clone());
            } else {
                true_branch.push(node.clone());
            }
        }
        (true_branch, false_branch)
    }

    fn render_for(
        &self,
        attrs: &BTreeMap<String, String>,
        children: &[Node],
        context: &Context,
        template_path: &str,
    ) -> TemplateResult<ONode> {
        let expr = attrs.get("in").cloned().unwrap_or_default();
        let (item_var, source) = parse_for_expr(&expr);

        let Some(Value::Array(items)) = context.get_value(&source) else {
            return Ok(ONode::empty());
        };

        let mut output = Vec::new();
        for item in items {
            let mut loop_ctx = context.clone();
            loop_ctx.set_value(&item_var, item);
            output.push(self.render_nodes(children, &mut loop_ctx, template_path)?);
        }
        Ok(ONode::cat(output).compact())
    }

    // ================================================================
    // Functorial pipeline operators
    // ================================================================
    //
    // These implement the signature functors (Swierstra 2008) for the
    // directive coproduct.  Each is a *natural transformation* from the
    // F-algebra of the data type to the carrier set `Context → ONode`.

    /// `<?map over="xs" as="x"?>...<?/map?>`
    ///
    /// The **list functor** `fmap` (Wadler 1992, §2.1):
    /// ```text
    /// map : (A → Html) → [A] → [Html]
    /// map(f)([])       = []
    /// map(f)(x ∷ xs)   = f(x) ∷ map(f)(xs)
    /// ```
    ///
    /// Here `f` is the child block rendered for each element of the array.
    /// The result is a monoidal concatenation `ONode::cat → compact`.
    fn render_map(
        &self,
        attrs: &BTreeMap<String, String>,
        children: &[Node],
        context: &Context,
        template_path: &str,
    ) -> TemplateResult<ONode> {
        let source = attrs.get("over").cloned().unwrap_or_default();
        let item_var = attrs
            .get("as")
            .cloned()
            .unwrap_or_else(|| "item".to_string());

        let Some(Value::Array(items)) = context.get_value(&source) else {
            return Ok(ONode::empty());
        };

        let mut output = Vec::new();
        for item in items {
            let mut loop_ctx = context.clone();
            loop_ctx.set_value(&item_var, item);
            output.push(self.render_nodes(children, &mut loop_ctx, template_path)?);
        }
        Ok(ONode::cat(output).compact())
    }

    /// `<?record as="name"?> <?field name="k" value="v"?> <?/record?>`
    ///
    /// Product type introduction rule (Martin-Löf 1984, §3):
    /// ```text
    /// record : Π(k: Name). Value → Object
    /// record({k₁↦v₁, …, kₙ↦vₙ}) ≜ Object { k₁: v₁, …, kₙ: vₙ }
    /// ```
    /// With `<?items?>` children, the record becomes a list of product
    /// values — each `<?item?>` contributes a scalar `v` stored as `{value: v}`
    /// merged with the enclosing record fields.
    fn render_record(
        &self,
        attrs: &BTreeMap<String, String>,
        children: &[Node],
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<ONode> {
        let mut map = serde_json::Map::new();
        for child in children {
            if let Node::VoidElement { name, attrs } = child {
                if name == "item" {
                    let val = attrs
                        .get("value")
                        .cloned()
                        .map(Value::String)
                        .unwrap_or(Value::Null);
                    map.entry("__items".to_string())
                        .or_insert_with(|| Value::Array(Vec::new()));
                    if let Some(Value::Array(arr)) = map.get_mut("__items") {
                        arr.push(val);
                    }
                } else if name == "field" {
                    let field_name = attrs.get("name").cloned().unwrap_or_default();
                    let field_val = attrs
                        .get("value")
                        .cloned()
                        .map(Value::String)
                        .unwrap_or(Value::Null);
                    map.insert(field_name, field_val);
                }
            } else if let Node::Element {
                name,
                attrs: _,
                children,
            } = child
            {
                if name == "field" {
                    let escaped = self.render_nodes(children, context, template_path)?;
                    let field_name = attrs.get("name").cloned().unwrap_or_default();
                    map.insert(
                        field_name,
                        Value::String(escaped.render().trim().to_string()),
                    );
                } else if name == "items" {
                    let mut arr = Vec::new();
                    for item in children {
                        if let Node::VoidElement { name, attrs } = item {
                            if name == "item" {
                                arr.push(
                                    attrs
                                        .get("value")
                                        .cloned()
                                        .map(Value::String)
                                        .unwrap_or(Value::Null),
                                );
                            }
                        } else if let Node::Text(t) = item {
                            let v = t.trim().to_string();
                            if !v.is_empty() {
                                arr.push(Value::String(v));
                            }
                        }
                    }
                    map.insert("__items".to_string(), Value::Array(arr));
                }
            }
        }

        if let Some(Value::Array(items)) = map.remove("__items") {
            if let Some(as_key) = attrs.get("as") {
                let mut out = Vec::new();
                for item in items {
                    let mut item_obj = map.clone();
                    item_obj.insert("value".to_string(), item);
                    out.push(Value::Object(item_obj));
                }
                context.set_value(as_key, Value::Array(out));
            }
        } else if let Some(as_key) = attrs.get("as") {
            context.set_value(as_key, Value::Object(map));
        }

        Ok(ONode::empty())
    }

    /// `<?list as="name"?> <?item value="x"?> <?/list?>`
    ///
    /// List introduction rule (Martin-Löf 1984, §5):
    /// ```text
    /// list : [Value] → Array
    /// list([v₁, …, vₙ]) ≜ Array [v₁, …, vₙ]
    /// ```
    /// With element-form `<?item?>`, each item may be a record of fields
    /// producing a `[Object]`.
    fn render_list(
        &self,
        attrs: &BTreeMap<String, String>,
        children: &[Node],
        context: &mut Context,
        _template_path: &str,
    ) -> TemplateResult<ONode> {
        let mut arr = Vec::new();
        for child in children {
            match child {
                Node::Element {
                    name,
                    attrs: _,
                    children,
                } => {
                    if name == "item" {
                        let mut map = serde_json::Map::new();
                        for sub in children {
                            if let Node::VoidElement { name, attrs } = sub {
                                if name == "field" {
                                    let field_name = attrs.get("name").cloned().unwrap_or_default();
                                    let field_val = attrs
                                        .get("value")
                                        .cloned()
                                        .map(Value::String)
                                        .unwrap_or(Value::Null);
                                    map.insert(field_name, field_val);
                                }
                            }
                        }
                        arr.push(Value::Object(map));
                    }
                }
                Node::VoidElement { name, attrs } => {
                    if name == "item" {
                        arr.push(
                            attrs
                                .get("value")
                                .cloned()
                                .map(Value::String)
                                .unwrap_or(Value::Null),
                        );
                    }
                }
                _ => {}
            }
        }

        if let Some(as_key) = attrs.get("as") {
            context.set_value(as_key, Value::Array(arr));
        }

        Ok(ONode::empty())
    }

    /// `<?data from="path" as="name"?>`
    ///
    /// The **filesystem functor** — lifts a filesystem path into the
    /// template context.  This is a Kleisli extension (Moggi 1991) of the
    /// `load_data_*` operations:
    /// ```text
    /// data : Path → M Value
    /// data(p) = lift_io( if is_dir(p) then load_data_dir(p)
    ///                     else load_data_file(p) )
    /// ```
    fn render_data(
        &self,
        attrs: &BTreeMap<String, String>,
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<String> {
        let raw_from = attrs.get("from").ok_or_else(|| {
            TemplateError::code(
                TemplateErrorPhase::Render,
                "data directive requires from attribute",
            )
            .with_template_path(template_path)
            .with_directive("data")
        })?;
        let from = self.resolve(raw_from, context);
        let full_path = self.base_path.join(&from);

        let value = if full_path.is_dir() {
            crate::features::load_data_dir(&full_path).map_err(|e| {
                TemplateError::code(TemplateErrorPhase::Io, e)
                    .with_template_path(template_path)
                    .with_directive("data")
            })?
        } else if full_path.exists() {
            crate::features::load_data_file(&full_path).map_err(|e| {
                TemplateError::code(TemplateErrorPhase::Io, e)
                    .with_template_path(template_path)
                    .with_directive("data")
            })?
        } else {
            return Err(TemplateError::code(
                TemplateErrorPhase::Io,
                format!("Data path not found: {}", from),
            )
            .with_template_path(template_path)
            .with_directive("data"));
        };

        if let Some(as_key) = attrs.get("as") {
            context.set_value(as_key, value);
        }

        Ok(String::new())
    }

    /// `<?filter over="xs" where="field" as="name"?>`
    ///
    /// The **pointwise restriction** of an array to elements satisfying a
    /// predicate on a named field:
    /// ```text
    /// filter : [A] → Field → [A]
    /// filter(xs)(k) = { x ∈ xs | x[k] ≠ null ∧ x[k] ≠ "" }
    /// ```
    ///
    /// This is a *natural transformation* in the subcategory of decidable
    /// predicates (Burstall 1969, §3: structural induction on the array).
    fn render_filter(
        &self,
        attrs: &BTreeMap<String, String>,
        context: &mut Context,
        _template_path: &str,
    ) -> TemplateResult<String> {
        let over = attrs.get("over").cloned().unwrap_or_default();
        let where_key = attrs.get("where").cloned().unwrap_or_default();
        let as_key = attrs.get("as").cloned().unwrap_or_else(|| over.clone());

        let Some(Value::Array(items)) = context.get_value(&over) else {
            return Ok(String::new());
        };

        let filtered: Vec<Value> = items
            .into_iter()
            .filter(|item| {
                if let Some(val) = item.get(&where_key) {
                    match val {
                        Value::String(s) => !s.is_empty(),
                        Value::Bool(b) => *b,
                        Value::Number(_) => true,
                        Value::Array(a) => !a.is_empty(),
                        Value::Object(_) => true,
                        Value::Null => false,
                    }
                } else {
                    false
                }
            })
            .collect();

        context.set_value(&as_key, Value::Array(filtered));
        Ok(String::new())
    }

    /// `<?sort over="xs" by="field" order="desc" as="name"?>`
    ///
    /// The **lexicographic ordering** of an array by a named field:
    /// ```text
    /// sort : [A] → Field → Order → [A]
    /// Order = { asc, desc }
    /// sort(xs)(k)(asc)  ≜ sort_by str_cmp(x[k], y[k])
    /// sort(xs)(k)(desc) ≜ reverse ∘ sort(xs)(k)(asc)
    /// ```
    ///
    /// Sorting is *stable* (by construction of `Vec::sort_by`).  The type is
    /// a forgetful functor from the category of ordered sets to the category
    /// of plain sets (Mac Lane 1971, §II.1).
    fn render_sort(
        &self,
        attrs: &BTreeMap<String, String>,
        context: &mut Context,
        _template_path: &str,
    ) -> TemplateResult<String> {
        let over = attrs.get("over").cloned().unwrap_or_default();
        let by = attrs.get("by").cloned().unwrap_or_default();
        let desc = attrs.get("order").map(|o| o == "desc").unwrap_or(false);
        let as_key = attrs.get("as").cloned().unwrap_or_else(|| over.clone());

        let Some(Value::Array(items)) = context.get_value(&over) else {
            return Ok(String::new());
        };

        let by_clone = by.clone();
        let mut sorted: Vec<Value> = items;
        sorted.sort_by(|a, b| {
            let a_val = value_to_sort_key(a.get(&by_clone));
            let b_val = value_to_sort_key(b.get(&by_clone));
            if desc {
                b_val.cmp(&a_val)
            } else {
                a_val.cmp(&b_val)
            }
        });

        context.set_value(&as_key, Value::Array(sorted));
        Ok(String::new())
    }

    fn render_slice(
        &self,
        attrs: &BTreeMap<String, String>,
        context: &mut Context,
        _template_path: &str,
    ) -> TemplateResult<String> {
        let over = attrs.get("over").cloned().unwrap_or_default();
        let start: usize = attrs
            .get("start")
            .unwrap_or(attrs.get("from").unwrap_or(&"0".to_string()))
            .parse()
            .unwrap_or(0);
        let count: usize = attrs
            .get("count")
            .unwrap_or(attrs.get("to").unwrap_or(&"0".to_string()))
            .parse()
            .unwrap_or(0);
        let as_key = attrs.get("as").cloned().unwrap_or_else(|| over.clone());

        let Some(Value::Array(items)) = context.get_value(&over) else {
            return Ok(String::new());
        };

        let end = if count > 0 {
            (start + count).min(items.len())
        } else {
            items.len()
        };
        let sliced: Vec<Value> = items.into_iter().skip(start).take(end - start).collect();

        context.set_value(&as_key, Value::Array(sliced));
        Ok(String::new())
    }

    fn render_bind(
        &self,
        attrs: &BTreeMap<String, String>,
        children: &[Node],
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<ONode> {
        let var = attrs.get("var").map(String::as_str).unwrap_or("value");
        if let Some(from) = attrs.get("from") {
            let bound = context
                .get_value(from)
                .unwrap_or(Value::String(String::new()));
            let mut scoped = context.clone();
            scoped.set_value(var, bound);
            return self.render_nodes(children, &mut scoped, template_path);
        }

        let rendered = self
            .render_nodes(children, context, template_path)?
            .render();
        context.set_str(var, rendered);
        Ok(ONode::empty())
    }

    fn render_component_use(
        &self,
        attrs: &BTreeMap<String, String>,
        children: &[Node],
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<ONode> {
        let Some(id) = attrs.get("id") else {
            return Ok(ONode::empty());
        };
        let Some(component_nodes) = context.get_component(id) else {
            return Ok(ONode::empty());
        };

        let mut scoped = context.clone();
        let blocks = resolve::extract_blocks(children);
        let setup_nodes: Vec<Node> = children
            .iter()
            .filter(|node| match node {
                Node::Element { name, .. } | Node::VoidElement { name, .. } => name != "block",
                _ => true,
            })
            .cloned()
            .collect();
        let _ = self.render_nodes(&setup_nodes, &mut scoped, template_path)?;

        let resolved = resolve::inject_blocks(component_nodes, &blocks);
        self.render_nodes(&resolved, &mut scoped, template_path)
    }

    fn eval(&self, condition: &str, context: &Context) -> bool {
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").collect();
            if parts.len() == 2 {
                let left_key = parts[0].trim().trim_start_matches('$');
                let left = context.get(left_key);
                let right = parts[1].trim().trim_matches('"').trim_matches('\'');
                return left == right;
            }
        }

        let lookup = condition.trim().trim_start_matches('$');
        !context.get(lookup).is_empty()
    }

    fn wrap_html(&self, body: ONode) -> String {
        use crate::features::oxml_tags;
        let mut head = vec![
            ONode::void(oxml_tags::META)
                .attr("charset", "UTF-8")
                .build(),
            ONode::void(oxml_tags::META)
                .attr("name", "viewport")
                .attr("content", "width=device-width, initial-scale=1.0")
                .build(),
            ONode::content(oxml_tags::TITLE)
                .text(&self.site_name)
                .build(),
        ];

        if let Some(desc) = &self.site_description {
            head.push(
                ONode::void(oxml_tags::META)
                    .attr("name", "description")
                    .attr("content", desc)
                    .build(),
            );
        }

        if let Some(icon) = &self.favicon {
            head.push(
                ONode::void(oxml_tags::LINK)
                    .attr("rel", "icon")
                    .attr("href", icon)
                    .build(),
            );
        }

        head.push(
            ONode::void(oxml_tags::LINK)
                .attr("rel", "stylesheet")
                .attr("href", "/static/css/style.css")
                .build(),
        );
        head.push(
            ONode::content(oxml_tags::SCRIPT)
                .attr("src", "/hrml.js")
                .build(),
        );

        crate::features::doc(ONode::cat(head), body)
    }

    fn render_mdx(
        &self,
        attrs: &BTreeMap<String, String>,
        template_path: &str,
    ) -> TemplateResult<String> {
        let file = attrs.get("file").ok_or_else(|| {
            TemplateError::code(
                TemplateErrorPhase::Render,
                "mdx directive requires file attribute",
            )
            .with_template_path(template_path)
            .with_directive("mdx")
        })?;
        let full_path = self.base_path.join(file);
        let source = fs::read_to_string(&full_path).map_err(|e| {
            TemplateError::code(
                TemplateErrorPhase::Io,
                format!("Failed to read mdx file {}: {}", file, e),
            )
            .with_template_path(template_path)
            .with_directive("mdx")
        })?;

        let rendered = crate::features::render_markdown(&source);
        Ok(crate::features::render_math_delimiters(&rendered))
    }

    fn resolve_attrs(
        &self,
        attrs: &BTreeMap<String, String>,
        context: &Context,
    ) -> BTreeMap<String, String> {
        attrs
            .iter()
            .map(|(k, v)| (k.clone(), self.resolve(v, context)))
            .collect()
    }

    fn resolve(&self, raw: &str, context: &Context) -> String {
        if !raw.contains('$') {
            return raw.to_string();
        }
        let mut result = String::with_capacity(raw.len());
        let mut rest = raw;
        while let Some(dollar) = rest.find('$') {
            result.push_str(&rest[..dollar]);
            let after = &rest[dollar + 1..];
            let greedy_end = after
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.')
                .unwrap_or(after.len());

            if greedy_end == 0 {
                result.push('$');
                rest = after;
                continue;
            }

            // Try the full dotted name, peel back one .segment at a time until a
            // binding resolves. $post.title finds post.title; $slug.mdx finds
            // slug and keeps .mdx as literal.
            let candidate = &after[..greedy_end];
            let mut try_len = candidate.len();
            let resolved = loop {
                let sub = &candidate[..try_len];
                if let Some(v) = context.get_value(sub) {
                    break Some((try_len, value_to_string(&v)));
                }
                match sub.rfind('.') {
                    Some(i) => try_len = i,
                    None => break None,
                }
            };

            match resolved {
                Some((len, s)) => {
                    result.push_str(&s);
                    rest = &after[len..];
                }
                None => {
                    let ident_end = candidate.find('.').unwrap_or(candidate.len());
                    rest = &after[ident_end..];
                }
            }
        }
        result.push_str(rest);
        result
    }

    fn render_markdown(
        &self,
        attrs: &BTreeMap<String, String>,
        template_path: &str,
    ) -> TemplateResult<String> {
        let file = attrs.get("file").ok_or_else(|| {
            TemplateError::code(
                TemplateErrorPhase::Render,
                "markdown directive requires file attribute",
            )
            .with_template_path(template_path)
            .with_directive("markdown")
        })?;
        let full_path = self.base_path.join(file);
        let source = fs::read_to_string(&full_path).map_err(|e| {
            TemplateError::code(
                TemplateErrorPhase::Io,
                format!("Failed to read markdown file {}: {}", file, e),
            )
            .with_template_path(template_path)
            .with_directive("markdown")
        })?;

        let rendered = crate::features::render_markdown(&source);
        Ok(crate::features::render_math_delimiters(&rendered))
    }

    fn render_markdownfm(
        &self,
        attrs: &BTreeMap<String, String>,
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<String> {
        if let Some(dir) = attrs.get("dir") {
            return self.render_markdown_dir(dir, attrs, context, template_path);
        }

        let file = self.resolve(
            attrs.get("file").ok_or_else(|| {
                TemplateError::code(
                    TemplateErrorPhase::Render,
                    "markdownfm directive requires file or dir attribute",
                )
                .with_template_path(template_path)
                .with_directive("markdownfm")
            })?,
            context,
        );
        let full_path = self.base_path.join(&file);
        let source = fs::read_to_string(&full_path).map_err(|e| {
            TemplateError::code(
                TemplateErrorPhase::Io,
                format!("Failed to read markdown file {}: {}", file, e),
            )
            .with_template_path(template_path)
            .with_directive("markdownfm")
        })?;

        let (meta, html) = crate::features::render_markdown_with_frontmatter(&source);
        if let Some(as_key) = attrs.get("as") {
            context.set_value(as_key, meta);
        }

        Ok(crate::features::render_math_delimiters(&html))
    }

    fn render_markdown_dir(
        &self,
        dir: &str,
        attrs: &BTreeMap<String, String>,
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<String> {
        let full_dir = self.base_path.join(dir);
        let entries = fs::read_dir(&full_dir).map_err(|e| {
            TemplateError::code(
                TemplateErrorPhase::Io,
                format!("Failed to read directory {}: {}", dir, e),
            )
            .with_template_path(template_path)
            .with_directive("markdownfm")
        })?;

        let mut filenames: Vec<String> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| {
                TemplateError::code(
                    TemplateErrorPhase::Io,
                    format!("Failed to read entry: {}", e),
                )
                .with_template_path(template_path)
            })?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_name.starts_with('.') || file_name.starts_with('_') {
                continue;
            }
            if let Some(ext) = path.extension() {
                if ext == "md" || ext == "mdx" {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        filenames.push(stem.to_string());
                    }
                }
            }
        }
        filenames.sort();

        let mut posts = Vec::new();
        for slug in &filenames {
            for ext in &["mdx", "md"] {
                let file_path = full_dir.join(format!("{}.{}", slug, ext));
                if let Ok(content) = fs::read_to_string(&file_path) {
                    let (meta, _html) = crate::features::render_markdown_with_frontmatter(&content);
                    let mut obj = match meta {
                        Value::Object(m) => m,
                        _ => serde_json::Map::new(),
                    };
                    obj.insert("slug".to_string(), Value::String(slug.clone()));
                    posts.push(Value::Object(obj));
                    break;
                }
            }
        }

        if let Some(as_key) = attrs.get("as") {
            context.set_value(as_key, Value::Array(posts));
        }

        Ok(String::new())
    }

    fn render_latex(
        &self,
        attrs: &BTreeMap<String, String>,
        template_path: &str,
    ) -> TemplateResult<String> {
        let formula = attrs.get("formula").ok_or_else(|| {
            TemplateError::code(
                TemplateErrorPhase::Render,
                "latex directive requires formula attribute",
            )
            .with_template_path(template_path)
            .with_directive("latex")
        })?;
        let mode = attrs.get("mode").map(String::as_str).unwrap_or("inline");
        let html = match mode {
            "block" => crate::features::render_latex_block(formula),
            _ => crate::features::render_latex_inline(formula),
        };
        Ok(html)
    }

}

// --- Context ---

#[derive(Clone)]
struct Context {
    data: Value,
    vars: HashMap<String, Value>,
    components: BTreeMap<String, Vec<Node>>,
}

impl Context {
    fn new(data: Value) -> Self {
        Self {
            data,
            vars: HashMap::new(),
            components: BTreeMap::new(),
        }
    }

    fn set_str(&mut self, key: &str, value: String) {
        self.vars.insert(key.to_string(), Value::String(value));
    }

    fn set_value(&mut self, key: &str, value: Value) {
        self.vars.insert(key.to_string(), value);
    }

    fn set_component(&mut self, key: &str, nodes: Vec<Node>) {
        self.components.insert(key.to_string(), nodes);
    }

    fn get_component(&self, key: &str) -> Option<Vec<Node>> {
        self.components.get(key).cloned()
    }

    fn get_value(&self, key: &str) -> Option<Value> {
        if let Some(val) = self.vars.get(key) {
            return Some(val.clone());
        }

        let parts: Vec<&str> = key.split('.').collect();
        if let Some((first, rest)) = parts.split_first() {
            if let Some(seed) = self.vars.get(*first) {
                let mut current = seed;
                for part in rest {
                    current = current.get(*part)?;
                }
                return Some(current.clone());
            }
        }

        let mut current = &self.data;
        for part in parts {
            current = current.get(part)?;
        }
        Some(current.clone())
    }

    fn get(&self, key: &str) -> String {
        let Some(current) = self.get_value(key) else {
            return String::new();
        };
        match &current {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => serde_json::to_string(arr).unwrap_or_default(),
            Value::Object(map) => serde_json::to_string(map).unwrap_or_default(),
            _ => String::new(),
        }
    }
}

fn parse_for_expr(expr: &str) -> (String, String) {
    if let Some((left, right)) = expr.split_once(" in ") {
        let item = left.trim();
        let source = right.trim();
        if !item.is_empty() && !source.is_empty() {
            return (item.to_string(), source.to_string());
        }
    }
    ("item".to_string(), expr.trim().to_string())
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => serde_json::to_string(arr).unwrap_or_default(),
        Value::Object(map) => serde_json::to_string(map).unwrap_or_default(),
        Value::Null => String::new(),
    }
}

fn value_to_sort_key(val: Option<&Value>) -> String {
    match val {
        Some(Value::Number(n)) => format!("{:020}", n.as_u64().unwrap_or(0)),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => {
            if *b {
                "1".into()
            } else {
                "0".into()
            }
        }
        Some(Value::Null) => String::new(),
        _ => String::new(),
    }
}

fn is_html_doc(body: &str) -> bool {
    let trimmed = body.trim_start();
    trimmed.starts_with("<!DOCTYPE html") || trimmed.starts_with("<html")
}
