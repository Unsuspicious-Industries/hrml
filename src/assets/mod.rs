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
        default_layout: None,
        auto_imports: Vec::new(),
        component_paths: vec!["components".to_string()],
        strict_colors: false,
    };
    toml::to_string_pretty(&config).unwrap()
}

pub fn readme(name: &str) -> String {
    format!(
        r#"# {}

Configuration is in `xrml.toml`. Templates are under `templates/pages`,
`templates/layouts` and `templates/components`; endpoint sources are under
`endpoints/api`; assets are under `static`.

Run from this project directory:

```sh
xrml dev
xrml serve
xrml check
xrml build
```

`dev` and `serve` run the source project. `build` exports static files to
`dist`; it does not deploy them. Use `--palette FILE` when the project uses
palette tokens. Files containing tokens are not automatically assigned a
palette.

Add pages under `templates/pages` and navigation links in
`templates/components/nav.hrml`. Use `xrml help` for command options and the
[HRML reference](https://github.com/Unsuspicious-Industries/hrml/blob/master/spec.md)
for template, routing and endpoint contracts.
"#,
        name
    )
}

pub const GITIGNORE: &str = r#"# HRML
dist/

# Environment
.env
.env.local

"#;
