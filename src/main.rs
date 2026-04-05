use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

mod algebra;
mod assets;
mod auth;
mod backend;
mod config;
mod features;
mod oxml;
mod router;
mod security;
mod ssg;
mod template;

use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{Request, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::services::ServeDir;

const VERSION: &str = "0.1.0";
const HRML_JS: &str = include_str!("runtime/client.js");

fn print_help() {
    println!("HRML - Hypertext Rust Markup Language v{}", VERSION);
    println!();
    println!("Usage: hrml <command> [options]");
    println!();
    println!("Commands:");
    println!("  new <name>          Create a new HRML project");
    println!("  dev [path]          Run development server with auto-reload");
    println!("  serve [path]        Run production server");
    println!("  build [path]        Build static site for deployment");
    println!("  check [path]        Validate templates and configuration");
    println!("  auth <user>         Authenticate user via PAM (reads password from stdin)");
    println!("  version             Show version information");
    println!("  help                Show this help message");
    println!();
    println!("Examples:");
    println!("  hrml new myapp              Create new project 'myapp'");
    println!("  hrml dev                    Start dev server in current directory");
    println!("  hrml serve ./myapp          Serve project from ./myapp");
    println!("  hrml build ./myapp          Build static site from ./myapp");
    println!("  hrml check                  Validate current project");
}

fn create_project(name: &str) -> io::Result<()> {
    let project_path = Path::new(name);
    
    if project_path.exists() {
        eprintln!("Error: Directory '{}' already exists", name);
        process::exit(1);
    }
    
    println!("Creating new HRML project: {}", name);
    
    std::env::set_current_dir(project_path)?;
    
    let mut config = config::Config::default();
    config.site_name = name.to_string();
    config.setup()?;
    
    println!("Project '{}' created successfully!", name);
    println!();
    println!("To get started:");
    println!("  cd {}", name);
    println!("  hrml dev");
    
    Ok(())
}

fn validate_project(path: &Path) -> Result<(), String> {
    let config_path = path.join("hrml.toml");
    if !config_path.exists() {
        return Err(format!(
            "No hrml.toml found in '{}'\nAre you in the right directory?",
            path.display()
        ));
    }
    
    let config = config::Config::load(&config_path.to_string_lossy())
        .map_err(|e| format!("Failed to load hrml.toml: {}", e))?;
    
    let templates_path = path.join(&config.templates_path);
    if !templates_path.exists() {
        return Err(format!(
            "Templates directory not found: {}\nCreate it or update hrml.toml",
            templates_path.display()
        ));
    }
    
    let base_layout = templates_path.join("layouts/base.hrml");
    if !base_layout.exists() {
        eprintln!("[WARNING] Base layout not found at {}", base_layout.display());
    }
    
    let index_page = templates_path.join("pages/index.hrml");
    if !index_page.exists() {
        eprintln!("[WARNING] Index page not found at {}", index_page.display());
    }
    
    let static_path = path.join(&config.static_path);
    if !static_path.exists() {
        eprintln!("[WARNING] Static directory not found: {}", static_path.display());
    }
    
    let engine = template::Engine::new(&templates_path.to_string_lossy());
    
    match engine.render("pages/index.hrml", &serde_json::json!({})) {
        Ok(_) => println!("[OK] Index template renders successfully"),
        Err(e) => eprintln!("[WARNING] Index template failed to render: {}", e),
    }
    
    println!("[OK] Project validation complete");
    Ok(())
}

fn check_project(path: &Path) -> Result<(), String> {
    if let Err(e) = env::set_current_dir(path) {
        return Err(format!("Cannot access '{}': {}", path.display(), e));
    }

    let config = config::Config::load("hrml.toml").unwrap_or_default();
    let templates_path = PathBuf::from(&config.templates_path);

    if !templates_path.exists() {
        return Err(format!("Templates dir not found: {}", templates_path.display()));
    }

    // Build router and check routes
    let pages_dir = templates_path.join("pages");
    let rt = router::Router::from_pages_dir(&pages_dir);
    println!("Routes found: {}", rt.routes.len());
    for route in &rt.routes {
        println!("  {} -> {} ({:?})", route.path, route.template, route.kind);
    }

    // Validate all static routes render
    let engine = template::Engine::new(&templates_path.to_string_lossy());
    let mut errors = 0;
    let mut ok = 0;

    for route in rt.static_routes() {
        match engine.render(&route.template, &serde_json::json!({})) {
            Ok(html) => {
                // Check for directive leakage
                if html.contains("<?") || html.contains("?>") {
                    eprintln!("[WARN] {} has unprocessed directives", route.path);
                }
                // Check HTML structure
                if html.contains("<!DOCTYPE") {
                    if html.matches("<html").count() != 1 {
                        eprintln!("[WARN] {} has malformed HTML", route.path);
                    }
                }
                ok += 1;
            }
            Err(e) => {
                eprintln!("[ERROR] {} failed: {}", route.path, e);
                errors += 1;
            }
        }
    }

    println!("\nCheck complete: {} ok, {} errors", ok, errors);
    if errors > 0 {
        Err(format!("{} templates failed to render", errors))
    } else {
        Ok(())
    }
}

fn read_password() -> String {
    use std::io::{self, BufRead};
    // Read from stdin, strip trailing newline
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).unwrap_or(0);
    line.trim_end_matches(&['\n', '\r'][..]).to_string()
}

#[derive(Clone)]
pub struct AppState {
    template_engine: Arc<template::Engine>,
    backend_runtime: Arc<backend::Runtime>,
    static_path: Arc<PathBuf>,
}

async fn run_server(project_path: &Path, dev_mode: bool) {
    if let Err(e) = env::set_current_dir(project_path) {
        eprintln!("Error: Cannot access directory '{}': {}", project_path.display(), e);
        process::exit(1);
    }
    
    if let Err(e) = validate_project(project_path) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
    
    let config = config::Config::load("hrml.toml").unwrap_or_default();
    
    if dev_mode {
        println!("Starting HRML development server on {}:{}", config.host, config.port);
        println!("   Watching for changes...");
    } else {
        println!("Starting HRML server on {}:{}", config.host, config.port);
    }
    
    let template_engine = Arc::new(
        template::Engine::new(&config.templates_path)
            .with_site_name(config.site_name.clone())
            .with_description(config.site_description.clone())
            .with_favicon(config.favicon.clone())
    );
    let backend_runtime = Arc::new(backend::Runtime::new(&config.endpoints_path));
    
    let state = AppState {
        template_engine,
        backend_runtime,
        static_path: Arc::new(PathBuf::from(config.static_path.clone())),
    };
    
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/hrml.js", get(hrml_js_handler))
        .route("/api/*path", get(api_get_handler).post(endpoint_handler).delete(endpoint_handler))
        .route("/*path", get(page_handler).post(endpoint_handler))
        .nest_service("/static", ServeDir::new(&config.static_path))
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.host, config.port))
        .await
        .unwrap();
    
    println!("   Server running at http://{}:{}", config.host, config.port);
    println!();
    println!("Press Ctrl+C to stop");
    
    axum::serve(listener, app).await.unwrap();
}

fn build_site(project_path: &Path) -> Result<(), String> {
    use std::fs::remove_dir_all;
    
    if let Err(e) = env::set_current_dir(project_path) {
        return Err(format!("Cannot access directory '{}': {}", project_path.display(), e));
    }
    
    let config_path = PathBuf::from("hrml.toml");
    if !config_path.exists() {
        return Err(format!("No hrml.toml found in {}", env::current_dir().unwrap().display()));
    }
    
    let config = config::Config::load(&config_path.to_string_lossy())
        .map_err(|e| format!("Failed to load config: {}", e))?;
    
    let dist_path = PathBuf::from("dist");
    if dist_path.exists() {
        remove_dir_all(&dist_path).map_err(|e| format!("Failed to remove dist: {}", e))?;
    }
    fs::create_dir_all(&dist_path).map_err(|e| format!("Failed to create dist: {}", e))?;
    
    println!("Building static site to 'dist/'...");
    
    let template_engine = template::Engine::new(&config.templates_path)
        .with_site_name(config.site_name.clone())
        .with_description(config.site_description.clone())
        .with_favicon(config.favicon.clone());
    
    fn collect_templates(base: &Path, prefix: &str) -> Vec<(String, PathBuf)> {
        let mut templates = Vec::new();
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let new_prefix = if prefix.is_empty() {
                        path.file_name().unwrap().to_string_lossy().to_string()
                    } else {
                        format!("{}/{}", prefix, path.file_name().unwrap().to_string_lossy())
                    };
                    templates.extend(collect_templates(&path, &new_prefix));
                } else if let Some(ext) = path.extension() {
                    if ext == "hrml" || ext == "html" {
                        let name = path.file_name().unwrap().to_string_lossy().to_string();
                        let rel = if prefix.is_empty() {
                            name.clone()
                        } else {
                            format!("{}/{}", prefix, name)
                        };
                        templates.push((rel, path));
                    }
                }
            }
        }
        templates
    }
    
    let templates_path = PathBuf::from(&config.templates_path);
    let all_templates = collect_templates(&templates_path, "");
    
    let mut rendered_count = 0;
    for (rel_path, _src_path) in &all_templates {
        if !rel_path.starts_with("pages/") {
            continue;
        }
        
        let html = match template_engine.render(&rel_path, &serde_json::json!({})) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Warning: Failed to render {}: {}", rel_path, e);
                continue;
            }
        };
        
        let out_name = if rel_path.starts_with("pages/") {
            let name = rel_path.strip_prefix("pages/").unwrap().to_string();
            if name.ends_with(".hrml") {
                name.strip_suffix(".hrml").unwrap().to_string() + ".html"
            } else {
                name
            }
        } else if rel_path.ends_with(".hrml") {
            rel_path.strip_suffix(".hrml").unwrap().to_string() + ".html"
        } else {
            rel_path.to_string()
        };
        
        let out_path = dist_path.join(&out_name);
        if let Some(parent) = out_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        
        fs::write(&out_path, &html).map_err(|e| format!("Failed to write {}: {}", out_name, e))?;
        rendered_count += 1;
    }
    
    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
        if !src.exists() {
            return Ok(());
        }
        fs::create_dir_all(dst).map_err(|e| format!("Failed to create dir: {}", e))?;
        
        for entry in fs::read_dir(src).map_err(|e| format!("Failed to read dir: {}", e))? {
            let entry = entry.map_err(|e| format!("Entry error: {}", e))?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path).map_err(|e| format!("Failed to copy: {}", e))?;
            }
        }
        Ok(())
    }
    
    let static_src = PathBuf::from(&config.static_path);
    copy_dir_recursive(&static_src, &dist_path)?;
    
    println!("Done! {} pages rendered to dist/", rendered_count);
    println!("Run 'hrml serve' to preview the built site.");
    
    Ok(())
}

async fn serve_static(project_path: &Path) -> Result<(), String> {
    use std::net::SocketAddr;
    
    if let Err(e) = env::set_current_dir(project_path) {
        return Err(format!("Cannot access directory '{}': {}", project_path.display(), e));
    }
    
    let config_path = PathBuf::from("hrml.toml");
    if !config_path.exists() {
        return Err(format!("No hrml.toml found in {}", env::current_dir().unwrap().display()));
    }
    
    let config = config::Config::load(&config_path.to_string_lossy())
        .map_err(|e| format!("Failed to load config: {}", e))?;
    
    if !PathBuf::from("dist").exists() {
        return Err("dist/ not found. Run 'hrml build' first.".to_string());
    }
    
    println!("Serving static files from 'dist/' on http://{}:{}", config.host, config.port);
    
    let app = Router::new()
        .fallback_service(ServeDir::new("dist"))
        .with_state(Arc::new(()));
    
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()
        .map_err(|e| format!("Invalid address: {}", e))?;
    
    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| format!("Failed to bind: {}", e))?;
    
    println!("Server running at http://{}", addr);
    axum::serve(listener, app).await
        .map_err(|e| format!("Server error: {}", e))?;
    
    Ok(())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_help();
        return;
    }
    
    let command = &args[1];
    
    match command.as_str() {
        "help" | "--help" | "-h" => {
            print_help();
        }
        "version" | "--version" | "-v" => {
            println!("HRML {}", VERSION);
        }
        "new" => {
            if args.len() < 3 {
                eprintln!("Error: Project name required");
                eprintln!("Usage: hrml new <name>");
                process::exit(1);
            }
            let name = &args[2];
            if let Err(e) = create_project(name) {
                eprintln!("Error creating project: {}", e);
                process::exit(1);
            }
        }
        "dev" => {
            let path = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            run_server(&path, true).await;
        }
        "serve" => {
            let path = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            if let Err(e) = serve_static(&path).await {
                eprintln!("Serve error: {}", e);
                process::exit(1);
            }
        }
        "build" => {
            let path = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            if let Err(e) = build_site(&path) {
                eprintln!("Build failed: {}", e);
                process::exit(1);
            }
        }
        "check" => {
            let path = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            if let Err(e) = check_project(&path) {
                eprintln!("Check failed: {}", e);
                process::exit(1);
            }
        }
        "auth" => {
            if args.len() < 3 {
                eprintln!("Error: Username required");
                eprintln!("Usage: hrml auth <user>");
                process::exit(1);
            }
            let username = &args[2];
            let password = read_password();
            match auth::authenticate(username, &password) {
                Ok(true) => {
                    println!("Authentication successful for '{}'", username);
                    process::exit(0);
                }
                Ok(false) => {
                    eprintln!("Authentication failed for '{}'", username);
                    process::exit(1);
                }
                Err(e) => {
                    eprintln!("PAM error: {}", e);
                    process::exit(2);
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!();
            print_help();
            process::exit(1);
        }
    }
}

async fn index_handler(State(state): State<AppState>) -> Response {
    for template_path in ["pages/index.hrml", "pages/index.html"] {
        if let Ok(html) = state
            .template_engine
            .render(template_path, &serde_json::json!({}))
        {
            return Html(html).into_response();
        }
    }

    if let Some(response) = try_serve_static(&state.static_path, "index.html") {
        response
    } else {
        (StatusCode::NOT_FOUND, "Page not found").into_response()
    }
}

async fn page_handler(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let normalized = path.trim_matches('/');

    if normalized.is_empty() {
        return index_handler(State(state)).await;
    }

    if normalized.contains("..") {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }

    let template_candidates = [
        format!("pages/{}.hrml", normalized),
        format!("pages/{}.html", normalized),
        format!("pages/{}/index.hrml", normalized),
        format!("pages/{}/index.html", normalized),
    ];

    for template_path in template_candidates {
        if let Ok(html) = state
            .template_engine
            .render(&template_path, &serde_json::json!({}))
        {
            return Html(html).into_response();
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

    match state.template_engine.render("pages/404.hrml", &serde_json::json!({})) {
        Ok(html) => (StatusCode::NOT_FOUND, Html(html)).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Page not found").into_response(),
    }
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
    match state.backend_runtime.call_endpoint(&format!("/api/{}", path), &serde_json::json!({})) {
        Ok(result) => {
            if let Some(html) = result.as_str() {
                Html(html.to_string()).into_response()
            } else {
                (StatusCode::OK, serde_json::to_string(&result).unwrap_or_default()).into_response()
            }
        }
        Err(e) => {
            eprintln!("[ERROR] API GET /api/{} failed: {}", path, e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Endpoint error: {}", e)).into_response()
        },
    }
}

async fn endpoint_handler(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    request: Request<Body>,
) -> Response {
    // Extract form data from request body
    let body_bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("[ERROR] POST /{} - failed to read body: {}", path, e);
            return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
        }
    };
    
    // Parse form data
    let form_data = if !body_bytes.is_empty() {
        let body_str = String::from_utf8_lossy(&body_bytes);
        let mut data = serde_json::Map::new();
        
        // Parse URL-encoded form data
        for pair in body_str.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                let decoded_value = urlencoding::decode(value).unwrap_or(std::borrow::Cow::Borrowed(value));
                data.insert(key.to_string(), serde_json::Value::String(decoded_value.to_string()));
            }
        }
        
        serde_json::Value::Object(data)
    } else {
        serde_json::json!({})
    };
    
    // Transform path to include /api/ prefix
    let full_path = format!("/api/{}", path);
    
    match state.backend_runtime.call_endpoint(&full_path, &form_data) {
        Ok(result) => {
            if let Some(html) = result.as_str() {
                Html(html.to_string()).into_response()
            } else {
                (StatusCode::OK, serde_json::to_string(&result).unwrap_or_default()).into_response()
            }
        }
        Err(e) => {
            eprintln!("[ERROR] POST /{} - endpoint error: {}", path, e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Endpoint error: {}", e)).into_response()
        },
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
