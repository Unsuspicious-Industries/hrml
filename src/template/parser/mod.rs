use crate::template::{error::TemplateResult, Node};

pub trait ParseTree {
    fn parse(&self, source: &str, template_path: Option<&str>) -> TemplateResult<Vec<Node>>;
}

pub mod hparser;
pub mod tparser;
