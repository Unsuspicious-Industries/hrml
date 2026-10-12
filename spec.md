# HRML Specification

## Scope

HRML is a Rust-native framework for server-rendered HTML applications with filesystem-based API endpoints.

## Configuration

The CLI reads `xrml.toml`. Nested tables and the existing flat configuration
keys are supported.

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

### Data pipes

- `data`: load JSON / TOML / MDX / BibTeX into the context
- `filter` + `sort` + `slice` + `tally` + `concat`: list transforms (`over` → `as`);
  `filter where` speaks the same predicate language as `if cond`
- `replace`: regex transform on a string binding (`$1`…`$N` capture refs)
- Field paths dot through objects and index arrays (`$items.0.title`)

### SQLite queries

A query binds rows from an owner-published public SQLite extract to the same
array-of-objects value shape as `data`:

```toml
[queries.public_people]
sql = "select name, initials, role, bio, website from public_people order by name"
fixture = "fixtures/public_people.json"
```

```hrml
<?query name="public_people" as="people"?>
<?map over="people" as="person"?>
  <h3>$person.name</h3>
<?/map?>
```

`USI_DB_PATH` selects an absolute extract path at runtime. The reader opens it
with `mode=ro&immutable=1` and `query_only=ON`: the publisher must replace a
complete file atomically, never update that published inode in place. It must
contain public data only. Read-only access to the private owner's database is
not a table or row authorization boundary.

Each query opens a fresh connection, so a later request sees a replaced file.
Multiple queries in one render may observe different published revisions;
there is no whole-page snapshot guarantee. SQL must be one read-only `SELECT`
or `WITH` statement. The query defines row order with `ORDER BY`; the renderer
does not silently sort, discard rows or overwrite duplicate column names.

SQLite `INTEGER` maps to signed i64, finite `REAL` to f64, UTF-8 `TEXT` to a
string, and `NULL` to null. BLOB columns, non-finite reals, invalid UTF-8 and
duplicate column names are errors, not substituted values. Transform laws for
`map`, `filter` and `sort` apply to fixed input values; source reads are effects,
not a claim that arbitrary queries are pure.

Without `USI_DB_PATH`, the declared fixture supplies an array of column-name
objects and the renderer identifies it as a fixture. Missing fixtures or an
unavailable or malformed extract fail visibly; zero rows is a valid empty set.
A configured extract failure never selects the fixture.

`xrml build` rejects declared queries before reading the environment. Explicit
`--query-fixtures` selects only source fixtures for a static build; it never
reads the database. Development shells do not set `USI_DB_PATH`.

The extract's `meta` table has exactly one row: INTEGER `id` equal to 1,
nonempty TEXT `revision`, positive INTEGER `published_at` in UTC Unix seconds,
and TEXT `source_digest` containing
64 lowercase hexadecimal SHA-256 digits. The reader validates those fields and
logs revision/digest changes; it does not invent missing metadata or recompute
the producer's digest. Revision is an opaque string to the reader. The producer
uses a decimal publication counter and hashes the canonical public rows, not
private store bytes, metadata or SQLite page layout.

Markdown headings retain explicit `{#id}` attributes. Otherwise their IDs use
HRML's ASCII `slugify` of heading text and inline code, with `-2`, `-3`, … for
collisions. Empty slugs use the heading's document position, `heading-N`.
Generated IDs avoid explicit IDs; duplicate explicit IDs remain author-owned.
These anchors do not change rendered heading text, code or math.

Unknown HRML directives are errors. Ordinary HTML and registered custom tags
remain distinct from unknown processing directives.

### Styling

- `[globals]` config keys are emitted as `:root` custom properties
  (`bg_code` → `--bg-code`) ahead of all component CSS
- Component `<?style?>` blocks are tree-shaken per page
- Utility classes (Tailwind-flavoured) are generated from the same tokens:
  `text-muted` → `color: var(--text-muted)`, `bg-accent`, `font-mono`,
  spacing on the quarter-rem scale (`px-4`, `gap-2`), and layout atoms
  (`flex`, `items-center`). Only classes present in reachable markup are
  emitted; unrecognised names are left to component stylesheets.

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
- standalone binary target `xrml`

The SQLite query reader uses `rusqlite` with bundled SQLite, so the CLI's
SQLite implementation is selected by its Cargo lock rather than an ambient
system library. Endpoint runtime remains template/file-based; no Python or
other process execution is provided.
