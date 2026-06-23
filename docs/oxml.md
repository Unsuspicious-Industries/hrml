# OXML - Oxidized Markup Language

## Formal Specification

OXML is a **closed algebraic system** for constructing valid HTML in Rust.
The type system guarantees HTML validity by construction - invalid states are unrepresentable.

## Algebraic Structure

### Core Types

```
ONode = Empty
      | Text(String)           -- escaped text
      | Raw(String)            -- raw HTML (escape hatch)
      | Content(OContent)      -- typed element with children
      | Void(OVoid)            -- typed void element
      | Cat(Vec<ONode>)        -- ordered sequence
```

### Laws

1. **Monoid**: `ONode` with `Empty` as identity and `cat2`/`+` as binary operation
   - Associativity: `(a.cat2(b)).cat2(c) = a.cat2(b.cat2(c))`
   - Identity: `Empty.cat2(a) = a.cat2(Empty) = a`

2. **Closed under composition**: For all `a, b : ONode`, `a.cat2(b) : ONode`

3. **Void isolation by construction**: `OVoid` has no `child()`/`text()` methods - void elements cannot have children at the type level.

4. **Content safety**: `OContent` always renders with matching open/close tags.

### Operations

| Operation | Signature | Description |
|-----------|-----------|-------------|
| `cat2`    | `ONode × ONode → ONode` | Monoidal append, flattens nested Cats |
| `+`       | `ONode × ONode → ONode` | Alias for `cat2` (Add trait) |
| `+=`      | `ONode × ONode → ()` | In-place append (AddAssign trait) |
| `wrap`    | `ONode × ElemTag → ONode` | Wrap node inside new content element |
| `insert_at`| `ONode × ℕ × ONode → ONode` | Insert at index in Cat sequence |
| `map_text`| `ONode × (str → str) → ONode` | Transform all text nodes |
| `compact` | `ONode → ONode` | Remove Empty nodes from Cat sequences |

### Element Classification

```
ElemTag = { name: &'static str, kind: ElemKind }

ElemKind = Void | Content
```

Void tags: `meta`, `link`, `img`, `input`, `br`, `hr`, `area`, `base`, `col`, `embed`, `param`, `source`, `track`, `wbr`

## Type-Level Invariants

### Invariant 1: Void elements cannot have children

The `VoidBuilder` type has no `child()`, `text()`, `raw()`, or `children()` methods:

```rust
// This compiles:
ONode::void(tags::META).attr("charset", "UTF-8").build()

// This does NOT compile:
// ONode::void(tags::META).child(ONode::text("x"))  // no method!
```

### Invariant 2: Content elements always have closing tags

`ContentBuilder` builds `OContent` which always renders `<tag>...</tag>`:

```rust
ONode::content(tags::DIV).build().render()  // "<div></div>"
```

### Invariant 3: Composition is closed

Any composition of valid `ONode` values yields a valid `ONode`:

```rust
let a = ONode::content(tags::DIV).build();
let b = ONode::void(tags::BR).build();
let c = ONode::Empty;
let composed = a.cat2(b).cat2(c);  // Still ONode, always valid
```

### Invariant 4: Deterministic output

Attributes are stored in `BTreeMap<String, String>`, ensuring deterministic attribute ordering regardless of insertion order.

## API Reference

### Constructors

```rust
// By type (compile-time checked)
ONode::content(ElemTag) -> ContentBuilder
ONode::void(ElemTag) -> VoidBuilder

// Terminals
ONode::text(impl Into<String>) -> ONode
ONode::raw(impl Into<String>) -> ONode
ONode::empty() -> ONode

// Sequences
ONode::cat(Vec<ONode>) -> ONode  // flattened Cat
```

### ContentBuilder

```rust
.attr(key, value)          // add attribute
.attr_if(cond, key, value) // conditional attribute
.class(value)              // shorthand for attr("class", ...)
.id(value)                 // shorthand for attr("id", ...)
.data(key, value)          // shorthand for attr("data-{key}", ...)
.child(node)               // add child node
.text(value)               // add escaped text child
.raw(value)                // add raw HTML child
.children(nodes)           // add multiple children
.build() -> ONode          // finalize
```

### VoidBuilder

```rust
.attr(key, value)          // add attribute
.attr_if(cond, key, value) // conditional attribute
.build() -> ONode          // finalize
```

### Convenience Shorthands

```rust
use hrml::features::{div, span, p, h1, h2, h3, h4, h5, h6};
use hrml::features::{section, article, nav, header, footer, main};
use hrml::features::{a, button, form, ul, ol, li};
use hrml::features::{table, tr, th, td};
use hrml::features::{title, style, script};
use hrml::features::{meta, link, img, br, hr, input};
```

### Document Builder

```rust
use hrml::oxml::doc;

let html = doc(
    ONode::content(tags::TITLE).text("My Page").build(),
    ONode::content(tags::MAIN).child(content).build(),
);
// Returns: "<!DOCTYPE html><html lang=\"en\">..."
```

### Typed Element Tags

```rust
use hrml::oxml::tags;

tags::DIV      // ElemTag::content("div")
tags::META     // ElemTag::void("meta")
tags::LINK     // ElemTag::void("link")
// ... complete HTML tag registry
```

## Integration with HRML Templates

OXML is used internally by the HRML template engine for:
- Building HTML documents (`wrap_html`)
- Rendering meta tags (`<?meta?>`, `<?og?>`, `<?twitter?>`)
- Rendering link tags (`<?linktag?>`, `<?canonical?>`, `<?stylesheet?>`)
- Rendering title tags (`<?title?>`)
- Rendering script tags (`<?script?>`)

Template directives render through OXML, ensuring all generated HTML is type-safe.

## Design Principles

1. **Void elements cannot have children by construction** - no error needed
2. **Composition of valid nodes is always valid** - closure property
3. **Deterministic output** - BTreeMap for attributes
4. **Algebraic operations** - monoidal cat, identity Empty
5. **Type-level classification** - `ElemTag` carries `ElemKind` at compile time
