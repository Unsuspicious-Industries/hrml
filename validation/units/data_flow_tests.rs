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
        let dir = format!("/tmp/hrml_unit_data_{}_{}", name, now);
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
    fn render_with_data(&self, path: &str, data: &serde_json::Value) -> Result<String, String> {
        Engine::new(&self.dir).render(path, data)
    }
}
impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn get_from_json_context() {
    let env = TestEnv::new("unit_data_json");
    env.write("pages/test.hrml", "<h1><?get id=\"user.name\"?></h1>");
    let out = env
        .render_with_data("pages/test.hrml", &json!({ "user": { "name": "Zoe" } }))
        .unwrap();
    assert!(out.contains("<h1>Zoe</h1>"));
}

#[test]
fn set_and_get_override_precedence() {
    let env = TestEnv::new("unit_data_setget");
    env.write(
        "pages/test.hrml",
        r#"<?set id=\"title\"?>Local</?set?>
<h1><?get id=\"title\"?></h1>"#,
    );
    let out = env
        .render_with_data("pages/test.hrml", &json!({ "title": "Remote" }))
        .unwrap();
    assert!(out.contains("<h1>Local</h1>"));
}
