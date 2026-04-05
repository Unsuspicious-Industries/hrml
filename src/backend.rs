use crate::template::Engine;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Runtime {
    endpoints_path: PathBuf,
}

impl Runtime {
    pub fn new(endpoints_path: &str) -> Self {
        Self {
            endpoints_path: PathBuf::from(endpoints_path),
        }
    }

    pub fn call_endpoint(&self, path: &str, data: &Value) -> Result<Value, String> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if parts.len() < 2 || parts[0] != "api" {
            return Err("Invalid endpoint path; expected /api/<name>[/... ]".to_string());
        }

        let module = parts[1];
        let (id, action) = if parts.len() > 2 {
            if parts[2].parse::<i64>().is_ok() {
                let action = if parts.len() > 3 { parts[3] } else { "" };
                (parts[2], action)
            } else {
                ("", parts[2])
            }
        } else {
            ("", "")
        };

        let context = json!({
            "id": id,
            "action": action,
            "data": data,
        });

        if let Some(path) = self.find_template_path(module, action) {
            let base = self.endpoints_path.to_string_lossy().to_string();
            let rel = path
                .strip_prefix(&self.endpoints_path)
                .map_err(|e| e.to_string())?;
            let rel = rel.to_string_lossy().replace('\\', "/");
            let rendered = Engine::new(&base).render_fragment(&rel, &context)?;
            return Ok(Value::String(rendered));
        }

        if let Some(path) = self.find_text_path(module, action) {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read endpoint file {}: {}", path.display(), e))?;
            return Ok(Value::String(content));
        }

        if let Some(path) = self.find_json_path(module, action) {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read endpoint file {}: {}", path.display(), e))?;
            let value: Value = serde_json::from_str(&content)
                .map_err(|e| format!("Invalid endpoint JSON in {}: {}", path.display(), e))?;
            return Ok(value);
        }

        Err(format!("Endpoint not found for path: {}", path))
    }

    fn find_template_path(&self, module: &str, action: &str) -> Option<PathBuf> {
        self.find_path(module, action, "hrml")
    }

    fn find_text_path(&self, module: &str, action: &str) -> Option<PathBuf> {
        self.find_path(module, action, "html")
            .or_else(|| self.find_path(module, action, "txt"))
    }

    fn find_json_path(&self, module: &str, action: &str) -> Option<PathBuf> {
        self.find_path(module, action, "json")
    }

    fn find_path(&self, module: &str, action: &str, ext: &str) -> Option<PathBuf> {
        let api_root = self.endpoints_path.join("api");
        let candidates = endpoint_candidates(&api_root, module, action, ext);
        candidates.into_iter().find(|candidate| candidate.exists())
    }
}

fn endpoint_candidates(api_root: &Path, module: &str, action: &str, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();

    if !action.is_empty() {
        out.push(api_root.join(module).join(format!("{}.{}", action, ext)));
        out.push(api_root.join(format!("{}_{}.{}", module, action, ext)));
    }

    out.push(api_root.join(format!("{}.{}", module, ext)));
    out
}
