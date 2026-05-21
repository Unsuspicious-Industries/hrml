use hrml::template::Engine;
use serde_json::json;
use std::fs;

use std::time::{SystemTime, UNIX_EPOCH};

fn setup_test_templates(test_name: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let test_dir = format!("/tmp/hrml_test_templates_{}_{}", test_name, now);
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(format!("{}/layouts", test_dir)).unwrap();
    fs::create_dir_all(format!("{}/pages", test_dir)).unwrap();
    fs::create_dir_all(format!("{}/components", test_dir)).unwrap();

    test_dir
}

#[test]
fn test_component_loaded_from_file() {
    let test_dir = setup_test_templates("component_loaded");

    // Create a component file - with slot for content
    fs::write(
        format!("{}/components/icon.hrml", test_dir),
        r#"<?component id="icon-cmp"?>
<svg class="icon">icon-<?slot id="content"?><?get id="name"?></?slot?></svg>
<?/component?>"#,
    )
    .unwrap();

    // Page that loads the component - using slot pattern like the working test
    fs::write(
        format!("{}/pages/test.hrml", test_dir),
        r#"<?load file="components/icon.hrml"?>

<div>Loaded with slot:</div>
<?bind var="name" value="loaded-value"/?>
<?use id="icon-cmp"?>
<?block slot="content"?>
<span><?get id="name"?></span>
</?block?>
</?use?>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine.render("pages/test.hrml", &json!({})).unwrap();

    println!("=== RESULT ===\n{}", result);

    // Loaded component should work with slot pattern
    assert!(
        result.contains("loaded-value"),
        "Loaded component should work with bindings"
    );
}
