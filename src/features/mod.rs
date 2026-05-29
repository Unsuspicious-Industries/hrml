pub mod data;
pub mod latex;
pub mod markdown;
pub mod tags;
pub mod text;

pub use crate::oxml::{
    a, article, br, button, div, doc, footer, form, h1, h2, h3, h4, h5, h6, header, hr, img, input,
    li, link, main, meta, nav, ol, p, script, section, span, style, table, tags as oxml_tags, td,
    th, title, tr, ul, ContentBuilder, ElemKind, ElemTag, OContent, ONode, OVoid, VoidBuilder,
};
pub use data::{load_data_dir, load_data_file};
pub use latex::{render_latex_block, render_latex_inline, render_math_delimiters};
#[allow(unused_imports)]
pub use markdown::{render_markdown, render_markdown_with_frontmatter, render_mdx_with_math};
pub use tags::{BlockTagHandler, TagRegistry, VoidTagHandler};
pub use text::{excerpt, slugify};
