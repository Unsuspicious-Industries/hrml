/// Functional HTML builder for Rust
/// Provides a composable, type-safe way to construct HTML

use std::collections::HashMap;

// ============================================================================
// FULL DOCUMENT BUILDER
// ============================================================================

pub struct HtmlDoc {
    head: String,
    body: String,
}

impl HtmlDoc {
    pub fn head(mut self, content: &str) -> Self {
        self.head = content.to_string();
        self
    }

    pub fn body(mut self, content: &str) -> Self {
        self.body = content.to_string();
        self
    }

    pub fn build(self) -> String {
        format!(
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n{}\n</head>\n<body>\n{}\n</body>\n</html>",
            self.head, self.body
        )
    }
}

pub fn html() -> HtmlDoc {
    HtmlDoc {
        head: String::new(),
        body: String::new(),
    }
}

// ============================================================================
// HEAD ELEMENTS
// ============================================================================

pub struct MetaTag {
    attrs: HashMap<String, String>,
}

impl MetaTag {
    fn new() -> Self {
        Self {
            attrs: HashMap::new(),
        }
    }

    pub fn charset(mut self, charset: &str) -> Self {
        self.attrs.insert("charset".to_string(), charset.to_string());
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.attrs.insert("name".to_string(), name.to_string());
        self
    }

    pub fn content(mut self, content: &str) -> Self {
        self.attrs.insert("content".to_string(), content.to_string());
        self
    }

    pub fn build(self) -> String {
        let attrs = self
            .attrs
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, escape_attr(v)))
            .collect::<Vec<_>>()
            .join(" ");
        format!("<meta {}>", attrs)
    }
}

pub fn meta() -> MetaTag {
    MetaTag::new()
}

pub struct LinkTag {
    attrs: HashMap<String, String>,
}

impl LinkTag {
    fn new() -> Self {
        Self {
            attrs: HashMap::new(),
        }
    }

    pub fn rel(mut self, rel: &str) -> Self {
        self.attrs.insert("rel".to_string(), rel.to_string());
        self
    }

    pub fn href(mut self, href: &str) -> Self {
        self.attrs.insert("href".to_string(), href.to_string());
        self
    }

    pub fn build(self) -> String {
        let attrs = self
            .attrs
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, escape_attr(v)))
            .collect::<Vec<_>>()
            .join(" ");
        format!("<link {}>", attrs)
    }
}

pub fn link() -> LinkTag {
    LinkTag::new()
}

pub fn title(content: &str) -> String {
    format!("<title>{}</title>", escape_html(content))
}

pub fn script(content: &str) -> String {
    format!("<script>{}</script>", content)
}

// ============================================================================
// COMPOSABLE ELEMENT BUILDER
// ============================================================================

pub struct Element {
    tag: String,
    attrs: HashMap<String, String>,
    children: Vec<String>,
    self_closing: bool,
}

impl Element {
    fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_string(),
            attrs: HashMap::new(),
            children: Vec::new(),
            self_closing: false,
        }
    }

    /// Add any attribute
    pub fn attr(mut self, name: &str, value: &str) -> Self {
        self.attrs.insert(name.to_string(), value.to_string());
        self
    }

    /// Fluent API for common attributes
    pub fn class(self, class: &str) -> Self {
        self.attr("class", class)
    }

    pub fn id(self, id: &str) -> Self {
        self.attr("id", id)
    }

    pub fn href(self, href: &str) -> Self {
        self.attr("href", href)
    }

    pub fn src(self, src: &str) -> Self {
        self.attr("src", src)
    }

    pub fn type_(self, type_val: &str) -> Self {
        self.attr("type", type_val)
    }

    pub fn name(self, name: &str) -> Self {
        self.attr("name", name)
    }

    pub fn value(self, value: &str) -> Self {
        self.attr("value", value)
    }

    pub fn placeholder(self, placeholder: &str) -> Self {
        self.attr("placeholder", placeholder)
    }

    /// Data attributes for HRML interactivity
    pub fn data_post(self, url: &str) -> Self {
        self.attr("data-post", url)
    }

    pub fn data_get(self, url: &str) -> Self {
        self.attr("data-get", url)
    }

    pub fn data_delete(self, url: &str) -> Self {
        self.attr("data-delete", url)
    }

    pub fn data_target(self, selector: &str) -> Self {
        self.attr("data-target", selector)
    }

    pub fn data_swap(self, mode: &str) -> Self {
        self.attr("data-swap", mode)
    }

    /// Add child element (raw HTML)
    pub fn child(mut self, html: &str) -> Self {
        self.children.push(html.to_string());
        self
    }

    /// Add multiple children at once
    pub fn children(mut self, items: Vec<String>) -> Self {
        self.children.extend(items);
        self
    }

    /// Add text content (auto-escaped)
    pub fn text(mut self, text: &str) -> Self {
        self.children.push(escape_html(text));
        self
    }

    /// Compose with another element
    pub fn with<F>(self, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        f(self)
    }

    /// Build the final HTML string
    pub fn build(self) -> String {
        let attrs = if self.attrs.is_empty() {
            String::new()
        } else {
            format!(
                " {}",
                self.attrs
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, escape_attr(v)))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };

        if self.self_closing {
            format!("<{}{}>", self.tag, attrs)
        } else {
            format!(
                "<{}{}>{}</{}>",
                self.tag,
                attrs,
                self.children.join(""),
                self.tag
            )
        }
    }
}

// ============================================================================
// ELEMENT CONSTRUCTORS
// ============================================================================

pub fn div() -> Element {
    Element::new("div")
}

pub fn span() -> Element {
    Element::new("span")
}

pub fn p() -> Element {
    Element::new("p")
}

pub fn h1() -> Element {
    Element::new("h1")
}

pub fn h2() -> Element {
    Element::new("h2")
}

pub fn h3() -> Element {
    Element::new("h3")
}

pub fn h4() -> Element {
    Element::new("h4")
}

pub fn h5() -> Element {
    Element::new("h5")
}

pub fn h6() -> Element {
    Element::new("h6")
}

pub fn button() -> Element {
    Element::new("button")
}

pub fn a() -> Element {
    Element::new("a")
}

pub fn input() -> Element {
    let mut el = Element::new("input");
    el.self_closing = true;
    el
}

pub fn form() -> Element {
    Element::new("form")
}

pub fn ul() -> Element {
    Element::new("ul")
}

pub fn ol() -> Element {
    Element::new("ol")
}

pub fn li() -> Element {
    Element::new("li")
}

pub fn nav() -> Element {
    Element::new("nav")
}

pub fn header() -> Element {
    Element::new("header")
}

pub fn footer() -> Element {
    Element::new("footer")
}

pub fn main_el() -> Element {
    Element::new("main")
}

pub fn section() -> Element {
    Element::new("section")
}

pub fn article() -> Element {
    Element::new("article")
}

pub fn table_el() -> Element {
    Element::new("table")
}

pub fn tr() -> Element {
    Element::new("tr")
}

pub fn td() -> Element {
    Element::new("td")
}

pub fn th() -> Element {
    Element::new("th")
}

// ============================================================================
// UTILITIES
// ============================================================================

/// HTML escape for content
pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// HTML escape for attributes
pub fn escape_attr(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ============================================================================
// CONVENIENCE BUILDERS
// ============================================================================

/// Create a button with HRML data-post
pub fn post_button(text: &str, url: &str, target: &str) -> String {
    button()
        .class("btn btn-primary")
        .data_post(url)
        .data_target(target)
        .data_swap("innerHTML")
        .text(text)
        .build()
}

/// Create a link with HRML data-get
pub fn get_link(text: &str, url: &str, target: &str) -> String {
    a()
        .href("#")
        .data_get(url)
        .data_target(target)
        .data_swap("innerHTML")
        .text(text)
        .build()
}

/// Create a list from items
pub fn list(items: Vec<String>) -> String {
    ul()
        .children(
            items
                .into_iter()
                .map(|item| li().child(&item).build())
                .collect()
        )
        .build()
}
