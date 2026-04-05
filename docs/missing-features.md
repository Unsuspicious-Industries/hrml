# HRML Missing Features Report: Modern Website Building

## Executive Summary

HRML is a powerful template engine with excellent directive support, composition algebra, and formal verification properties. However, building a **production-ready modern website** requires infrastructure and features beyond template rendering. This report catalogs all missing features needed for a complete web framework.

---

## 1. ROUTING & NAVIGATION

### 1.1 File-Based Routing
- **Missing**: No automatic URL-to-template mapping
- **Impact**: Every page must be manually rendered; no `/about` → `pages/about.hrml` mapping
- **Priority**: Critical

### 1.2 Dynamic Routes
- **Missing**: No `[slug].hrml`, `[id].hrml`, or catch-all `[...rest].hrml` patterns
- **Impact**: Cannot generate blog posts, product pages, or user profiles dynamically
- **Priority**: Critical

### 1.3 Route Parameters
- **Missing**: No access to URL params (`/blog/:slug` → `params.slug`)
- **Impact**: Dynamic content cannot be parameterized
- **Priority**: Critical

### 1.4 Route Guards
- **Missing**: No authentication/authorization middleware on routes
- **Impact**: Cannot protect admin pages or user dashboards
- **Priority**: High

---

## 2. STATIC SITE GENERATION (SSG)

### 2.1 Build Command
- **Missing**: No `hrml build` command to generate static HTML
- **Impact**: Cannot deploy to Netlify, Vercel, GitHub Pages
- **Priority**: Critical

### 2.2 Incremental Builds
- **Missing**: No changed-file detection; full rebuild every time
- **Impact**: Slow builds for large sites
- **Priority**: Medium

### 2.3 Output Configuration
- **Missing**: No control over output directory, URL structure, or file naming
- **Impact**: Cannot customize deployment targets
- **Priority**: Medium

---

## 3. DATA FETCHING

### 3.1 External Data Sources
- **Missing**: No built-in HTTP client for fetching API data at build time
- **Impact**: Cannot pull data from headless CMS, databases, or external APIs
- **Priority**: High

### 3.2 Data Transformation
- **Missing**: No pipeline for transforming fetched data before rendering
- **Impact**: Raw API responses must be manually processed
- **Priority**: Medium

### 3.3 Caching
- **Missing**: No template or data caching layer
- **Impact**: Every render re-parses templates; no performance optimization
- **Priority**: High

---

## 4. SEO & METADATA

### 4.1 Automatic Sitemap
- **Missing**: No `sitemap.xml` generation from page structure
- **Impact**: Poor search engine indexing
- **Priority**: High

### 4.2 RSS/Atom Feeds
- **Missing**: No feed generation for blogs or content sites
- **Impact**: No syndication support
- **Priority**: Medium

### 4.3 Open Graph / Twitter Cards
- **Missing**: Partial support via `<?og?>` and `<?twitter?>` but no automatic generation
- **Impact**: Manual OG tag management for every page
- **Priority**: Medium

### 4.4 Canonical URLs
- **Missing**: Partial support but no automatic canonical URL generation
- **Impact**: Duplicate content issues
- **Priority**: Low

---

## 5. INTERNATIONALIZATION (i18n)

### 5.1 Multi-Language Support
- **Missing**: No locale routing (`/en/about`, `/fr/about`)
- **Impact**: Cannot build multilingual sites
- **Priority**: High

### 5.2 Translation Files
- **Missing**: No `.json` or `.toml` translation file loading
- **Impact**: Hardcoded text in templates
- **Priority**: High

### 5.3 Date/Number Formatting
- **Missing**: No locale-aware formatting
- **Impact**: Dates and numbers display incorrectly for international users
- **Priority**: Medium

---

## 6. SECURITY

### 6.1 XSS Protection
- **Missing**: No automatic HTML escaping for user-generated content
- **Impact**: Cross-site scripting vulnerabilities
- **Priority**: Critical

### 6.2 CSRF Protection
- **Missing**: No CSRF token generation/validation
- **Impact**: Form submission vulnerabilities
- **Priority**: High

### 6.3 Content Security Policy
- **Missing**: No CSP header generation
- **Impact**: Weaker security posture
- **Priority**: Medium

### 6.4 Rate Limiting
- **Missing**: No API endpoint rate limiting
- **Impact**: API abuse vulnerability
- **Priority**: Medium

---

## 7. DEVELOPER EXPERIENCE

### 7.1 Dev Server
- **Missing**: No `hrml dev` command with hot reload
- **Impact**: Manual rebuild required for every change
- **Priority**: High

### 7.2 Error Pages
- **Missing**: No 404/500 error page handling
- **Impact**: Broken links show raw errors
- **Priority**: High

### 7.3 Template Linting
- **Missing**: No `hrml lint` for syntax checking
- **Impact**: Errors only caught at render time
- **Priority**: Medium

### 7.4 Debug Mode
- **Missing**: No debug output showing template resolution chain
- **Impact**: Hard to debug complex template compositions
- **Priority**: Medium

---

## 8. ASSET MANAGEMENT

### 8.1 Image Optimization
- **Missing**: No automatic image resizing, WebP conversion, or lazy loading
- **Impact**: Poor performance, large payloads
- **Priority**: High

### 8.2 CSS/JS Bundling
- **Missing**: No asset pipeline for Tailwind, PostCSS, or JS minification
- **Impact**: Manual asset management
- **Priority**: Medium

### 8.3 Font Optimization
- **Missing**: No font subsetting or preloading
- **Impact**: Slow font loading
- **Priority**: Low

---

## 9. BACKEND INTEGRATION

### 9.1 HTTP Server
- **Missing**: No built-in web server
- **Impact**: Requires external server (nginx, etc.)
- **Priority**: High

### 9.2 Database Integration
- **Missing**: No ORM or database connectivity
- **Impact**: Cannot build data-driven applications
- **Priority**: High

### 9.3 Authentication
- **Missing**: No session management or JWT support
- **Impact**: Cannot build authenticated applications
- **Priority**: High

---

## 10. DEPLOYMENT

### 10.1 Deployment Targets
- **Missing**: No one-click deploy to Vercel, Netlify, Cloudflare
- **Impact**: Manual deployment configuration
- **Priority**: Medium

### 10.2 Environment Variables
- **Missing**: No `.env` file support
- **Impact**: Hardcoded configuration
- **Priority**: Medium

### 10.3 CI/CD Integration
- **Missing**: No GitHub Actions or CI templates
- **Impact**: Manual testing and deployment
- **Priority**: Low

---

## Implementation Priority

| Priority | Features | Effort |
|----------|----------|--------|
| **P0 - Critical** | File-based routing, SSG build, XSS protection, error pages | 2-3 weeks |
| **P1 - High** | Dynamic routes, dev server, i18n, caching, HTTP server | 3-4 weeks |
| **P2 - Medium** | Sitemap, RSS, image optimization, linting, env vars | 2-3 weeks |
| **P3 - Nice-to-have** | Font optimization, CI/CD, incremental builds | 1-2 weeks |

---

## Conclusion

HRML's template engine is **feature-complete for rendering** but lacks the **infrastructure** needed for modern web development. The core engine is solid (307 passing tests, 15 formal properties verified). The missing features are primarily around routing, build tooling, security, and developer experience.

**Recommendation**: Implement P0 and P1 features to make HRML viable for production websites. P2 and P3 can be added incrementally.
