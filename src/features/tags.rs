use std::collections::HashMap;

pub type VoidTagHandler = fn(&HashMap<String, String>) -> Result<String, String>;
pub type BlockTagHandler = fn(&HashMap<String, String>, &str) -> Result<String, String>;

#[derive(Clone, Default)]
pub struct TagRegistry {
    void_handlers: HashMap<String, VoidTagHandler>,
    block_handlers: HashMap<String, BlockTagHandler>,
}

impl TagRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_void(&mut self, name: &str, handler: VoidTagHandler) {
        self.void_handlers.insert(name.to_string(), handler);
    }

    pub fn register_block(&mut self, name: &str, handler: BlockTagHandler) {
        self.block_handlers.insert(name.to_string(), handler);
    }

    pub fn render_void(
        &self,
        name: &str,
        attrs: &HashMap<String, String>,
    ) -> Option<Result<String, String>> {
        self.void_handlers.get(name).map(|h| h(attrs))
    }

    pub fn render_block(
        &self,
        name: &str,
        attrs: &HashMap<String, String>,
        inner_html: &str,
    ) -> Option<Result<String, String>> {
        self.block_handlers.get(name).map(|h| h(attrs, inner_html))
    }
}
