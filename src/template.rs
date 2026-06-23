//! The HRML template engine - an algebraically closed evaluator for the
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
//!   types).  The `Node` type is an *inductive sum-of-products* - each
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
//!   composition* pattern applied to template rendering - each stage produces
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
//!             |  Object(Map String Value)          - product type
//!             |  Array(Vec Value)                  - list type
//!
//! Node       ≜  Text(String)
//!             |  VoidElement { name: String, attrs: Map String String }
//!             |  Element   { name: String, attrs: ..., children: [Node] }
//!             |  Load      { file: String, blocks: Map String [Node] }
//!             - inductive sum-of-products (Pierce §11)
//!
//! Attr       ≜  Map String String                   - association list
//!
//! Dir ::=  data  (from: Path,  as: Name)
//!        | map   (over: Name, as: Name)  { block }
//!        | filter(over: Name, where: Field, as: Name)
//!        | sort  (over: Name, by: Field, order: {asc,desc}, as: Name)
//!        | slice (over: Name, start: Nat, count: Nat, as: Name)
//!        | record(as: Name) { field | items }
//!        | list  (as: Name) { item* }
//!             - signature functors (Swierstra 2008)
//!
//! Context    ≜  { data: Value, vars: Map String Value,
//!                 components: Map String [Node], load_stack: [String] }
//!             - reader monad: TemplateResult ≜ Context → Result Html Error
//!
//! render     :  Path → Value → Context → Html
//! render_node:  Node → Context → Html                 - structural recursion
//! render_map :  [A] → (A → Html) → [Html]              - list functor fmap
//! eval_pred  :  Cond → Value → Bool                    - semantic predicate
//! ```
//!
//! ## Structural induction scheme
//!
//! Correctness of the evaluator is established by induction on `Node`:
//! ```text
//! Base cases:
//!   • render_node(Text(t)) ⇝ t                     - identity on literals
//!   • render_node(VoidElement(d, attrs)) ⇝ H_d(attrs) - directive handler
//!
//! Inductive step:
//!   • render_node(Element(name, attrs, children))
//!     ≜  render_nodes(children) ∘ handler(name, attrs)   - compositional
//!
//! Load induction:
//!   • render_node(Load(file, blocks))
//!     ≜  inject_blocks(resolve(file), blocks)             - substitution
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
use self::parser::hparser::HTML_TAG_PREFIX;
use self::parser::ParseTree;

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

/// One pass over a resolved tree that partitions `<?style?>` blocks and `<?use?>`
/// references by owner - each `<?component?>` versus the page body - so CSS can
/// be tree-shaken to the components a page actually reaches.
#[derive(Default)]
struct StyleIndex {
    /// component id → its own `<?style?>` blocks (verbatim CSS).
    comp_css: BTreeMap<String, Vec<String>>,
    /// component id → component ids it `<?use?>`s in its body.
    comp_uses: BTreeMap<String, Vec<String>>,
    /// components in first-seen (definition) order, for a stable cascade.
    comp_order: Vec<String>,
    /// `<?style?>` blocks written directly in the page body.
    page_css: Vec<String>,
    /// component ids the page body `<?use?>`s - the tree-shake roots.
    page_uses: Vec<String>,
    /// component id → literal class tokens in its markup (utility candidates).
    comp_classes: BTreeMap<String, std::collections::BTreeSet<String>>,
    /// literal class tokens in the page body's markup.
    page_classes: std::collections::BTreeSet<String>,
}

impl StyleIndex {
    fn walk(&mut self, nodes: &[Node], owner: Option<&str>) {
        for node in nodes {
            match node {
                Node::Element {
                    name,
                    attrs,
                    children,
                } => match name.as_str() {
                    "component" => match attrs.get("id") {
                        Some(id) => {
                            if !self.comp_order.contains(id) {
                                self.comp_order.push(id.clone());
                            }
                            self.walk(children, Some(id));
                        }
                        None => self.walk(children, owner),
                    },
                    "style" => {
                        let mut css = String::new();
                        for child in children {
                            if let Node::Text(t) = child {
                                css.push_str(t);
                            }
                        }
                        match owner {
                            Some(id) => self.comp_css.entry(id.to_string()).or_default().push(css),
                            None => self.page_css.push(css),
                        }
                    }
                    "use" => {
                        self.record_use(attrs, owner);
                        self.walk(children, owner);
                    }
                    _ => {
                        self.record_classes(attrs, owner);
                        self.walk(children, owner);
                    }
                },
                Node::VoidElement { name, attrs } => {
                    if name == "use" {
                        self.record_use(attrs, owner);
                    } else {
                        self.record_classes(attrs, owner);
                    }
                }
                _ => {}
            }
        }
    }

    /// Collect literal class tokens (dynamic `$…` segments can never name a
    /// utility, so they are skipped).
    fn record_classes(&mut self, attrs: &BTreeMap<String, String>, owner: Option<&str>) {
        let Some(class) = attrs.get("class") else {
            return;
        };
        let tokens = class
            .split_whitespace()
            .filter(|t| !t.contains('$'))
            .map(String::from);
        match owner {
            Some(id) => self
                .comp_classes
                .entry(id.to_string())
                .or_default()
                .extend(tokens),
            None => self.page_classes.extend(tokens),
        }
    }

    fn record_use(&mut self, attrs: &BTreeMap<String, String>, owner: Option<&str>) {
        if let Some(id) = attrs.get("id") {
            match owner {
                Some(c) => self
                    .comp_uses
                    .entry(c.to_string())
                    .or_default()
                    .push(id.clone()),
                None => self.page_uses.push(id.clone()),
            }
        }
    }
}

/// Recursively list `.hrml`/`.trml` files under `dir`, sorted by path so the
/// resulting component-registration order is deterministic.
fn template_files_under(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(template_files_under(&path));
            } else if path
                .extension()
                .map(|e| e == "hrml" || e == "trml")
                .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Structural/binding directives that stay active when they appear as a direct
/// child of `<?use?>`. Any *other* directive element there is interpreted as a
/// named prop (e.g. `<?title?>…<?/title?>` binds `title`), so this set is the
/// boundary between control flow and user-named component arguments.
///
/// Deliberately excludes head directives (`title`, `description`, …) and pure
/// output directives (`mdx`, `markdown`, …): those carry no meaning as setup
/// children, and their names are exactly the ones authors reach for as props.
const USE_SETUP_DIRECTIVES: &[&str] = &[
    "load",
    "else",
    "set",
    "data",
    "filter",
    "sort",
    "slice",
    "bind",
    "slot",
    "block",
    "use",
    "if",
    "for",
    "map",
    "record",
    "list",
    "field",
    "items",
    "item",
    "component",
];

fn is_use_setup_directive(name: &str) -> bool {
    USE_SETUP_DIRECTIVES.contains(&name)
}

/// A reference-position attribute names a variable to read. `$x` and `x` both
/// denote the variable `x`; the optional sigil keeps reference sites visually
/// consistent with `$x` interpolation. Declaration positions (`var`, `as`,
/// `id` on `set`/`component`) keep bare names.
fn deref<'a>(name: &'a str) -> &'a str {
    name.strip_prefix('$').unwrap_or(name).trim()
}
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

mod ast;
mod error;
mod head;
mod pipeline;
mod predicate;
pub mod resolve;
mod utility;

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
    default_layout: Option<String>,
    auto_imports: Vec<String>,
    component_paths: Vec<String>,
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
            default_layout: None,
            auto_imports: Vec::new(),
            component_paths: vec!["components".to_string()],
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
            default_layout: config.default_layout.clone(),
            auto_imports: config.auto_imports.clone(),
            component_paths: config.component_paths.clone(),
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

    /// The layout wrapping pages that declare no `<?load?>` of their own.
    pub fn with_default_layout(mut self, layout: Option<String>) -> Self {
        self.default_layout = layout;
        self
    }

    /// Files auto-loaded ahead of the default layout for every wrapped page.
    pub fn with_auto_imports(mut self, imports: Vec<String>) -> Self {
        self.auto_imports = imports;
        self
    }

    /// Directories under the base path whose `<?component?>` definitions are
    /// auto-registered for every page (the shared component library).
    pub fn with_component_paths(mut self, paths: Vec<String>) -> Self {
        self.component_paths = paths;
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
        self.collect_styles(nodes, &mut context);
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
        let parsed = parse_with_extension(content, path)?;
        let nodes = resolve::with_default_layout(
            &parsed,
            self.default_layout.as_deref(),
            &self.auto_imports,
        );
        let fetch = |file: &str| parse_with_extension(&self.read_template(file)?, file);
        let mut visited = vec![path.to_string()];
        let resolved = resolve::resolve_loads(&nodes, &fetch, &mut visited, true)?;

        // Prepend the shared component library (every `<?component?>` under a
        // configured component directory), so a page can use any component with
        // no `<?load?>`. Mirrors the in-memory `Project` path.
        let mut out = self.component_library();
        out.extend(resolved);
        Ok(out)
    }

    /// Parse and collect every `<?component?>` definition under the configured
    /// component directories (relative to the base path). Unreadable or
    /// unparseable files are skipped - the library is best-effort discovery,
    /// not a hard dependency of any single page.
    fn component_library(&self) -> Vec<Node> {
        let mut defs = Vec::new();
        for dir in &self.component_paths {
            for file in template_files_under(&self.base_path.join(dir)) {
                if let Ok(text) = fs::read_to_string(&file) {
                    let rel = file.to_string_lossy();
                    if let Ok(nodes) = parse_with_extension(&text, &rel) {
                        resolve::collect_components(&nodes, &mut defs);
                    }
                }
            }
        }
        defs
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
        self.collect_styles(&nodes, &mut context);
        let body_node = self.render_nodes(&nodes, &mut context, path)?;
        let body = body_node.render();
        if !wrap || is_html_doc(&body) {
            Ok(body)
        } else {
            Ok(self.wrap_html(body_node))
        }
    }

    /// Hoist and **tree-shake** CSS for this page. A component owns its
    /// `<?style?>` blocks; a page ships only the styles of the components it
    /// actually instantiates (transitively), plus any page-level `<?style?>`.
    /// `$refs` (e.g. design tokens `$globals.*`) are resolved against the
    /// context, identical blocks de-duplicated, and the result stashed for the
    /// `<?styles?>` head sink to emit once.
    fn collect_styles(&self, nodes: &[Node], context: &mut Context) {
        let mut index = StyleIndex::default();
        index.walk(nodes, None);

        // Components reachable from the page body, following nested uses.
        let mut reachable = std::collections::BTreeSet::new();
        let mut queue = index.page_uses.clone();
        while let Some(id) = queue.pop() {
            if reachable.insert(id.clone()) {
                if let Some(children) = index.comp_uses.get(&id) {
                    queue.extend(children.iter().cloned());
                }
            }
        }

        // Page-level styles first, then each reachable component in the order it
        // was defined (sorted file order, so the cascade is stable).
        let mut raw = index.page_css;
        for id in &index.comp_order {
            if reachable.contains(id) {
                if let Some(blocks) = index.comp_css.get(id) {
                    raw.extend(blocks.iter().cloned());
                }
            }
        }

        let mut seen = std::collections::BTreeSet::new();
        let mut blocks = Vec::new();
        // Design tokens lead the cascade: every `[globals]` key is exposed to
        // CSS as a custom property (`snake_case` → `--kebab-case`), so projects
        // never hand-map config values into a `:root` block.
        if let Some(tokens) = globals_root_block(&self.globals) {
            blocks.push(tokens);
        }
        for css in raw {
            let resolved = self.resolve(&css, context);
            let trimmed = resolved.trim().to_string();
            if !trimmed.is_empty() && seen.insert(trimmed.clone()) {
                blocks.push(trimmed);
            }
        }

        // Utility classes close the cascade: generated from the same reachable
        // set as component CSS, ordered after it so a utility on an element
        // wins ties against the component's own rule.
        let mut classes = index.page_classes;
        for id in &reachable {
            if let Some(set) = index.comp_classes.get(id) {
                classes.extend(set.iter().cloned());
            }
        }
        let utilities = utility::rules(&classes, &self.globals);
        if !utilities.is_empty() {
            blocks.push(utilities.join("\n"));
        }

        context.styles = blocks.join("\n\n");
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
                    // Head sink: emit the CSS hoisted from every used component's
                    // `<?style?>` (assembled by collect_styles), once, or nothing.
                    "styles" => Ok(if context.styles.is_empty() {
                        ONode::empty()
                    } else {
                        ONode::raw(format!("<style>\n{}\n</style>", context.styles))
                    }),
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
                    pipe @ ("filter" | "sort" | "slice" | "tally" | "concat") => {
                        Ok(self.render_pipeline(pipe, attrs, context))
                    }
                    "replace" => self.render_replace(attrs, context, template_path),
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
                        let var = attrs.get("var").map(String::as_str).unwrap_or("value");
                        if let Some(from) = attrs.get("from") {
                            let bound = context
                                .get_value(deref(from))
                                .unwrap_or(Value::String(String::new()));
                            context.set_value(var, bound);
                        } else if let Some(default) = attrs.get("default") {
                            // A declared default fills the prop only when the caller
                            // passed nothing - so components carry their own fallbacks
                            // and call sites omit the common case.
                            if context.get(var).is_empty() {
                                context.set_str(var, self.resolve(default, context));
                            }
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
                            Ok(ONode::raw(context.get(deref(id))))
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
                    // A `<?style?>` block is hoisted to the `<?styles?>` head sink
                    // by collect_styles; it emits nothing where it is written.
                    "style" => Ok(ONode::empty()),
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

        let Some(Value::Array(items)) = context.get_value(deref(&source)) else {
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
        let source = deref(&attrs.get("over").cloned().unwrap_or_default()).to_string();
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
    /// values - each `<?item?>` contributes a scalar `v` stored as `{value: v}`
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
    /// The **filesystem functor** - lifts a filesystem path into the
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
    /// The shared Kleisli wrapper for the array transforms `<?filter?>`,
    /// `<?sort?>`, `<?slice?>`: read the `over` array, apply the pure
    /// [`pipeline`] transform, bind the result to `as` (defaulting to `over`).
    fn render_pipeline(
        &self,
        name: &str,
        attrs: &BTreeMap<String, String>,
        context: &mut Context,
    ) -> ONode {
        let over = deref(&attrs.get("over").cloned().unwrap_or_default()).to_string();
        let as_key = attrs.get("as").cloned().unwrap_or_else(|| over.clone());

        if let Some(Value::Array(mut items)) = context.get_value(&over) {
            if name == "concat" {
                // Binary, so it lives here where the second operand can be
                // resolved against the context: <?concat over="a" with="b"?>.
                if let Some(with) = attrs.get("with") {
                    if let Some(Value::Array(more)) = context.get_value(deref(with)) {
                        items.extend(more);
                    }
                }
                context.set_value(&as_key, Value::Array(items));
            } else if let Some(out) = pipeline::transform(name, items, attrs) {
                context.set_value(&as_key, Value::Array(out));
            }
        }
        ONode::empty()
    }

    /// `<?replace over="path" match="regex" with="replacement" as="name"?>` —
    /// the string counterpart of the array pipes: a regex endomorphism on a
    /// string binding. `match` and `with` are taken raw (no `$` resolution),
    /// so `$1`…`$N` in `with` are capture-group references.
    fn render_replace(
        &self,
        attrs: &BTreeMap<String, String>,
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<ONode> {
        let over = deref(&attrs.get("over").cloned().unwrap_or_default()).to_string();
        let as_key = attrs.get("as").cloned().unwrap_or_else(|| over.clone());
        let pattern = attrs.get("match").map(String::as_str).unwrap_or_default();
        let replacement = attrs.get("with").map(String::as_str).unwrap_or_default();

        let re = regex::Regex::new(pattern).map_err(|e| {
            TemplateError::code(
                TemplateErrorPhase::Render,
                format!("replace: invalid regex {:?}: {}", pattern, e),
            )
            .with_template_path(template_path)
            .with_directive("replace")
        })?;

        if let Some(Value::String(s)) = context.get_value(&over) {
            let out = re.replace_all(&s, replacement).into_owned();
            context.set_value(&as_key, Value::String(out));
        }
        Ok(ONode::empty())
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
                .get_value(deref(from))
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

        // Attribute props: every `<?use?>` attribute other than `id` is a scalar
        // prop, resolved and bound. This is how a call site passes overrides —
        // `<?use id="card" class="featured" style="--card-accent:#c00">` - that a
        // component spreads onto its root (inline custom properties scope the
        // override to that one instance). Named-tag children (below) override
        // attributes for the same name, since richer content wins.
        for (key, raw) in attrs {
            if key != "id" {
                let value = self.resolve(raw, &scoped);
                scoped.set_str(key, value);
            }
        }

        // Children of `<?use?>` fall into three roles: `<?block?>` fills a named
        // slot (handled above), a *reserved* directive runs as ordinary setup,
        // and any other directive `<?name?>…<?/name?>` is a named prop - sugar
        // for `<?bind var="name">…</?bind>`.
        for node in children {
            match node {
                Node::Element { name, children, .. }
                    if !is_html_node(name) && name != "block" && !is_use_setup_directive(name) =>
                {
                    let value = self
                        .render_nodes(children, &mut scoped, template_path)?
                        .render();
                    scoped.set_str(name, value);
                }
                Node::VoidElement { name, attrs }
                    if !is_html_node(name) && !is_use_setup_directive(name) =>
                {
                    let value = attrs
                        .get("from")
                        .and_then(|f| scoped.get_value(deref(f)))
                        .map(|v| value_to_string(&v))
                        .unwrap_or_default();
                    scoped.set_str(name, value);
                }
                Node::Element { name, .. } if name == "block" => {}
                _ => {
                    let _ = self.render_node(node, &mut scoped, template_path)?;
                }
            }
        }

        let resolved = resolve::inject_blocks(component_nodes, &blocks);
        self.render_nodes(&resolved, &mut scoped, template_path)
    }

    fn eval(&self, condition: &str, context: &Context) -> bool {
        predicate::eval(condition, &|path| {
            context
                .get_value(path)
                .map(|v| truthy_str(&v))
                .unwrap_or_default()
        })
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
    /// CSS hoisted from `<?style?>` blocks, resolved and de-duplicated, ready to
    /// be emitted once at the `<?styles?>` head sink.
    styles: String,
}

impl Context {
    fn new(data: Value) -> Self {
        Self {
            data,
            vars: HashMap::new(),
            components: BTreeMap::new(),
            styles: String::new(),
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

        // A var named by the first segment shadows the data root; the rest of
        // the path projects into it (object fields, numeric array indices).
        if let Some((first, rest)) = key.split_once('.') {
            if let Some(seed) = self.vars.get(first) {
                return project(seed, rest).cloned();
            }
        }

        project(&self.data, key).cloned()
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

/// Walk a dotted path into a value: object fields by name, array elements by
/// numeric index - the single data-access rule behind `$a.b.0` references,
/// `sort by="a.b"` projections, and `filter where` lookups.
fn project<'a>(mut current: &'a Value, path: &str) -> Option<&'a Value> {
    for part in path.split('.') {
        current = match (current, part.parse::<usize>()) {
            (Value::Array(arr), Ok(i)) => arr.get(i)?,
            (value, _) => value.get(part)?,
        };
    }
    Some(current)
}

/// Stringify a value for predicate evaluation: falsy values (`false`, `null`,
/// `[]`) become the empty string, so "truthy" is exactly "non-empty" for
/// every shape of data.
fn truthy_str(value: &Value) -> String {
    match value {
        Value::Bool(false) | Value::Null => String::new(),
        Value::Array(arr) if arr.is_empty() => String::new(),
        other => value_to_string(other),
    }
}

/// Render config `[globals]` as a `:root` custom-property block, or `None`
/// when there are no scalar tokens to emit.
fn globals_root_block(globals: &Value) -> Option<String> {
    let map = globals.as_object()?;
    let vars: String = map
        .iter()
        .filter(|(_, v)| !matches!(v, Value::Array(_) | Value::Object(_) | Value::Null))
        .map(|(k, v)| format!("    --{}: {};\n", k.replace('_', "-"), value_to_string(v)))
        .collect();
    if vars.is_empty() {
        None
    } else {
        Some(format!(":root {{\n{}}}", vars))
    }
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

fn is_html_doc(body: &str) -> bool {
    let trimmed = body.trim_start();
    trimmed.starts_with("<!DOCTYPE html") || trimmed.starts_with("<html")
}
