# HRML Rework Plan

Pickup notes for whoever continues this. Read end-to-end before touching code.

## North star

The framework manipulates a **single labelled, ranked tree** (the IR). Every
operation — parsing, rendering, conversion between syntaxes, variable
resolution, layout block injection — is a tree algorithm. No raw HTML strings
inside tree nodes. No regex-on-attribute-values for `$var` resolution. With the
tree as the single source of truth, correctness is statable as MSO-style
invariants over node labels and structure, and is property-testable.

Two surface syntaxes serialise the same tree:

- **TRML** (`.trml`) — indentation-based, `[name attrs]` per node. Canonical /
  default authoring format going forward.
- **HRML** (`.hrml`) — HTML-like, `<?directive?>` for HRML nodes, `<tag>` for
  HTML nodes. Supported as an alternative input format.

Both parsers produce the same `Node` tree. The renderer walks that tree and
emits HTML.

## Where we are (state at start of this work)

Codebase: `/home/pkd/code/hrml`, branch `master`.

- TRML parser (`src/template/parser/tparser.rs`) already produces a fully
  structured tree.
- HRML parser (`src/template/parser/hparser.rs`) produces tree nodes **only
  for HRML directives**. Raw HTML in source ends up as `Node::Text` blobs.
  Example: in `<a href="x"><?get id="y"?></a>`, the parser emits
  `[Text("<a href=\"x\">"), Element(get, ...), Text("</a>")]`.
- Renderer (`src/template.rs`) already has fallthrough cases that emit
  `Node::Element { name: "div", ... }` as `<div>...</div>` — so the moment the
  HRML parser starts producing HTML tree nodes, rendering still works.
- Variable references `$x.y` are resolved by `Engine::resolve()` via byte
  scanning on attribute-value strings (see `src/template.rs:1680`). Just fixed
  a bug there (dot-paths now work); the whole function should eventually be
  replaced with a tree pass.
- Converter (`src/convert.rs`) only handles HRML directives correctly; HTML
  inside text nodes degrades to escaped raw strings. Output is unusable for USI.
- USI project (`usi/templates/*.hrml`) is dense mixed HRML+HTML — the main
  real-world target.
- 5 compiler warnings fixed; failing test `test_markdown_frontmatter_and_meta_tag_library`
  fixed (dot-path resolution bug).
- Stage 1 prep done:
  - Nested-quote `<?get id="x"?>` inside `"…"` attribute values lifted to
    `$x` form across USI templates and test fixtures.
  - `Node::Text` is now resolved against the context — `$var` substitution
    applies inside text fragments, not just directive attrs (`src/template.rs:782`).
  - `src/generate.rs` now sorts HTML attrs alphabetically when serialising
    `Node::Element` / `Node::VoidElement`. Currently dormant (no HTML element
    nodes reach the renderer yet); turns deterministic the moment Stage 1
    starts producing HTML element nodes.
  - `tests/var_resolution_tests.rs` added with four invariants: $-substitution
    in text; dotted-path resolution; literal-suffix preservation; *no template
    file may contain `<?…?>` inside an attribute value* (regression guard).

Test baseline after current fixes: 155 tests pass.

**Open issue for Stage 1 body.** `tests/template_tests.rs:172` asserts
`<input type="text" name="name">` (attribute order: type, name). After HTML
tokenisation, this becomes `Node::VoidElement` and `generate::void` emits
sorted form (`name="name" type="text"`) plus ` />` (void marker). The fix is
either to update the test to the sorted form, or to preserve insertion order
on the attrs map (swap `HashMap` for `indexmap::IndexMap` or `Vec<(String,String)>`).
Recommend updating the test — sorted output is what we want long-term for
canonical serialisation.

## Plan (three stages, B-order from chat: ship IR unification + USI migration
first, automaton rewrite second).

### Stage 1 — unify the tree IR

**Goal.** HParser produces `Node::Element` / `Node::VoidElement` for HTML tags,
not `Node::Text`. After this stage, no `Node::Text` instance contains a `<` that
opens an HTML tag — text is real text only.

**Why first.** Every downstream operation (converter, renderer, future
automaton) gets simpler once the tree is real. Conversion HRML→TRML becomes a
serialiser, not a string-recovery hack.

**Scope of change.**

1. Extend `hparser::InnerParser` to recognise HTML tags as elements:
   - At `parse_node`, dispatch on lookahead:
     - `<?`  but not `</?` or `<?/`  → HRML directive (existing `parse_element`)
     - `</?` or `<?/`                → HRML closing (existing)
     - `<` followed by ASCII alpha   → **new** HTML open tag
     - `</` followed by ASCII alpha  → **new** HTML close (siblings stop here)
     - `<!--`                        → skip comment
     - anything else                 → text
   - Add `parse_html_element()` mirroring `parse_element()` but for `<tag>`
     syntax: name, attrs (quoted/unquoted, may contain `<?...?>`), then either
     `/>` (self-close) or `>` + children + `</name>`.
   - Add a void-tag table (`area, base, br, col, embed, hr, img, input, link,
     meta, source, track, wbr`). Anything else is a content element with an
     explicit closing tag.

2. Attribute-value parser **rejects** embedded `<?…?>` directives — putting
   code in quotes is illegal. Any `<?` encountered while reading a `"…"` or
   `'…'` attribute value is a parse error pointing at the lift to `$var` form.
   (The lift across the repo is done; the parser change makes regressions
   loud.) `$var` substitution happens at render time via `Engine::resolve`.

3. Renderer should already work — fallthrough at `template.rs:904` and
   `template.rs:1026` emits unknown names as raw HTML. Audit that this still
   round-trips correctly (especially self-closing void tags like
   `<img src="…"/>`).

4. Update `convert.rs` to walk the now-fully-structured tree and emit TRML
   directly. `try_html_to_trml` and its helpers (`parse_html_open_tag`,
   `find_closing`) get deleted — they were workarounds for the missing tree
   structure.

**Invariants to assert (property tests).**

- *Parser totality.* For every `.hrml` template in the repo, `HParser::parse`
  returns `Ok`.
- *No-raw-HTML.* For the same templates, no `Node::Text` value contains `<x`
  for any ASCII alpha `x` at byte boundary (i.e., text never holds a tag-open
  pattern).
- *Render stability.* For every existing template test, the rendered HTML
  (after whitespace normalisation) is byte-identical before and after the
  parser change. This is the regression gate.

**Verification.**

```
cargo test                         # all green
cargo build 2>&1 | grep warning    # no warnings
```

**Files touched (estimate).**

- `src/template/parser/hparser.rs` — ~150 lines added (HTML branch + void
  table + attribute-value HRML skip).
- `src/template.rs` — possibly minor (audit render fallthrough).
- `src/convert.rs` — replace string-recovery logic with tree serialisation.
- `tests/` — add `parser_no_raw_html.rs` (the invariants above).

**Risk and rollback.** All changes are additive; if a USI template breaks,
the symptom will be a render-stability test failing. Bisect on the new test.

### Stage 2 — ship TRML default + convert USI

**Goal.** TRML is the canonical/default extension. USI is fully ported to
`.trml`. The `xrml` CLI still reads `.hrml` (back-compat) but new projects
emit `.trml`.

**Depends on.** Stage 1 (otherwise the converter produces garbage).

**Scope of change.**

1. `src/template.rs:177` `parse_with_extension` — default to TRML parser.
   `.hrml` keeps the HRML parser. Anything else → TRML.

2. `src/router.rs:29` — accept both extensions. If both `foo.trml` and
   `foo.hrml` exist, prefer `.trml` (or warn). Same for `template_to_url` which
   strips the extension.

3. `src/ssg.rs:125,157,170` — same: trim either extension; look up `404.trml`
   first then `404.hrml`.

4. `src/project.rs:364` — file glob accepts both.

5. `src/assets/mod.rs` — the bundled templates used by `xrml new` (currently
   `.hrml`). Convert them to `.trml` and update `include_str!` paths. Keep the
   `.hrml` versions out of the binary.

6. **Converter sweep on USI:**
   ```
   for f in usi/templates/**/*.hrml; do
     xrml convert "$f" > "${f%.hrml}.trml"
   done
   ```
   Then a second pass rewrites `file="…/foo.hrml"` to `file="…/foo.trml"` inside
   the produced `.trml` files (load directives, component refs).

   The converter itself should do step 2 in-process when it serialises a `load`
   or `use file=…` attribute — saves a separate pass.

7. Delete the old `.hrml` files in USI **only after** `cargo test` is still
   green and `xrml build usi` produces the same output as before
   (diff `usi/dist` before vs. after).

**Verification.**

```
diff -r usi/dist.before/ usi/dist.after/   # empty
cargo test                                   # green
```

**Files touched.**

- `src/template.rs`, `src/router.rs`, `src/ssg.rs`, `src/project.rs`,
  `src/assets/mod.rs`, `src/convert.rs`.
- `usi/templates/**/*.hrml` → `*.trml` (mass rename).
- `usi/xrml.toml` — no change expected (paths are extension-agnostic).

**Risk.** Mass rename. Keep the `.hrml` originals around in git history (don't
`git rm` them in the same commit as the conversion — separate commits so a
diff-only review is possible).

### Stage 3 — get/set/resolve as tree automaton

**Goal.** Kill `Engine::resolve()` (string-level `$x` scanning) and the
attribute-value-as-string discipline. Variable references become typed
tree decorations, resolved by a single deterministic pass.

**Depends on.** Stage 1 — needs HRML directives inside attribute values to
be tree nodes, not literal strings.

**Scope of change.**

1. Promote attribute values from `String` to a small enum:

   ```rust
   enum AttrValue {
       Literal(String),
       Ref(Vec<String>),     // $a.b.c → Ref(["a","b","c"])
       Mixed(Vec<AttrPart>), // "hello $name" → [Lit("hello "), Ref(["name"])]
   }
   ```

   (Or a single `Vec<AttrPart>` always — simpler. Decide once the shape is
   driven by test cases.)

2. Both parsers populate this richer type at parse time. No more `$` scanning
   later.

3. Replace `Engine::resolve()` with `Context::eval_attr(&AttrValue) -> String`
   — a fold over the decorated leaves. Same with `Context::eval_text` for
   `$var` inside `Node::Text`.

4. `<?get id="…"?>` becomes equivalent to an `AttrPart::Ref` in a text
   position. Same tree, same automaton evaluating it.

**Invariants (property tests).**

- *Resolution monotonicity.* Adding a binding only ever turns `Ref`s into
  `Literal`s. No binding mutation ever shortens output (modulo the empty-string
  fallback for missing vars).
- *Determinism.* Two passes over the same tree with the same context produce
  the same output.
- *Idempotence.* `eval(eval(t))` == `eval(t)` once all refs are bound.

**Verification.**

```
cargo test
# new property test file: tests/automaton_resolve.rs
```

**Files touched.**

- `src/template.rs` (big surgery in `Engine::resolve`, `render_meta_tag`,
  `render_link_tag`, every `attrs.get(...).cloned()` site that's followed by a
  `$`-resolve).
- `src/template/ast.rs` — `AstNode::Element.attrs` type changes.
- Both parsers — produce `AttrValue` at parse time.

**Risk.** Largest change, touches a lot of files. Save for last, after Stages
1 + 2 are merged and stable.

## Cleanup / release-readiness checklist (independent of stages)

These can be done in parallel with any stage; not blockers, but should land
before "ready for release":

- [x] Fix 5 compiler warnings (done)
- [x] Fix `test_markdown_frontmatter_and_meta_tag_library` (done — was a bug in
      `Engine::resolve` where `.` wasn't a valid var-name continuation char)
- [ ] Audit `src/template.rs` (currently 2200 lines): a natural split is
      `template/render.rs`, `template/context.rs`, `template/extract.rs`
      (the block/component extraction passes), keeping `template.rs` as the
      `Engine` struct + public API only.
- [ ] Property tests for the parser invariants (Stage 1) and the resolve
      automaton (Stage 3).
- [ ] Delete the orphan dist files in `usi/dist/blog/*.html` referenced in
      `git status` (they are build artefacts, not source).
- [ ] `README.md` reflects TRML-first authoring after Stage 2.

## Open questions for whoever picks this up

1. **HRML attribute values containing nested quotes.** `<a href="<?get id="x"?>">`
   has nested `"`. Parser strategy: switch on `<?` → read until `?>`, ignoring
   inner `"`. Confirm this matches existing USI uses.

2. **HTML void-tag list — strict or permissive?** WHATWG has a fixed list of
   void elements. Should `<custom-tag />` self-close be respected too? Default
   yes (it's how SVG `<use href="..."/>` works in `usi/templates/components/card.hrml`).

3. **TRML round-trip for whitespace.** `convert.rs` currently collapses
   indentation. Decide whether `.trml` output should preserve significant
   whitespace from HRML text content (`<p>Hi</p>` vs `<pre>  code\n</pre>`).
   Probably yes for `<pre>` and `<code>`; investigate at Stage 2 time.

4. **Stage 3 shape: `AttrValue` enum vs. uniform `Vec<AttrPart>`.** Pick by
   prototyping on three representative attributes (`href`, `class`, `cond`).

## How to continue

1. Read this file end to end.
2. Read `STAGE` notes above for whichever stage is next.
3. Run baseline `cargo test` — must be green before starting any stage.
4. Run `cargo build 2>&1 | grep warning` — must be empty.
5. Pick the smallest sub-task in the next stage. Property tests *before* code
   changes where possible.
6. Update this `PLAN.md` as the source of truth: mark items done, add new
   open questions, refine estimates.
