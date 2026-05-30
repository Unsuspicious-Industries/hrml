use serde::Deserialize;
use serde::Serialize;
use serde_json::{Map, Value};

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_templates_path() -> String {
    "templates".to_string()
}

fn default_endpoints_path() -> String {
    "endpoints".to_string()
}

fn default_static_path() -> String {
    "static".to_string()
}

fn default_site_name() -> String {
    "HRML App".to_string()
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_templates_path")]
    pub templates_path: String,

    #[serde(default = "default_endpoints_path")]
    pub endpoints_path: String,

    #[serde(default = "default_static_path")]
    pub static_path: String,

    #[serde(default = "default_site_name")]
    pub site_name: String,

    #[serde(default)]
    pub site_description: Option<String>,

    #[serde(default)]
    pub favicon: Option<String>,

    #[serde(default)]
    pub site_url: Option<String>,

    #[serde(default = "default_globals")]
    pub globals: Value,

    /// The layout a page is wrapped in when it declares no `<?load?>` of its own.
    /// Lets authors write only their `<?block?>` fills; the engine supplies the
    /// surrounding document. Configured as `[templates] layout = "…"`.
    #[serde(default)]
    pub default_layout: Option<String>,

    /// Files auto-loaded ahead of the default layout (component libraries, etc.),
    /// so pages never repeat their imports. Configured as `[templates] imports = […]`.
    #[serde(default)]
    pub auto_imports: Vec<String>,
}

fn default_globals() -> Value {
    Value::Object(Map::new())
}

#[derive(Deserialize, Default)]
struct RawConfig {
    server: Option<RawServer>,
    paths: Option<RawPaths>,
    site: Option<RawSite>,
    templates: Option<RawTemplates>,
    host: Option<String>,
    port: Option<u16>,
    templates_path: Option<String>,
    endpoints_path: Option<String>,
    static_path: Option<String>,
    site_name: Option<String>,
    site_description: Option<String>,
    favicon: Option<String>,
    site_url: Option<String>,
    globals: Option<toml::Value>,
}

#[derive(Deserialize, Default)]
struct RawServer {
    host: Option<String>,
    port: Option<u16>,
}

#[derive(Deserialize, Default)]
struct RawPaths {
    templates: Option<String>,
    endpoints: Option<String>,
    #[serde(rename = "static")]
    static_: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawSite {
    name: Option<String>,
    description: Option<String>,
    favicon: Option<String>,
    url: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawTemplates {
    layout: Option<String>,
    #[serde(default)]
    imports: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            templates_path: "templates".to_string(),
            endpoints_path: "endpoints".to_string(),
            static_path: "static".to_string(),
            site_name: "HRML App".to_string(),
            site_description: Some("A web application built with HRML".to_string()),
            favicon: None,
            site_url: None,
            globals: default_globals(),
            default_layout: None,
            auto_imports: Vec::new(),
        }
    }
}

impl Config {
    pub fn from_toml(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let raw: RawConfig = toml::from_str(&content)?;

        let mut config = Config::default();

        if let Some(server) = raw.server {
            if let Some(host) = server.host {
                config.host = host;
            }
            if let Some(port) = server.port {
                config.port = port;
            }
        }

        if let Some(paths) = raw.paths {
            if let Some(templates) = paths.templates {
                config.templates_path = templates;
            }
            if let Some(endpoints) = paths.endpoints {
                config.endpoints_path = endpoints;
            }
            if let Some(static_path) = paths.static_ {
                config.static_path = static_path;
            }
        }

        if let Some(site) = raw.site {
            if let Some(name) = site.name {
                config.site_name = name;
            }
            if let Some(description) = site.description {
                config.site_description = Some(description);
            }
            if let Some(favicon) = site.favicon {
                config.favicon = Some(favicon);
            }
            if let Some(url) = site.url {
                config.site_url = Some(url);
            }
        }

        if let Some(templates) = raw.templates {
            if let Some(layout) = templates.layout {
                config.default_layout = Some(layout);
            }
            if !templates.imports.is_empty() {
                config.auto_imports = templates.imports;
            }
        }

        if let Some(host) = raw.host {
            config.host = host;
        }
        if let Some(port) = raw.port {
            config.port = port;
        }
        if let Some(templates_path) = raw.templates_path {
            config.templates_path = templates_path;
        }
        if let Some(endpoints_path) = raw.endpoints_path {
            config.endpoints_path = endpoints_path;
        }
        if let Some(static_path) = raw.static_path {
            config.static_path = static_path;
        }
        if let Some(site_name) = raw.site_name {
            config.site_name = site_name;
        }
        if let Some(site_description) = raw.site_description {
            config.site_description = Some(site_description);
        }
        if let Some(favicon) = raw.favicon {
            config.favicon = Some(favicon);
        }
        if let Some(site_url) = raw.site_url {
            config.site_url = Some(site_url);
        }
        if let Some(globals) = raw.globals {
            config.globals = toml_value_to_json(globals);
        }

        Ok(config)
    }

    pub fn to_toml_string(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(toml::to_string_pretty(self)?)
    }
}

fn toml_value_to_json(value: toml::Value) -> Value {
    match value {
        toml::Value::String(v) => Value::String(v),
        toml::Value::Integer(v) => Value::Number(v.into()),
        toml::Value::Float(v) => serde_json::Number::from_f64(v)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        toml::Value::Boolean(v) => Value::Bool(v),
        toml::Value::Datetime(v) => Value::String(v.to_string()),
        toml::Value::Array(values) => {
            Value::Array(values.into_iter().map(toml_value_to_json).collect())
        }
        toml::Value::Table(table) => Value::Object(
            table
                .into_iter()
                .map(|(key, value)| (key, toml_value_to_json(value)))
                .collect(),
        ),
    }
}
