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
        let dir = format!("/tmp/hrml_unit_comp_{}_{}", name, now);
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
fn define_and_use_simple_component() {
    let env = TestEnv::new("unit_comp_basic");
    env.write(
        "pages/test.hrml",
        r#"<?component id=\"x\"?><span>X</span></?component?>
<?use id=\"x\"?></?use?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<span>X</span>"));
}

#[test]
fn component_slots_and_defaults() {
    let env = TestEnv::new("unit_comp_slots");
    env.write("pages/test.hrml",
        r#"<?component id=\"c\"?><article><?slot id=\"body\"?>Default</?slot?></article></?component?>
<?use id=\"c\"?><?block slot=\"body\"?>Custom</?block?></?use?>"#);
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<article>"));
    assert!(out.contains("Custom"));
    assert!(!out.contains("Default"));
}
