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
    fs::create_dir_all(format!("{}/content", test_dir)).unwrap();

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
    assert!(
        !result.contains("<?slot"),
        "Output contains unprocessed slot tag"
    );
    assert!(
        !result.contains("<?block"),
        "Output contains unprocessed block tag"
    );
    assert!(
        !result.contains("</?slot?>"),
        "Output contains closing slot tag"
    );
    assert!(
        !result.contains("</?block?>"),
        "Output contains closing block tag"
    );

    // Should contain the custom content from the block
    assert!(
        result.contains("<h1>Test Page</h1>"),
        "Missing custom content"
    );
    assert!(
        result.contains("<p>Custom content</p>"),
        "Missing custom content"
    );

    // Should NOT contain the default slot content
    assert!(
        !result.contains("Default content"),
        "Contains default content when it should be replaced"
    );

    // Should have the layout wrapper
    assert!(
        result.contains("<div class=\"layout\">"),
        "Missing layout wrapper"
    );
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
    assert!(
        !result.contains("<?btn"),
        "Output contains unprocessed btn tag"
    );
    assert!(result.contains("<button"), "Missing button element");
    assert!(
        result.contains("data-post=\"/api/increment\""),
        "Missing data-post attribute"
    );
    assert!(
        result.contains("data-target=\"#counter\""),
        "Missing data-target attribute"
    );
    assert!(
        result.contains("data-swap=\"innerHTML\""),
        "Missing data-swap attribute"
    );
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
    assert!(
        !result.contains("<?form"),
        "Output contains unprocessed form tag"
    );
    assert!(result.contains("<form"), "Missing form element");
    assert!(
        result.contains("data-post=\"/api/submit\""),
        "Missing data-post attribute"
    );
    assert!(
        result.contains("data-target=\"#result\""),
        "Missing data-target attribute"
    );
    assert!(
        result.contains("<input type=\"text\" name=\"name\">"),
        "Missing form input"
    );
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
    assert!(
        !result.contains("<?set"),
        "Output contains unprocessed set tag"
    );
    assert!(
        !result.contains("<?get"),
        "Output contains unprocessed get tag"
    );

    // Should have the replaced value
    assert!(
        result.contains("<p>Hello, Alice!</p>"),
        "Variable not replaced correctly"
    );
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
    assert!(
        result.contains("<h1>Nested Page</h1>"),
        "Missing nested content"
    );

    // Should NOT have unprocessed tags
    assert!(!result.contains("<?load"), "Unprocessed load tag");
    assert!(!result.contains("<?slot"), "Unprocessed slot tag");
    assert!(!result.contains("<?block"), "Unprocessed block tag");
}

#[test]
fn test_no_double_html_wrap_when_layout_is_full_document() {
    let test_dir = setup_test_templates("full_doc");
    fs::write(
        format!("{}/layouts/doc.hrml", test_dir),
        r#"<!DOCTYPE html>
<html lang="en">
<head><title><?get id="site_name"?></title></head>
<body>
<?slot id="content"?></?slot?>
</body>
</html>"#,
    )
    .unwrap();

    fs::write(
        format!("{}/pages/full_doc_page.hrml", test_dir),
        r#"<?load file="layouts/doc.hrml"?>
<?block slot="content"?><p>Hello</p></?block?>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir).with_site_name("My Site".to_string());
    let result = engine
        .render("pages/full_doc_page.hrml", &json!({}))
        .unwrap();

    assert_eq!(result.matches("<!DOCTYPE html>").count(), 1);
    assert_eq!(result.matches("<html").count(), 1);
    assert!(result.contains("<title>My Site</title>"));
}

#[test]
fn test_site_vars_available_in_context() {
    let test_dir = setup_test_templates("site_vars");
    fs::write(
        format!("{}/pages/site_vars.hrml", test_dir),
        r#"<h1><?get id="site_name"?></h1>
<p><?get id="site_description"?></p>
<img src="<?get id="favicon"?>">"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir)
        .with_site_name("Acme".to_string())
        .with_description(Some("Desc".to_string()))
        .with_favicon(Some("/icon.ico".to_string()));

    let result = engine.render("pages/site_vars.hrml", &json!({})).unwrap();
    assert!(result.contains("<h1>Acme</h1>"));
    assert!(result.contains("<p>Desc</p>"));
    assert!(result.contains("<img src=\"/icon.ico\">"));
}

#[test]
fn test_globals_available_in_context() {
    let test_dir = setup_test_templates("globals_vars");
    fs::write(
        format!("{}/pages/globals.hrml", test_dir),
        r#"<p><?get id="globals.colors.accent"?></p>
<p><?get id="globals.fonts.heading"?></p>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir).with_globals(json!({
        "colors": { "accent": "#0ea5e9" },
        "fonts": { "heading": "Cormorant Garamond" }
    }));

    let result = engine.render("pages/globals.hrml", &json!({})).unwrap();
    assert!(result.contains("<p>#0ea5e9</p>"));
    assert!(result.contains("<p>Cormorant Garamond</p>"));
}

#[test]
fn test_wasm_directive_renders_window_attributes() {
    let test_dir = setup_test_templates("wasm");
    fs::write(
        format!("{}/pages/wasm.hrml", test_dir),
        r#"<?wasm module="/static/js/app.mjs" export="mount" props='{"mode":"3d"}'?>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine.render("pages/wasm.hrml", &json!({})).unwrap();
    assert!(result.contains("data-wasm-module=\"/static/js/app.mjs\""));
    assert!(result.contains("data-wasm-export=\"mount\""));
    assert!(result.contains("data-wasm-props=\"{"));
    assert!(result.contains("class=\"hrml-wasm-window\""));
}

#[test]
fn test_component_composition_with_bindings() {
    let test_dir = setup_test_templates("component_composition");
    fs::write(
        format!("{}/pages/component_composition.hrml", test_dir),
        r#"<?component id="shell"?><section><?slot id="body"?></?slot?></section></?component?>
<?component id="card"?><article><?slot id="body"?></?slot?></article></?component?>
<?bind var="title" value="Composed"?>
<?use id="shell"?><?block slot="body"?><?use id="card"?><?block slot="body"?><h2><?get id="title"?></h2></?block?></?use?></?block?></?use?>
</?bind?>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine
        .render("pages/component_composition.hrml", &json!({}))
        .unwrap();
    assert!(result.contains("<section><article><h2>Composed</h2></article></section>"));
}

#[test]
fn test_component_use_and_bind_and_for() {
    let test_dir = setup_test_templates("component_use");
    fs::write(
        format!("{}/pages/component_use.hrml", test_dir),
        r#"<?component id="card"?>
<article class="card"><?slot id="content"?><p>Default</p></?slot?></article>
</?component?>

<?bind var="title" value="Hello"?>
<?use id="card"?>
<?block slot="content"?><h2><?get id="title"?></h2></?block?>
</?use?>
</?bind?>

<ul>
<?for in="item in data.items"?>
  <li><?get id="item.name"?></li>
</?for?>
</ul>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine
        .render(
            "pages/component_use.hrml",
            &json!({
                "data": {
                    "items": [
                        {"name": "One"},
                        {"name": "Two"}
                    ]
                }
            }),
        )
        .unwrap();

    assert!(result.contains("<article class=\"card\"><h2>Hello</h2></article>"));
    assert!(result.contains("<li>One</li>"));
    assert!(result.contains("<li>Two</li>"));
}

#[test]
fn test_markdown_and_latex_directives() {
    let test_dir = setup_test_templates("features");
    fs::write(
        format!("{}/content/post.md", test_dir),
        "# Hello\n\nThis has $a+b$ and $$c=d$$.",
    )
    .unwrap();
    fs::write(
        format!("{}/pages/features.hrml", test_dir),
        r#"<?markdown file="content/post.md"?>
<?latex mode="block" formula="\int_0^1 x^2 dx"?>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine.render("pages/features.hrml", &json!({})).unwrap();
    assert!(result.contains("<h1>Hello</h1>"));
    assert!(result.contains("math-inline"));
    assert!(result.contains("math-block"));
    assert!(result.contains("\\[\\int_0^1 x^2 dx\\]"));
}

#[test]
fn test_markdown_frontmatter_and_meta_tag_library() {
    let test_dir = setup_test_templates("taglib_meta");
    fs::write(
        format!("{}/content/post_fm.md", test_dir),
        "---\ntitle = \"Frontmatter Title\"\ndescription = \"Frontmatter Desc\"\n---\n\n# Hello FM\n",
    )
    .unwrap();
    fs::write(
        format!("{}/pages/taglib.hrml", test_dir),
        r#"<?markdownfm file="content/post_fm.md" as="post"?>
<?title value="$post.title"?>
<?charset?>
<?viewport?>
<?description content="$post.description"?>
<?robots content="noindex,nofollow"?>
<?canonical href="https://example.test/post"?>
<?stylesheet href="/assets/site.css"?>
<?script src="/assets/app.js" defer?>
<?og name="title" content="$post.title"?>
<?twitter name="title" content="$post.title"?>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine.render("pages/taglib.hrml", &json!({})).unwrap();

    assert!(result.contains("<h1>Hello FM</h1>"));
    assert!(result.contains("<title>Frontmatter Title</title>"));
    assert!(result.contains("<meta charset=\"UTF-8\">"));
    assert!(result.contains("name=\"viewport\""));
    assert!(
        result.contains("name=\"description\"") && result.contains("content=\"Frontmatter Desc\"")
    );
    assert!(result.contains("name=\"robots\"") && result.contains("content=\"noindex,nofollow\""));
    assert!(
        result.contains("rel=\"canonical\"")
            && result.contains("href=\"https://example.test/post\"")
    );
    assert!(result.contains("rel=\"stylesheet\"") && result.contains("href=\"/assets/site.css\""));
    assert!(
        result.contains("<script")
            && result.contains("src=\"/assets/app.js\"")
            && result.contains("defer=\"defer\"")
            && result.contains("</script>")
    );
    assert!(
        result.contains("property=\"og:title\"")
            && result.contains("content=\"Frontmatter Title\"")
    );
    assert!(
        result.contains("name=\"twitter:title\"")
            && result.contains("content=\"Frontmatter Title\"")
    );
}

#[test]
fn test_unmatched_slot_keeps_default_content() {
    let test_dir = setup_test_templates("slot_default");
    fs::write(
        format!("{}/pages/default_slot.hrml", test_dir),
        r#"<?load file="layouts/base.hrml"?>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine
        .render("pages/default_slot.hrml", &json!({}))
        .unwrap();

    assert!(result.contains("Default content"));
    assert!(!result.contains("<?slot"));
}

#[test]
fn test_slot_replacement_with_large_block_content() {
    let test_dir = setup_test_templates("large_block");
    // Build block content > 300 chars (the old search limit)
    let long_text = "Large block content. ".repeat(20);
    assert!(
        long_text.len() > 300,
        "Test requires block content longer than 300 chars, got {}",
        long_text.len()
    );

    fs::write(
        format!("{}/layouts/doc.hrml", test_dir),
        r#"<!DOCTYPE html>
<html><head><title>Test</title></head>
<body>
<main>
<?slot id="content"?>
</main>
</body>
</html>"#,
    )
    .unwrap();

    let page_content = format!(
        r#"<?load file="layouts/doc.hrml"?>
<?block slot="content"?>
    <p>{}</p>
</?block?>"#,
        long_text
    );
    fs::write(format!("{}/pages/large.hrml", test_dir), &page_content).unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine.render("pages/large.hrml", &json!({})).unwrap();

    // No unprocessed tags
    assert!(!result.contains("<?load"));
    assert!(!result.contains("<?slot"));
    assert!(!result.contains("<?block"));
    assert!(!result.contains("</?block?>"));

    // Content appears inside the layout, not after </html>
    assert!(result.contains("<main>"));
    assert!(result.contains("</main>"));
    let main_start = result.find("<main>").unwrap();
    let main_end = result.find("</main>").unwrap();
    let main_content = &result[main_start..main_end];
    assert!(
        main_content.contains("Large block content."),
        "Block content should be inside <main>, got: {}..{}",
        main_start,
        main_end
    );

    // Content should NOT appear after </html>
    let html_end = result.find("</html>").unwrap();
    let after_html = &result[html_end..];
    assert!(
        !after_html.contains("Large block content."),
        "Block content found after </html>"
    );
}

#[test]
fn test_render_fragment_does_not_wrap_html_document() {
    let test_dir = setup_test_templates("fragment");
    fs::write(
        format!("{}/pages/fragment.hrml", test_dir),
        r#"<section><h1>Fragment</h1></section>"#,
    )
    .unwrap();

    let engine = Engine::new(&test_dir);
    let result = engine
        .render_fragment("pages/fragment.hrml", &json!({}))
        .unwrap();

    assert_eq!(result.trim(), "<section><h1>Fragment</h1></section>");
    assert!(!result.contains("<!DOCTYPE html>"));
}
