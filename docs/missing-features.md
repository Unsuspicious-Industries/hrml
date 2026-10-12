# HRML Feature Status Report

## Implemented Features

### Core Template Engine
- [x] HRML directive parser (load, slot/block, set/get, if, for, component/use, bind, compose, btn, link, form, wasm, markdown, latex, meta tags)
- [x] Block injection with slot replacement
- [x] Nested load resolution inside any element
- [x] Circular dependency detection
- [x] HTML document auto-wrapping (fragments get `<html><head><body>` wrapper)
- [x] Full HTML document passthrough (no double wrapping)
- [x] Context variable system with dot-notation access (`user.name`)
- [x] Component system with named slots
- [x] Compose algebra (sum/product operations)
- [x] Markdown rendering with math delimiters
- [x] LaTeX inline and block rendering
- [x] Meta tag library (charset, viewport, og, twitter, canonical, etc.)
- [x] Custom tag registry (extendable void/block handlers)

### Routing
- [x] File-based routing (`pages/about.hrml` → `/about`)
- [x] Dynamic routes (`pages/blog/[slug].hrml` → `/blog/:slug`)
- [x] Catch-all routes (`pages/docs/[...rest].hrml` → `/docs/*`)
- [x] Route priority: static > dynamic > catch-all
- [x] URL parameter extraction
- [x] 404 route detection

### Static Site Generation
- [x] `xrml build` exports pages to `dist/`
- [x] Automatic `sitemap.xml` generation
- [x] Automatic 404 page generation (with fallback)
- [x] Static asset copying (`static/` → `dist/static/`)
- [x] Build reports (pages rendered, errors)

### Security helpers
- [x] XSS protection (`escape_html`, `escape_attr`, `escape_url`)
- [x] URL sanitization (blocks `javascript:`, `data:`, `vbscript:` schemes)
- [x] HTML stripping (`strip_html`)
- [x] CSRF token generation
- [x] Nonce validation (timestamp-based)

### Authentication helpers
- [x] Library PAM authentication via dlopen (no link-time dependency); the CLI has no `auth` command
- [x] Group membership checks (`is_user_in_group`)
- [x] Current user/UID detection
- [x] Root check

### CLI
- [x] `xrml new <name>` creates a project
- [x] `xrml dev [path]` and `xrml serve [path]` serve the source project
- [x] `xrml build [path]` exports a static site
- [x] `xrml check [path]` diagnoses template and configuration errors
- [x] `xrml version` / `xrml help`
- [x] Source reload after a 500 ms quiet interval; failed reloads retain the current project
- [x] Named read-only SQLite extract queries with declared fixtures; static builds require `--query-fixtures`

### Validation record

An earlier report listed 398 tests across 11 suites, 20 executable properties
(P1-P20), stress cases with 20 components, 15 compose segments and 30 variables,
and 6 XSS vectors. It recorded neither a source revision nor a test-run result.
Those counts are not evidence for the current release. Use the checked-in
validation sources and a recorded command/result for a verification claim.

## Not implemented

### Data and execution
- [ ] HTTP data fetching
- [ ] Python or other process-executing endpoints
- [ ] Live mutable database reads during static builds; deliberately prohibited
- [ ] Session management

### P2 - Medium
- [ ] **RSS/Atom feed generation**
- [ ] **Image optimization** - no WebP conversion or lazy loading
- [ ] **CSS/JS bundling** - no Tailwind/PostCSS pipeline
- [ ] **Environment variables** - no `.env` file support

### P3 - Nice-to-have
- [ ] **Incremental builds** - changed-file detection
- [ ] **Font optimization** - no subsetting or preloading
- [ ] **CI/CD templates** - GitHub Actions

## Boundaries

The crate contains separate parser, renderer, router, endpoint and native-helper
modules. Endpoint templates do not execute application code. SQLite queries
read a published extract; the renderer does not own its schema, credentials,
publication or mutable records. [spec.md](../spec.md) defines the interfaces.
