use hrml::config::Config;
use hrml::template::Engine;
use serde_json::{json, Value};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(prefix: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = format!("/tmp/hrml_config_editor_tests_{}_{}", prefix, now);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn config_loads_nested_site_and_globals_tables() {
    let dir = temp_dir("config");
    let path = format!("{}/hrml.toml", dir);
    fs::write(
        &path,
        r#"[site]
name = "Unsuspicious Industries"
description = "Research lab"
favicon = "/icon.png"
url = "https://unsuspicious.org"

[globals.colors]
accent = "red"

[globals.background]
variant = "constellation"
gap = 96
"#,
    )
    .unwrap();

    let config = Config::from_toml(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(config.site_name, "Unsuspicious Industries");
    assert_eq!(config.site_description.as_deref(), Some("Research lab"));
    assert_eq!(config.favicon.as_deref(), Some("/icon.png"));
    assert_eq!(config.site_url.as_deref(), Some("https://unsuspicious.org"));
    assert_eq!(config.globals["colors"]["accent"], json!("red"));
    assert_eq!(
        config.globals["background"]["variant"],
        json!("constellation")
    );
    assert_eq!(config.globals["background"]["gap"], json!(96));
}

#[test]
fn template_context_exposes_site_url_and_nested_globals() {
    let dir = temp_dir("template_context");
    fs::create_dir_all(format!("{}/pages", dir)).unwrap();
    fs::write(
        format!("{}/pages/index.hrml", dir),
        r#"<a href="<?get id="site_url"?>/docs">Docs</a>
<p><?get id="globals.background.variant"?></p>
<p><?get id="globals.background.gap"?></p>"#,
    )
    .unwrap();

    let engine = Engine::new(&dir)
        .with_site_url(Some("https://example.test".to_string()))
        .with_globals(json!({
            "background": {
                "variant": "constellation",
                "gap": 88
            }
        }));

    let html = engine
        .render_fragment("pages/index.hrml", &json!({}))
        .unwrap();
    assert!(html.contains("<a href=\"https://example.test/docs\">Docs</a>"));
    assert!(html.contains("<p>constellation</p>"));
    assert!(html.contains("<p>88</p>"));
}

#[test]
fn template_wasm_props_support_reusable_background_variants() {
    let dir = temp_dir("wasm_variants");
    fs::create_dir_all(format!("{}/pages", dir)).unwrap();
    fs::write(
        format!("{}/pages/index.hrml", dir),
        r#"<?wasm module="/assets/js/geogrid.mjs" export="mount" props='{"variant":"constellation","gap":92}'?>"#,
    )
    .unwrap();

    let engine = Engine::new(&dir);
    let html = engine
        .render_fragment("pages/index.hrml", &json!({}))
        .unwrap();
    assert!(html.contains("data-wasm-module=\"/assets/js/geogrid.mjs\""));
    assert!(html.contains("&quot;variant&quot;:&quot;constellation&quot;"));
    assert!(html.contains("&quot;gap&quot;:92"));
}

#[test]
fn vscode_extension_contributes_html_injection_grammar() {
    let package: Value =
        serde_json::from_str(&fs::read_to_string("vscode-hrml/package.json").unwrap()).unwrap();
    let grammars = package["contributes"]["grammars"].as_array().unwrap();

    assert!(grammars.iter().any(|grammar| {
        grammar["scopeName"] == json!("text.html.hrml.injection")
            && grammar["path"] == json!("./syntaxes/hrml-html-injection.tmLanguage.json")
    }));
}

#[test]
fn vscode_html_injection_targets_standard_html_scope() {
    let grammar: Value = serde_json::from_str(
        &fs::read_to_string("vscode-hrml/syntaxes/hrml-html-injection.tmLanguage.json").unwrap(),
    )
    .unwrap();

    assert_eq!(grammar["scopeName"], json!("text.html.hrml.injection"));
    assert_eq!(grammar["injectionSelector"], json!("L:text.html.basic"));
    assert!(grammar["patterns"].as_array().unwrap().len() >= 2);
}
