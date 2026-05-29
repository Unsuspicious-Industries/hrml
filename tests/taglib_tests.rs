use xrml::template::Engine;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn setup(test_name: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = format!("/tmp/hrml_taglib_tests_{}_{}", test_name, now);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(format!("{}/pages", dir)).unwrap();
    dir
}

#[test]
fn test_meta_tag_library_tags_render() {
    let dir = setup("meta");
    fs::write(
        format!("{}/pages/index.hrml", dir),
        r#"<?set id="page_title" value="USI"?>
<?set id="page_desc" value="Deep Research"?>
<?title value="$page_title"?>
<?meta name="description" content="$page_desc"?>
<?og name="title" content="$page_title"?>
<?twitter name="card" content="summary_large_image"?>
<?linktag rel="canonical" href="https://example.com"?>"#,
    )
    .unwrap();

    let engine = Engine::new(&dir);
    let html = engine.render("pages/index.hrml", &json!({})).unwrap();
    assert!(html.contains("<title>USI</title>"), "{}", html);
    assert!(html.contains("name=\"description\""), "{}", html);
    assert!(html.contains("content=\"Deep Research\""), "{}", html);
    assert!(html.contains("property=\"og:title\""), "{}", html);
    assert!(html.contains("name=\"twitter:card\""), "{}", html);
    assert!(html.contains("rel=\"canonical\""), "{}", html);
    assert!(html.contains("href=\"https://example.com\""), "{}", html);
}

fn custom_stamp(attrs: &BTreeMap<String, String>) -> Result<String, String> {
    let value = attrs.get("value").cloned().unwrap_or_default();
    Ok(format!("<span class=\"stamp\">{}</span>", value))
}

fn custom_wrap(attrs: &BTreeMap<String, String>, inner: &str) -> Result<String, String> {
    let class = attrs
        .get("class")
        .cloned()
        .unwrap_or_else(|| "box".to_string());
    Ok(format!("<div class=\"{}\">{}</div>", class, inner))
}

#[test]
fn test_extendable_tag_registry_custom_tags() {
    let dir = setup("registry");
    fs::write(
        format!("{}/pages/index.hrml", dir),
        r#"<?stamp value="hello"?>
<?wrap class="fancy"?><p>inside</p></?wrap?>"#,
    )
    .unwrap();

    let mut engine = Engine::new(&dir);
    engine.register_void_tag("stamp", custom_stamp);
    engine.register_block_tag("wrap", custom_wrap);

    let html = engine.render("pages/index.hrml", &json!({})).unwrap();
    assert!(
        html.contains("<span class=\"stamp\">hello</span>"),
        "{}",
        html
    );
    assert!(
        html.contains("<div class=\"fancy\"><p>inside</p></div>"),
        "{}",
        html
    );
}
