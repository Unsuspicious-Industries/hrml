use crate::config::Config;
use crate::template::{
    AstNode, Engine, TemplateAst, TemplateError, TemplateErrorPhase, TemplateResult,
};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct HrFile {
    pub path: String,
    pub text: String,
    pub tree: Option<TemplateAst>,
}

impl HrFile {
    pub fn new(path: String, text: String) -> Self {
        Self {
            path,
            text,
            tree: None,
        }
    }

    pub fn parse(&mut self) -> TemplateResult<()> {
        if self.tree.is_none() {
            let ast = Engine::parse_source_with_path(&self.text, Some(&self.path))?;
            self.tree = Some(ast);
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Project {
    pub config: Config,
    pub files: BTreeMap<String, HrFile>,
    pub dependency_order: Vec<String>,
    engine: Engine,
}

impl Project {
    pub fn new(config: Config) -> Self {
        Self {
            config: config.clone(),
            files: BTreeMap::new(),
            engine: Engine::with_config(&config),
            dependency_order: Vec::new(),
        }
    }

    pub fn add_file(&mut self, path: String, text: String) {
        self.files.insert(path.clone(), HrFile::new(path, text));
    }

    pub fn get_file(&self, path: &str) -> Option<&HrFile> {
        self.files.get(path)
    }

    pub fn get_file_mut(&mut self, path: &str) -> Option<&mut HrFile> {
        self.files.get_mut(path)
    }

    pub fn update_file(&mut self, path: &str, text: String) -> TemplateResult<()> {
        let file = self.files.get_mut(path).ok_or_else(|| {
            TemplateError::code(
                TemplateErrorPhase::Resolve,
                format!("File not found: {}", path),
            )
            .with_template_path(path)
        })?;
        file.text = text;
        file.tree = None;
        file.parse()
    }

    pub fn parse_all(&mut self) -> TemplateResult<()> {
        for file in self.files.values_mut() {
            file.parse()?;
        }
        Ok(())
    }

    /// Compute topological resolution order (sinks first).
    pub fn resolve_order(&self) -> TemplateResult<Vec<String>> {
        let mut deps_of: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut loaded_by: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for (path, file) in &self.files {
            let tree = match file.tree.as_ref() {
                Some(t) => t,
                None => continue,
            };
            let loads = extract_load_targets(&tree.nodes);
            let owned: Vec<String> = loads
                .into_iter()
                .filter(|t| self.files.contains_key(t))
                .collect();
            if !owned.is_empty() {
                for t in &owned {
                    loaded_by.entry(t.clone()).or_default().push(path.clone());
                }
                deps_of.insert(path.clone(), owned);
            }
        }

        let mut in_degree: BTreeMap<String, usize> =
            self.files.keys().map(|k| (k.clone(), 0)).collect();
        for (path, deps) in &deps_of {
            in_degree.insert(path.clone(), deps.len());
        }

        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(k, _)| k.clone())
            .collect();
        let mut order = Vec::new();

        while let Some(node) = queue.pop() {
            order.push(node.clone());
            if let Some(dependents) = loaded_by.get(&node) {
                for dep in dependents {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        }

        if order.len() != self.files.len() {
            let remaining: Vec<&str> = self
                .files
                .keys()
                .filter(|k| !order.contains(k))
                .map(|s| s.as_str())
                .collect();
            let mut msg = String::from("Circular dependency. Remaining: ");
            for r in &remaining {
                msg.push_str(r);
                msg.push_str(", ");
            }
            return Err(TemplateError::code(TemplateErrorPhase::Resolve, msg));
        }

        Ok(order)
    }

    /// Inject block content into slot placeholders at the AST level.
    fn inject_blocks_ast(
        nodes: Vec<AstNode>,
        blocks: &BTreeMap<String, Vec<AstNode>>,
    ) -> Vec<AstNode> {
        let mut result = Vec::new();
        for node in nodes {
            match &node {
                AstNode::Element {
                    name,
                    attrs,
                    children,
                } if name == "slot" => {
                    if let Some(id) = attrs.get("id") {
                        if let Some(block_content) = blocks.get(id) {
                            result.extend(block_content.clone());
                            continue;
                        }
                    }
                    result.push(AstNode::Element {
                        name: name.clone(),
                        attrs: attrs.clone(),
                        children: Self::inject_blocks_ast(children.clone(), blocks),
                    });
                }
                AstNode::VoidElement { name, attrs } if name == "slot" => {
                    if let Some(id) = attrs.get("id") {
                        if let Some(block_content) = blocks.get(id) {
                            result.extend(block_content.clone());
                            continue;
                        }
                    }
                    result.push(node);
                }
                _ => result.push(node),
            }
        }
        result
    }

    /// Resolve `<?load file="...">` through the in-memory project tree (no disk I/O).
    fn resolve_loads_in_ast(
        &self,
        nodes: &[AstNode],
        path: &str,
        visited: &mut Vec<String>,
    ) -> TemplateResult<Vec<AstNode>> {
        let blocks = extract_ast_blocks(nodes);
        let mut result = Vec::new();

        for node in nodes {
            match node {
                AstNode::VoidElement { name, attrs } if name == "load" => {
                    let file = attrs.get("file").ok_or_else(|| {
                        TemplateError::code(
                            TemplateErrorPhase::Resolve,
                            "Load missing 'file' attribute".to_string(),
                        )
                        .with_template_path(path)
                    })?;

                    if visited.contains(&file.to_string()) {
                        let cycle: Vec<&str> = visited.iter().map(|s| s.as_str()).collect();
                        return Err(TemplateError::code(
                            TemplateErrorPhase::Resolve,
                            format!("Circular dependency: {} -> {}", cycle.join(" -> "), file),
                        )
                        .with_template_path(path));
                    }
                    visited.push(file.to_string());

                    let loaded = self.files.get(file).ok_or_else(|| {
                        TemplateError::code(
                            TemplateErrorPhase::Resolve,
                            format!("Loaded file not found: {}", file),
                        )
                        .with_template_path(path)
                    })?;
                    let loaded_ast = loaded.tree.as_ref().ok_or_else(|| {
                        TemplateError::code(
                            TemplateErrorPhase::Resolve,
                            format!("File not parsed: {}", file),
                        )
                        .with_template_path(path)
                    })?;

                    let resolved_loaded =
                        self.resolve_loads_in_ast(&loaded_ast.nodes, file, visited)?;
                    let injected = Self::inject_blocks_ast(resolved_loaded, &blocks);
                    visited.pop();
                    result.extend(injected);
                }
                AstNode::Element {
                    name,
                    attrs,
                    children,
                } => {
                    let resolved_children = self.resolve_loads_in_ast(children, path, visited)?;
                    result.push(AstNode::Element {
                        name: name.clone(),
                        attrs: attrs.clone(),
                        children: resolved_children,
                    });
                }
                _ => result.push(node.clone()),
            }
        }

        Ok(result)
    }

    pub fn render(&self, path: &str, data: &Value) -> TemplateResult<String> {
        let file = self.files.get(path).ok_or_else(|| {
            TemplateError::code(
                TemplateErrorPhase::Resolve,
                format!("File not found: {}", path),
            )
            .with_template_path(path)
        })?;

        let tree = file.tree.as_ref().ok_or_else(|| {
            TemplateError::code(
                TemplateErrorPhase::Resolve,
                format!("File not parsed: {}", path),
            )
            .with_template_path(path)
        })?;

        let mut visited = Vec::new();
        let resolved = self.resolve_loads_in_ast(&tree.nodes, path, &mut visited)?;

        let nodes: Vec<crate::template::Node> = resolved
            .into_iter()
            .map(crate::template::Node::from_ast_node)
            .collect();

        let result = self.engine.render_nodes_from_tree(&nodes, data)?;
        Ok(result.render())
    }

    pub fn render_fragment(&self, path: &str, data: &Value) -> TemplateResult<String> {
        let file = self.files.get(path).ok_or_else(|| {
            TemplateError::code(
                TemplateErrorPhase::Resolve,
                format!("File not found: {}", path),
            )
            .with_template_path(path)
        })?;

        let tree = file.tree.as_ref().ok_or_else(|| {
            TemplateError::code(
                TemplateErrorPhase::Resolve,
                format!("File not parsed: {}", path),
            )
            .with_template_path(path)
        })?;

        let mut visited = Vec::new();
        let resolved = self.resolve_loads_in_ast(&tree.nodes, path, &mut visited)?;

        let nodes: Vec<crate::template::Node> = resolved
            .into_iter()
            .map(crate::template::Node::from_ast_node)
            .collect();

        let result = self.engine.render_nodes_from_tree(&nodes, data)?;
        Ok(result.render())
    }

    pub fn pages(&self) -> impl Iterator<Item = &str> {
        self.files
            .keys()
            .filter(|p| p.ends_with(".hrml"))
            .map(|s| s.as_str())
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

fn extract_load_targets(nodes: &[AstNode]) -> Vec<String> {
    let mut targets = Vec::new();
    for node in nodes {
        match node {
            AstNode::VoidElement { name, attrs } if name == "load" => {
                if let Some(file) = attrs.get("file") {
                    targets.push(file.clone());
                }
            }
            AstNode::Element { children, .. } => {
                targets.extend(extract_load_targets(children));
            }
            _ => {}
        }
    }
    targets
}

fn extract_ast_blocks(nodes: &[AstNode]) -> BTreeMap<String, Vec<AstNode>> {
    let mut blocks = BTreeMap::new();
    for node in nodes {
        if let AstNode::Element {
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
