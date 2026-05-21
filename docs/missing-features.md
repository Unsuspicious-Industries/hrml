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

### Routing (NEW)
- [x] File-based routing (`pages/about.hrml` → `/about`)
- [x] Dynamic routes (`pages/blog/[slug].hrml` → `/blog/:slug`)
- [x] Catch-all routes (`pages/docs/[...rest].hrml` → `/docs/*`)
- [x] Route priority: static > dynamic > catch-all
- [x] URL parameter extraction
- [x] 404 route detection

### Static Site Generation (NEW)
- [x] `hrml build` command - renders all pages to `dist/`
- [x] Automatic `sitemap.xml` generation
- [x] Automatic 404 page generation (with fallback)
- [x] Static asset copying (`static/` → `dist/static/`)
- [x] Build reports (pages rendered, errors)

### Security (NEW)
- [x] XSS protection (`escape_html`, `escape_attr`, `escape_url`)
- [x] URL sanitization (blocks `javascript:`, `data:`, `vbscript:` schemes)
- [x] HTML stripping (`strip_html`)
- [x] CSRF token generation
- [x] Nonce validation (timestamp-based)

### Authentication (NEW)
- [x] PAM authentication via dlopen (no link-time dependency)
- [x] `hrml auth <user>` command (reads password from stdin)
- [x] Group membership checks (`is_user_in_group`)
- [x] Current user/UID detection
- [x] Root check

### CLI
- [x] `hrml new <name>` - create project
- [x] `hrml dev [path]` - development server
- [x] `hrml serve [path]` - serve static dist
- [x] `hrml build [path]` - build static site
- [x] `hrml check [path]` - validate templates, routes, HTML structure
- [x] `hrml auth <user>` - PAM authentication
- [x] `hrml version` / `hrml help`

### Validation & Testing
- [x] 398 tests across 11 test suites
- [x] 20 formal properties (P1-P20) with executable specifications
- [x] Comprehensive directive coverage
- [x] Chain/pipeline tests
- [x] Stress tests (20 components, 15 compose segments, 30 variables)
- [x] XSS prevention tests (6 attack vectors)
- [x] Routing correctness tests
- [x] Security invariant tests
- [x] Auth correctness tests
- [x] SSG correctness tests

## Still Missing (P1-P3 from original report)

### P0 - Critical
- [ ] **Dev server hot reload** - currently requires manual restart

### P1 - High
- [ ] **External data fetching** - no HTTP client for build-time API calls
- [ ] **Database integration** - no ORM or DB connectivity
- [ ] **Session management** - no JWT/cookie-based sessions

### P2 - Medium
- [ ] **RSS/Atom feed generation**
- [ ] **Image optimization** - no WebP conversion or lazy loading
- [ ] **CSS/JS bundling** - no Tailwind/PostCSS pipeline
- [ ] **Environment variables** - no `.env` file support

### P3 - Nice-to-have
- [ ] **Incremental builds** - changed-file detection
- [ ] **Font optimization** - no subsetting or preloading
- [ ] **CI/CD templates** - GitHub Actions

## Design Philosophy

HRML follows Unix philosophy:
- **Do one thing well**: Template rendering is the core. Everything else is a separate module.
- **Composability**: Router, SSG, Security, Auth are independent crates that can be used separately.
- **Text streams**: Templates are text in, HTML is text out. No AST manipulation required.
