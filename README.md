# HRML

HRML is a Rust-first web framework focused on server-rendered HTML, small runtime JS, and file-based endpoints.

## Highlights

- Rust-native backend runtime (no Python)
- HTML templates with layout/slot/block directives
- File-based API endpoints (`.xrml`, `.html`, `.json`)
- Built-in lightweight frontend runtime at `/xrml.js`
- WebAssembly window directive for embeddable WASM spaces

## Install and Run

```bash
cargo build --release
./target/release/xrml new myapp
cd myapp
../target/release/xrml dev
```

## Project Structure

```text
myapp/
├── xrml.toml
├── templates/
│   ├── layouts/
│   │   └── base.xrml
│   ├── components/
│   └── pages/
│       └── index.xrml
├── endpoints/
│   └── api/
│       └── hello.xrml
└── static/
    ├── css/
    └── js/
```

## Templates

Supported directives:

- `<?load file="..."?>`
- `<?slot id="..."?>...<?/slot?>`
- `<?block slot="..."?>...<?/block?>`
- `<?set id="..."?>...<?/set?>`
- `<?get id="..."?>`
- `<?if cond="..."?>...<?else?>...<?/if?>`
- `<?for in="item in data.items"?>...<?/for?>`
- `<?component id="..."?>...<?/component?>`
- `<?use id="..."?>...<?/use?>`
- `<?compose op="sum|product"?>...<?then?>...<?/compose?>`
- `<?bind var="x" from="..."?>...<?/bind?>`
- `<?pure value="..."?>`
- `<?mdx file="..."?>`
- `<?markdown file="..."?>`
- `<?markdownfm file="..." as="..."?>`
- `<?latex formula="..." mode="inline|block"?>`
- `<?title value="..."?>`
- `<?meta ...?>`, `<?linktag ...?>`, `<?og ...?>`, `<?twitter ...?>`
- `<?charset?>`, `<?viewport?>`, `<?canonical ...?>`, `<?description ...?>`, `<?robots ...?>`
- `<?stylesheet href="..."?>`, `<?script src="..." defer?>`
- `<?btn ...?>...<?/btn?>`
- `<?form ...?>...<?/form?>`
- `<?wasm module="..." export="mount" props='{}'?>`

Tag library reference and extension guide: `docs/taglib.md`

Rust-native composable HTML utilities (OXML) are available under `src/features/oxml.rs`.

## Rust Backend Endpoints

API requests map to files under `endpoints/api`:

- `/api/todos` -> `endpoints/api/todos.xrml` (or `.html`, `.json`)
- `/api/todos/create` -> `endpoints/api/todos/create.xrml`
- `/api/todos/create` also supports flat fallback `endpoints/api/todos_create.xrml`

`.xrml` endpoints are rendered as fragments with request context:

- `id` (if present in path)
- `action` (if present in path)
- `data` (request JSON/form payload)

Example endpoint file:

```html
<div class="card">
  Action: <?get id="action"?>
  Title: <?get id="data.title"?>
</div>
```

## WebAssembly Windows

Use the template directive:

```html
<?wasm module="/static/js/my_wasm_app.mjs" export="mount" props='{"scene":"main"}'?>
```

This renders a WASM window placeholder. On page load, `/xrml.js` mounts the module export into that window.

## OXML — Oxidized Markup Language

OXML is a type-safe, closed algebraic system for constructing valid HTML in Rust.

- Void elements cannot have children (enforced by type system)
- Composition (`cat2` / `+`) of valid nodes is always valid
- Deterministic attribute ordering via `BTreeMap`
- Typed tag registry with compile-time void/content classification

```rust
use xrml::oxml::{doc, tags, ONode};

let html = doc(
    ONode::content(tags::TITLE).text("My Page").build(),
    ONode::content(tags::MAIN)
        .child(ONode::content(tags::H1).text("Hello").build())
        .child(ONode::void(tags::BR).build())
        .build(),
);
```

See `docs/oxml.md` for the full specification.

## Native Rust HTML Utils

- `app_shell(site_name, body)`
- `card(title, body)`
- `wasm_window(module, export, props_json)`

## CLI

- `xrml new <name>`
- `xrml dev [path]`
- `xrml serve [path]`
- `xrml check [path]`
- `xrml build [path]` (placeholder)
- `xrml version`
- `xrml help`

## Publish to crates.io

1. Ensure metadata in `Cargo.toml` is complete.
2. Run:
   - `cargo test`
   - `cargo package`
   - `cargo publish`

The crate is configured as a standalone executable binary (`xrml`).
