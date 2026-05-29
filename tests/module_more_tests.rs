use xrml::config::Config;
use xrml::ssg::SSG;
use xrml::template::Engine;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn tmp_dir(name: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("hrml_test_{}_{}", name, now));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn config_load_save_roundtrip() {
    let dir = tmp_dir("cfg_roundtrip");
    let toml_path = dir.join("cfg.toml");
    let content = r#"
[server]
host = "0.0.0.0"
port = 9000

[paths]
templates = "tpls"
endpoints = "eps"
static = "stat"

[site]
name = "RoundSite"
description = "Desc"
"#;
    fs::write(&toml_path, content).unwrap();

    let cfg = Config::from_toml(&fs::read_to_string(&toml_path).unwrap()).expect("load cfg");
    assert_eq!(cfg.host, "0.0.0.0");
    assert_eq!(cfg.port, 9000);
    assert_eq!(cfg.templates_path, "tpls");
    assert_eq!(cfg.site_name, "RoundSite");

    // save
    let out = dir.join("out.toml");
    fs::write(&out, cfg.to_toml_string().expect("save")).expect("write out");
    let cfg2 = Config::from_toml(&fs::read_to_string(&out).unwrap()).expect("load2");
    assert_eq!(cfg2.port, 9000);
}

#[test]
fn ssg_default_404_generated_when_missing() {
    let src = tmp_dir("ssg_404_src");
    let pages = src.join("pages");
    fs::create_dir_all(&pages).unwrap();
    fs::write(pages.join("index.hrml"), "<p>Home</p>").unwrap();

    let out = tmp_dir("ssg_404_out");
    let ssg = SSG::new(pages.to_str().unwrap(), out.to_str().unwrap()).with_data(json!({}));
    let report = ssg.build().expect("ssg build");
    // report should include /404
    assert!(report.pages.iter().any(|p| p.url == "/404"));
    let out404 = out.join("404.html");
    assert!(out404.exists());
    let body = fs::read_to_string(out404).unwrap();
    assert!(body.contains("404") && body.contains("Page not found"));
}

#[test]
fn engine_renders_wasm_and_btn_and_form() {
    let dir = tmp_dir("engine_widgets");
    fs::create_dir_all(dir.join("pages")).unwrap();
    let tpl = r#"<?wasm module="/app.wasm"?>
<?btn post="/api/x"?>Go</?btn?>
<?form post="/api/post"?>
<input name="a">
</?form?>"#;
    fs::write(dir.join("pages/test.hrml"), tpl).unwrap();

    let e = Engine::new(dir.to_str().unwrap());
    let out = e.render("pages/test.hrml", &json!({})).unwrap();
    assert!(
        out.contains("data-wasm-module=\"/app.wasm\"")
            || out.contains("data-wasm-module=&#34;/app.wasm&#34;")
    );
    assert!(out.contains("data-post=\"/api/x\"") || out.contains("data-post=&#34;/api/x&#34;"));
    assert!(
        out.contains("data-post=\"/api/post\"") || out.contains("data-post=&#34;/api/post&#34;")
    );
}
