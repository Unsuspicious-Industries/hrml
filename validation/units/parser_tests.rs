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
        let dir = format!("/tmp/hrml_unit_{}_{}", name, now);
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

    fn engine(&self) -> Engine {
        Engine::new(&self.dir)
    }

    fn render(&self, path: &str) -> Result<String, String> {
        self.engine().render(path, &json!({}))
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn parses_basic_text_and_html() {
    let env = TestEnv::new("unit_parser_basic");
    env.write("pages/a.hrml", "Hello Unit");
    env.write("pages/b.hrml", "<div><p>Block</p></div>");
    let ha = env.render("pages/a.hrml").unwrap();
    let hb = env.render("pages/b.hrml").unwrap();
    assert!(ha.contains("Hello Unit"));
    assert!(hb.contains("<div><p>Block</p></div>"));
}

#[test]
fn parses_attributes_and_voids() {
    let env = TestEnv::new("unit_parser_attrs");
    env.write(
        "pages/test.hrml",
        r#"<img src=\"/x.png\"> <input required>"#,
    );
    let html = env.render("pages/test.hrml").unwrap();
    assert!(html.contains("<img"));
    assert!(html.contains("required"));
}
