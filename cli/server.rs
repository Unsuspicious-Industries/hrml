use crate::{
    ast_log,
    palette::{scan_static_dir, Palette},
    project::{load_project, validate_project},
};
use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{Request, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use xrml::{
    backend::Runtime, config::Config, project::Project, router::Router as HrmlRouter,
    template::TemplateErrorPhase,
};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;

const HRML_JS: &str = include_str!("../src/runtime/client.js");

#[derive(Clone)]
struct AppState {
    project: Arc<RwLock<Project>>,
    backend_runtime: Arc<Runtime>,
    static_files: Arc<RwLock<BTreeMap<String, StaticEntry>>>,
    templates_path: Arc<PathBuf>,
    static_namespace: Arc<String>,
    palette: Option<Arc<Palette>>,
}

#[derive(Clone)]
struct StaticEntry {
    bytes: Vec<u8>,
    content_type: &'static str,
}

pub async fn run_dev(
    project_path: &Path,
    palette: Option<&Palette>,
    log_ast: bool,
    debug: bool,
) -> Result<(), String> {
    serve_app(project_path, palette, None, None, log_ast, debug, true).await
}

pub async fn run_serve(
    project_path: &Path,
    palette: Option<&Palette>,
    host: Option<String>,
    port: Option<u16>,
    log_ast: bool,
) -> Result<(), String> {
    serve_app(project_path, palette, host, port, log_ast, false, false).await
}

async fn serve_app(
    project_path: &Path,
    palette: Option<&Palette>,
    host_override: Option<String>,
    port_override: Option<u16>,
    log_ast: bool,
    debug: bool,
    dev: bool,
) -> Result<(), String> {
    if debug {
        std::env::set_var("HRML_DEBUG", "1");
    }
    validate_project(project_path, palette)?;

    let mut project = load_project(project_path, palette)?;

    if log_ast {
        ast_log::write_ast_log(project_path, &project.config)?;
    }

    let host = host_override.unwrap_or_else(|| project.config.host.clone());
    let port = port_override.unwrap_or(project.config.port);
    let backend_runtime = Arc::new(build_backend_runtime(project_path, &project.config));

    let static_root = project_path.join(&project.config.static_path);
    let static_files = Arc::new(RwLock::new(
        scan_static_dir(&static_root, palette)?
            .into_iter()
            .map(|(rel, bytes)| {
                let content_type = content_type_for(Path::new(&rel));
                (rel, StaticEntry { bytes, content_type })
            })
            .collect::<BTreeMap<String, StaticEntry>>(),
    ));

    project.parse_all().map_err(|e| e.to_string())?;
    validate_index_render(&project)?;

    let static_namespace = Arc::new(project.config.static_path.clone());

    let state = AppState {
        project: Arc::new(RwLock::new(project)),
        backend_runtime,
        static_files,
        templates_path: Arc::new(project_path.join("templates")),
        static_namespace,
        palette: palette.map(|palette| Arc::new(palette.clone())),
    };

    let state_for_watcher = state.clone();
    let project_path_buf = project_path.to_path_buf();
    tokio::spawn(async move {
        watch_for_changes(project_path_buf, state_for_watcher).await;
    });

    let mut app = Router::new()
        .route("/", get(index_handler))
        .route(
            "/api/*path",
            get(api_get_handler)
                .post(endpoint_handler)
                .delete(endpoint_handler),
        )
        .route("/*path", get(page_handler).post(endpoint_handler))
        .with_state(state);

    if dev {
        app = app.route("/hrml.js", get(hrml_js_handler));
    }

    println!(
        "Starting HRML {} server on {}:{}",
        if dev { "development" } else { "production" },
        host,
        port
    );

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port))
        .await
        .map_err(|e| format!("Failed to bind server: {}", e))?;

    println!("   Server running at http://{}:{}", host, port);
    println!();
    println!("Press Ctrl+C to stop");

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))
}

async fn watch_for_changes(project_path: PathBuf, state: AppState) {
    let (tx, mut rx) = mpsc::unbounded_channel::<notify::Result<Event>>();

    let mut watcher =
        match RecommendedWatcher::new(move |res| drop(tx.send(res)), notify::Config::default()) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("   Failed to create file watcher: {}", e);
                return;
            }
        };

    if let Err(e) = watcher.watch(&project_path, RecursiveMode::Recursive) {
        eprintln!("   Failed to start file watcher: {}", e);
        return;
    }

    println!("   Watching for changes...");

    let mut last_reload = tokio::time::Instant::now();
    const DEBOUNCE_MS: u64 = 500;

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                if !is_relevant_change(&event) {
                    continue;
                }

                let now = tokio::time::Instant::now();
                if now.duration_since(last_reload) < Duration::from_millis(DEBOUNCE_MS) {
                    continue;
                }
                last_reload = now;

                println!("\n   Change detected, reloading...");
                match reload_project(&project_path, &state) {
                    Ok(true) => println!("   ✓ Reloaded\n"),
                    Ok(false) => println!("   ✓ Reloaded (config may have changed)\n"),
                    Err(e) => eprintln!("   ✗ Reload error: {}\n", e),
                }
            }
            Err(e) => eprintln!("   Watch error: {}\n", e),
        }
    }
}

fn is_relevant_change(event: &Event) -> bool {
    if !matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) {
        return false;
    }

    for path in &event.paths {
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        if file_name.starts_with('.') || file_name.starts_with('#') || file_name.ends_with('~') {
            continue;
        }

        // Hidden directories (`.git`, editor state) and the `dist/` build
        // output are not project source; ignore their churn.
        let mut ignored = false;
        for component in path.components() {
            if let Some(name) = component.as_os_str().to_str() {
                if name == "dist" || (name.starts_with('.') && name != "." && name != "..") {
                    ignored = true;
                    break;
                }
            }
        }
        if ignored {
            continue;
        }

        // Any project file - template, static asset, data file, config - can
        // change what the server renders, so any of them triggers a reload.
        return true;
    }

    false
}

fn reload_project(project_path: &Path, state: &AppState) -> Result<bool, String> {
    let palette = state.palette.as_ref().map(|palette| palette.as_ref());
    let mut new_project = load_project(project_path, palette)?;
    new_project.parse_all().map_err(|e| e.to_string())?;
    validate_index_render(&new_project)?;

    let static_root = project_path.join(&new_project.config.static_path);
    let new_static = scan_static_dir(&static_root, palette)?
        .into_iter()
        .map(|(rel, bytes)| {
            let content_type = content_type_for(Path::new(&rel));
            (rel, StaticEntry { bytes, content_type })
        })
        .collect::<BTreeMap<String, StaticEntry>>();

    let config_changed = {
        let old = state.project.read().unwrap();
        new_project.config.site_name != old.config.site_name
            || new_project.config.host != old.config.host
            || new_project.config.port != old.config.port
    };

    *state.project.write().unwrap() = new_project;
    *state.static_files.write().unwrap() = new_static;

    Ok(config_changed)
}

fn build_backend_runtime(project_path: &Path, config: &Config) -> Runtime {
    let endpoints_root = project_path.join(&config.endpoints_path);
    let endpoints_root = endpoints_root.to_string_lossy().into_owned();
    Runtime::new(&endpoints_root)
}

/// Do not expose a project until its entry page and all loaded templates render.
/// A git checkout can briefly contain only part of a new tree during reset.
fn validate_index_render(project: &Project) -> Result<(), String> {
    for template_path in ["pages/index.trml", "pages/index.hrml", "pages/index.html"] {
        if project.get_file(template_path).is_some() {
            project
                .render(template_path, &serde_json::json!({}))
                .map(|_| ())
                .map_err(|error| format!("Index template is not renderable: {}", error))?;
            return Ok(());
        }
    }

    Ok(())
}

async fn index_handler(State(state): State<AppState>) -> Response {
    let project = state.project.read().unwrap();
    for template_path in ["pages/index.trml", "pages/index.hrml", "pages/index.html"] {
        if project.get_file(template_path).is_some() {
            match project.render(template_path, &serde_json::json!({})) {
                Ok(html) => return Html(html).into_response(),
                Err(error) => {
                    eprintln!("[ERROR] {}", error);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Template error: {}", error),
                    )
                        .into_response();
                }
            }
        }
    }

    if let Some(response) = try_serve_static(&state, "index.html") {
        response
    } else {
        (StatusCode::NOT_FOUND, "Page not found").into_response()
    }
}

async fn page_handler(State(state): State<AppState>, AxumPath(path): AxumPath<String>) -> Response {
    let normalized = path.trim_matches('/');

    if normalized.is_empty() {
        return index_handler(State(state)).await;
    }

    if normalized.contains("..") {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }

    // The static namespace (default `static/`) is served verbatim, ahead of
    // any page logic - it is the site's asset directory, not a route space.
    let namespace_prefix = format!("{}/", state.static_namespace.as_str());
    if let Some(rel) = normalized.strip_prefix(&namespace_prefix) {
        return try_serve_static(&state, rel)
            .unwrap_or_else(|| (StatusCode::NOT_FOUND, "Page not found").into_response());
    }

    // Clean URLs are canonical for hrml-served sites: `/blog/a-post`, not
    // `/blog/a-post.html`. A `.html` suffix on a page that exists clean is a
    // legacy link or a hand-typed URL; send it to the canonical form.
    if let Some(clean) = normalized.strip_suffix(".html") {
        if !clean.is_empty() && page_exists(&state, clean) {
            return axum::response::Redirect::temporary(&format!("/{}", clean)).into_response();
        }
    }

    let template_candidates = [
        format!("pages/{}.trml", normalized),
        format!("pages/{}.hrml", normalized),
        format!("pages/{}.html", normalized),
        format!("pages/{}/index.trml", normalized),
        format!("pages/{}/index.hrml", normalized),
        format!("pages/{}/index.html", normalized),
    ];

    {
        let project = state.project.read().unwrap();
        for template_path in template_candidates.iter() {
            if project.get_file(&template_path).is_some() {
                match project.render(&template_path, &serde_json::json!({})) {
                    Ok(html) => return Html(html).into_response(),
                    Err(error) => {
                        eprintln!("[ERROR] {}", error);
                        // A missing data file (blog post, listing) means the
                        // resource does not exist - 404, not a server fault.
                        if error.phase == TemplateErrorPhase::Io {
                            return (StatusCode::NOT_FOUND, "Page not found").into_response();
                        }
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Template error: {}", error),
                        )
                            .into_response();
                    }
                }
            }
        }
    }

    let pages_dir = state.templates_path.join("pages");
    let router = HrmlRouter::from_pages_dir(&pages_dir);
    let url = format!("/{}", normalized);

    if let Some((route, params)) = router.resolve(&url) {
        let project = state.project.read().unwrap();
        let render_data = serde_json::to_value(params).unwrap_or(serde_json::json!({}));
        let template_path = format!("pages/{}", route.template);
        if project.get_file(&template_path).is_some() {
            match project.render(&template_path, &render_data) {
                Ok(html) => return Html(html).into_response(),
                Err(error) => {
                    eprintln!("[ERROR] {}", error);
                    if error.phase == TemplateErrorPhase::Io {
                        return (StatusCode::NOT_FOUND, "Page not found").into_response();
                    }
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Template error: {}", error),
                    )
                        .into_response();
                }
            }
        }
    }

    {
        let project = state.project.read().unwrap();
        for err_page in &["pages/404.trml", "pages/404.hrml"] {
            if let Ok(html) = project.render(err_page, &serde_json::json!({})) {
                return (StatusCode::NOT_FOUND, Html(html)).into_response();
            }
        }
    }

    let static_candidates = [
        normalized.to_string(),
        format!("{}.html", normalized),
        format!("{}/index.html", normalized),
    ];

    for static_path in static_candidates {
        if let Some(response) = try_serve_static(&state, &static_path) {
            return response;
        }
    }

    (StatusCode::NOT_FOUND, "Page not found").into_response()
}

fn try_serve_static(state: &AppState, rel_path: &str) -> Option<Response> {
    let entry = state.static_files.read().unwrap().get(rel_path)?.clone();

    Some(
        (
            StatusCode::OK,
            [("content-type", entry.content_type)],
            entry.bytes,
        )
            .into_response(),
    )
}

/// Whether `normalized` names a page: a template candidate or a dynamic
/// `[param]` route.
fn page_exists(state: &AppState, normalized: &str) -> bool {
    let candidates = [
        format!("pages/{}.trml", normalized),
        format!("pages/{}.hrml", normalized),
        format!("pages/{}.html", normalized),
        format!("pages/{}/index.trml", normalized),
        format!("pages/{}/index.hrml", normalized),
        format!("pages/{}/index.html", normalized),
    ];

    let project = state.project.read().unwrap();
    if candidates.iter().any(|candidate| project.get_file(candidate).is_some()) {
        return true;
    }

    let pages_dir = state.templates_path.join("pages");
    let router = HrmlRouter::from_pages_dir(&pages_dir);
    router.resolve(&format!("/{}", normalized)).is_some()
}

fn content_type_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

async fn api_get_handler(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    match state
        .backend_runtime
        .call_endpoint(&format!("/api/{}", path), &serde_json::json!({}))
    {
        Ok(result) => {
            if let Some(html) = result.as_str() {
                Html(html.to_string()).into_response()
            } else {
                (
                    StatusCode::OK,
                    serde_json::to_string(&result).unwrap_or_default(),
                )
                    .into_response()
            }
        }
        Err(e) => {
            eprintln!("[ERROR] API GET /api/{} failed: {}", path, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Endpoint error: {}", e),
            )
                .into_response()
        }
    }
}

async fn endpoint_handler(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    request: Request<Body>,
) -> Response {
    let body_bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("[ERROR] POST /{} - failed to read body: {}", path, e);
            return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
        }
    };

    let form_data = if !body_bytes.is_empty() {
        let body_str = String::from_utf8_lossy(&body_bytes);
        let mut data = serde_json::Map::new();

        for pair in body_str.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                let decoded_value =
                    urlencoding::decode(value).unwrap_or(std::borrow::Cow::Borrowed(value));
                data.insert(
                    key.to_string(),
                    serde_json::Value::String(decoded_value.to_string()),
                );
            }
        }

        serde_json::Value::Object(data)
    } else {
        serde_json::json!({})
    };

    let full_path = format!("/api/{}", path);

    match state.backend_runtime.call_endpoint(&full_path, &form_data) {
        Ok(result) => {
            if let Some(html) = result.as_str() {
                Html(html.to_string()).into_response()
            } else {
                (
                    StatusCode::OK,
                    serde_json::to_string(&result).unwrap_or_default(),
                )
                    .into_response()
            }
        }
        Err(e) => {
            eprintln!("[ERROR] POST /{} - endpoint error: {}", path, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Endpoint error: {}", e),
            )
                .into_response()
        }
    }
}

async fn hrml_js_handler() -> Response {
    (
        StatusCode::OK,
        [("content-type", "application/javascript")],
        HRML_JS,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::validate_index_render;
    use xrml::{config::Config, project::Project};

    fn project_with(index: &str, layout: Option<&str>) -> Project {
        let mut project = Project::new(Config::default());
        project.add_file("pages/index.hrml".to_string(), index.to_string());
        if let Some(layout) = layout {
            project.add_file("layouts/base.hrml".to_string(), layout.to_string());
        }
        project.parse_all().unwrap();
        project
    }

    #[test]
    fn rejects_an_index_with_a_missing_loaded_template() {
        let project = project_with(r#"<?load file="layouts/base.hrml"?>"#, None);

        let error = validate_index_render(&project).unwrap_err();

        assert!(error.contains("Loaded file not found: layouts/base.hrml"));
    }

    #[test]
    fn accepts_a_fully_renderable_index() {
        let project = project_with(
            r#"<?load file="layouts/base.hrml"?>"#,
            Some("<main>ok</main>"),
        );

        validate_index_render(&project).unwrap();
    }
}
