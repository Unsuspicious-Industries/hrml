use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::io;
use std::path::Path;

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
}

#[derive(Deserialize, Default)]
struct RawConfig {
    server: Option<RawServer>,
    paths: Option<RawPaths>,
    site: Option<RawSite>,
    host: Option<String>,
    port: Option<u16>,
    templates_path: Option<String>,
    endpoints_path: Option<String>,
    static_path: Option<String>,
    site_name: Option<String>,
    site_description: Option<String>,
    favicon: Option<String>,
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
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
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

        Ok(config)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn setup(&mut self) -> io::Result<()> {
        fs::create_dir_all(self.templates_path.clone() + "/pages")?;
        fs::create_dir_all(self.templates_path.clone() + "/layouts")?;
        fs::create_dir_all(self.templates_path.clone() + "/components")?;
        fs::create_dir_all(self.endpoints_path.clone() + "/api")?;
        fs::create_dir_all(self.static_path.clone() + "/css")?;
        fs::create_dir_all(self.static_path.clone() + "/js")?;
        fs::create_dir_all(self.static_path.clone() + "/images")?;

        let config_path = "hrml.toml";
        if !Path::new(config_path).exists() {
            let config_str = crate::assets::default_config(&self.site_name);
            fs::write(config_path, config_str)?;
        }

        let templates_dir = Path::new(&self.templates_path);

        let base_layout_path = templates_dir.join("layouts/base.hrml");
        if !base_layout_path.exists() {
            fs::write(base_layout_path, crate::assets::BASE_LAYOUT)?;
        }

        let nav_path = templates_dir.join("components/nav.hrml");
        if !nav_path.exists() {
            fs::write(nav_path, crate::assets::NAV)?;
        }

        let index_path = templates_dir.join("pages/index.hrml");
        if !index_path.exists() {
            fs::write(index_path, crate::assets::INDEX_PAGE)?;
        }

        let about_path = templates_dir.join("pages/about.hrml");
        if !about_path.exists() {
            fs::write(about_path, crate::assets::ABOUT_PAGE)?;
        }

        let css_path = Path::new(&self.static_path).join("css/style.css");
        if !css_path.exists() {
            fs::write(css_path, crate::assets::STYLE_CSS)?;
        }

        let endpoints_dir = Path::new(&self.endpoints_path);
        let hello_path = endpoints_dir.join("api/hello.hrml");
        if !hello_path.exists() {
            fs::write(hello_path, crate::assets::HELLO_ENDPOINT)?;
        }

        if !Path::new("README.md").exists() {
            fs::write("README.md", crate::assets::readme(&self.site_name))?;
        }

        if !Path::new(".gitignore").exists() {
            fs::write(".gitignore", crate::assets::GITIGNORE)?;
        }

        Ok(())
    }
}
