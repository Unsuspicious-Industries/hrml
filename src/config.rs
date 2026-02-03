use serde::Deserialize;
use std::fs;

#[derive(Clone, Deserialize)]
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

impl Default for Config {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            templates_path: default_templates_path(),
            endpoints_path: default_endpoints_path(),
            static_path: default_static_path(),
            site_name: default_site_name(),
            site_description: None,
            favicon: None,
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
