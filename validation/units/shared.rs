use xrml::config::Config;
use xrml::project::Project;
use xrml::template::Engine;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) struct TestEnv {
    dir: PathBuf,
}

impl TestEnv {
    pub(crate) fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hrml_unit_{}_{}", name, now));
        let _ = fs::create_dir_all(&dir);
        Self { dir }
    }

    pub(crate) fn write(&self, path: &str, content: &str) {
        let full = self.dir.join(path);
        if let Some(parent) = Path::new(&full).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&full, content).unwrap();
    }

    pub(crate) fn engine(&self) -> Engine {
        Engine::new(self.dir.to_str().unwrap())
    }

    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    pub(crate) fn render(&self, path: &str) -> Result<String, String> {
        self.engine()
            .render(path, &json!({}))
            .map_err(|e| e.to_string())
    }

    pub(crate) fn render_fragment(&self, path: &str) -> Result<String, String> {
        self.engine()
            .render_fragment(path, &json!({}))
            .map_err(|e| e.to_string())
    }

    pub(crate) fn render_with_data(&self, path: &str, data: &Value) -> Result<String, String> {
        self.engine().render(path, data).map_err(|e| e.to_string())
    }

    pub(crate) fn render_project(&self, files: &[&str], target: &str) -> Result<String, String> {
        let config = Config::default();
        let mut project = Project::new(config).with_base_path(&self.dir);
        for &path in files {
            let source = fs::read_to_string(self.dir.join(path))
                .map_err(|e| format!("no such test file {path}: {e}"))?;
            project.add_file(path.to_string(), source);
        }
        project.parse_all().map_err(|e| e.to_string())?;
        project
            .render(target, &json!({}))
            .map_err(|e| e.to_string())
    }

}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
