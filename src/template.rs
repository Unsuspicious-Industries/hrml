use crate::algebra::{ComposeOp, Hypertext};
use crate::features::ONode;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn escape_attr(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub struct Engine {
    base_path: PathBuf,
    site_name: String,
    site_description: Option<String>,
    favicon: Option<String>,
    tag_registry: crate::features::TagRegistry,
}

impl Engine {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: PathBuf::from(base_path),
            site_name: "HRML App".to_string(),
            site_description: None,
            favicon: None,
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

    pub fn register_void_tag(&mut self, name: &str, handler: crate::features::VoidTagHandler) {
        self.tag_registry.register_void(name, handler);
    }

    pub fn register_block_tag(&mut self, name: &str, handler: crate::features::BlockTagHandler) {
        self.tag_registry.register_block(name, handler);
    }

    pub fn render(&self, template_path: &str, data: &Value) -> Result<String, String> {
        let mut context = self.build_context(data);
        let mut visited = std::collections::HashSet::new();
        let resolved_nodes = self.resolve_with_tracking(template_path, &mut visited)?;
        let body = self.render_nodes(&resolved_nodes, &mut context)?;

        if looks_like_html_document(&body) {
            Ok(body)
        } else {
            Ok(self.wrap_html(&body))
        }
    }

    pub fn render_fragment(&self, template_path: &str, data: &Value) -> Result<String, String> {
        let mut context = self.build_context(data);
        let mut visited = std::collections::HashSet::new();
        let resolved_nodes = self.resolve_with_tracking(template_path, &mut visited)?;
        self.render_nodes(&resolved_nodes, &mut context)
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
        context
    }

    fn resolve_with_tracking(
        &self,
        template_path: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<Vec<Node>, String> {
        if visited.contains(template_path) {
            return Err(format!(
                "Circular template dependency detected: {}",
                template_path
            ));
        }
        visited.insert(template_path.to_string());

        let full_path = self.base_path.join(template_path);
        let content = fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read template {}: {}", template_path, e))?;

        let mut nodes = Parser::new(&content).parse()?;

        // 1. Extract blocks defined in this template
        let blocks = self.extract_blocks(&nodes, visited)?;

        // 2. Remove block Nodes from the tree
        nodes.retain(|n| !matches!(n, Node::Element { name, .. } if name == "block"));

        // 3. Process loads recursively and resolve nested loads in all elements
        let mut resolved_nodes = Vec::new();

        for node in nodes {
            match node {
                Node::VoidElement { name, attrs } if name == "load" => {
                    if let Some(file) = attrs.get("file") {
                        let mut loaded_nodes = self.resolve_with_tracking(file, visited)?;
                        loaded_nodes = self.inject_blocks(loaded_nodes, &blocks);
                        resolved_nodes.extend(loaded_nodes);
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
        Ok(resolved_nodes)
    }

    fn resolve_nodes_recursive(
        &self,
        nodes: Vec<Node>,
        visited: &mut std::collections::HashSet<String>,
        blocks: &HashMap<String, Vec<Node>>,
    ) -> Result<Vec<Node>, String> {
        let mut resolved = Vec::new();

        for node in nodes {
            match node {
                Node::VoidElement { name, attrs } if name == "load" => {
                    if let Some(file) = attrs.get("file") {
                        let mut loaded = self.resolve_with_tracking(file, visited)?;
                        loaded = self.inject_blocks(loaded, blocks);
                        resolved.extend(loaded);
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
    ) -> Result<HashMap<String, Vec<Node>>, String> {
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

    fn render_nodes(&self, nodes: &[Node], context: &mut Context) -> Result<String, String> {
        let mut result = String::new();
        for node in nodes {
            result.push_str(&self.render_node(node, context)?);
        }
        Ok(result)
    }

    fn render_node(&self, node: &Node, context: &mut Context) -> Result<String, String> {
        match node {
            Node::Text(text) => Ok(text.clone()),
            Node::VoidElement { name, attrs } => {
                if let Some(result) = self.tag_registry.render_void(name, attrs) {
                    return result;
                }
                match name.as_str() {
                    "load" => Ok(String::new()),
                    "else" => Ok(String::new()),
                    "wasm" => {
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

                        Ok(format!(
                            r#"<div class="{}" data-wasm-module="{}" data-wasm-src="{}" data-wasm-export="{}" data-wasm-start="{}" data-wasm-props='{}'></div>"#,
                            escape_attr(&target_class),
                            escape_attr(&module),
                            escape_attr(&src),
                            escape_attr(&export),
                            escape_attr(&start),
                            escape_attr(&props)
                        ))
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
                        Ok(String::new())
                    }
                    "pure" => Ok(attrs.get("value").cloned().unwrap_or_default()),
                    "mdx" => self.render_mdx(attrs),
                    "markdown" => self.render_markdown(attrs),
                    "markdownfm" => self.render_markdown_with_frontmatter(attrs, context),
                    "latex" => self.render_latex(attrs),
                    "meta" => Ok(self.render_meta_tag(attrs, context)),
                    "linktag" => Ok(self.render_link_tag(attrs, context)),
                    "title" => Ok(self.render_title_tag(attrs, context)),
                    "og" => Ok(self.render_og_tag(attrs, context)),
                    "twitter" => Ok(self.render_twitter_tag(attrs, context)),
                    "charset" => Ok(self.render_charset_tag(attrs)),
                    "viewport" => Ok(self.render_viewport_tag(attrs, context)),
                    "canonical" => Ok(self.render_canonical_tag(attrs, context)),
                    "description" => Ok(self.render_description_tag(attrs, context)),
                    "robots" => Ok(self.render_robots_tag(attrs, context)),
                    "stylesheet" => Ok(self.render_stylesheet_tag(attrs, context)),
                    "script" => Ok(self.render_script_tag(attrs, context)),
                    "use" => self.render_component_use(attrs, &[], context),
                    "get" => {
                        if let Some(id) = attrs.get("id") {
                            Ok(context.get(id))
                        } else {
                            Ok(String::new())
                        }
                    }
                    _ => Ok(String::new()),
                }
            }
            Node::Element {
                name,
                attrs,
                children,
            } => {
                if let Some(result) = self.tag_registry.render_block(
                    name,
                    attrs,
                    &self.render_nodes(children, context)?,
                ) {
                    return result;
                }
                match name.as_str() {
                    "block" => self.render_nodes(children, context),
                    "slot" => self.render_nodes(children, context),
                    "if" => self.render_if(attrs, children, context),
                    "for" => self.render_for(attrs, children, context),
                    "set" => {
                        if let Some(id) = attrs.get("id") {
                            let content = self.render_nodes(children, context)?;
                            context.set_str(id, content);
                        } else {
                            if let (Some(id), Some(val)) = (attrs.get("id"), attrs.get("value")) {
                                context.set_str(id, val.clone());
                            }
                        }
                        Ok(String::new())
                    }
                    "component" => {
                        if let Some(id) = attrs.get("id") {
                            context.set_component(id, children.to_vec());
                        }
                        Ok(String::new())
                    }
                    "use" => self.render_component_use(attrs, children, context),
                    "bind" => self.render_bind(attrs, children, context),
                    "compose" => self.render_compose(attrs, children, context),
                    "btn" => {
                        let inner = self.render_nodes(children, context)?;
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

                        Ok(format!(
                            r#"<button class="btn btn-primary" data-{}="{}" data-target="{}" data-swap="{}">{}</button>"#,
                            method, endpoint, target, swap, inner
                        ))
                    }
                    "link" => {
                        let inner = self.render_nodes(children, context)?;
                        let endpoint = attrs.get("get").unwrap_or(&String::new()).clone();
                        let target = attrs
                            .get("target")
                            .cloned()
                            .unwrap_or_else(|| "#body".to_string());
                        let swap = attrs
                            .get("swap")
                            .cloned()
                            .unwrap_or_else(|| "innerHTML".to_string());

                        Ok(format!(
                            r##"<a href="#" data-get="{}" data-target="{}" data-swap="{}">{}</a>"##,
                            endpoint, target, swap, inner
                        ))
                    }
                    "form" => {
                        let inner = self.render_nodes(children, context)?;
                        let endpoint = attrs.get("post").unwrap_or(&String::new()).clone();
                        let target = attrs
                            .get("target")
                            .cloned()
                            .unwrap_or_else(|| "#body".to_string());
                        let swap = attrs
                            .get("swap")
                            .cloned()
                            .unwrap_or_else(|| "innerHTML".to_string());

                        Ok(format!(
                            r#"<form data-post="{}" data-target="{}" data-swap="{}">{}</form>"#,
                            endpoint, target, swap, inner
                        ))
                    }
                    _ => self.render_nodes(children, context),
                }
            }
        }
    }

    fn render_if(
        &self,
        attrs: &HashMap<String, String>,
        children: &[Node],
        context: &mut Context,
    ) -> Result<String, String> {
        let condition = attrs.get("cond").cloned().unwrap_or_default();
        let is_true = self.eval_condition(&condition, context);

        let (true_nodes, false_nodes) = self.split_if_children(children);

        if is_true {
            self.render_nodes(&true_nodes, context)
        } else {
            self.render_nodes(&false_nodes, context)
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
    ) -> Result<String, String> {
        let expr = attrs.get("in").cloned().unwrap_or_default();
        let (item_var, source) = parse_for_expr(&expr);

        let Some(Value::Array(items)) = context.get_value(&source) else {
            return Ok(String::new());
        };

        let mut output = String::new();
        for item in items {
            let mut loop_ctx = context.clone();
            loop_ctx.set_value(&item_var, item);
            output.push_str(&self.render_nodes(children, &mut loop_ctx)?);
        }
        Ok(output)
    }

    fn render_bind(
        &self,
        attrs: &HashMap<String, String>,
        children: &[Node],
        context: &mut Context,
    ) -> Result<String, String> {
        let var = attrs.get("var").map(String::as_str).unwrap_or("value");
        let bound = if let Some(from) = attrs.get("from") {
            context
                .get_value(from)
                .unwrap_or(Value::String(String::new()))
        } else {
            Value::String(attrs.get("value").cloned().unwrap_or_default())
        };

        let mut scoped = context.clone();
        scoped.set_value(var, bound);
        self.render_nodes(children, &mut scoped)
    }

    fn render_compose(
        &self,
        attrs: &HashMap<String, String>,
        children: &[Node],
        context: &mut Context,
    ) -> Result<String, String> {
        let op = attrs
            .get("op")
            .map(|v| ComposeOp::from_attr(v))
            .unwrap_or(ComposeOp::Sum);
        let segments = self.split_compose_children(children);
        let mut composed = Hypertext::empty();

        for segment in segments {
            let rendered = self.render_nodes(&segment, context)?;
            composed = composed.compose(Hypertext::pure(rendered), op);
        }

        Ok(composed.render())
    }

    fn split_compose_children(&self, children: &[Node]) -> Vec<Vec<Node>> {
        let mut groups: Vec<Vec<Node>> = vec![Vec::new()];
        for node in children {
            if let Node::VoidElement { name, .. } = node {
                if name == "then" {
                    groups.push(Vec::new());
                    continue;
                }
            }
            if let Some(last) = groups.last_mut() {
                last.push(node.clone());
            }
        }
        groups.into_iter().filter(|g| !g.is_empty()).collect()
    }

    fn render_component_use(
        &self,
        attrs: &HashMap<String, String>,
        children: &[Node],
        context: &mut Context,
    ) -> Result<String, String> {
        let Some(id) = attrs.get("id") else {
            return Ok(String::new());
        };
        let Some(component_nodes) = context.get_component(id) else {
            return Ok(String::new());
        };

        let blocks = self.extract_blocks_local(children);
        let resolved = self.inject_blocks(component_nodes, &blocks);
        self.render_nodes(&resolved, context)
    }

    fn eval_condition(&self, condition: &str, context: &Context) -> bool {
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").collect();
            if parts.len() == 2 {
                let left = context.get(parts[0].trim());
                let right = parts[1].trim().trim_matches('"').trim_matches('\'');
                return left == right;
            }
        }
        !context.get(condition).is_empty()
    }

    fn wrap_html(&self, body: &str) -> String {
        use crate::features::oxml_tags;
        let mut head = vec![
            ONode::void(oxml_tags::META)
                .attr("charset", "UTF-8")
                .build()
                .render(),
            ONode::void(oxml_tags::META)
                .attr("name", "viewport")
                .attr("content", "width=device-width, initial-scale=1.0")
                .build()
                .render(),
            ONode::content(oxml_tags::TITLE)
                .text(&self.site_name)
                .build()
                .render(),
        ];

        if let Some(desc) = &self.site_description {
            head.push(
                ONode::void(oxml_tags::META)
                    .attr("name", "description")
                    .attr("content", desc)
                    .build()
                    .render(),
            );
        }

        if let Some(icon) = &self.favicon {
            head.push(
                ONode::void(oxml_tags::LINK)
                    .attr("rel", "icon")
                    .attr("href", icon)
                    .build()
                    .render(),
            );
        }

        head.push(
            ONode::void(oxml_tags::LINK)
                .attr("rel", "stylesheet")
                .attr("href", "/static/css/style.css")
                .build()
                .render(),
        );
        head.push(format!("<script src=\"/hrml.js\"></script>"));

        crate::features::doc(ONode::raw(head.join("\n")), ONode::raw(body.to_string()))
    }

    fn render_mdx(&self, attrs: &HashMap<String, String>) -> Result<String, String> {
        let file = attrs
            .get("file")
            .ok_or_else(|| "mdx directive requires file attribute".to_string())?;
        let full_path = self.base_path.join(file);
        let source = fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read mdx file {}: {}", file, e))?;

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

    fn render_markdown(&self, attrs: &HashMap<String, String>) -> Result<String, String> {
        let file = attrs
            .get("file")
            .ok_or_else(|| "markdown directive requires file attribute".to_string())?;
        let full_path = self.base_path.join(file);
        let source = fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read markdown file {}: {}", file, e))?;

        let rendered = crate::features::render_markdown(&source);
        Ok(crate::features::render_math_delimiters(&rendered))
    }

    fn render_markdown_with_frontmatter(
        &self,
        attrs: &HashMap<String, String>,
        context: &mut Context,
    ) -> Result<String, String> {
        let file = attrs
            .get("file")
            .ok_or_else(|| "markdownfm directive requires file attribute".to_string())?;
        let full_path = self.base_path.join(file);
        let source = fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read markdown file {}: {}", file, e))?;

        let (meta, html) = crate::features::render_markdown_with_frontmatter(&source);
        if let Some(as_key) = attrs.get("as") {
            context.set_value(as_key, meta);
        }

        Ok(crate::features::render_math_delimiters(&html))
    }

    fn render_latex(&self, attrs: &HashMap<String, String>) -> Result<String, String> {
        let formula = attrs
            .get("formula")
            .ok_or_else(|| "latex directive requires formula attribute".to_string())?;
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
enum Node {
    Text(String),
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
    fn is_void(name: &str) -> bool {
        matches!(
            name,
            "load"
                | "get"
                | "else"
                | "include"
                | "pure"
                | "then"
                | "mdx"
                | "markdown"
                | "markdownfm"
                | "latex"
                | "meta"
                | "linktag"
                | "title"
                | "og"
                | "twitter"
                | "charset"
                | "viewport"
                | "canonical"
                | "description"
                | "robots"
                | "stylesheet"
                | "script"
                | "wasm"
                | "input"
                | "br"
                | "hr"
                | "img"
                | "area"
                | "base"
                | "col"
                | "embed"
                | "param"
                | "source"
                | "track"
                | "wbr"
        )
    }
}

// --- Parser ---

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn parse(&mut self) -> Result<Vec<Node>, String> {
        let mut nodes = Vec::new();
        let mut iterations = 0;
        while self.pos < self.chars.len() {
            iterations += 1;
            if iterations > 10000 {
                return Err(format!("Parser infinite loop detected at pos {}", self.pos));
            }
            if let Some(node) = self.parse_node()? {
                nodes.push(node);
            } else {
                break;
            }
        }
        Ok(nodes)
    }

    fn parse_until(&mut self, closing_tag: &str) -> Result<Vec<Node>, String> {
        let mut nodes = Vec::new();
        let mut iterations = 0;
        while self.pos < self.chars.len() {
            iterations += 1;
            if iterations > 10000 {
                return Err(format!(
                    "Parser infinite loop in parse_until('{}') at pos {}",
                    closing_tag, self.pos
                ));
            }
            if self.is_closing(closing_tag) {
                self.consume_closing(closing_tag);
                return Ok(nodes);
            }
            if let Some(node) = self.parse_node()? {
                nodes.push(node);
            } else {
                break;
            }
        }
        Ok(nodes)
    }

    fn is_closing(&self, name: &str) -> bool {
        let pattern = format!("</?{}", name);

        if self.starts_with(&pattern) {
            let end = self.pos + pattern.len();
            if end >= self.chars.len() {
                return true;
            }
            let next = self.chars[end];
            if next.is_whitespace() || next == '?' || next == '>' {
                return true;
            }
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

    fn parse_node(&mut self) -> Result<Option<Node>, String> {
        if self.pos >= self.chars.len() {
            return Ok(None);
        }

        if self.starts_with("<?") && !self.starts_with("</?") {
            return self.parse_element().map(Some);
        } else if self.starts_with("</?") {
            return Ok(Some(Node::Text(self.consume_text())));
        }

        Ok(Some(Node::Text(self.consume_text())))
    }

    fn consume_text(&mut self) -> String {
        let mut text = String::new();
        while self.pos < self.chars.len() {
            if self.starts_with("<?") && !self.starts_with("</?") {
                break;
            }
            if self.is_any_closing() {
                break;
            }
            text.push(self.chars[self.pos]);
            self.pos += 1;
        }
        text
    }

    fn is_any_closing(&self) -> bool {
        self.starts_with("</?")
    }

    fn parse_element(&mut self) -> Result<Node, String> {
        self.pos += 2;

        let name = self.consume_identifier();
        let attrs = self.parse_attributes();

        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }

        if self.starts_with("?>") {
            self.pos += 2;
        } else {
            while self.pos < self.chars.len() && !self.starts_with("?>") {
                self.pos += 1;
            }
            if self.starts_with("?>") {
                self.pos += 2;
            }
        }

        let is_void = Node::is_void(&name) || !self.has_matching_closing(&name);

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

    fn has_matching_closing(&self, name: &str) -> bool {
        let pattern = format!("</?{}", name);
        let mut i = self.pos;
        while i + pattern.len() <= self.chars.len() {
            let mut matched = true;
            for (j, ch) in pattern.chars().enumerate() {
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
}

impl Context {
    fn new(data: Value) -> Self {
        Self {
            data,
            vars: HashMap::new(),
            components: HashMap::new(),
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
