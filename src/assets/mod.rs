pub const BASE_LAYOUT: &str = include_str!("layout/base.hrml");
pub const NAV: &str = include_str!("components/nav.hrml");
pub const INDEX_PAGE: &str = include_str!("pages/index.hrml");
pub const ABOUT_PAGE: &str = include_str!("pages/about.hrml");
pub const STYLE_CSS: &str = include_str!("css/style.css");
pub const HELLO_ENDPOINT: &str = include_str!("endpoints/api/hello.hrml");

pub const VERSION: &str = "0.1.0";

use crate::config::Config;

pub fn default_config(name: &str) -> String {
    let config = Config {
        host: "127.0.0.1".to_string(),
        port: 8080,
        templates_path: "templates".to_string(),
        endpoints_path: "endpoints".to_string(),
        static_path: "static".to_string(),
        site_name: name.to_string(),
        site_description: Some("A web application built with HRML".to_string()),
        favicon: Some("/static/favicon.ico".to_string()),
        site_url: None,
        globals: serde_json::Value::Object(serde_json::Map::new()),
    };
    toml::to_string_pretty(&config).unwrap()
}

pub fn readme(name: &str) -> String {
    format!(
        r#"# {}

A web application built with HRML - Minimal Web Framework

## Project Structure

```
{}/
├── hrml.toml              # Configuration
├── templates/             # HTML templates
│   ├── layouts/          # Layout templates
│   ├── components/       # Reusable components
│   └── pages/            # Page templates
├── endpoints/            # Rust-native endpoint templates
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

1. Create a `.hrml`, `.html`, or `.json` file in `endpoints/api/`
2. Optional actions can be defined as `<name>/<action>.hrml`
3. Access the endpoint at `/api/<name>/<action>`

See the HRML documentation for more details.
"#,
        name, name
    )
}

pub const GITIGNORE: &str = r#"# HRML
dist/

# Environment
.env
.env.local

"#;
