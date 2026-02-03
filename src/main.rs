use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

mod config;
mod router;
mod template;
mod python;
mod html;
mod db;

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
    println!("HRML - Minimal Web Framework v{}", VERSION);
    println!();
    println!("Usage: hrml <command> [options]");
    println!();
    println!("Commands:");
    println!("  new <name>          Create a new HRML project");
    println!("  dev [path]          Run development server with auto-reload");
    println!("  serve [path]        Run production server");
    println!("  build [path]        Build static site for deployment");
    println!("  check [path]        Validate templates and configuration");
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
    
    // Create directory structure
    fs::create_dir_all(project_path.join("templates/pages"))?;
    fs::create_dir_all(project_path.join("templates/layouts"))?;
    fs::create_dir_all(project_path.join("templates/components"))?;
    fs::create_dir_all(project_path.join("endpoints/api"))?;
    fs::create_dir_all(project_path.join("static/css"))?;
    fs::create_dir_all(project_path.join("static/js"))?;
    fs::create_dir_all(project_path.join("static/images"))?;
    
    // Create hrml.toml
    let config = format!(r#"[project]
name = "{}"
version = "0.1.0"

[server]
host = "127.0.0.1"
port = 8080

[paths]
templates = "templates"
endpoints = "endpoints"
static = "static"

[site]
name = "{}"
description = "A web application built with HRML"
favicon = "/static/favicon.ico"
"#, name, name);
    fs::write(project_path.join("hrml.toml"), config)?;
    
    // Create base layout
    let base_layout = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title><?get id="site_name"?></title>
    <link rel="stylesheet" href="/static/css/style.css">
    <script src="/hrml.js"></script>
</head>
<body>
    <?load file="components/nav.hrml"?>
    
    <main class="container">
        <?slot id="content"?>
    </main>
    
    <footer>
        <p>&copy; 2024 <?get id="site_name"?></p>
    </footer>
</body>
</html>"#;
    fs::write(project_path.join("templates/layouts/base.hrml"), base_layout)?;
    
    // Create nav component
    let nav = r#"<nav class="navbar">
    <div class="nav-brand">
        <a href="/"><?get id="site_name"?></a>
    </div>
    <ul class="nav-links">
        <li><a href="/">Home</a></li>
        <li><a href="/about">About</a></li>
    </ul>
</nav>"#;
    fs::write(project_path.join("templates/components/nav.hrml"), nav)?;
    
    // Create index page
    let index = r#"<?load file="layouts/base.hrml"?>
<?block slot="content"?>
    <div class="hero">
        <h1>Welcome to <?get id="site_name"?></h1>
        <p>Your new HRML application is ready!</p>
    </div>
    
    <div class="card">
        <h2>Getting Started</h2>
        <p>Edit <code>templates/pages/index.hrml</code> to customize this page.</p>
        <p>Add Python endpoints in <code>endpoints/api/</code> for dynamic functionality.</p>
    </div>
</?block?>"#;
    fs::write(project_path.join("templates/pages/index.hrml"), index)?;
    
    // Create about page
    let about = r#"<?load file="layouts/base.hrml"?>
<?block slot="content"?>
    <h1>About</h1>
    <p>This is the about page.</p>
</?block?>"#;
    fs::write(project_path.join("templates/pages/about.hrml"), about)?;
    
    // Create default CSS
    let css = r#"/* HRML Default Styles */
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    line-height: 1.6;
    color: #333;
    background: #f5f5f5;
}

.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
}

.navbar {
    background: #2c3e50;
    color: white;
    padding: 1rem 2rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.nav-brand a {
    color: white;
    text-decoration: none;
    font-size: 1.5rem;
    font-weight: bold;
}

.nav-links {
    display: flex;
    list-style: none;
    gap: 2rem;
}

.nav-links a {
    color: white;
    text-decoration: none;
}

.nav-links a:hover {
    text-decoration: underline;
}

.hero {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
    padding: 4rem 2rem;
    text-align: center;
    border-radius: 8px;
    margin-bottom: 2rem;
}

.hero h1 {
    font-size: 2.5rem;
    margin-bottom: 1rem;
}

.card {
    background: white;
    padding: 2rem;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    margin-bottom: 1rem;
}

.btn {
    display: inline-block;
    padding: 0.75rem 1.5rem;
    background: #667eea;
    color: white;
    text-decoration: none;
    border-radius: 4px;
    border: none;
    cursor: pointer;
    font-size: 1rem;
}

.btn:hover {
    background: #5568d3;
}

footer {
    text-align: center;
    padding: 2rem;
    color: #666;
}

code {
    background: #f4f4f4;
    padding: 0.2rem 0.4rem;
    border-radius: 3px;
    font-family: 'Courier New', monospace;
}"#;
    fs::write(project_path.join("static/css/style.css"), css)?;
    
    // Create empty __init__.py files
    fs::write(project_path.join("endpoints/__init__.py"), "")?;
    fs::write(project_path.join("endpoints/api/__init__.py"), "")?;
    
    // Create a sample endpoint
    let sample_endpoint = r#"def handler(req):
    # GET /api/hello - Returns a simple greeting
    return "<div class='card'><h2>Hello from Python!</h2><p>This response came from a Python endpoint.</p></div>"
"#;
    fs::write(project_path.join("endpoints/api/hello.py"), sample_endpoint)?;
    
    // Create README
    let readme = format!(r#"# {}

A web application built with HRML - Minimal Web Framework

## Project Structure

```
{}/
├── hrml.toml              # Configuration
├── templates/             # HTML templates
│   ├── layouts/          # Layout templates
│   ├── components/       # Reusable components
│   └── pages/            # Page templates
├── endpoints/            # Python backend endpoints
│   └── api/              # API endpoints
└── static/               # Static assets
    ├── css/              # Stylesheets
    ├── js/               # JavaScript files
    └── images/           # Images
```

## Development

```bash
# Run development server with auto-reload
hrml dev

# Or serve from this directory
hrml serve
```

## Building for Production

```bash
# Build static site
hrml build

# Output will be in the `dist/` directory
```

## Adding Pages

1. Create a new template in `templates/pages/`
2. Link to it from navigation in `templates/components/nav.hrml`

## Adding API Endpoints

1. Create a new Python file in `endpoints/api/`
2. Define a `handler(req)` function
3. Access the endpoint at `/api/<filename>/<action>`

See the HRML documentation for more details.
"#, name, name);
    fs::write(project_path.join("README.md"), readme)?;
    
    // Create .gitignore
    let gitignore = r#"# HRML
dist/
*.pyc
__pycache__/
.DS_Store
*.db
*.sqlite3

# Environment
.env
.env.local

# IDE
.vscode/
.idea/
*.swp
*.swo

# Dependencies
node_modules/
vendor/
"#;
    fs::write(project_path.join(".gitignore"), gitignore)?;
    
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
    
    // Try to load config
    let config = config::Config::load(&config_path.to_string_lossy())
        .map_err(|e| format!("Failed to load hrml.toml: {}", e))?;
    
    // Check templates directory
    let templates_path = path.join(&config.templates_path);
    if !templates_path.exists() {
        return Err(format!(
            "Templates directory not found: {}\nCreate it or update hrml.toml",
            templates_path.display()
        ));
    }
    
    // Check required templates
    let base_layout = templates_path.join("layouts/base.hrml");
    if !base_layout.exists() {
        eprintln!("[WARNING] Base layout not found at {}", base_layout.display());
    }
    
    let index_page = templates_path.join("pages/index.hrml");
    if !index_page.exists() {
        eprintln!("[WARNING] Index page not found at {}", index_page.display());
    }
    
    // Check static directory
    let static_path = path.join(&config.static_path);
    if !static_path.exists() {
        eprintln!("[WARNING] Static directory not found: {}", static_path.display());
    }
    
    // Validate template engine can be created
    let engine = template::Engine::new(&templates_path.to_string_lossy());
    
    // Try to render index
    match engine.render("pages/index.hrml", &serde_json::json!({})) {
        Ok(_) => println!("[OK] Index template renders successfully"),
        Err(e) => eprintln!("[WARNING] Index template failed to render: {}", e),
    }
    
    println!("[OK] Project validation complete");
    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    config: Arc<config::Config>,
    template_engine: Arc<template::Engine>,
    python_runtime: Arc<python::Runtime>,
}

async fn run_server(project_path: &Path, dev_mode: bool) {
    // Change to project directory
    if let Err(e) = env::set_current_dir(project_path) {
        eprintln!("Error: Cannot access directory '{}': {}", project_path.display(), e);
        process::exit(1);
    }
    
    // Validate project first
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
    let python_runtime = Arc::new(python::Runtime::new(&config.endpoints_path));
    
    let state = AppState {
        config: Arc::new(config.clone()),
        template_engine,
        python_runtime,
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
            run_server(&path, false).await;
        }
        "check" => {
            let path = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            match validate_project(&path) {
                Ok(_) => {
                    println!("Project is valid!");
                }
                Err(e) => {
                    eprintln!("Validation failed: {}", e);
                    process::exit(1);
                }
            }
        }
        "build" => {
            let path = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            println!("Building static site from '{}'...", path.display());
            println!("Note: Build feature coming in next version!");
            println!("For now, use 'hrml serve' to run the server.");
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
    match state.template_engine.render("pages/index.hrml", &serde_json::json!({})) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("[ERROR] Template render failed for /: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Template error: {}", e)).into_response()
        },
    }
}

async fn page_handler(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let template_path = format!("pages/{}.hrml", path);
    
    match state.template_engine.render(&template_path, &serde_json::json!({})) {
        Ok(html) => Html(html).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Page not found").into_response(),
    }
}

async fn api_get_handler(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    match state.python_runtime.call_endpoint(&format!("/api/{}", path), &serde_json::json!({})) {
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
    
    match state.python_runtime.call_endpoint(&full_path, &form_data) {
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