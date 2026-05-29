---
name: hrml-architecture
description: Core architecture of the HRML/xrml template engine after the 2026-05 unification
metadata:
  type: project
---

HRML/xrml is a declarative, functional web framework (template engine + SSG + dev server) built for Unsuspicious Industries. See [[unsuspicious-style]].

**Pipeline:** parse → resolve → render. One canonical tree type `Node` (`src/template/ast.rs`), `BTreeMap` attrs for deterministic output. `Engine` (disk-backed) and `Project` (in-memory file map) both delegate to ONE resolver, `src/template/resolve.rs`, parameterized by a `Fetch` closure.

**resolve.rs invariants:**
- `inject_blocks` = capture-avoiding slot substitution; `<?component?>` subtrees are substitution boundaries (own their slots until `<?use?>` instantiates them).
- A file's *top-level* `<?block?>`s fill the slots of files it loads; `<?block?>`s nested inside `<?use?>` are component-instantiation args and are preserved.
- Only pure data-pipeline void directives (`PREAMBLE_DIRECTIVES` = data/sort/filter/slice) are hoisted ahead of expanded loads.

**Production path:** CLI (`cli/`) uses `Project` exclusively (build/serve/check). `Engine` disk methods are used by tests/validation and as the render primitive (`render_nodes_from_tree`).

**Directives** live in `render_node` (`src/template.rs`); the `<head>` family is factored into the composable `src/template/head.rs`.

**Determinism check:** `cargo run --bin xrml -- build usi` then md5 `usi/dist/*.html`; stable hash = 03d7e9eca33eb4138a6f73a9a62ca302 as of this writing.

Binary name is `xrml` (not `hrml`). Tests: `cargo test` (unit + validation + prop).
