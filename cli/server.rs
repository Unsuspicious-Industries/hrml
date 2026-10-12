use crate::{
    ast_log,
    project::{load_project, validate_project},
};
use axum::{
    body::Body,
    extract::{Json, Path as AxumPath, State},
    http::{Request, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use xrml::{backend::Runtime, config::Config, project::Project, router::Router as HrmlRouter, template::Engine as HrmlEngine};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tower_http::services::ServeDir;

const HRML_JS: &str = include_str!("../src/runtime/client.js");

#[derive(Clone)]
struct AppState {
    project: Arc<RwLock<Project>>,
    backend_runtime: Arc<Runtime>,
    static_path: Arc<PathBuf>,
    templates_path: Arc<PathBuf>,
    project_path: Arc<PathBuf>,
}

pub async fn run_dev(project_path: &Path, log_ast: bool, debug: bool) -> Result<(), String> {
    if debug {
        std::env::set_var("HRML_DEBUG", "1");
    }
    validate_project(project_path)?;

    let mut project = load_project(project_path)?;

    if log_ast {
        ast_log::write_ast_log(project_path, &project.config)?;
    }

    let host = project.config.host.clone();
    let port = project.config.port;
    let static_path_str = project.config.static_path.clone();
    let static_path_arc = Arc::new(project_path.join(&static_path_str));
    let backend_runtime = Arc::new(build_backend_runtime(project_path, &project.config));

    project.parse_all().map_err(|e| e.to_string())?;

    println!("Starting HRML development server on {}:{}", host, port);

    let state = AppState {
        project: Arc::new(RwLock::new(project)),
        backend_runtime,
        static_path: static_path_arc.clone(),
        templates_path: Arc::new(project_path.join("templates")),
        project_path: Arc::new(project_path.to_path_buf()),
    };

    let state_for_watcher = state.clone();
    let project_path_buf = project_path.to_path_buf();
    tokio::spawn(async move {
        watch_for_changes(project_path_buf, state_for_watcher).await;
    });

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/hrml.js", get(hrml_js_handler))
        .route(
            "/api/*path",
            get(api_get_handler)
                .post(endpoint_handler)
                .delete(endpoint_handler),
        )
        .route("/*path", get(page_handler).post(endpoint_handler))
        .route("/api/music", post(music_handler))
        .nest_service("/static", ServeDir::new(&*static_path_arc))
        .with_state(state);

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

pub async fn serve_static(project_path: &Path, log_ast: bool) -> Result<(), String> {
    use std::net::SocketAddr;

    let project = load_project(project_path)?;

    if log_ast {
        ast_log::write_ast_log(project_path, &project.config)?;
    }

    let dist_path = project_path.join("dist");

    if !dist_path.exists() {
        return Err(format!(
            "dist/ not found. Run '{} build' first.",
            env!("CARGO_BIN_NAME")
        ));
    }

    println!(
        "Serving static files from 'dist/' on http://{}:{}",
        project.config.host, project.config.port
    );

    let app = Router::new().fallback_service(ServeDir::new(dist_path));

    let addr: SocketAddr = format!("{}:{}", project.config.host, project.config.port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind: {}", e))?;

    println!("Server running at http://{}", addr);
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    Ok(())
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

        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e,
            None => continue,
        };

        if matches!(ext, "hrml" | "trml" | "html" | "toml") {
            return true;
        }
    }

    false
}

fn reload_project(project_path: &Path, state: &AppState) -> Result<bool, String> {
    let mut new_project = load_project(project_path)?;
    new_project.parse_all().map_err(|e| e.to_string())?;

    let mut project = state.project.write().unwrap();
    let config_changed = project.config.site_name != new_project.config.site_name
        || project.config.host != new_project.config.host
        || project.config.port != new_project.config.port;
    *project = new_project;

    Ok(config_changed)
}

fn build_backend_runtime(project_path: &Path, config: &Config) -> Runtime {
    let endpoints_root = project_path.join(&config.endpoints_path);
    let endpoints_root = endpoints_root.to_string_lossy().into_owned();
    Runtime::new(&endpoints_root)
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

    if let Some(response) = try_serve_static(&state.static_path, "index.html") {
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
        if let Some(response) = try_serve_static(&state.static_path, &static_path) {
            return response;
        }
    }

    (StatusCode::NOT_FOUND, "Page not found").into_response()
}

fn try_serve_static(static_root: &Path, rel_path: &str) -> Option<Response> {
    let candidate = static_root.join(rel_path);
    if !candidate.exists() || !candidate.is_file() {
        return None;
    }

    let bytes = fs::read(&candidate).ok()?;
    let content_type = content_type_for(&candidate);

    Some((StatusCode::OK, [("content-type", content_type)], bytes).into_response())
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

async fn music_handler(
    State(state): State<AppState>,
    Json(data): Json<serde_json::Value>,
) -> Response {
    let action = data
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let music_path = state.project_path.join("data").join("music.json");

    let result = match action {
        "add" => add_record(&music_path, &data),
        "delete" => delete_record(&music_path, &data),
        _ => return (StatusCode::BAD_REQUEST, "Unknown action").into_response(),
    };

    if let Err(e) = result {
        eprintln!("[ERROR] /api/music {}: {}", action, e);
        return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
    }

    let engine = HrmlEngine::new(&state.project_path.to_string_lossy());
    match engine.render_fragment("templates/components/jukebox.hrml", &serde_json::json!({})) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("[ERROR] /api/music render failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Render error: {}", e),
            )
                .into_response()
        }
    }
}

fn add_record(path: &std::path::Path, data: &serde_json::Value) -> Result<(), String> {
    use serde_json::json;

    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read: {}", e))?;
    let mut records: Vec<serde_json::Value> =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {}", e))?;

    let next_id = records
        .iter()
        .filter_map(|r| r.get("id").and_then(|v| v.as_i64()))
        .max()
        .unwrap_or(-1)
        + 1;

    let artist = data
        .get("artist")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let album = data
        .get("album")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let slug = format!(
        "{}-{}.mp3",
        artist.to_lowercase().replace(' ', "-"),
        album.to_lowercase().replace(' ', "-")
    );

    let mut record = serde_json::Map::new();
    record.insert("id".to_string(), json!(next_id));
    record.insert("artist".to_string(), json!(artist));
    record.insert("album".to_string(), json!(album));
    let year_val = data.get("year").and_then(|v| {
        v.as_i64().map(|n| json!(n))
            .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()).map(|n| json!(n)))
            .or_else(|| v.as_str().map(|s| json!(s)))
    }).unwrap_or(json!(null));
    record.insert("year".to_string(), year_val);
    record.insert(
        "label".to_string(),
        data.get("label").and_then(|v| v.as_str()).map(|s| json!(s)).unwrap_or(json!("")),
    );
    record.insert(
        "status".to_string(),
        data.get("status").and_then(|v| v.as_str()).map(|s| json!(s)).unwrap_or(json!("added")),
    );
    record.insert(
        "note".to_string(),
        data.get("note").and_then(|v| v.as_str()).map(|s| json!(s)).unwrap_or(json!("")),
    );
    record.insert("file".to_string(), json!(slug));

    records.push(serde_json::Value::Object(record));

    let json_str =
        serde_json::to_string_pretty(&records).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(path, json_str).map_err(|e| format!("Failed to write: {}", e))?;

    Ok(())
}

fn delete_record(path: &std::path::Path, data: &serde_json::Value) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read: {}", e))?;
    let mut records: Vec<serde_json::Value> =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {}", e))?;

    let target_id = data
        .get("id")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
        .ok_or_else(|| "Missing or invalid id".to_string())?;

    let len_before = records.len();
    records.retain(|r| r.get("id").and_then(|v| v.as_i64()) != Some(target_id));

    if records.len() == len_before {
        return Err(format!("Record with id {} not found", target_id));
    }

    let json_str =
        serde_json::to_string_pretty(&records).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(path, json_str).map_err(|e| format!("Failed to write: {}", e))?;

    Ok(())
}
