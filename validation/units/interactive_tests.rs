use hrml::template::Engine;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

struct TestEnv {
    dir: String,
}
impl TestEnv {
    fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = format!("/tmp/hrml_unit_inter_{}_{}", name, now);
        let _ = fs::create_dir_all(&dir);
        TestEnv { dir }
    }
    fn write(&self, path: &str, content: &str) {
        let full = format!("{}/{}", self.dir, path);
        if let Some(parent) = Path::new(&full).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&full, content).unwrap();
    }
    fn render(&self, path: &str) -> Result<String, String> {
        Engine::new(&self.dir).render(path, &json!({}))
    }
}
impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn btn_renders_button_with_data_attrs() {
    let env = TestEnv::new("unit_btn");
    env.write(
        "pages/test.hrml",
        r##"<?btn post="/api/x" target="#out" swap="innerHTML"?>Click</?btn?>"##,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<button") && out.contains("data-post=\"/api/x\""));
}

#[test]
fn form_renders_form_tag() {
    let env = TestEnv::new("unit_form");
    env.write(
        "pages/test.hrml",
        r##"<?form post="/api/submit"?>
<input name="e">
</?form?>"##,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<form") && out.contains("data-post"));
}
