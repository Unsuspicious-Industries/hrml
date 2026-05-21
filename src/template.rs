use crate::config::Config;
use crate::features::ONode;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

mod ast;
mod error;

pub use ast::{AstNode, TemplateAst};
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
        let mut context = self.build_context(data);
        let mut visited = std::collections::HashSet::new();
        let (resolved_nodes, loaded_components) =
            self.resolve_with_tracking(template_path, &mut visited)?;

        for (id, nodes) in loaded_components {
            context.set_component(&id, nodes);
        }

        self.register_components_from_tree(&resolved_nodes, &mut context)?;

        let body_node = self.render_nodes(&resolved_nodes, &mut context, template_path)?;
        let body = body_node.render();

        if looks_like_html_document(&body) {
            Ok(body)
        } else {
            Ok(self.wrap_html(body_node))
        }
    }

    pub fn render_fragment(&self, template_path: &str, data: &Value) -> TemplateResult<String> {
        let mut context = self.build_context(data);
        let mut visited = std::collections::HashSet::new();
        let (resolved_nodes, loaded_components) =
            self.resolve_with_tracking(template_path, &mut visited)?;

        for (id, nodes) in loaded_components {
            context.set_component(&id, nodes);
        }

        self.register_components_from_tree(&resolved_nodes, &mut context)?;

        Ok(self
            .render_nodes(&resolved_nodes, &mut context, template_path)?
            .render())
    }

    pub fn parse_template(&self, template_path: &str) -> TemplateResult<TemplateAst> {
        let mut visited = std::collections::HashSet::new();
        let (nodes, _) = self.resolve_with_tracking(template_path, &mut visited)?;
        Ok(TemplateAst {
            nodes: nodes.into_iter().map(AstNode::from).collect(),
        })
    }

    pub fn parse_source(source: &str) -> TemplateResult<TemplateAst> {
        Self::parse_source_with_path(source, None)
    }

    pub fn parse_source_with_path(
        source: &str,
        template_path: Option<&str>,
    ) -> TemplateResult<TemplateAst> {
        let nodes = Parser::new(source, template_path).parse()?;
        Ok(TemplateAst {
            nodes: nodes.into_iter().map(AstNode::from).collect(),
        })
    }

    pub fn render_content(
        &self,
        content: &str,
        path: &str,
        data: &Value,
    ) -> TemplateResult<String> {
        let mut context = self.build_context(data);
        let ast = Parser::new(content, Some(path)).parse()?;

        let blocks = self.extract_blocks_local(&ast);
        let components = self.extract_components(&ast);

        for (id, nodes) in components {
            context.set_component(&id, nodes);
        }

        self.register_components_from_ast(&ast, &mut context)?;

        let resolved_nodes = self.resolve_nodes_from_ast(&ast, &blocks)?;

        let body_node = self.render_nodes(&resolved_nodes, &mut context, path)?;
        let body = body_node.render();

        if looks_like_html_document(&body) {
            Ok(body)
        } else {
            Ok(self.wrap_html(body_node))
        }
    }

    pub fn render_content_fragment(
        &self,
        content: &str,
        path: &str,
        data: &Value,
    ) -> TemplateResult<String> {
        let mut context = self.build_context(data);
        let ast = Parser::new(content, Some(path)).parse()?;

        let blocks = self.extract_blocks_local(&ast);
        let components = self.extract_components(&ast);

        for (id, nodes) in components {
            context.set_component(&id, nodes);
        }

        self.register_components_from_ast(&ast, &mut context)?;

        let resolved_nodes = self.resolve_nodes_from_ast(&ast, &blocks)?;

        Ok(self
            .render_nodes(&resolved_nodes, &mut context, path)?
            .render())
    }

    pub fn render_nodes_from_tree(&self, nodes: &[Node], data: &Value) -> TemplateResult<ONode> {
        let mut context = self.build_context(data);
        self.register_components_from_tree(nodes, &mut context)?;
        self.render_nodes(nodes, &mut context, "")
    }

    fn extract_blocks_local(&self, nodes: &[Node]) -> HashMap<String, Vec<Node>> {
        let mut blocks = HashMap::new();
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

    fn extract_components(&self, nodes: &[Node]) -> Vec<(String, Vec<Node>)> {
        let mut components = Vec::new();
        for node in nodes {
            if let Node::Element {
                name,
                attrs,
                children,
            } = node
            {
                if name == "component" {
                    if let Some(id) = attrs.get("id") {
                        components.push((id.clone(), children.clone()));
                    }
                }
            }
        }
        components
    }

    fn register_components_from_ast(
        &self,
        nodes: &[Node],
        context: &mut Context,
    ) -> TemplateResult<()> {
        for node in nodes {
            match node {
                Node::Element {
                    name,
                    attrs,
                    children,
                } => {
                    if name == "component" {
                        if let Some(id) = attrs.get("id") {
                            context.set_component(id, children.clone());
                        }
                    }
                    self.register_components_from_ast(children, context)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn resolve_nodes_from_ast(
        &self,
        nodes: &[Node],
        blocks: &HashMap<String, Vec<Node>>,
    ) -> TemplateResult<Vec<Node>> {
        let mut resolved = Vec::new();
        for node in nodes {
            match node {
                Node::VoidElement { name, attrs } if name == "load" => {
                    if let Some(file) = attrs.get("file") {
                        resolved.push(Node::Load {
                            file: file.clone(),
                            blocks: blocks.clone(),
                        });
                    }
                }
                Node::VoidElement { name, attrs } if name == "slot" => {
                    if let Some(id) = attrs.get("id") {
                        if let Some(block_content) = blocks.get(id) {
                            resolved.extend(block_content.clone());
                            continue;
                        }
                    }
                    resolved.push(Node::VoidElement {
                        name: name.clone(),
                        attrs: attrs.clone(),
                    });
                }
                Node::Element {
                    name,
                    attrs,
                    children,
                } => {
                    let resolved_children = self.resolve_nodes_from_ast(children, blocks)?;
                    resolved.push(Node::Element {
                        name: name.clone(),
                        attrs: attrs.clone(),
                        children: resolved_children,
                    });
                }
                other => resolved.push(other.clone()),
            }
        }
        Ok(resolved)
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

    fn resolve_with_tracking(
        &self,
        template_path: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> TemplateResult<(Vec<Node>, Vec<(String, Vec<Node>)>)> {
        if visited.contains(template_path) {
            return Err(TemplateError::code(
                TemplateErrorPhase::Resolve,
                format!("Circular template dependency detected: {}", template_path),
            )
            .with_template_path(template_path));
        }
        visited.insert(template_path.to_string());

        let full_path = self.base_path.join(template_path);
        let content = fs::read_to_string(&full_path).map_err(|e| {
            TemplateError::code(
                TemplateErrorPhase::Io,
                format!("Failed to read template {}: {}", template_path, e),
            )
            .with_template_path(template_path)
        })?;

        let mut nodes = Parser::new(&content, Some(template_path)).parse()?;

        // 1. Extract blocks defined in this template
        let blocks = self.extract_blocks(&nodes, visited)?;

        // 2. Extract components defined in this template
        let local_components = self.extract_components(&nodes);

        // 3. Remove block and component Nodes from the tree
        nodes.retain(|n| match n {
            Node::Element { name, .. } if name == "block" => false,
            Node::Element { name, attrs, .. } if name == "component" => {
                if attrs.get("id").is_some() {
                    return false;
                }
                false
            }
            _ => true,
        });

        // 4. Process loads recursively and resolve nested loads in all elements
        let mut resolved_nodes = Vec::new();
        let all_components = local_components;

        for node in nodes {
            match node {
                Node::VoidElement { name, attrs } if name == "load" => {
                    if let Some(file) = attrs.get("file") {
                        resolved_nodes.push(Node::Load {
                            file: file.clone(),
                            blocks: blocks.clone(),
                        });
                        continue;
                    }
                }
                Node::Element {
                    name,
                    attrs,
                    children,
                } => {
                    let resolved_children =
                        self.resolve_nodes_recursive(children, visited, &blocks)?;
                    resolved_nodes.push(Node::Element {
                        name,
                        attrs,
                        children: resolved_children,
                    });
                }
                other => resolved_nodes.push(other),
            }
        }

        visited.remove(template_path);
        Ok((resolved_nodes, all_components))
    }

    pub(crate) fn register_components_from_tree(
        &self,
        nodes: &[Node],
        context: &mut Context,
    ) -> TemplateResult<()> {
        for node in nodes {
            match node {
                Node::Element {
                    name,
                    attrs,
                    children,
                } => {
                    if name == "component" {
                        if let Some(id) = attrs.get("id") {
                            context.set_component(id, children.clone());
                        }
                    }
                    self.register_components_from_tree(children, context)?;
                }
                Node::Load { file, .. } => {
                    if context
                        .component_load_stack
                        .iter()
                        .any(|loaded| loaded == file)
                    {
                        continue;
                    }

                    context.component_load_stack.push(file.clone());
                    let register_result = (|| {
                        let mut visited = std::collections::HashSet::new();
                        let (linked_nodes, linked_components) =
                            self.resolve_with_tracking(file, &mut visited)?;

                        for (id, component_nodes) in linked_components {
                            context.set_component(&id, component_nodes);
                        }

                        self.register_components_from_tree(&linked_nodes, context)
                    })();
                    context.component_load_stack.pop();
                    register_result?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn resolve_nodes_recursive(
        &self,
        nodes: Vec<Node>,
        visited: &mut std::collections::HashSet<String>,
        blocks: &HashMap<String, Vec<Node>>,
    ) -> TemplateResult<Vec<Node>> {
        let mut resolved = Vec::new();

        for node in nodes {
            match node {
                Node::VoidElement { name, attrs } if name == "load" => {
                    if let Some(file) = attrs.get("file") {
                        resolved.push(Node::Load {
                            file: file.clone(),
                            blocks: blocks.clone(),
                        });
                    }
                }
                Node::VoidElement { name, attrs } if name == "slot" => {
                    if let Some(id) = attrs.get("id") {
                        if let Some(block_content) = blocks.get(id) {
                            resolved.extend(block_content.clone());
                            continue;
                        }
                    }
                    // Keep void slot as-is if no block matches
                    resolved.push(Node::VoidElement { name, attrs });
                }
                Node::Element {
                    name,
                    attrs,
                    children,
                } => {
                    let resolved_children =
                        self.resolve_nodes_recursive(children, visited, blocks)?;
                    resolved.push(Node::Element {
                        name,
                        attrs,
                        children: resolved_children,
                    });
                }
                other => resolved.push(other),
            }
        }

        Ok(resolved)
    }

    fn extract_blocks(
        &self,
        nodes: &[Node],
        visited: &mut std::collections::HashSet<String>,
    ) -> TemplateResult<HashMap<String, Vec<Node>>> {
        let mut blocks = HashMap::new();
        for node in nodes {
            if let Node::Element {
                name,
                attrs,
                children,
            } = node
            {
                if name == "block" {
                    if let Some(slot) = attrs.get("slot") {
                        let resolved_children = self.resolve_nodes_recursive(
                            children.clone(),
                            visited,
                            &HashMap::new(),
                        )?;
                        blocks.insert(slot.clone(), resolved_children);
                    }
                }
            }
        }
        Ok(blocks)
    }

    fn inject_blocks(
        &self,
        parent_nodes: Vec<Node>,
        blocks: &HashMap<String, Vec<Node>>,
    ) -> Vec<Node> {
        let mut new_nodes = Vec::new();

        for node in parent_nodes {
            match node {
                Node::Element {
                    name,
                    attrs,
                    children,
                } => {
                    if name == "slot" {
                        if let Some(id) = attrs.get("id") {
                            if let Some(block_content) = blocks.get(id) {
                                new_nodes.extend(block_content.clone());
                                continue;
                            }
                        }
                        let processed_children = self.inject_blocks(children, blocks);
                        new_nodes.extend(processed_children);
                    } else {
                        new_nodes.push(Node::Element {
                            name,
                            attrs,
                            children: self.inject_blocks(children, blocks),
                        });
                    }
                }
                Node::VoidElement { name, attrs } if name == "slot" => {
                    if let Some(id) = attrs.get("id") {
                        if let Some(block_content) = blocks.get(id) {
                            new_nodes.extend(block_content.clone());
                            continue;
                        }
                    }
                }
                other => new_nodes.push(other),
            }
        }

        new_nodes
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
            Node::Text(text) => Ok(ONode::raw(text.clone())),
            Node::Load { file, blocks } => self.render_load(file, blocks, context),
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
                        for (k, _) in attrs {
                            if k == "id" {
                                if let Some(val) = attrs.get("value") {
                                    context.set_str(k, val.clone());
                                }
                            } else if k != "value" {
                                if k == "id" && attrs.contains_key("value") {
                                    context.set_str(
                                        attrs.get("id").unwrap(),
                                        attrs.get("value").unwrap().clone(),
                                    );
                                }
                            }
                        }
                        if let (Some(id), Some(val)) = (attrs.get("id"), attrs.get("value")) {
                            context.set_str(id, val.clone());
                        }
                        Ok(ONode::empty())
                    }
                    "pure" => Ok(ONode::raw(attrs.get("value").cloned().unwrap_or_default())),
                    "mdx" => self.render_mdx(attrs, template_path).map(ONode::raw),
                    "markdown" => self.render_markdown(attrs, template_path).map(ONode::raw),
                    "markdownfm" => self
                        .render_markdown_with_frontmatter(attrs, context, template_path)
                        .map(ONode::raw),
                    "latex" => self.render_latex(attrs, template_path).map(ONode::raw),
                    "meta" => Ok(ONode::raw(self.render_meta_tag(attrs, context))),
                    "linktag" => Ok(ONode::raw(self.render_link_tag(attrs, context))),
                    "title" => Ok(ONode::raw(self.render_title_tag(attrs, context))),
                    "og" => Ok(ONode::raw(self.render_og_tag(attrs, context))),
                    "twitter" => Ok(ONode::raw(self.render_twitter_tag(attrs, context))),
                    "charset" => Ok(ONode::raw(self.render_charset_tag(attrs))),
                    "viewport" => Ok(ONode::raw(self.render_viewport_tag(attrs, context))),
                    "canonical" => Ok(ONode::raw(self.render_canonical_tag(attrs, context))),
                    "description" => Ok(ONode::raw(self.render_description_tag(attrs, context))),
                    "robots" => Ok(ONode::raw(self.render_robots_tag(attrs, context))),
                    "stylesheet" => Ok(ONode::raw(self.render_stylesheet_tag(attrs, context))),
                    "script" => Ok(ONode::raw(self.render_script_tag(attrs, context))),
                    "use" => self.render_component_use(attrs, &[], context, template_path),
                    "bind" => {
                        if attrs.contains_key("from") || attrs.contains_key("value") {
                            let var = attrs.get("var").map(String::as_str).unwrap_or("value");
                            let bound = if let Some(from) = attrs.get("from") {
                                context
                                    .get_value(from)
                                    .unwrap_or(Value::String(String::new()))
                            } else {
                                Value::String(attrs.get("value").cloned().unwrap_or_default())
                            };
                            context.set_value(var, bound);
                        }
                        Ok(ONode::empty())
                    }
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
                    _ => Err(self.unknown_directive_error(template_path, name)),
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
                    "set" => {
                        if let Some(id) = attrs.get("id") {
                            let content = self
                                .render_nodes(children, context, template_path)?
                                .render();
                            context.set_str(id, content);
                        } else {
                            if let (Some(id), Some(val)) = (attrs.get("id"), attrs.get("value")) {
                                context.set_str(id, val.clone());
                            }
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
                    _ => Err(self.unknown_directive_error(template_path, name)),
                }
            }
        }
    }

    fn render_if(
        &self,
        attrs: &HashMap<String, String>,
        children: &[Node],
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<ONode> {
        let condition = attrs.get("cond").cloned().unwrap_or_default();
        let is_true = self.eval_condition(&condition, context);

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
        attrs: &HashMap<String, String>,
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

    fn render_bind(
        &self,
        attrs: &HashMap<String, String>,
        children: &[Node],
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<ONode> {
        let var = attrs.get("var").map(String::as_str).unwrap_or("value");
        if attrs.contains_key("from") || attrs.contains_key("value") {
            let bound = if let Some(from) = attrs.get("from") {
                context
                    .get_value(from)
                    .unwrap_or(Value::String(String::new()))
            } else {
                Value::String(attrs.get("value").cloned().unwrap_or_default())
            };

            let mut scoped = context.clone();
            scoped.set_value(var, bound);
            return self.render_nodes(children, &mut scoped, template_path);
        }

        let rendered = self
            .render_nodes(children, context, template_path)?
            .render();
        if !rendered.trim().is_empty() {
            context.set_str(var, rendered);
        }

        Ok(ONode::empty())
    }

    fn render_component_use(
        &self,
        attrs: &HashMap<String, String>,
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
        let blocks = self.extract_blocks_local(children);
        let setup_nodes: Vec<Node> = children
            .iter()
            .filter(|node| match node {
                Node::Element { name, .. } | Node::VoidElement { name, .. } => name != "block",
                _ => true,
            })
            .cloned()
            .collect();
        let _ = self.render_nodes(&setup_nodes, &mut scoped, template_path)?;

        let resolved = self.inject_blocks(component_nodes, &blocks);
        self.render_nodes(&resolved, &mut scoped, template_path)
    }

    fn render_load(
        &self,
        file: &str,
        blocks: &HashMap<String, Vec<Node>>,
        context: &mut Context,
    ) -> TemplateResult<ONode> {
        if context.load_stack.iter().any(|loaded| loaded == file) {
            return Err(TemplateError::code(
                TemplateErrorPhase::Resolve,
                format!("Circular template dependency detected: {}", file),
            )
            .with_template_path(file)
            .with_directive("load"));
        }

        context.load_stack.push(file.to_string());
        let result = (|| {
            let mut visited = std::collections::HashSet::new();
            let (loaded_nodes, loaded_components) =
                self.resolve_with_tracking(file, &mut visited)?;

            for (id, nodes) in loaded_components {
                context.set_component(&id, nodes);
            }

            let linked_nodes = self.inject_blocks(loaded_nodes, blocks);
            self.render_nodes(&linked_nodes, context, file)
        })();
        context.load_stack.pop();

        result
    }

    fn eval_condition(&self, condition: &str, context: &Context) -> bool {
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

    fn unknown_directive_error(&self, template_path: &str, directive: &str) -> TemplateError {
        TemplateError::code(
            TemplateErrorPhase::Render,
            format!(
                "Unknown HRML directive '<?{}?>'. Use normal HTML for HTML tags and reserve '<?...?>' for HRML directives",
                directive
            ),
        )
        .with_template_path(template_path)
        .with_directive(directive)
    }

    fn render_mdx(
        &self,
        attrs: &HashMap<String, String>,
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

    fn resolve_value_or_literal(&self, raw: &str, context: &Context) -> String {
        if let Some(stripped) = raw.strip_prefix('$') {
            context.get(stripped)
        } else {
            raw.to_string()
        }
    }

    fn render_meta_tag(&self, attrs: &HashMap<String, String>, context: &Context) -> String {
        use crate::features::oxml_tags;
        let mut node = ONode::void(oxml_tags::META);
        let mut keys = attrs.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        for key in keys {
            let resolved = self.resolve_value_or_literal(attrs.get(&key).unwrap(), context);
            node = node.attr(key, resolved);
        }
        node.build().render()
    }

    fn render_link_tag(&self, attrs: &HashMap<String, String>, context: &Context) -> String {
        use crate::features::oxml_tags;
        let mut node = ONode::void(oxml_tags::LINK);
        let mut keys = attrs.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        for key in keys {
            let resolved = self.resolve_value_or_literal(attrs.get(&key).unwrap(), context);
            node = node.attr(key, resolved);
        }
        node.build().render()
    }

    fn render_title_tag(&self, attrs: &HashMap<String, String>, context: &Context) -> String {
        use crate::features::oxml_tags;
        let value = attrs
            .get("value")
            .map(|v| self.resolve_value_or_literal(v, context))
            .unwrap_or_default();
        ONode::content(oxml_tags::TITLE)
            .text(&value)
            .build()
            .render()
    }

    fn render_og_tag(&self, attrs: &HashMap<String, String>, context: &Context) -> String {
        use crate::features::oxml_tags;
        let Some(name) = attrs.get("name") else {
            return String::new();
        };
        let Some(content) = attrs.get("content") else {
            return String::new();
        };
        let resolved_content = self.resolve_value_or_literal(content, context);
        let property = format!("og:{}", name);
        ONode::void(oxml_tags::META)
            .attr("property", &property)
            .attr("content", &resolved_content)
            .build()
            .render()
    }

    fn render_twitter_tag(&self, attrs: &HashMap<String, String>, context: &Context) -> String {
        use crate::features::oxml_tags;
        let Some(name) = attrs.get("name") else {
            return String::new();
        };
        let Some(content) = attrs.get("content") else {
            return String::new();
        };
        let resolved_content = self.resolve_value_or_literal(content, context);
        let key = format!("twitter:{}", name);
        ONode::void(oxml_tags::META)
            .attr("name", &key)
            .attr("content", &resolved_content)
            .build()
            .render()
    }

    fn render_markdown(
        &self,
        attrs: &HashMap<String, String>,
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

    fn render_markdown_with_frontmatter(
        &self,
        attrs: &HashMap<String, String>,
        context: &mut Context,
        template_path: &str,
    ) -> TemplateResult<String> {
        let file = attrs.get("file").ok_or_else(|| {
            TemplateError::code(
                TemplateErrorPhase::Render,
                "markdownfm directive requires file attribute",
            )
            .with_template_path(template_path)
            .with_directive("markdownfm")
        })?;
        let full_path = self.base_path.join(file);
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

    fn render_latex(
        &self,
        attrs: &HashMap<String, String>,
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

    fn render_charset_tag(&self, attrs: &HashMap<String, String>) -> String {
        use crate::features::oxml_tags;
        let charset = attrs
            .get("value")
            .cloned()
            .unwrap_or_else(|| "UTF-8".to_string());
        ONode::void(oxml_tags::META)
            .attr("charset", &charset)
            .build()
            .render()
    }

    fn render_viewport_tag(&self, attrs: &HashMap<String, String>, context: &Context) -> String {
        use crate::features::oxml_tags;
        let content = attrs
            .get("content")
            .map(|v| self.resolve_value_or_literal(v, context))
            .unwrap_or_else(|| "width=device-width,initial-scale=1".to_string());
        ONode::void(oxml_tags::META)
            .attr("name", "viewport")
            .attr("content", &content)
            .build()
            .render()
    }

    fn render_canonical_tag(&self, attrs: &HashMap<String, String>, context: &Context) -> String {
        use crate::features::oxml_tags;
        let href = attrs
            .get("href")
            .map(|v| self.resolve_value_or_literal(v, context))
            .unwrap_or_default();
        ONode::void(oxml_tags::LINK)
            .attr("rel", "canonical")
            .attr("href", &href)
            .build()
            .render()
    }

    fn render_description_tag(&self, attrs: &HashMap<String, String>, context: &Context) -> String {
        use crate::features::oxml_tags;
        let content = attrs
            .get("content")
            .map(|v| self.resolve_value_or_literal(v, context))
            .unwrap_or_default();
        ONode::void(oxml_tags::META)
            .attr("name", "description")
            .attr("content", &content)
            .build()
            .render()
    }

    fn render_robots_tag(&self, attrs: &HashMap<String, String>, context: &Context) -> String {
        use crate::features::oxml_tags;
        let content = attrs
            .get("content")
            .map(|v| self.resolve_value_or_literal(v, context))
            .unwrap_or_else(|| "index,follow".to_string());
        ONode::void(oxml_tags::META)
            .attr("name", "robots")
            .attr("content", &content)
            .build()
            .render()
    }

    fn render_stylesheet_tag(&self, attrs: &HashMap<String, String>, context: &Context) -> String {
        use crate::features::oxml_tags;
        let href = attrs
            .get("href")
            .map(|v| self.resolve_value_or_literal(v, context))
            .unwrap_or_default();
        ONode::void(oxml_tags::LINK)
            .attr("rel", "stylesheet")
            .attr("href", &href)
            .build()
            .render()
    }

    fn render_script_tag(&self, attrs: &HashMap<String, String>, context: &Context) -> String {
        use crate::features::oxml_tags;
        let src = attrs
            .get("src")
            .map(|v| self.resolve_value_or_literal(v, context))
            .unwrap_or_default();
        let defer_attr = attrs.contains_key("defer");
        let async_attr = attrs.contains_key("async");
        ONode::content(oxml_tags::SCRIPT)
            .attr("src", &src)
            .attr_if(defer_attr, "defer", "defer")
            .attr_if(async_attr, "async", "async")
            .build()
            .render()
    }
}

// --- AST ---

#[derive(Debug, Clone)]
pub enum Node {
    Text(String),
    Load {
        file: String,
        blocks: HashMap<String, Vec<Node>>,
    },
    Element {
        name: String,
        attrs: HashMap<String, String>,
        children: Vec<Node>,
    },
    VoidElement {
        name: String,
        attrs: HashMap<String, String>,
    },
}

impl Node {
    pub fn from_ast_node(ast: AstNode) -> Self {
        match ast {
            AstNode::Text(text) => Node::Text(text),
            AstNode::Load { file, blocks } => Node::Load {
                file,
                blocks: blocks
                    .into_iter()
                    .map(|(key, nodes)| (key, nodes.into_iter().map(Node::from_ast_node).collect()))
                    .collect(),
            },
            AstNode::Element {
                name,
                attrs,
                children,
            } => Node::Element {
                name,
                attrs: attrs.into_iter().collect(),
                children: children.into_iter().map(Node::from_ast_node).collect(),
            },
            AstNode::VoidElement { name, attrs } => Node::VoidElement {
                name,
                attrs: attrs.into_iter().collect(),
            },
        }
    }
}

impl From<Node> for AstNode {
    fn from(node: Node) -> Self {
        match node {
            Node::Text(text) => AstNode::Text(text),
            Node::Load { file, blocks } => AstNode::Load {
                file,
                blocks: blocks
                    .into_iter()
                    .map(|(key, nodes)| (key, nodes.into_iter().map(AstNode::from).collect()))
                    .collect(),
            },
            Node::Element {
                name,
                attrs,
                children,
            } => AstNode::Element {
                name,
                attrs: attrs.into_iter().collect(),
                children: children.into_iter().map(AstNode::from).collect(),
            },
            Node::VoidElement { name, attrs } => AstNode::VoidElement {
                name,
                attrs: attrs.into_iter().collect(),
            },
        }
    }
}

// --- Parser ---

struct Parser {
    chars: Vec<char>,
    pos: usize,
    template_path: Option<String>,
}

impl Parser {
    fn new(input: &str, template_path: Option<&str>) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            template_path: template_path.map(ToOwned::to_owned),
        }
    }

    fn parse(&mut self) -> TemplateResult<Vec<Node>> {
        let mut nodes = Vec::new();
        let mut iterations = 0;
        while self.pos < self.chars.len() {
            iterations += 1;
            if iterations > 10000 {
                let remaining: String = self.chars[self.pos..].iter().take(50).collect();
                return Err(self.internal_error(
                    self.pos,
                    format!(
                        "Parser infinite loop at pos {}, remaining='{}'",
                        self.pos, remaining
                    ),
                ));
            }
            if let Some(node) = self.parse_node(false)? {
                nodes.push(node);
            } else {
                break;
            }
        }
        Ok(nodes)
    }

    fn parse_until(&mut self, closing_tag: &str) -> TemplateResult<Vec<Node>> {
        let mut nodes = Vec::new();
        let mut iterations = 0;
        while self.pos < self.chars.len() {
            iterations += 1;
            if iterations > 10000 {
                let remaining: String = self.chars[self.pos..].iter().take(50).collect();
                return Err(self.internal_error(
                    self.pos,
                    format!(
                        "Parser infinite loop in parse_until('{}') at pos {}, remaining='{}'",
                        closing_tag, self.pos, remaining
                    ),
                ));
            }
            if closing_tag.is_empty() {
                let end = std::cmp::min(self.pos + 60, self.chars.len());
                let remaining: String = self.chars[self.pos..end].iter().collect();
                return Err(self.internal_error(
                    self.pos,
                    format!(
                        "BUG: parse_until empty tag at pos {}, remaining='{}'",
                        self.pos, remaining
                    ),
                ));
            }
            if self.is_closing(closing_tag) {
                self.consume_closing(closing_tag);
                return Ok(nodes);
            }
            if let Some(node) = self.parse_node(true)? {
                nodes.push(node);
            } else {
                // parse_node returned None - could be a closing tag, check before breaking
                if self.is_closing(closing_tag) {
                    self.consume_closing(closing_tag);
                    return Ok(nodes);
                }
                break;
            }
        }
        Err(self.code_error(
            self.pos,
            format!(
                "Unclosed HRML directive '{}': missing closing tag",
                closing_tag
            ),
        ))
    }

    fn is_closing(&self, name: &str) -> bool {
        // Check for </?name pattern (HRML paired style)
        let pattern1 = format!("</?{}", name);
        if self.starts_with(&pattern1) {
            let end = self.pos + pattern1.len();
            if end >= self.chars.len() {
                return true;
            }
            let next = self.chars[end];
            if next.is_whitespace() || next == '?' || next == '>' {
                return true;
            }
        }

        // Check for <?/name?> pattern (HRML alternative closing)
        let pattern2 = format!("<?/{}?>", name);
        if self.starts_with(&pattern2) {
            return true;
        }

        // Check for </name> pattern (HTML style closing)
        let html_pattern = format!("</{}>", name);
        if self.starts_with(&html_pattern) {
            return true;
        }

        false
    }

    fn consume_closing(&mut self, _name: &str) {
        while self.pos < self.chars.len() {
            if self.starts_with("?>") {
                self.pos += 2;
                return;
            }
            self.pos += 1;
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        if self.pos + chars.len() > self.chars.len() {
            return false;
        }
        for (i, c) in chars.iter().enumerate() {
            if self.chars[self.pos + i] != *c {
                return false;
            }
        }
        true
    }

    fn parse_node(&mut self, allow_closing: bool) -> TemplateResult<Option<Node>> {
        if self.pos >= self.chars.len() {
            return Ok(None);
        }

        if self.starts_with("<?") && !self.starts_with("</?") && !self.starts_with("<?/") {
            return self.parse_element().map(Some);
        } else if self.starts_with("</?") || self.starts_with("<?/") {
            if !allow_closing {
                return Err(self.code_error(
                    self.pos,
                    "Unexpected closing HRML tag without matching opener",
                ));
            }
            self.consume_closing_tag();
            return Ok(None);
        }

        Ok(Some(Node::Text(self.consume_text())))
    }

    fn consume_text(&mut self) -> String {
        let mut text = String::new();
        while self.pos < self.chars.len() {
            // Break on opening HRML tags
            if self.starts_with("<?") && !self.starts_with("</?") && !self.starts_with("<?/") {
                break;
            }
            // Break on closing tags too - let parse_node handle them
            if self.starts_with("</?") || self.starts_with("<?/") {
                break;
            }
            text.push(self.chars[self.pos]);
            self.pos += 1;
        }
        text
    }

    fn consume_closing_tag(&mut self) {
        while self.pos < self.chars.len() {
            if self.starts_with("?>") {
                self.pos += 2;
                return;
            }
            self.pos += 1;
        }
    }

    fn parse_element(&mut self) -> TemplateResult<Node> {
        let start = self.pos;
        self.pos += 2;

        let name = self.consume_identifier();
        if name.is_empty() {
            let remaining: String = self.chars[self.pos..].iter().take(40).collect();
            return Err(self.internal_error(
                start,
                format!(
                    "BUG: parse_element got empty name at pos {}, remaining='{}'",
                    self.pos, remaining
                ),
            ));
        }
        let attrs = self.parse_attributes();

        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }

        let explicit_self_closing = if self.starts_with("/?>") {
            self.pos += 3;
            true
        } else if self.starts_with("?>") {
            self.pos += 2;
            false
        } else {
            return Err(self.code_error(
                start,
                format!("Unclosed HRML directive '<?{}': missing '?>'", name),
            ));
        };

        if explicit_self_closing {
            return Ok(Node::VoidElement { name, attrs });
        }

        let has_closing = self.has_matching_closing(&name);
        let is_void = !has_closing;

        if is_void {
            Ok(Node::VoidElement { name, attrs })
        } else {
            let children = self.parse_until(&name)?;
            Ok(Node::Element {
                name,
                attrs,
                children,
            })
        }
    }

    fn error_location(&self, pos: usize) -> TemplateErrorLocation {
        let mut line = 1;
        let mut column = 1;

        for ch in self.chars.iter().take(pos) {
            if *ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }

        TemplateErrorLocation { line, column }
    }

    fn internal_error(&self, pos: usize, message: impl Into<String>) -> TemplateError {
        let location = self.error_location(pos);
        let mut error = TemplateError::internal(TemplateErrorPhase::Parse, message)
            .with_location(location.line, location.column);

        if let Some(path) = &self.template_path {
            error = error.with_template_path(path.clone());
        }

        error
    }

    fn code_error(&self, pos: usize, message: impl Into<String>) -> TemplateError {
        let location = self.error_location(pos);
        let mut error = TemplateError::code(TemplateErrorPhase::Parse, message)
            .with_location(location.line, location.column);

        if let Some(path) = &self.template_path {
            error = error.with_template_path(path.clone());
        }

        error
    }

    fn has_matching_closing(&self, name: &str) -> bool {
        // Check for </?name pattern (HRML paired style)
        let pattern1 = format!("</?{}", name);
        let mut i = self.pos;
        while i + pattern1.len() <= self.chars.len() {
            let mut matched = true;
            for (j, ch) in pattern1.chars().enumerate() {
                if self.chars[i + j] != ch {
                    matched = false;
                    break;
                }
            }
            if matched {
                return true;
            }
            i += 1;
        }

        // Check for <?/name?> pattern (HRML alternative closing)
        let pattern2 = format!("<?/{}?>", name);
        i = self.pos;
        while i + pattern2.len() <= self.chars.len() {
            let mut matched = true;
            for (j, ch) in pattern2.chars().enumerate() {
                if self.chars[i + j] != ch {
                    matched = false;
                    break;
                }
            }
            if matched {
                return true;
            }
            i += 1;
        }

        false
    }

    fn consume_identifier(&mut self) -> String {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }

        let mut id = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_alphanumeric() || c == '_' || c == '-' || c == ':' || c == '.' {
                id.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        id
    }

    fn parse_attributes(&mut self) -> HashMap<String, String> {
        let mut attrs = HashMap::new();
        loop {
            while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                self.pos += 1;
            }

            if self.pos >= self.chars.len() || self.starts_with("?>") || self.starts_with("/?>") {
                break;
            }

            let key = self.consume_identifier();
            if key.is_empty() {
                if !self.starts_with("?>") {
                    self.pos += 1;
                }
                continue;
            }

            let mut value = String::new();
            while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                self.pos += 1;
            }

            if self.pos < self.chars.len() && self.chars[self.pos] == '=' {
                self.pos += 1;
                while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                    self.pos += 1;
                }

                if self.pos < self.chars.len() {
                    let quote = self.chars[self.pos];
                    if quote == '"' || quote == '\'' {
                        self.pos += 1;
                        while self.pos < self.chars.len() && self.chars[self.pos] != quote {
                            value.push(self.chars[self.pos]);
                            self.pos += 1;
                        }
                        self.pos += 1;
                    } else {
                        while self.pos < self.chars.len()
                            && !self.chars[self.pos].is_whitespace()
                            && self.chars[self.pos] != '?'
                            && self.chars[self.pos] != '>'
                        {
                            value.push(self.chars[self.pos]);
                            self.pos += 1;
                        }
                    }
                }
            }

            attrs.insert(key, value);
        }
        attrs
    }
}

#[derive(Clone)]
struct Context {
    data: Value,
    vars: HashMap<String, Value>,
    components: HashMap<String, Vec<Node>>,
    load_stack: Vec<String>,
    component_load_stack: Vec<String>,
}

impl Context {
    fn new(data: Value) -> Self {
        Self {
            data,
            vars: HashMap::new(),
            components: HashMap::new(),
            load_stack: Vec::new(),
            component_load_stack: Vec::new(),
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

fn looks_like_html_document(body: &str) -> bool {
    let trimmed = body.trim_start();
    trimmed.starts_with("<!DOCTYPE html") || trimmed.starts_with("<html")
}
