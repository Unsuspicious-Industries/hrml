// HRML Router
//
// File-based routing with dynamic route parameters, catch-all routes,
// and route guards. Maps URL paths to template files automatically.

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteKind {
    Static,
    Dynamic(String), // [slug], [id], etc.
    CatchAll,        // [...rest]
}

#[derive(Debug, Clone)]
pub struct Route {
    pub path: String,
    pub template: String,
    pub kind: RouteKind,
    pub params: Vec<String>,
}

impl Route {
    pub fn from_file(base: &Path, file: &Path) -> Option<Self> {
        let rel = file.strip_prefix(base).ok()?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        if !rel_str.ends_with(".hrml") && !rel_str.ends_with(".trml") {
            return None;
        }

        let template = rel_str.clone();
        let path = Self::template_to_url(&rel_str);
        let (kind, params) = Self::analyze_route(&path);

        Some(Self {
            path,
            template,
            kind,
            params,
        })
    }

    fn template_to_url(template: &str) -> String {
        let path = template
            .trim_start_matches("pages/")
            .trim_start_matches("templates/pages/")
            .trim_end_matches(".hrml")
            .trim_end_matches(".trml")
            .trim_end_matches("/index");

        if path.is_empty() || path == "index" {
            "/".to_string()
        } else {
            format!("/{}", path)
        }
    }

    fn analyze_route(path: &str) -> (RouteKind, Vec<String>) {
        let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut params = Vec::new();
        let mut kind = RouteKind::Static;

        for segment in &segments {
            if segment.starts_with("[...") && segment.ends_with(']') {
                let param = segment.trim_start_matches("[...").trim_end_matches(']');
                params.push(param.to_string());
                kind = RouteKind::CatchAll;
            } else if segment.starts_with('[') && segment.ends_with(']') {
                let param = segment.trim_start_matches('[').trim_end_matches(']');
                params.push(param.to_string());
                if kind != RouteKind::CatchAll {
                    kind = RouteKind::Dynamic(param.to_string());
                }
            }
        }

        (kind, params)
    }

    /// Match a URL against this route, extracting params if matched
    pub fn match_url(&self, url: &str) -> Option<HashMap<String, String>> {
        let url_segments: Vec<&str> = url.trim_start_matches('/').split('/').collect();
        let route_segments: Vec<&str> = self.path.trim_start_matches('/').split('/').collect();

        if self.kind == RouteKind::CatchAll {
            // Catch-all: match prefix, rest goes to param
            if url_segments.len() < route_segments.len() - 1 {
                return None;
            }
            let mut params = HashMap::new();
            for (i, route_seg) in route_segments.iter().enumerate() {
                if route_seg.starts_with("[...") {
                    let param = route_seg.trim_start_matches("[...").trim_end_matches(']');
                    let rest = url_segments[i..].join("/");
                    params.insert(param.to_string(), rest);
                    return Some(params);
                }
                if i >= url_segments.len() {
                    return None;
                }
                if !route_seg.starts_with('[') && *route_seg != url_segments[i] {
                    return None;
                }
                if route_seg.starts_with('[') {
                    let param = route_seg.trim_start_matches('[').trim_end_matches(']');
                    params.insert(param.to_string(), url_segments[i].to_string());
                }
            }
            Some(params)
        } else {
            // Exact match or dynamic
            if url_segments.len() != route_segments.len() {
                return None;
            }

            let mut params = HashMap::new();
            for (route_seg, url_seg) in route_segments.iter().zip(url_segments.iter()) {
                if route_seg.starts_with('[') {
                    let param = route_seg.trim_start_matches('[').trim_end_matches(']');
                    params.insert(param.to_string(), url_seg.to_string());
                } else if route_seg != url_seg {
                    return None;
                }
            }
            Some(params)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Router {
    pub routes: Vec<Route>,
}

impl Router {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Build router from a pages directory
    pub fn from_pages_dir(pages_dir: &Path) -> Self {
        let mut routes = Vec::new();
        if pages_dir.exists() {
            Self::collect_routes(pages_dir, pages_dir, &mut routes);
        }
        routes.sort_by(|a, b| {
            // Static routes first, then dynamic, then catch-all
            let priority = |r: &Route| match r.kind {
                RouteKind::Static => 0,
                RouteKind::Dynamic(_) => 1,
                RouteKind::CatchAll => 2,
            };
            priority(a).cmp(&priority(b)).then(a.path.cmp(&b.path))
        });
        Self { routes }
    }

    fn collect_routes(base: &Path, dir: &Path, routes: &mut Vec<Route>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::collect_routes(base, &path, routes);
                } else if let Some(route) = Route::from_file(base, &path) {
                    routes.push(route);
                }
            }
        }
    }

    /// Find the best matching route for a URL
    pub fn resolve(&self, url: &str) -> Option<(&Route, HashMap<String, String>)> {
        for route in &self.routes {
            if let Some(params) = route.match_url(url) {
                return Some((route, params));
            }
        }
        None
    }

    /// Find a 404 error page route
    pub fn find_404(&self) -> Option<&Route> {
        self.routes
            .iter()
            .find(|r| r.path == "/404" || r.template.contains("404"))
    }

    /// List all static routes
    pub fn static_routes(&self) -> Vec<&Route> {
        self.routes
            .iter()
            .filter(|r| r.kind == RouteKind::Static)
            .collect()
    }

    /// List all dynamic routes
    pub fn dynamic_routes(&self) -> Vec<&Route> {
        self.routes
            .iter()
            .filter(|r| r.kind != RouteKind::Static)
            .collect()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_to_url_index() {
        assert_eq!(Route::template_to_url("pages/index.hrml"), "/");
        assert_eq!(Route::template_to_url("pages/index.trml"), "/");
    }

    #[test]
    fn template_to_url_static() {
        assert_eq!(Route::template_to_url("pages/about.hrml"), "/about");
        assert_eq!(Route::template_to_url("pages/about.trml"), "/about");
        assert_eq!(Route::template_to_url("pages/blog.hrml"), "/blog");
        assert_eq!(Route::template_to_url("pages/blog/index.hrml"), "/blog");
        assert_eq!(Route::template_to_url("pages/blog/index.trml"), "/blog");
    }

    #[test]
    fn template_to_url_nested() {
        assert_eq!(Route::template_to_url("pages/blog/post.hrml"), "/blog/post");
        assert_eq!(Route::template_to_url("pages/blog/post.trml"), "/blog/post");
        assert_eq!(
            Route::template_to_url("pages/docs/api/index.hrml"),
            "/docs/api"
        );
    }

    #[test]
    fn template_to_url_dynamic() {
        assert_eq!(
            Route::template_to_url("pages/blog/[slug].hrml"),
            "/blog/[slug]"
        );
        assert_eq!(
            Route::template_to_url("pages/blog/[slug].trml"),
            "/blog/[slug]"
        );
        assert_eq!(
            Route::template_to_url("pages/users/[id].hrml"),
            "/users/[id]"
        );
    }

    #[test]
    fn template_to_url_catch_all() {
        assert_eq!(
            Route::template_to_url("pages/docs/[...rest].hrml"),
            "/docs/[...rest]"
        );
        assert_eq!(
            Route::template_to_url("pages/docs/[...rest].trml"),
            "/docs/[...rest]"
        );
    }

    #[test]
    fn match_static_url() {
        let route = Route {
            path: "/about".to_string(),
            template: "pages/about.hrml".to_string(),
            kind: RouteKind::Static,
            params: vec![],
        };
        let params = route.match_url("/about").unwrap();
        assert!(params.is_empty());
        assert!(route.match_url("/contact").is_none());
    }

    #[test]
    fn match_dynamic_url() {
        let route = Route {
            path: "/blog/[slug]".to_string(),
            template: "pages/blog/[slug].hrml".to_string(),
            kind: RouteKind::Dynamic("slug".to_string()),
            params: vec!["slug".to_string()],
        };
        let params = route.match_url("/blog/hello-world").unwrap();
        assert_eq!(params.get("slug").unwrap(), "hello-world");
        assert!(route.match_url("/blog").is_none());
    }

    #[test]
    fn match_catch_all_url() {
        let route = Route {
            path: "/docs/[...rest]".to_string(),
            template: "pages/docs/[...rest].hrml".to_string(),
            kind: RouteKind::CatchAll,
            params: vec!["rest".to_string()],
        };
        let params = route.match_url("/docs/api/reference").unwrap();
        assert_eq!(params.get("rest").unwrap(), "api/reference");
        let params = route.match_url("/docs").unwrap();
        assert_eq!(params.get("rest").unwrap(), "");
    }

    #[test]
    fn match_multiple_dynamic_params() {
        let route = Route {
            path: "/users/[id]/posts/[postId]".to_string(),
            template: "pages/users/[id]/posts/[postId].hrml".to_string(),
            kind: RouteKind::Dynamic("postId".to_string()),
            params: vec!["id".to_string(), "postId".to_string()],
        };
        let params = route.match_url("/users/42/posts/7").unwrap();
        assert_eq!(params.get("id").unwrap(), "42");
        assert_eq!(params.get("postId").unwrap(), "7");
    }

    #[test]
    fn route_priority_static_over_dynamic() {
        let mut router = Router::new();
        router.routes.push(Route {
            path: "/blog".to_string(),
            template: "pages/blog/index.hrml".to_string(),
            kind: RouteKind::Static,
            params: vec![],
        });
        router.routes.push(Route {
            path: "/blog/[slug]".to_string(),
            template: "pages/blog/[slug].hrml".to_string(),
            kind: RouteKind::Dynamic("slug".to_string()),
            params: vec!["slug".to_string()],
        });

        let (route, _) = router.resolve("/blog").unwrap();
        assert_eq!(route.path, "/blog");
        assert_eq!(route.kind, RouteKind::Static);

        let (route, params) = router.resolve("/blog/hello").unwrap();
        assert_eq!(route.path, "/blog/[slug]");
        assert_eq!(params.get("slug").unwrap(), "hello");
    }
}
