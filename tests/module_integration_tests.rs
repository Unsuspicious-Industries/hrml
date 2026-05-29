use xrml::config::Config;
use xrml::router::Router;
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
fn config_serializes_without_filesystem_side_effects() {
    let mut cfg = Config::default();
    cfg.templates_path = "templates_test".to_string();
    cfg.endpoints_path = "endpoints_test".to_string();
    cfg.static_path = "static_test".to_string();
    cfg.site_name = "TestSite".to_string();
    cfg.site_description = Some("Integration description".to_string());

    let toml = cfg.to_toml_string().expect("serialize config");
    let roundtrip = Config::from_toml(&toml).expect("deserialize config");

    assert_eq!(roundtrip.templates_path, "templates_test");
    assert_eq!(roundtrip.endpoints_path, "endpoints_test");
    assert_eq!(roundtrip.static_path, "static_test");
    assert_eq!(roundtrip.site_name, "TestSite");
    assert_eq!(
        roundtrip.site_description.as_deref(),
        Some("Integration description")
    );
}

#[test]
fn engine_render_and_fragment_behavior() {
    let dir = tmp_dir("engine_render");
    fs::create_dir_all(dir.join("pages")).unwrap();
    fs::write(dir.join("pages/test.hrml"), "<p>Hello Engine</p>").unwrap();

    let e = Engine::new(dir.to_str().unwrap());
    let full = e.render("pages/test.hrml", &json!({})).unwrap();
    assert!(full.contains("<html") && full.contains("<p>Hello Engine</p>"));

    let frag = e.render_fragment("pages/test.hrml", &json!({})).unwrap();
    assert!(frag.contains("<p>Hello Engine</p>") && !frag.contains("<html"));
}

#[test]
fn router_from_pages_dir_and_resolve() {
    let dir = tmp_dir("router_pages");
    let pages = dir.join("pages");
    fs::create_dir_all(&pages).unwrap();
    fs::write(pages.join("index.hrml"), "index").unwrap();
    fs::create_dir_all(pages.join("blog")).unwrap();
    fs::write(pages.join("blog").join("[slug].hrml"), "post").unwrap();
    fs::create_dir_all(pages.join("docs")).unwrap();
    fs::write(pages.join("docs").join("[...rest].hrml"), "docs").unwrap();

    let router = Router::from_pages_dir(&pages);
    // should have at least 3 routes
    assert!(router.routes.len() >= 3);

    let res = router.resolve("/").expect("no index route");
    assert_eq!(res.0.path, "/");

    let (_, params) = router.resolve("/blog/hello").expect("no blog route");
    assert_eq!(params.get("slug").unwrap(), "hello");

    let (_, p2) = router
        .resolve("/docs/api/reference")
        .expect("no docs route");
    assert_eq!(p2.get("rest").unwrap(), "api/reference");
}

#[test]
fn ssg_build_generates_files_and_sitemap() {
    let src = tmp_dir("ssg_src");
    let pages = src.join("pages");
    fs::create_dir_all(&pages).unwrap();
    fs::write(pages.join("index.hrml"), "<p>Home</p>").unwrap();
    fs::write(pages.join("about.hrml"), "<p>About</p>").unwrap();

    let out = tmp_dir("ssg_out");

    let ssg = SSG::new(pages.to_str().unwrap(), out.to_str().unwrap()).with_data(json!({}));
    let report = ssg.build().expect("ssg build failed");
    assert!(report.pages.len() >= 2);
    assert!(out.join("sitemap.xml").exists());
}
