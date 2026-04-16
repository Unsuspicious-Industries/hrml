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
        let dir = format!("/tmp/hrml_unit_compose_{}_{}", name, now);
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
fn compose_sum_basic() {
    let env = TestEnv::new("unit_compose_sum");
    env.write(
        "pages/test.hrml",
        r#"<?compose op=\"sum\"?><span>A</span><?then?><span>B</span></?compose?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<span>A</span>") && out.contains("<span>B</span>"));
}

#[test]
fn compose_product_wrap() {
    let env = TestEnv::new("unit_compose_prod");
    env.write(
        "pages/test.hrml",
        r#"<?compose op=\"product\"?><div>L</div><?then?><div>R</div></?compose?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("hrml-product") || out.contains("hrml-factor"));
}
