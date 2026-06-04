//! `<head>` metadata directives, as one composable family.
//!
//! Every head directive (`<?meta?>`, `<?og?>`, `<?title?>`, `<?stylesheet?>`, …)
//! denotes a single `<head>` element. Rather than a flat method per directive,
//! they share a handful of small builders (`meta_named`, `link_rel`, …) composed
//! by one [`render`] dispatcher. Adding a head directive is one match arm, not a
//! new method on the engine.

use crate::features::oxml_tags;
use crate::oxml::{ElemTag, ONode};
use std::collections::BTreeMap;

/// Interpolates `$var` references in a directive attribute against the context.
pub type Resolve<'a> = dyn Fn(&str) -> String + 'a;

/// The directive names this module handles.
pub const DIRECTIVES: &[&str] = &[
    "meta",
    "linktag",
    "title",
    "og",
    "twitter",
    "charset",
    "viewport",
    "canonical",
    "description",
    "robots",
    "stylesheet",
    "script",
];

/// True when `name` is a `<head>` metadata directive owned by this module.
pub fn is_head_directive(name: &str) -> bool {
    DIRECTIVES.contains(&name)
}

/// Render a head directive to its HTML element, or `None` if `name` is not one.
pub fn render(name: &str, attrs: &BTreeMap<String, String>, resolve: &Resolve) -> Option<String> {
    let node = match name {
        // Pass every attribute straight through (already key-ordered by BTreeMap).
        "meta" => passthrough(oxml_tags::META, attrs, resolve),
        "linktag" => passthrough(oxml_tags::LINK, attrs, resolve),

        "title" => ONode::content(oxml_tags::TITLE)
            .text(&opt(attrs, "value", resolve).unwrap_or_default())
            .build(),

        // charset is copied verbatim (never interpolated), defaulting to UTF-8.
        "charset" => ONode::void(oxml_tags::META)
            .attr(
                "charset",
                attrs.get("value").map(String::as_str).unwrap_or("UTF-8"),
            )
            .build(),

        "viewport" => meta_named(
            "viewport",
            &or(
                attrs,
                "content",
                "width=device-width,initial-scale=1",
                resolve,
            ),
        ),
        "description" => meta_named(
            "description",
            &opt(attrs, "content", resolve).unwrap_or_default(),
        ),
        "robots" => meta_named("robots", &or(attrs, "content", "index,follow", resolve)),

        "canonical" => link_rel(
            "canonical",
            &opt(attrs, "href", resolve).unwrap_or_default(),
        ),
        "stylesheet" => link_rel(
            "stylesheet",
            &opt(attrs, "href", resolve).unwrap_or_default(),
        ),

        // Social cards prefix the attribute name with their namespace.
        "og" => prefixed_meta("property", "og:", attrs, resolve)?,
        "twitter" => prefixed_meta("name", "twitter:", attrs, resolve)?,

        "script" => {
            let src = opt(attrs, "src", resolve).unwrap_or_default();
            ONode::content(oxml_tags::SCRIPT)
                .attr("src", &src)
                .attr_if(attrs.contains_key("defer"), "defer", "defer")
                .attr_if(attrs.contains_key("async"), "async", "async")
                .build()
        }

        _ => return None,
    };
    Some(node.render())
}

/// Resolve a single attribute if present.
fn opt(attrs: &BTreeMap<String, String>, key: &str, resolve: &Resolve) -> Option<String> {
    attrs.get(key).map(|v| resolve(v))
}

/// Resolve an attribute, or fall back to a default literal.
fn or(attrs: &BTreeMap<String, String>, key: &str, default: &str, resolve: &Resolve) -> String {
    opt(attrs, key, resolve).unwrap_or_else(|| default.to_string())
}

/// `<tag a="…" b="…">` echoing every (resolved) attribute.
fn passthrough(tag: ElemTag, attrs: &BTreeMap<String, String>, resolve: &Resolve) -> ONode {
    let mut node = ONode::void(tag);
    for (key, value) in attrs {
        node = node.attr(key, resolve(value));
    }
    node.build()
}

/// `<meta name="…" content="…">`.
fn meta_named(name: &str, content: &str) -> ONode {
    ONode::void(oxml_tags::META)
        .attr("name", name)
        .attr("content", content)
        .build()
}

/// `<link rel="…" href="…">`.
fn link_rel(rel: &str, href: &str) -> ONode {
    ONode::void(oxml_tags::LINK)
        .attr("rel", rel)
        .attr("href", href)
        .build()
}

/// `<meta {key}="{prefix}{name}" content="…">` for og:/twitter: cards.
/// Returns `None` when the required `name`/`content` attributes are absent.
fn prefixed_meta(
    key: &str,
    prefix: &str,
    attrs: &BTreeMap<String, String>,
    resolve: &Resolve,
) -> Option<ONode> {
    let name = attrs.get("name")?;
    let content = attrs.get("content")?;
    Some(
        ONode::void(oxml_tags::META)
            .attr(key, &format!("{}{}", prefix, name))
            .attr("content", &resolve(content))
            .build(),
    )
}
