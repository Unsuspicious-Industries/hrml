# HRML Tag Library

This document describes the built-in tag library, metadata tags, and extension model.

## Core Composition Tags

- `<?load file="..."?>` include another template
- `<?slot id="..."?> ... </?slot?>` declare replaceable slot
- `<?block slot="..."?> ... </?block?>` fill a slot
- `<?component id="..."?> ... </?component?>` declare reusable component
- `<?use id="..."?> ... </?use?>` instantiate component and pass blocks
- `<?compose op="sum|product"?> ... <?then?> ... </?compose?>` algebraic composition
- `<?bind var="x" from="path"?> ... </?bind?>` scoped binding

## Data and Control Tags

- `<?set id="k" value="v"?>` set scalar variable
- `<?get id="k"?>` get variable or JSON path
- `<?if cond="expr"?> ... <?else?> ... </?if?>` conditional render
- `<?for in="item in data.items"?> ... </?for?>` array iteration

## Content Feature Tags

- `<?mdx file="content/post.mdx"?>` markdown/MDX render with math delimiter pass
- `<?markdown file="content/post.md"?>` markdown render with math delimiter pass
- `<?markdownfm file="content/post.md" as="post"?>` render markdown and expose frontmatter JSON in variable `post`
- `<?latex formula="x^2+y^2" mode="inline|block"?>` latex wrapper tag

## Metadata and Head Tags

- `<?title value="My Title"?>`
- `<?meta name="description" content="..."?>`
- `<?linktag rel="canonical" href="https://..."?>`
- `<?og name="title" content="..."?>` -> `property="og:title"`
- `<?twitter name="card" content="summary_large_image"?>` -> `name="twitter:card"`
- `<?charset value="UTF-8"?>`
- `<?viewport content="width=device-width,initial-scale=1"?>`
- `<?canonical href="https://..."?>`
- `<?description content="..."?>`
- `<?robots content="index,follow"?>`
- `<?stylesheet href="/assets/site.css"?>`
- `<?script src="/assets/app.js" defer?>`

### Variable Resolution in Meta Tags

For meta-like tags, values prefixed with `$` are resolved from template context.

Example:

```hrml
<?set id="page_title" value="USI"?>
<?title value="$page_title"?>
<?og name="title" content="$page_title"?>
```

## Extension System

`Engine` supports custom tag registration through `TagRegistry`.

### Register a custom void tag

```rust
use hrml::template::Engine;
use std::collections::HashMap;

fn stamp(attrs: &HashMap<String, String>) -> Result<String, String> {
    Ok(format!("<span>{}</span>", attrs.get("value").cloned().unwrap_or_default()))
}

let mut engine = Engine::new("./templates");
engine.register_void_tag("stamp", stamp);
```

### Register a custom block tag

```rust
use hrml::template::Engine;
use std::collections::HashMap;

fn wrap(attrs: &HashMap<String, String>, inner: &str) -> Result<String, String> {
    let class = attrs.get("class").cloned().unwrap_or_else(|| "box".into());
    Ok(format!("<div class=\"{}\">{}</div>", class, inner))
}

let mut engine = Engine::new("./templates");
engine.register_block_tag("wrap", wrap);
```

The registry is intended for domain-specific tags and future plug-in style extension.

## OXML (Oxidized Markup Language)

For Rust-native composable HTML construction, HRML exposes OXML utilities in `src/features/oxml.rs`.

Key types:

- `ONode`: algebraic node enum (`Text`, `Raw`, `Elem`, `Fragment`, `Empty`)
- `OElemBuilder`: native builder API
- `doc(head, body)`: full document helper

Core algebraic operations:

- `a.then(b)` to concatenate fragments
- `a + b` as an alias for concatenation
- `node.wrap("section")` to wrap a node in a parent tag
- `node.map_text(f)` for structural text transforms
- `node.render_pretty()` for debug-friendly output

Example:

```rust
use hrml::features::{doc, ONode};

let head = ONode::elem("title").text("USI").build();

let body = ONode::elem("main")
    .class("page")
    .child(ONode::elem("h1").text("Hello").build())
    .child(ONode::elem("p").text("Composable Rust HTML").build())
    .build();

let html = doc(head, body);
```
