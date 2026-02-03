use hrml::template::Engine;
use serde_json::json;
use std::fs;
use std::path::Path;

use std::time::{SystemTime, UNIX_EPOCH};

fn setup_test_templates(test_name: &str) -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let test_dir = format!("/tmp/hrml_test_templates_{}_{}", test_name, now);
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(format!("{}/layouts", test_dir)).unwrap();
    fs::create_dir_all(format!("{}/pages", test_dir)).unwrap();
    fs::create_dir_all(format!("{}/components", test_dir)).unwrap();

    // Base layout with slot
    fs::write(
        format!("{}/layouts/base.hrml", test_dir),
        r#"<div class="layout">
<?slot id="content"?>
    <p>Default content</p>
</?slot?>
</div>"#,
    )
    .unwrap();

    // Page with block
    fs::write(
        format!("{}/pages/test.hrml", test_dir),
        r#"<?load file="layouts/base.hrml"?>

<?block slot="content"?>
    <h1>Test Page</h1>
    <p>Custom content</p>
</?block?>"#,
    )
    .unwrap();

    // Component with nav
    fs::write(
        format!("{}/components/nav.hrml", test_dir),
        r#"<nav>
    <a href="/">Home</a>
    <a href="/about">About</a>
</nav>"#,
    )
    .unwrap();

    test_dir
}

#[test]
fn test_slot_and_block_replacement() {
    let test_dir = setup_test_templates("slot_block");
    let engine = Engine::new(&test_dir);

    let result = engine.render("pages/test.hrml", &json!({})).unwrap();
    
    // Should not contain slot or block tags
    assert!(!result.contains("<?slot"), "Output contains unprocessed slot tag");
    assert!(!result.contains("<?block"), "Output contains unprocessed block tag");
    assert!(!result.contains("</?slot?>"), "Output contains closing slot tag");
    assert!(!result.contains("</?block?>"), "Output contains closing block tag");
    
    // Should contain the custom content from the block
    assert!(result.contains("<h1>Test Page</h1>"), "Missing custom content");
    assert!(result.contains("<p>Custom content</p>"), "Missing custom content");
    
    // Should NOT contain the default slot content
    assert!(!result.contains("Default content"), "Contains default content when it should be replaced");
    
    // Should have the layout wrapper
    assert!(result.contains("<div class=\"layout\">"), "Missing layout wrapper");
}

#[test]
fn test_btn_processing() {
    let test_dir = setup_test_templates("btn");
    fs::write(
        format!("{}/pages/button_test.hrml", test_dir),
        r##"<?btn post="/api/increment" target="#counter" swap="innerHTML"?>
    Click Me
</?btn?>"##,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine.render("pages/button_test.hrml", &json!({})).unwrap();
    
    println!("Button test result:\n{}", result);
    
    // Should convert to proper button HTML
    assert!(!result.contains("<?btn"), "Output contains unprocessed btn tag");
    assert!(result.contains("<button"), "Missing button element");
    assert!(result.contains("data-post=\"/api/increment\""), "Missing data-post attribute");
    assert!(result.contains("data-target=\"#counter\""), "Missing data-target attribute");
    assert!(result.contains("data-swap=\"innerHTML\""), "Missing data-swap attribute");
    assert!(result.contains("Click Me"), "Missing button text");
}

#[test]
fn test_form_processing() {
    let test_dir = setup_test_templates("form");
    fs::write(
        format!("{}/pages/form_test.hrml", test_dir),
        r##"<?form post="/api/submit" target="#result"?>
    <input type="text" name="name">
    <button type="submit">Submit</button>
</?form?>"##,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine.render("pages/form_test.hrml", &json!({})).unwrap();
    
    println!("Form test result:\n{}", result);
    
    // Should convert to proper form HTML
    assert!(!result.contains("<?form"), "Output contains unprocessed form tag");
    assert!(result.contains("<form"), "Missing form element");
    assert!(result.contains("data-post=\"/api/submit\""), "Missing data-post attribute");
    assert!(result.contains("data-target=\"#result\""), "Missing data-target attribute");
    assert!(result.contains("<input type=\"text\" name=\"name\">"), "Missing form input");
}

#[test]
fn test_set_and_get() {
    let test_dir = setup_test_templates("var");
    fs::write(
        format!("{}/pages/var_test.hrml", test_dir),
        r#"<?set id="username"?>Alice</?set?>
<p>Hello, <?get id="username"?>!</p>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine.render("pages/var_test.hrml", &json!({})).unwrap();
    
    // Should not contain set/get tags
    assert!(!result.contains("<?set"), "Output contains unprocessed set tag");
    assert!(!result.contains("<?get"), "Output contains unprocessed get tag");
    
    // Should have the replaced value
    assert!(result.contains("<p>Hello, Alice!</p>"), "Variable not replaced correctly");
}

#[test]
fn test_nested_load() {
    let test_dir = setup_test_templates("nested");
    
    fs::write(
        format!("{}/layouts/with_nav.hrml", test_dir),
        r#"<?load file="components/nav.hrml"?>
<main>
<?slot id="content"?>
    <p>Default</p>
</?slot?>
</main>"#,
    )
    .unwrap();
    
    fs::write(
        format!("{}/pages/nested.hrml", test_dir),
        r#"<?load file="layouts/with_nav.hrml"?>

<?block slot="content"?>
    <h1>Nested Page</h1>
</?block?>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine.render("pages/nested.hrml", &json!({})).unwrap();
    
    println!("Nested test result:\n{}", result);
    
    // Should have nav from component
    assert!(result.contains("<nav>"), "Missing nav from component");
    assert!(result.contains("href=\"/\""), "Missing nav link");
    
    // Should have custom content
    assert!(result.contains("<h1>Nested Page</h1>"), "Missing nested content");
    
    // Should NOT have unprocessed tags
    assert!(!result.contains("<?load"), "Unprocessed load tag");
    assert!(!result.contains("<?slot"), "Unprocessed slot tag");
    assert!(!result.contains("<?block"), "Unprocessed block tag");
}
