use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

pub struct Engine {
    base_path: PathBuf,
    site_name: String,
    site_description: Option<String>,
    favicon: Option<String>,
}

impl Engine {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: PathBuf::from(base_path),
            site_name: "HRML App".to_string(),
            site_description: None,
            favicon: None,
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

    pub fn render(&self, template_path: &str, data: &Value) -> Result<String, String> {
        let full_path = self.base_path.join(template_path);
        
        let mut context = Context::new(data.clone());
        
        // Fully resolve the template tree (handling loads and blocks recursively)
        let mut visited = std::collections::HashSet::new();
        let resolved_nodes = self.resolve_with_tracking(template_path, &mut visited)?;

        // Render the final tree
        let body = self.render_nodes(&resolved_nodes, &mut context)?;
        
        Ok(self.wrap_html(&body))
    }

    fn resolve_with_tracking(&self, template_path: &str, visited: &mut std::collections::HashSet<String>) -> Result<Vec<Node>, String> {
        eprintln!("[RESOLVE] Starting: {} (visited: {:?})", template_path, visited);
        // Check for circular dependencies
        if visited.contains(template_path) {
            return Err(format!("Circular template dependency detected: {}", template_path));
        }
        visited.insert(template_path.to_string());
        eprintln!("[RESOLVE] Reading file: {}", template_path);
        
        let full_path = self.base_path.join(template_path);
        let content = fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read template {}: {}", template_path, e))?;
        eprintln!("[RESOLVE] Read {} bytes from {}", content.len(), template_path);
            
        let mut nodes = Parser::new(&content).parse()?;
        eprintln!("[RESOLVE] Parsed {} nodes from {}", nodes.len(), template_path);
        
        // 1. Extract blocks defined in this template
        let blocks = self.extract_blocks(&nodes);
        eprintln!("[RESOLVE] Extracted {} blocks from {}", blocks.len(), template_path);
        
        // 2. Remove block Nodes from the tree
        nodes.retain(|n| !matches!(n, Node::Element { name, .. } if name == "block"));
        
        // 3. Process loads recursively
        let mut resolved_nodes = Vec::new();
        
        for node in nodes {
            if let Node::VoidElement { name, attrs } = &node {
                if name == "load" {
                    if let Some(file) = attrs.get("file") {
                        eprintln!("[RESOLVE] Found <?load file=\"{}\"?> in {}", file, template_path);
                        // Recursively resolve the loaded file with the SAME visited set
                        let mut loaded_nodes = self.resolve_with_tracking(file, visited)?;
                        eprintln!("[RESOLVE] Loaded {} nodes from {}", loaded_nodes.len(), file);
                        
                        // Apply OUR blocks to the LOADED nodes
                        eprintln!("[RESOLVE] Injecting blocks into {}", file);
                        loaded_nodes = self.inject_blocks(loaded_nodes, &blocks);
                        eprintln!("[RESOLVE] Injected blocks into {}", file);
                        
                        resolved_nodes.extend(loaded_nodes);
                        continue;
                    }
                }
            }
            resolved_nodes.push(node);
        }
        eprintln!("[RESOLVE] Finished resolving {}: {} final nodes", template_path, resolved_nodes.len());
        
        visited.remove(template_path);
        Ok(resolved_nodes)
    }

    fn resolve(&self, template_path: &str) -> Result<Vec<Node>, String> {
        let mut visited = std::collections::HashSet::new();
        self.resolve_with_tracking(template_path, &mut visited)
    }

    fn extract_blocks(&self, nodes: &[Node]) -> HashMap<String, Vec<Node>> {
        let mut blocks = HashMap::new();
        for node in nodes {
            if let Node::Element { name, attrs, children } = node {
                if name == "block" {
                    if let Some(slot) = attrs.get("slot") {
                        blocks.insert(slot.clone(), children.clone());
                    }
                }
            }
        }
        blocks
    }

    fn inject_blocks(&self, parent_nodes: Vec<Node>, blocks: &HashMap<String, Vec<Node>>) -> Vec<Node> {
        let mut new_nodes = Vec::new();
        
        for node in parent_nodes {
            match node {
                Node::Element { name, attrs, children } => {
                    if name == "slot" {
                        if let Some(id) = attrs.get("id") {
                            if let Some(block_content) = blocks.get(id) {
                                // Replace slot with block content
                                new_nodes.extend(block_content.clone());
                                continue;
                            }
                        }
                        // Default slot content
                        // Recurse into children of slot just in case
                        let processed_children = self.inject_blocks(children.clone(), blocks);
                        new_nodes.extend(processed_children);
                    } else {
                        // Recurse for other elements
                        new_nodes.push(Node::Element {
                            name,
                            attrs,
                            children: self.inject_blocks(children, blocks),
                        });
                    }
                }
                _ => new_nodes.push(node),
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
                match name.as_str() {
                    "load" => Ok(String::new()),
                    "else" => Ok(String::new()),
                    "set" => {
                        // Void set: <?set key="val" ?>
                        // We check attrs
                        for (k, v) in attrs {
                             // "key" and "value" might be explicit, or just k=v
                             if k == "id" {
                                  if let Some(val) = attrs.get("value") {
                                      context.set(k, val.clone());
                                  }
                             } else if k != "value" {
                                  // Generic <?set var="val"?>
                                  // This implementation uses strict attrs from parser.
                                  // But user might use <?set x="y"?> ??
                                  // Let's assume standard HRML: <?set id="name">val<?/set> or <?set id="name" value="val"?>
                                  if k == "id" && attrs.contains_key("value") {
                                      context.set(attrs.get("id").unwrap(), attrs.get("value").unwrap().clone());
                                  }
                             }
                        }
                        // Alternate interpretation: ANY Attribute is a set? 
                        // Let's stick to explicit id/value or matching tests.
                        if let (Some(id), Some(val)) = (attrs.get("id"), attrs.get("value")) {
                            context.set(id, val.clone());
                        }
                        Ok(String::new())
                    },
                    "get" => {
                         if let Some(id) = attrs.get("id") {
                             Ok(context.get(id))
                         } else {
                             Ok(String::new())
                         }
                    },
                    _ => Ok(String::new()), 
                }
            },
            Node::Element { name, attrs, children } => {
                match name.as_str() {
                    "block" => self.render_nodes(children, context),
                    "slot" => self.render_nodes(children, context),
                    "if" => self.render_if(attrs, children, context),
                    "for" => self.render_for(attrs, children, context),
                    "set" => {
                        if let Some(id) = attrs.get("id") {
                            let content = self.render_nodes(children, context)?;
                            context.set(id, content);
                        } else {
                            // Support <?set id="x" value="y"?> style as Element too if children empty
                            if let (Some(id), Some(val)) = (attrs.get("id"), attrs.get("value")) {
                                context.set(id, val.clone());
                            }
                        }
                        Ok(String::new())
                    },
                    "btn" => {
                        let inner = self.render_nodes(children, context)?;
                        let method = if attrs.contains_key("post") { "post" } else { "get" };
                        let endpoint = attrs.get(method).unwrap_or(&String::new()).clone();
                        let target = attrs.get("target").cloned().unwrap_or_else(|| "#body".to_string());
                        let swap = attrs.get("swap").cloned().unwrap_or_else(|| "innerHTML".to_string());
                        
                        Ok(format!(
                            r#"<button class="btn btn-primary" data-{}="{}" data-target="{}" data-swap="{}">{}</button>"#,
                            method, endpoint, target, swap, inner
                        ))
                    },
                    "link" => {
                        let inner = self.render_nodes(children, context)?;
                        let endpoint = attrs.get("get").unwrap_or(&String::new()).clone();
                        let target = attrs.get("target").cloned().unwrap_or_else(|| "#body".to_string());
                        let swap = attrs.get("swap").cloned().unwrap_or_else(|| "innerHTML".to_string());
                        
                        Ok(format!(
                            r##"<a href="#" data-get="{}" data-target="{}" data-swap="{}">{}</a>"##,
                            endpoint, target, swap, inner
                        ))
                    },
                    "form" => {
                        let inner = self.render_nodes(children, context)?;
                        let endpoint = attrs.get("post").unwrap_or(&String::new()).clone();
                        let target = attrs.get("target").cloned().unwrap_or_else(|| "#body".to_string());
                        let swap = attrs.get("swap").cloned().unwrap_or_else(|| "innerHTML".to_string());
                        
                        Ok(format!(
                            r#"<form data-post="{}" data-target="{}" data-swap="{}">{}</form>"#,
                            endpoint, target, swap, inner
                        ))
                    },
                    _ => self.render_nodes(children, context),
                }
            }
        }
    }

    fn render_if(&self, attrs: &HashMap<String, String>, children: &[Node], context: &mut Context) -> Result<String, String> {
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

    fn render_for(&self, attrs: &HashMap<String, String>, children: &[Node], context: &Context) -> Result<String, String> {
        let item_var = attrs.get("in").and_then(|s| s.split_whitespace().next()).unwrap_or("item");
        // For simplicity, just handling basic list iteration placeholder
        let items = vec!["item1", "item2", "item3"];
        
        let mut output = String::new();
        for item in items {
            let mut loop_ctx = context.clone();
            loop_ctx.set(item_var, item.to_string());
            output.push_str(&self.render_nodes(children, &mut loop_ctx)?);
        }
        Ok(output)
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
        use crate::html::{html, meta, link, title};
        
        let mut head = vec![
            meta().charset("UTF-8").build(),
            meta().name("viewport").content("width=device-width, initial-scale=1.0").build(),
            title(&self.site_name),
        ];

        if let Some(desc) = &self.site_description {
            head.push(meta().name("description").content(desc).build());
        }

        if let Some(icon) = &self.favicon {
            head.push(link().rel("icon").href(icon).build());
        }

        head.push(link().rel("stylesheet").href("/static/css/style.css").build());
        head.push(format!("<script src=\"/hrml.js\"></script>"));

        html()
            .head(&head.join("\n"))
            .body(body)
            .build()
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
        matches!(name, 
            "load" | "get" | "else" | "include" |  // HRML tags
            "input" | "br" | "hr" | "img" | "meta" | "link" | "area" | "base" | "col" | "embed" | "param" | "source" | "track" | "wbr"  // HTML void elements
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
        eprintln!("[PARSER] Starting parse, {} chars total", self.chars.len());
        let mut nodes = Vec::new();
        let mut iterations = 0;
        while self.pos < self.chars.len() {
            iterations += 1;
            if iterations % 100 == 0 {
                eprintln!("[PARSER] Iteration {}, pos={}/{}", iterations, self.pos, self.chars.len());
            }
            if iterations > 10000 {
                return Err(format!("Parser infinite loop detected at pos {}", self.pos));
            }
            if let Some(node) = self.parse_node()? {
                nodes.push(node);
            } else {
                break;
            }
        }
        eprintln!("[PARSER] Finished parsing, {} nodes, {} iterations", nodes.len(), iterations);
        Ok(nodes)
    }

    fn parse_until(&mut self, closing_tag: &str) -> Result<Vec<Node>, String> {
        eprintln!("[PARSER] Parsing until </?{}?>", closing_tag);
        let mut nodes = Vec::new();
        let mut iterations = 0;
        while self.pos < self.chars.len() {
            iterations += 1;
            if iterations > 10000 {
                return Err(format!("Parser infinite loop in parse_until('{}') at pos {}", closing_tag, self.pos));
            }
            if self.is_closing(closing_tag) {
                eprintln!("[PARSER] Found closing tag for {}", closing_tag);
                self.consume_closing(closing_tag);
                return Ok(nodes);
            }
            if let Some(node) = self.parse_node()? {
                nodes.push(node);
            } else {
                break;
            }
        }
        eprintln!("[PARSER] Reached end without finding </?{}?>", closing_tag);
        Ok(nodes)
    }

    fn is_closing(&self, name: &str) -> bool {
        let pattern = format!("</?{}", name);
        
        if self.starts_with(&pattern) {
            let end = self.pos + pattern.len();
            if end >= self.chars.len() { return true; }
            let next = self.chars[end];
            if next.is_whitespace() || next == '?' || next == '>' {
                return true;
            }
        }
        false
    }
    
    fn consume_closing(&mut self, _name: &str) {
        // Consume until ?>
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
        if self.pos >= self.chars.len() { return Ok(None); }

        if self.starts_with("<?") && !self.starts_with("</?") {
             return self.parse_element().map(Some);
        } else if self.starts_with("</?") {
             // Unexpected closing, treat as text
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
        self.pos += 2; // Skip <?
        
        let name = self.consume_identifier();
        let attrs = self.parse_attributes();
        
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() { self.pos += 1; }
        
        // Check for self-closing marker /?> which is sometimes used
        // But mainly rely on node_is_void or closing ?>
        if self.starts_with("?>") {
            self.pos += 2;
        } else {
             // Just scan for ?>
             while self.pos < self.chars.len() && !self.starts_with("?>") {
                 self.pos += 1;
             }
             if self.starts_with("?>") { self.pos += 2; }
        }

        if Node::is_void(&name) {
            Ok(Node::VoidElement { name, attrs })
        } else {
            let children = self.parse_until(&name)?;
            Ok(Node::Element { name, attrs, children })
        }
    }

    fn consume_identifier(&mut self) -> String {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() { self.pos += 1; }
        
        let mut id = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_alphanumeric() || c == '_' || c == '-' {
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
            while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() { self.pos += 1; }
            
            if self.pos >= self.chars.len() || self.starts_with("?>") || self.starts_with("/?>") {
                break;
            }
            
            let key = self.consume_identifier();
            if key.is_empty() { 
                if !self.starts_with("?>") { self.pos += 1; }
                continue; 
            }
            
            let mut value = String::new();
            while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() { self.pos += 1; }
            
            if self.pos < self.chars.len() && self.chars[self.pos] == '=' {
                self.pos += 1; // skip =
                while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() { self.pos += 1; }
                
                if self.pos < self.chars.len() {
                    let quote = self.chars[self.pos];
                    if quote == '"' || quote == '\'' {
                        self.pos += 1;
                        while self.pos < self.chars.len() && self.chars[self.pos] != quote {
                            value.push(self.chars[self.pos]);
                            self.pos += 1;
                        }
                        self.pos += 1; // skip closing quote
                    } else {
                         // Unquoted value
                         while self.pos < self.chars.len() && !self.chars[self.pos].is_whitespace() && self.chars[self.pos] != '?' && self.chars[self.pos] != '>' {
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
    vars: HashMap<String, String>,
}

impl Context {
    fn new(data: Value) -> Self {
        Self {
            data,
            vars: HashMap::new(),
        }
    }

    fn set(&mut self, key: &str, value: String) {
        self.vars.insert(key.to_string(), value);
    }

    fn get(&self, key: &str) -> String {
        if let Some(val) = self.vars.get(key) {
            return val.clone();
        }
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = &self.data;
        for part in parts {
            current = match current.get(part) {
                Some(v) => v,
                None => return String::new(),
            };
        }
        match current {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => String::new(),
        }
    }
}
