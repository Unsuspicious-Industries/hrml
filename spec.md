# HRML Specification

## Scope

HRML is a Rust-native framework for server-rendered HTML applications with filesystem-based API endpoints.

## Configuration

`hrml.toml` supports nested tables and flat fallbacks.

Preferred nested form:

```toml
[server]
host = "127.0.0.1"
port = 8080

[paths]
templates = "templates"
endpoints = "endpoints"
static = "static"

[site]
name = "My Site"
description = "Description"
favicon = "/static/favicon.ico"
```

## Templating

### Core directives

- `load`: include another template
- `slot` + `block`: layout content projection
- `set` + `get`: scoped values and data reads
- `if` + `else`: conditional rendering
- `for`: basic loop rendering

### Interaction directives

- `btn`: transforms to interactive button with data attributes
- `form`: transforms to interactive form with data attributes
- `link`: transforms to AJAX-style anchor

### WebAssembly directive

- `wasm`: renders a mount window with attributes:
  - `data-wasm-module`
  - `data-wasm-src`
  - `data-wasm-export`
  - `data-wasm-start`
  - `data-wasm-props`

## Rendering Model

1. Resolve `load` graph recursively.
2. Extract and inject `block` content into matching `slot`s.
3. Render final node tree with context.
4. If output is a full HTML document (`<!DOCTYPE html>` or `<html`), return as-is.
5. Otherwise wrap with default document shell.

## Routing

- `GET /` -> `templates/pages/index.hrml`
- `GET /<name>` -> `templates/pages/<name>.hrml`
- `GET|POST|DELETE /api/*` -> Rust backend runtime
- `/static/*` -> static file service
- `/hrml.js` -> built-in client runtime

## Endpoint Runtime (Rust)

Endpoint source root: `<paths.endpoints>/api`

Resolution order per request:

1. `<module>/<action>.hrml` (if action exists)
2. `<module>_<action>.hrml` (if action exists)
3. `<module>.hrml`
4. same order with `.html`
5. same order with `.txt`
6. same order with `.json`

Context passed to `.hrml` endpoint templates:

- `id`: numeric segment when present
- `action`: action segment when present
- `data`: parsed request payload map

`.html` and `.txt` return raw text.

`.json` returns parsed JSON.

## Client Runtime

`/hrml.js` handles:

- `data-post` click actions
- `data-get` click actions
- `data-post` form submissions
- checkbox `data-post` updates
- WASM window mounting on `DOMContentLoaded`

## Packaging

Crate exposes:

- library modules (`template`, `backend`, `html`, `native_utils`, etc.)
- standalone binary target `hrml`

`rusqlite` uses `bundled` feature for easier standalone builds.
