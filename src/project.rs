use crate::config::Config;
use crate::template::resolve::{self, extract_load_targets};
use crate::template::{
    Engine, Node, TemplateAst, TemplateError, TemplateErrorPhase, TemplateResult,
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

    pub fn with_base_path(mut self, path: &std::path::Path) -> Self {
        let mut engine = Engine::new(path.to_string_lossy().as_ref());
        engine = engine.with_site_name(self.config.site_name.clone());
        if let Some(ref desc) = self.config.site_description {
            engine = engine.with_description(Some(desc.clone()));
        }
        engine = engine.with_favicon(self.config.favicon.clone());
        engine = engine.with_site_url(self.config.site_url.clone());
        engine = engine.with_globals(self.config.globals.clone());
        self.engine = engine;
        self
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

    /// Fully resolve a file's `<?load?>`s against the in-memory file map (no I/O).
    fn resolve(&self, path: &str) -> TemplateResult<Vec<Node>> {
        let tree = self
            .files
            .get(path)
            .and_then(|f| f.tree.as_ref())
            .ok_or_else(|| {
                TemplateError::code(
                    TemplateErrorPhase::Resolve,
                    format!("File not parsed: {}", path),
                )
                .with_template_path(path)
            })?;

        let fetch = |file: &str| -> TemplateResult<Vec<Node>> {
            self.files
                .get(file)
                .and_then(|f| f.tree.as_ref())
                .map(|t| t.nodes.clone())
                .ok_or_else(|| {
                    TemplateError::code(
                        TemplateErrorPhase::Resolve,
                        format!("Loaded file not found: {}", file),
                    )
                    .with_template_path(file)
                })
        };

        let nodes = resolve::with_default_layout(
            &tree.nodes,
            self.config.default_layout.as_deref(),
            &self.config.auto_imports,
        );
        let mut visited = vec![path.to_string()];
        let resolved = resolve::resolve_loads(&nodes, &fetch, &mut visited, true)?;

        // Prepend the shared component library: every `<?component?>` defined
        // under a configured component path is registered for this page, so it
        // can be used with no `<?load?>`. Definitions emit nothing, so order is
        // irrelevant to output. Explicit loads still work (and simply re-define).
        let mut out = self.component_library();
        out.extend(resolved);
        Ok(out)
    }

    /// True when `path` lives under one of the configured component directories.
    fn is_component_file(&self, path: &str) -> bool {
        self.config
            .component_paths
            .iter()
            .any(|dir| path == dir || path.starts_with(&format!("{}/", dir.trim_end_matches('/'))))
    }

    /// Collect every `<?component?>` definition from every component-library
    /// file — the implicit prelude prepended to each rendered page.
    fn component_library(&self) -> Vec<Node> {
        let mut defs = Vec::new();
        for (path, file) in &self.files {
            if self.is_component_file(path) {
                if let Some(tree) = file.tree.as_ref() {
                    resolve::collect_components(&tree.nodes, &mut defs);
                }
            }
        }
        defs
    }

    pub fn render(&self, path: &str, data: &Value) -> TemplateResult<String> {
        let nodes = self.resolve(path)?;
        let result = self.engine.render_nodes_from_tree(&nodes, data)?;

        if std::env::var("HRML_DEBUG").is_ok() {
            let html = result.render();
            eprintln!(
                "[DEBUG] render({:?}) → {} nodes → {} bytes",
                path,
                nodes.len(),
                html.len()
            );
            return Ok(html);
        }

        Ok(result.render())
    }

    pub fn render_fragment(&self, path: &str, data: &Value) -> TemplateResult<String> {
        let nodes = self.resolve(path)?;
        let result = self.engine.render_nodes_from_tree(&nodes, data)?;
        Ok(result.render())
    }

    pub fn pages(&self) -> impl Iterator<Item = &str> {
        self.files
            .keys()
            .filter(|p| p.ends_with(".hrml") || p.ends_with(".trml"))
            .map(|s| s.as_str())
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new(Config::default())
    }
}
