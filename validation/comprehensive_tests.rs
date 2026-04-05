use hrml::template::Engine;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================
// Test Harness
// ============================================================

struct TestEnv {
    dir: String,
}

impl TestEnv {
    fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = format!("/tmp/hrml_val_{}_{}", name, now);
        let dirs = [
            "layouts",
            "components",
            "pages",
            "content",
            "sections",
            "chrome",
            "partials",
            "static",
        ];
        for d in &dirs {
            let _ = fs::create_dir_all(format!("{}/{}", dir, d));
        }
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

    fn engine_with_config(
        &self,
        site_name: &str,
        description: Option<&str>,
        favicon: Option<&str>,
    ) -> Engine {
        let mut e = Engine::new(&self.dir).with_site_name(site_name.to_string());
        if let Some(d) = description {
            e = e.with_description(Some(d.to_string()));
        }
        if let Some(f) = favicon {
            e = e.with_favicon(Some(f.to_string()));
        }
        e
    }

    fn render(&self, path: &str) -> Result<String, String> {
        self.engine().render(path, &json!({}))
    }

    fn render_with_data(&self, path: &str, data: &serde_json::Value) -> Result<String, String> {
        self.engine().render(path, data)
    }

    fn render_fragment(&self, path: &str) -> Result<String, String> {
        self.engine().render_fragment(path, &json!({}))
    }

    fn render_fragment_with_data(
        &self,
        path: &str,
        data: &serde_json::Value,
    ) -> Result<String, String> {
        self.engine().render_fragment(path, data)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

// ============================================================
// Assertion Helpers
// ============================================================

fn assert_contains(html: &str, needle: &str) {
    assert!(
        html.contains(needle),
        "\nExpected output to contain: {}\n\nActual output:\n{}\n",
        needle,
        html
    );
}

fn assert_not_contains(html: &str, needle: &str) {
    assert!(
        !html.contains(needle),
        "\nExpected output NOT to contain: {}\n\nActual output:\n{}\n",
        needle,
        html
    );
}

// ============================================================
// 1. PARSER TESTS
// ============================================================

mod parser_tests {
    use super::*;

    #[test]
    fn parses_plain_text() {
        let env = TestEnv::new("plain_text");
        env.write("pages/test.hrml", "Hello World");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Hello World");
    }

    #[test]
    fn parses_plain_html() {
        let env = TestEnv::new("plain_html");
        env.write("pages/test.hrml", "<div><p>Hello</p></div>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<div><p>Hello</p></div>");
    }

    #[test]
    fn parses_void_directive() {
        let env = TestEnv::new("void_directive");
        env.write("pages/test.hrml", r#"<?pure value="hello"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "hello");
    }

    #[test]
    fn circular_load_detection() {
        let env = TestEnv::new("circular_load");
        env.write("pages/a.hrml", r#"<?load file="pages/b.hrml"?>A"#);
        env.write("pages/b.hrml", r#"<?load file="pages/a.hrml"?>B"#);
        let result = env.render("pages/a.hrml");
        assert!(result.is_err());
        assert_contains(&result.unwrap_err(), "Circular");
    }

    #[test]
    fn parses_nested_elements() {
        let env = TestEnv::new("nested_elements");
        env.write(
            "pages/test.hrml",
            r#"<div><span><a href="/">link</a></span></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<div><span><a href=\"/\">link</a></span></div>");
    }

    #[test]
    fn parses_mixed_text_and_elements() {
        let env = TestEnv::new("mixed_text_elements");
        env.write(
            "pages/test.hrml",
            "Hello <strong>world</strong> and <em>more</em> end",
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Hello <strong>world</strong>");
        assert_contains(&html, "<em>more</em> end");
    }

    #[test]
    fn parses_attributes_double_quoted() {
        let env = TestEnv::new("attrs_double");
        env.write("pages/test.hrml", r#"<?pure value="hello world"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "hello world");
    }

    #[test]
    fn parses_attributes_single_quoted() {
        let env = TestEnv::new("attrs_single");
        env.write("pages/test.hrml", r#"<?pure value='hello world'?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "hello world");
    }

    #[test]
    fn parses_multiple_attributes() {
        let env = TestEnv::new("multi_attrs");
        env.write(
            "pages/test.hrml",
            r##"<?btn post="/api/test" target="#result" swap="innerHTML"?>Click</?btn?>"##,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "data-post=\"/api/test\"");
        assert_contains(&html, "data-target=\"#result\"");
        assert_contains(&html, "data-swap=\"innerHTML\"");
    }

    #[test]
    fn parses_boolean_attributes() {
        let env = TestEnv::new("bool_attrs");
        env.write("pages/test.hrml", r#"<?script src="/app.js" defer?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "defer=\"defer\"");
    }

    #[test]
    fn parses_html_void_elements() {
        let env = TestEnv::new("html_void");
        env.write(
            "pages/test.hrml",
            r#"<img src="/test.jpg"><br><hr><input type="text">"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<img");
        assert_contains(&html, "<br>");
        assert_contains(&html, "<hr>");
        assert_contains(&html, "<input");
    }

    #[test]
    fn parses_special_chars_in_text() {
        let env = TestEnv::new("special_chars");
        env.write("pages/test.hrml", "a < b && c > d & e");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "a < b && c > d & e");
    }

    #[test]
    fn parses_unicode_content() {
        let env = TestEnv::new("unicode");
        env.write("pages/test.hrml", "Hello 世界 Ελληνικά العربية");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Hello 世界");
        assert_contains(&html, "Ελληνικά");
        assert_contains(&html, "العربية");
    }

    #[test]
    fn parses_empty_template() {
        let env = TestEnv::new("empty_template");
        env.write("pages/test.hrml", "");
        let html = env.render("pages/test.hrml").unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn parses_whitespace_only_template() {
        let env = TestEnv::new("whitespace_only");
        env.write("pages/test.hrml", "   \n\n   \t   ");
        let html = env.render("pages/test.hrml").unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn parses_deeply_nested_elements() {
        let env = TestEnv::new("deep_nesting");
        let content = (0..20).fold(String::from("leaf"), |acc, i| {
            format!("<div class=\"level-{}\">{}</div>", i, acc)
        });
        env.write("pages/test.hrml", &content);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "leaf");
        assert_contains(&html, "class=\"level-0\"");
        assert_contains(&html, "class=\"level-19\"");
    }

    #[test]
    fn parses_directive_with_dashes_in_name() {
        let env = TestEnv::new("dash_attrs");
        env.write(
            "pages/test.hrml",
            r#"<div data-foo-bar="baz" data-123="num">test</div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "data-foo-bar=\"baz\"");
        assert_contains(&html, "data-123=\"num\"");
    }

    #[test]
    fn parses_colon_in_attributes() {
        let env = TestEnv::new("colon_attrs");
        env.write(
            "pages/test.hrml",
            r#"<meta property="og:title" content="Test">"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "property=\"og:title\"");
    }

    #[test]
    fn parses_dot_in_directive_names() {
        let env = TestEnv::new("dot_names");
        env.write("pages/test.hrml", r#"<?load file="components/nav.hrml"?>"#);
        env.write("components/nav.hrml", "<nav>nav</nav>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<nav>nav</nav>");
    }

    #[test]
    fn parser_infinite_loop_guard() {
        let env = TestEnv::new("loop_guard");
        // Create deeply nested elements that trigger the iteration counter
        let content = (0..5000)
            .map(|i| format!("<?if cond=\"x\"?>a<?else?>b</?if?>"))
            .collect::<Vec<_>>()
            .join("");
        env.write("pages/test.hrml", &content);
        let result = env.render("pages/test.hrml");
        // Should either succeed (rendering all the if/else) or fail with loop guard
        // The important thing is it doesn't hang
        assert!(result.is_ok() || result.as_ref().unwrap_err().contains("infinite loop"));
    }
}

// ============================================================
// 2. LOAD DIRECTIVE TESTS
// ============================================================

mod load_tests {
    use super::*;

    #[test]
    fn loads_simple_component() {
        let env = TestEnv::new("load_simple");
        env.write(
            "pages/test.hrml",
            r#"<?load file="components/header.hrml"?>"#,
        );
        env.write("components/header.hrml", "<header>Header</header>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<header>Header</header>");
    }

    #[test]
    fn loads_layout_with_slots() {
        let env = TestEnv::new("load_layout");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/base.hrml"?>
<?block slot="content"?><h1>Page</h1></?block?>"#,
        );
        env.write(
            "layouts/base.hrml",
            r#"<div class="layout"><?slot id="content"?><p>default</p></?slot?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<h1>Page</h1>");
        assert_not_contains(&html, "default");
        assert_contains(&html, "class=\"layout\"");
    }

    #[test]
    fn loads_nested_templates() {
        let env = TestEnv::new("load_nested");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/wrapper.hrml"?>
<?block slot="main"?><p>inner</p></?block?>"#,
        );
        env.write(
            "layouts/wrapper.hrml",
            r#"<?load file="components/nav.hrml"?>
<main><?slot id="main"?></?slot?></main>"#,
        );
        env.write("components/nav.hrml", "<nav>Nav</nav>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<nav>Nav</nav>");
        assert_contains(&html, "<p>inner</p>");
    }

    #[test]
    fn loads_multiple_components() {
        let env = TestEnv::new("load_multiple");
        env.write(
            "pages/test.hrml",
            r#"<?load file="components/header.hrml"?>
<?load file="components/footer.hrml"?>"#,
        );
        env.write("components/header.hrml", "<header>H</header>");
        env.write("components/footer.hrml", "<footer>F</footer>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<header>H</header>");
        assert_contains(&html, "<footer>F</footer>");
    }

    #[test]
    fn load_missing_file_returns_error() {
        let env = TestEnv::new("load_missing");
        env.write("pages/test.hrml", r#"<?load file="nonexistent.hrml"?>"#);
        let result = env.render("pages/test.hrml");
        assert!(result.is_err());
    }

    #[test]
    fn load_from_subdirectory() {
        let env = TestEnv::new("load_subdir");
        env.write("pages/test.hrml", r#"<?load file="chrome/header.hrml"?>"#);
        env.write("chrome/header.hrml", "<header>Chrome Header</header>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<header>Chrome Header</header>");
    }

    #[test]
    fn load_deep_chain() {
        let env = TestEnv::new("load_deep_chain");
        env.write("pages/test.hrml", r#"<?load file="partials/a.hrml"?>"#);
        env.write("partials/a.hrml", r#"<?load file="partials/b.hrml"?>A"#);
        env.write("partials/b.hrml", r#"<?load file="partials/c.hrml"?>B"#);
        env.write("partials/c.hrml", "C");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "A");
        assert_contains(&html, "B");
        assert_contains(&html, "C");
    }

    #[test]
    fn circular_load_detection() {
        let env = TestEnv::new("circular_load");
        env.write("a.hrml", r#"<?load file="b.hrml"?>A"#);
        env.write("b.hrml", r#"<?load file="a.hrml"?>B"#);
        let result = env.render("a.hrml");
        assert!(result.is_err());
        assert_contains(&result.unwrap_err(), "Circular");
    }

    #[test]
    fn load_with_block_injection() {
        let env = TestEnv::new("load_block_inject");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/card.hrml"?>
<?block slot="title"?><h2>Card Title</h2></?block?>
<?block slot="body"?><p>Card Body</p></?block?>"#,
        );
        env.write(
            "layouts/card.hrml",
            r#"<article class="card">
<h1><?slot id="title"?></h1>
<div><?slot id="body"?></div>
</article>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<article class=\"card\">");
        assert_contains(&html, "<h2>Card Title</h2>");
        assert_contains(&html, "<p>Card Body</p>");
    }

    #[test]
    fn load_preserves_block_content_with_embedded_load() {
        let env = TestEnv::new("load_block_embedded");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/page.hrml"?>
<?block slot="content"?><?load file="sections/hero.hrml"?></?block?>"#,
        );
        env.write(
            "layouts/page.hrml",
            r#"<html><body><?slot id="content"?></body></html>"#,
        );
        env.write(
            "sections/hero.hrml",
            "<section class=\"hero\">Hero</section>",
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<section class=\"hero\">Hero</section>");
    }
}

// ============================================================
// 3. SLOT AND BLOCK TESTS
// ============================================================

mod slot_block_tests {
    use super::*;

    #[test]
    fn slot_with_default_content() {
        let env = TestEnv::new("slot_default");
        env.write(
            "pages/test.hrml",
            r#"<div><?slot id="content"?>Default Content</?slot?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Default Content");
    }

    #[test]
    fn block_replaces_slot() {
        let env = TestEnv::new("block_replaces");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/base.hrml"?>
<?block slot="content"?>Custom</?block?>"#,
        );
        env.write(
            "layouts/base.hrml",
            r#"<div><?slot id="content"?>Default</?slot?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Custom");
        assert_not_contains(&html, "Default");
    }

    #[test]
    fn multiple_slots_and_blocks() {
        let env = TestEnv::new("multi_slots");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/two_col.hrml"?>
<?block slot="left"?>Left Panel</?block?>
<?block slot="right"?>Right Panel</?block?>"#,
        );
        env.write(
            "layouts/two_col.hrml",
            r#"<div class="grid">
<aside><?slot id="left"?></aside>
<main><?slot id="right"?></main>
</div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Left Panel");
        assert_contains(&html, "Right Panel");
        assert_contains(&html, "class=\"grid\"");
    }

    #[test]
    fn nested_slots() {
        let env = TestEnv::new("nested_slots");
        env.write(
            "pages/test.hrml",
            r#"<outer><?slot id="outer"?>
  <inner><?slot id="inner"?>Deep Default</?slot?></inner>
</?slot?></outer>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Deep Default");
    }

    #[test]
    fn block_with_nested_elements() {
        let env = TestEnv::new("block_nested");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/base.hrml"?>
<?block slot="content"?>
<div class="wrapper">
  <h1>Title</h1>
  <p>Description</p>
</div>
</?block?>"#,
        );
        env.write("layouts/base.hrml", r#"<body><?slot id="content"?></body>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<div class=\"wrapper\">");
        assert_contains(&html, "<h1>Title</h1>");
        assert_contains(&html, "<p>Description</p>");
    }

    #[test]
    fn unmatched_slot_uses_default() {
        let env = TestEnv::new("unmatched_slot");
        env.write("pages/test.hrml", r#"<?load file="layouts/base.hrml"?>"#);
        env.write(
            "layouts/base.hrml",
            r#"<div><?slot id="missing"?>Fallback</?slot?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Fallback");
    }

    #[test]
    fn slot_with_html_attributes() {
        let env = TestEnv::new("slot_attrs");
        env.write(
            "pages/test.hrml",
            r##"<?load file="layouts/base.hrml"?>
<?block slot="body_attrs"?>class="dark bg-gray-900"</?block?>"##,
        );
        env.write(
            "layouts/base.hrml",
            r##"<html><body <?slot id="body_attrs"?>class="light"</?slot?>>Content</body></html>"##,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "class=\"dark bg-gray-900\"");
        assert_not_contains(&html, "class=\"light\"");
    }
}

// ============================================================
// 4. SET AND GET TESTS
// ============================================================

mod set_get_tests {
    use super::*;

    #[test]
    fn set_block_and_get() {
        let env = TestEnv::new("set_block");
        env.write(
            "pages/test.hrml",
            r#"<?set id="name"?>Alice</?set?>
<p>Hello <?get id="name"?></p>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>Hello Alice</p>");
    }

    #[test]
    fn set_void_with_value_attr() {
        let env = TestEnv::new("set_void");
        env.write(
            "pages/test.hrml",
            r#"<?set id="title" value="My Page"?>
<h1><?get id="title"?></h1>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<h1>My Page</h1>");
    }

    #[test]
    fn get_undefined_returns_empty() {
        let env = TestEnv::new("get_undefined");
        env.write("pages/test.hrml", r#"<p>[<?get id="nonexistent"?>]</p>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "[]");
    }

    #[test]
    fn set_overwrites_previous() {
        let env = TestEnv::new("set_overwrite");
        env.write(
            "pages/test.hrml",
            r#"<?set id="x"?>first</?set?>
<?set id="x"?>second</?set?>
<?get id="x"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "second");
        assert_not_contains(&html, "first");
    }

    #[test]
    fn set_multiline_content() {
        let env = TestEnv::new("set_multiline");
        env.write(
            "pages/test.hrml",
            r#"<?set id="bio"?>
Line 1
Line 2
Line 3
</?set?>
<div><?get id="bio"?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Line 1");
        assert_contains(&html, "Line 2");
        assert_contains(&html, "Line 3");
    }

    #[test]
    fn set_with_html_content() {
        let env = TestEnv::new("set_html");
        env.write(
            "pages/test.hrml",
            r#"<?set id="card"?>
<article>
  <h2>Title</h2>
  <p>Body</p>
</article>
</?set?>
<section><?get id="card"?></section>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<article>");
        assert_contains(&html, "<h2>Title</h2>");
    }

    #[test]
    fn get_from_json_data() {
        let env = TestEnv::new("get_json");
        env.write(
            "pages/test.hrml",
            r#"<h1><?get id="user.name"?></h1>
<p><?get id="user.email"?></p>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({
                    "user": {
                        "name": "Bob",
                        "email": "bob@test.com"
                    }
                }),
            )
            .unwrap();
        assert_contains(&html, "<h1>Bob</h1>");
        assert_contains(&html, "<p>bob@test.com</p>");
    }

    #[test]
    fn set_takes_precedence_over_data() {
        let env = TestEnv::new("set_precedence");
        env.write(
            "pages/test.hrml",
            r#"<?set id="title"?>Local</?set?>
<h1><?get id="title"?></h1>"#,
        );
        let html = env
            .render_with_data("pages/test.hrml", &json!({ "title": "Remote" }))
            .unwrap();
        assert_contains(&html, "<h1>Local</h1>");
    }
}

// ============================================================
// 5. IF CONDITIONAL TESTS
// ============================================================

mod if_tests {
    use super::*;

    #[test]
    fn if_true_branch() {
        let env = TestEnv::new("if_true");
        env.write(
            "pages/test.hrml",
            r#"<?set id="status"?>active</?set?>
<?if cond="status=='active'"?>
<p>Active</p>
<?else?>
<p>Inactive</p>
</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>Active</p>");
        assert_not_contains(&html, "<p>Inactive</p>");
    }

    #[test]
    fn if_false_branch() {
        let env = TestEnv::new("if_false");
        env.write(
            "pages/test.hrml",
            r#"<?set id="status"?>inactive</?set?>
<?if cond="status=='active'"?>
<p>Active</p>
<?else?>
<p>Inactive</p>
</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>Inactive</p>");
        assert_not_contains(&html, "<p>Active</p>");
    }

    #[test]
    fn if_without_else() {
        let env = TestEnv::new("if_no_else");
        env.write(
            "pages/test.hrml",
            r#"<?set id="show"?>yes</?set?>
<?if cond="show"?>
<p>Visible</p>
</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>Visible</p>");
    }

    #[test]
    fn if_false_without_else() {
        let env = TestEnv::new("if_false_no_else");
        env.write(
            "pages/test.hrml",
            r#"<?if cond="missing"?>
<p>Should not appear</p>
</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "Should not appear");
    }

    #[test]
    fn if_with_html_content() {
        let env = TestEnv::new("if_html");
        env.write(
            "pages/test.hrml",
            r#"<?set id="admin"?>true</?set?>
<?if cond="admin=='true'"?>
<div class="admin-panel">
  <h2>Admin</h2>
  <button>Delete</button>
</div>
<?else?>
<p>Access denied</p>
</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "class=\"admin-panel\"");
        assert_contains(&html, "<h2>Admin</h2>");
        assert_not_contains(&html, "Access denied");
    }

    #[test]
    fn if_from_json_data() {
        let env = TestEnv::new("if_json");
        env.write(
            "pages/test.hrml",
            r#"<?if cond="user.role=='admin'"?>
<p>Admin</p>
<?else?>
<p>User</p>
</?if?>"#,
        );
        let html = env
            .render_with_data("pages/test.hrml", &json!({ "user": { "role": "admin" } }))
            .unwrap();
        assert_contains(&html, "<p>Admin</p>");
    }

    #[test]
    fn nested_if() {
        let env = TestEnv::new("nested_if");
        env.write(
            "pages/test.hrml",
            r#"<?set id="a"?>1</?set?>
<?set id="b"?>2</?set?>
<?if cond="a=='1'"?>
<?if cond="b=='2'"?>
<p>Both true</p>
<?else?>
<p>Only a</p>
</?if?>
<?else?>
<p>Neither</p>
</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>Both true</p>");
    }
}

// ============================================================
// 6. FOR LOOP TESTS
// ============================================================

mod for_tests {
    use super::*;

    #[test]
    fn for_over_array_of_strings() {
        let env = TestEnv::new("for_strings");
        env.write(
            "pages/test.hrml",
            r#"<ul>
<?for in="item in items"?>
<li><?get id="item"?></li>
</?for?>
</ul>"#,
        );
        let html = env
            .render_with_data("pages/test.hrml", &json!({ "items": ["a", "b", "c"] }))
            .unwrap();
        assert_contains(&html, "<li>a</li>");
        assert_contains(&html, "<li>b</li>");
        assert_contains(&html, "<li>c</li>");
    }

    #[test]
    fn for_over_array_of_objects() {
        let env = TestEnv::new("for_objects");
        env.write(
            "pages/test.hrml",
            r#"<ul>
<?for in="user in users"?>
<li><?get id="user.name"?> - <?get id="user.age"?></li>
</?for?>
</ul>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({
                    "users": [
                        { "name": "Alice", "age": 30 },
                        { "name": "Bob", "age": 25 }
                    ]
                }),
            )
            .unwrap();
        assert_contains(&html, "<li>Alice - 30</li>");
        assert_contains(&html, "<li>Bob - 25</li>");
    }

    #[test]
    fn for_empty_array() {
        let env = TestEnv::new("for_empty");
        env.write(
            "pages/test.hrml",
            r#"<?for in="x in items"?>
<p><?get id="x"?></p>
</?for?>"#,
        );
        let html = env
            .render_with_data("pages/test.hrml", &json!({ "items": [] }))
            .unwrap();
        assert_not_contains(&html, "<p>");
    }

    #[test]
    fn for_missing_source() {
        let env = TestEnv::new("for_missing");
        env.write(
            "pages/test.hrml",
            r#"<?for in="x in nonexistent"?>
<p><?get id="x"?></p>
</?for?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<p>");
    }

    #[test]
    fn for_with_html_content() {
        let env = TestEnv::new("for_html");
        env.write(
            "pages/test.hrml",
            r#"<div class="cards">
<?for in="card in cards"?>
<article class="card">
  <h3><?get id="card.title"?></h3>
  <p><?get id="card.body"?></p>
</article>
</?for?>
</div>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({
                    "cards": [
                        { "title": "T1", "body": "B1" },
                        { "title": "T2", "body": "B2" }
                    ]
                }),
            )
            .unwrap();
        assert_contains(&html, "class=\"cards\"");
        assert_contains(&html, "<h3>T1</h3>");
        assert_contains(&html, "<h3>T2</h3>");
    }

    #[test]
    fn for_with_nested_elements() {
        let env = TestEnv::new("for_nested");
        env.write(
            "pages/test.hrml",
            r#"<table>
<?for in="row in rows"?>
<tr>
  <td><?get id="row.id"?></td>
  <td><?get id="row.name"?></td>
</tr>
</?for?>
</table>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({
                    "rows": [
                        { "id": 1, "name": "First" },
                        { "id": 2, "name": "Second" }
                    ]
                }),
            )
            .unwrap();
        assert_contains(&html, "<td>1</td>");
        assert_contains(&html, "<td>First</td>");
        assert_contains(&html, "<td>2</td>");
        assert_contains(&html, "<td>Second</td>");
    }
}

// ============================================================
// 7. COMPONENT AND USE TESTS
// ============================================================

mod component_tests {
    use super::*;

    #[test]
    fn define_and_use_component() {
        let env = TestEnv::new("component_basic");
        env.write(
            "pages/test.hrml",
            r#"<?component id="greeting"?>
<div class="greeting">Hello, <?slot id="name"?>World</?slot?>!</div>
</?component?>

<?use id="greeting"?>
<?block slot="name"?>Alice</?block?>
</?use?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<div class=\"greeting\">Hello, Alice!</div>");
    }

    #[test]
    fn component_with_default_slot() {
        let env = TestEnv::new("component_default");
        env.write(
            "pages/test.hrml",
            r#"<?component id="card"?>
<article><?slot id="content"?>Default</?slot?></article>
</?component?>

<?use id="card"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<article>Default</article>");
    }

    #[test]
    fn component_with_multiple_slots() {
        let env = TestEnv::new("component_multi_slot");
        env.write(
            "pages/test.hrml",
            r#"<?component id="layout"?>
<div class="container">
  <header><?slot id="header"?></header>
  <main><?slot id="main"?></main>
  <footer><?slot id="footer"?></footer>
</div>
</?component?>

<?use id="layout"?>
<?block slot="header"?>Top</?block?>
<?block slot="main"?>Center</?block?>
<?block slot="footer"?>Bottom</?block?>
</?use?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<header>Top</header>");
        assert_contains(&html, "<main>Center</main>");
        assert_contains(&html, "<footer>Bottom</footer>");
    }

    #[test]
    fn use_unknown_component_returns_empty() {
        let env = TestEnv::new("use_unknown");
        env.write("pages/test.hrml", r#"<?use id="nonexistent"?></?use?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?use");
    }

    #[test]
    fn component_inside_load() {
        let env = TestEnv::new("component_in_load");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/base.hrml"?>
<?block slot="content"?>
<?component id="badge"?>
<span class="badge"><?slot id="text"?></span>
</?component?>
<?use id="badge"?>
<?block slot="text"?>New</?block?>
</?use?>
</?block?>"#,
        );
        env.write(
            "layouts/base.hrml",
            r#"<html><body><?slot id="content"?></body></html>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<span class=\"badge\">New</span>");
    }
}

// ============================================================
// 8. BIND TESTS
// ============================================================

mod bind_tests {
    use super::*;

    #[test]
    fn bind_with_value() {
        let env = TestEnv::new("bind_value");
        env.write(
            "pages/test.hrml",
            r#"<?bind var="name" value="Alice"?>
<p>Hello <?get id="name"?></p>
</?bind?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>Hello Alice</p>");
    }

    #[test]
    fn bind_from_context() {
        let env = TestEnv::new("bind_from");
        env.write(
            "pages/test.hrml",
            r#"<?bind var="n" from="user.name"?>
<p><?get id="n"?></p>
</?bind?>"#,
        );
        let html = env
            .render_with_data("pages/test.hrml", &json!({ "user": { "name": "Bob" } }))
            .unwrap();
        assert_contains(&html, "<p>Bob</p>");
    }

    #[test]
    fn bind_scope_is_isolated() {
        let env = TestEnv::new("bind_scope");
        env.write(
            "pages/test.hrml",
            r#"<?set id="x"?>outer</?set?>
<?bind var="x" value="inner"?>
<p><?get id="x"?></p>
</?bind?>
<p><?get id="x"?></p>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        let parts: Vec<&str> = html.split("<p>").collect();
        assert!(parts.iter().any(|p| p.contains("inner")));
        assert!(parts.iter().any(|p| p.contains("outer")));
    }

    #[test]
    fn bind_with_component() {
        let env = TestEnv::new("bind_component");
        env.write(
            "pages/test.hrml",
            r#"<?component id="card"?>
<article><h2><?slot id="title"?></h2></article>
</?component?>

<?bind var="t" value="My Title"?>
<?use id="card"?>
<?block slot="title"?><?get id="t"?></?block?>
</?use?>
</?bind?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<h2>My Title</h2>");
    }
}

// ============================================================
// 9. COMPOSE TESTS
// ============================================================

mod compose_tests {
    use super::*;

    #[test]
    fn compose_sum() {
        let env = TestEnv::new("compose_sum");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?><span>A</span><?then?><span>B</span></?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<span>A</span><span>B</span>");
    }

    #[test]
    fn compose_product() {
        let env = TestEnv::new("compose_product");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="product"?><section>L</section><?then?><section>R</section></?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "class=\"hrml-product\"");
        assert_contains(&html, "<section>L</section>");
        assert_contains(&html, "<section>R</section>");
    }

    #[test]
    fn compose_with_load() {
        let env = TestEnv::new("compose_load");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?>
<?load file="components/header.hrml"?>
<?then?>
<?load file="components/footer.hrml"?>
</?compose?>"#,
        );
        env.write("components/header.hrml", "<header>H</header>");
        env.write("components/footer.hrml", "<footer>F</footer>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<header>H</header>");
        assert_contains(&html, "<footer>F</footer>");
    }

    #[test]
    fn compose_three_segments() {
        let env = TestEnv::new("compose_three");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?><p>1</p><?then?><p>2</p><?then?><p>3</p></?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>1</p>");
        assert_contains(&html, "<p>2</p>");
        assert_contains(&html, "<p>3</p>");
    }
}

// ============================================================
// 10. INTERACTIVE DIRECTIVES (btn, link, form)
// ============================================================

mod interactive_tests {
    use super::*;

    #[test]
    fn btn_post() {
        let env = TestEnv::new("btn_post");
        env.write(
            "pages/test.hrml",
            r##"<?btn post="/api/action" target="#result" swap="innerHTML"?>
Click Me
</?btn?>"##,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<button");
        assert_contains(&html, "data-post=\"/api/action\"");
        assert_contains(&html, "data-target=\"#result\"");
        assert_contains(&html, "data-swap=\"innerHTML\"");
        assert_contains(&html, "Click Me");
    }

    #[test]
    fn btn_get() {
        let env = TestEnv::new("btn_get");
        env.write(
            "pages/test.hrml",
            r##"<?btn get="/api/fetch" target="#out"?>
Load
</?btn?>"##,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "data-get=\"/api/fetch\"");
        assert_contains(&html, "data-target=\"#out\"");
    }

    #[test]
    fn btn_default_target() {
        let env = TestEnv::new("btn_default");
        env.write("pages/test.hrml", r#"<?btn post="/api/go"?>Go</?btn?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "data-target=\"#body\"");
        assert_contains(&html, "data-swap=\"innerHTML\"");
    }

    #[test]
    fn link_directive() {
        let env = TestEnv::new("link_directive");
        env.write(
            "pages/test.hrml",
            r##"<?link get="/api/page" target="#content" swap="innerHTML"?>
Next Page
</?link?>"##,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<a href=\"#\"");
        assert_contains(&html, "data-get=\"/api/page\"");
        assert_contains(&html, "data-target=\"#content\"");
        assert_contains(&html, "Next Page");
    }

    #[test]
    fn form_directive() {
        let env = TestEnv::new("form_directive");
        env.write(
            "pages/test.hrml",
            r##"<?form post="/api/submit" target="#result" swap="innerHTML"?>
<input type="text" name="email">
<button type="submit">Submit</button>
</?form?>"##,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<form");
        assert_contains(&html, "data-post=\"/api/submit\"");
        assert_contains(&html, "data-target=\"#result\"");
        assert_contains(&html, "data-swap=\"innerHTML\"");
        assert_contains(&html, "<input type=\"text\" name=\"email\">");
    }

    #[test]
    fn btn_with_nested_html() {
        let env = TestEnv::new("btn_nested");
        env.write(
            "pages/test.hrml",
            r##"<?btn post="/api/delete" target="#list"?>
<span class="icon">X</span> Delete
</?btn?>"##,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<span class=\"icon\">X</span> Delete");
    }
}

// ============================================================
// 11. WASM DIRECTIVE TESTS
// ============================================================

mod wasm_tests {
    use super::*;

    #[test]
    fn wasm_basic() {
        let env = TestEnv::new("wasm_basic");
        env.write(
            "pages/test.hrml",
            r#"<?wasm module="/static/app.wasm" export="mount"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "data-wasm-module=\"/static/app.wasm\"");
        assert_contains(&html, "data-wasm-export=\"mount\"");
        assert_contains(&html, "class=\"hrml-wasm-window\"");
    }

    #[test]
    fn wasm_with_props() {
        let env = TestEnv::new("wasm_props");
        env.write(
            "pages/test.hrml",
            r#"<?wasm module="/app.wasm" export="init" props='{"mode":"3d"}'?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "data-wasm-props='");
        assert_contains(&html, "mode");
    }

    #[test]
    fn wasm_with_src() {
        let env = TestEnv::new("wasm_src");
        env.write(
            "pages/test.hrml",
            r#"<?wasm src="/static/app.wasm" module="app" export="mount" start="true"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "data-wasm-src=\"/static/app.wasm\"");
        assert_contains(&html, "data-wasm-start=\"true\"");
    }

    #[test]
    fn wasm_custom_class() {
        let env = TestEnv::new("wasm_class");
        env.write(
            "pages/test.hrml",
            r#"<?wasm module="/app.wasm" class="my-wasm-container"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "class=\"my-wasm-container\"");
    }

    #[test]
    fn wasm_defaults() {
        let env = TestEnv::new("wasm_defaults");
        env.write("pages/test.hrml", r#"<?wasm module="/app.wasm"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "data-wasm-export=\"mount\"");
        assert_contains(&html, "data-wasm-start=\"\"");
        assert_contains(&html, "data-wasm-props='{}'");
    }
}

// ============================================================
// 12. PURE DIRECTIVE TESTS
// ============================================================

mod pure_tests {
    use super::*;

    #[test]
    fn pure_value() {
        let env = TestEnv::new("pure_value");
        env.write("pages/test.hrml", r#"<?pure value="raw output"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "raw output");
    }

    #[test]
    fn pure_empty() {
        let env = TestEnv::new("pure_empty");
        env.write("pages/test.hrml", r#"<?pure value=""?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert!(!html.contains("<?pure"));
    }
}

// ============================================================
// 13. MARKDOWN TESTS
// ============================================================

mod markdown_tests {
    use super::*;

    #[test]
    fn markdown_basic() {
        let env = TestEnv::new("md_basic");
        env.write(
            "content/post.md",
            "# Hello World\n\nThis is **bold** and *italic*.",
        );
        env.write("pages/test.hrml", r#"<?markdown file="content/post.md"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<h1>Hello World</h1>");
        assert_contains(&html, "<strong>bold</strong>");
        assert_contains(&html, "<em>italic</em>");
    }

    #[test]
    fn markdown_with_lists() {
        let env = TestEnv::new("md_lists");
        env.write("content/list.md", "- Item 1\n- Item 2\n- Item 3");
        env.write("pages/test.hrml", r#"<?markdown file="content/list.md"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<li>Item 1</li>");
        assert_contains(&html, "<li>Item 2</li>");
        assert_contains(&html, "<li>Item 3</li>");
    }

    #[test]
    fn markdown_with_code() {
        let env = TestEnv::new("md_code");
        env.write("content/code.md", "Use `let x = 1;` in Rust.");
        env.write("pages/test.hrml", r#"<?markdown file="content/code.md"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<code>");
    }

    #[test]
    fn markdown_with_links() {
        let env = TestEnv::new("md_links");
        env.write("content/links.md", "[Click here](https://example.com)");
        env.write("pages/test.hrml", r#"<?markdown file="content/links.md"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "href=\"https://example.com\"");
        assert_contains(&html, ">Click here<");
    }

    #[test]
    fn markdown_missing_file() {
        let env = TestEnv::new("md_missing");
        env.write("pages/test.hrml", r#"<?markdown file="nonexistent.md"?>"#);
        let result = env.render("pages/test.hrml");
        assert!(result.is_err());
    }

    #[test]
    fn markdown_with_math_delimiters() {
        let env = TestEnv::new("md_math");
        env.write(
            "content/math.md",
            "# Math\n\nInline: $a + b = c$\n\nBlock: $$\\int_0^1 x^2 dx$$",
        );
        env.write("pages/test.hrml", r#"<?markdown file="content/math.md"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "math-inline");
        assert_contains(&html, "math-block");
    }
}

// ============================================================
// 14. MARKDOWN FRONTMATTER TESTS
// ============================================================

mod markdownfm_tests {
    use super::*;

    #[test]
    fn markdownfm_parses_frontmatter() {
        let env = TestEnv::new("mdfm_basic");
        env.write(
            "content/post.md",
            "---\ntitle = \"My Post\"\ndescription = \"A great post\"\n---\n\n# Content",
        );
        env.write(
            "pages/test.hrml",
            r#"<?markdownfm file="content/post.md" as="post"?>
<h1><?get id="post.title"?></h1>
<p><?get id="post.description"?></p>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<h1>My Post</h1>");
        assert_contains(&html, "<p>A great post</p>");
        assert_contains(&html, "<h1>Content</h1>");
    }

    #[test]
    fn markdownfm_with_tags() {
        let env = TestEnv::new("mdfm_tags");
        env.write(
            "content/tagged.md",
            "---\ntitle = \"Tagged\"\ntags = [\"rust\", \"web\"]\n---\n\nBody",
        );
        env.write(
            "pages/test.hrml",
            r##"<?markdownfm file="content/tagged.md" as="p"?>
<?title value="$p.title"?>"##,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<title>Tagged</title>");
    }
}

// ============================================================
// 15. MDX TESTS
// ============================================================

mod mdx_tests {
    use super::*;

    #[test]
    fn mdx_basic() {
        let env = TestEnv::new("mdx_basic");
        env.write("content/page.mdx", "# MDX Page\n\nSome content.");
        env.write("pages/test.hrml", r#"<?mdx file="content/page.mdx"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<h1>MDX Page</h1>");
        assert_contains(&html, "<p>Some content.</p>");
    }

    #[test]
    fn mdx_missing_file() {
        let env = TestEnv::new("mdx_missing");
        env.write("pages/test.hrml", r#"<?mdx file="nonexistent.mdx"?>"#);
        let result = env.render("pages/test.hrml");
        assert!(result.is_err());
    }
}

// ============================================================
// 16. LATEX TESTS
// ============================================================

mod latex_tests {
    use super::*;

    #[test]
    fn latex_inline() {
        let env = TestEnv::new("latex_inline");
        env.write("pages/test.hrml", r#"<?latex formula="E = mc^2"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "math-inline");
        assert_contains(&html, "E = mc^2");
    }

    #[test]
    fn latex_block() {
        let env = TestEnv::new("latex_block");
        env.write(
            "pages/test.hrml",
            r#"<?latex mode="block" formula="\int_0^1 x^2 dx"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "math-block");
        assert_contains(&html, "\\[");
    }

    #[test]
    fn latex_default_mode() {
        let env = TestEnv::new("latex_default");
        env.write("pages/test.hrml", r#"<?latex formula="a^2 + b^2 = c^2"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "math-inline");
    }

    #[test]
    fn latex_missing_formula() {
        let env = TestEnv::new("latex_missing");
        env.write("pages/test.hrml", r#"<?latex mode="inline"?>"#);
        let result = env.render("pages/test.hrml");
        assert!(result.is_err());
    }
}

// ============================================================
// 17. META TAG LIBRARY TESTS
// ============================================================

mod meta_tag_tests {
    use super::*;

    #[test]
    fn charset_tag() {
        let env = TestEnv::new("meta_charset");
        env.write("pages/test.hrml", r#"<?charset?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<meta charset=\"UTF-8\">");
    }

    #[test]
    fn charset_custom() {
        let env = TestEnv::new("meta_charset_custom");
        env.write("pages/test.hrml", r#"<?charset value="ISO-8859-1"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<meta charset=\"ISO-8859-1\">");
    }

    #[test]
    fn viewport_tag() {
        let env = TestEnv::new("meta_viewport");
        env.write("pages/test.hrml", r#"<?viewport?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "name=\"viewport\"");
        assert_contains(&html, "width=device-width");
    }

    #[test]
    fn viewport_custom_content() {
        let env = TestEnv::new("meta_viewport_custom");
        env.write("pages/test.hrml", r#"<?viewport content="width=1024"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "content=\"width=1024\"");
    }

    #[test]
    fn title_tag() {
        let env = TestEnv::new("meta_title");
        env.write("pages/test.hrml", r#"<?title value="My Page"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<title>My Page</title>");
    }

    #[test]
    fn title_with_variable() {
        let env = TestEnv::new("meta_title_var");
        env.write(
            "pages/test.hrml",
            r#"<?set id="page_title"?>Dynamic Title</?set?>
<?title value="$page_title"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<title>Dynamic Title</title>");
    }

    #[test]
    fn description_tag() {
        let env = TestEnv::new("meta_desc");
        env.write(
            "pages/test.hrml",
            r#"<?description content="A test page"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "name=\"description\"");
        assert_contains(&html, "content=\"A test page\"");
    }

    #[test]
    fn robots_tag() {
        let env = TestEnv::new("meta_robots");
        env.write(
            "pages/test.hrml",
            r#"<?robots content="noindex,nofollow"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "name=\"robots\"");
        assert_contains(&html, "content=\"noindex,nofollow\"");
    }

    #[test]
    fn canonical_tag() {
        let env = TestEnv::new("meta_canonical");
        env.write(
            "pages/test.hrml",
            r#"<?canonical href="https://example.com/page"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "rel=\"canonical\"");
        assert_contains(&html, "href=\"https://example.com/page\"");
    }

    #[test]
    fn stylesheet_tag() {
        let env = TestEnv::new("meta_stylesheet");
        env.write(
            "pages/test.hrml",
            r#"<?stylesheet href="/assets/style.css"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "rel=\"stylesheet\"");
        assert_contains(&html, "href=\"/assets/style.css\"");
    }

    #[test]
    fn script_tag() {
        let env = TestEnv::new("meta_script");
        env.write("pages/test.hrml", r#"<?script src="/app.js"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "src=\"/app.js\"");
        assert_contains(&html, "</script>");
    }

    #[test]
    fn script_defer() {
        let env = TestEnv::new("meta_script_defer");
        env.write("pages/test.hrml", r#"<?script src="/app.js" defer?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "defer=\"defer\"");
    }

    #[test]
    fn script_async() {
        let env = TestEnv::new("meta_script_async");
        env.write("pages/test.hrml", r#"<?script src="/app.js" async?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "async=\"async\"");
    }

    #[test]
    fn og_tag() {
        let env = TestEnv::new("meta_og");
        env.write(
            "pages/test.hrml",
            r#"<?og name="title" content="My Page"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "property=\"og:title\"");
        assert_contains(&html, "content=\"My Page\"");
    }

    #[test]
    fn og_with_variable() {
        let env = TestEnv::new("meta_og_var");
        env.write(
            "pages/test.hrml",
            r#"<?set id="og_title"?>Dynamic OG</?set?>
<?og name="title" content="$og_title"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "content=\"Dynamic OG\"");
    }

    #[test]
    fn twitter_tag() {
        let env = TestEnv::new("meta_twitter");
        env.write(
            "pages/test.hrml",
            r#"<?twitter name="title" content="My Page"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "name=\"twitter:title\"");
        assert_contains(&html, "content=\"My Page\"");
    }

    #[test]
    fn generic_meta_tag() {
        let env = TestEnv::new("meta_generic");
        env.write(
            "pages/test.hrml",
            r#"<?meta name="author" content="Test"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<meta");
        assert_contains(&html, "name=\"author\"");
        assert_contains(&html, "content=\"Test\"");
    }

    #[test]
    fn generic_link_tag() {
        let env = TestEnv::new("link_generic");
        env.write(
            "pages/test.hrml",
            r#"<?linktag rel="preconnect" href="https://fonts.googleapis.com"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<link");
        assert_contains(&html, "rel=\"preconnect\"");
        assert_contains(&html, "href=\"https://fonts.googleapis.com\"");
    }
}

// ============================================================
// 18. FULL DOCUMENT RENDERING TESTS
// ============================================================

mod document_tests {
    use super::*;

    #[test]
    fn full_html_document_no_wrap() {
        let env = TestEnv::new("doc_full_html");
        env.write(
            "pages/test.hrml",
            r#"<!DOCTYPE html>
<html lang="en">
<head><title>Test</title></head>
<body><h1>Hello</h1></body>
</html>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_eq!(html.matches("<!DOCTYPE html>").count(), 1);
        assert_eq!(html.matches("<html").count(), 1);
        assert_eq!(html.matches("</html>").count(), 1);
    }

    #[test]
    fn fragment_gets_wrapped() {
        let env = TestEnv::new("doc_wrap");
        env.write("pages/test.hrml", r#"<p>Hello</p>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<html");
        assert_contains(&html, "<head>");
        assert_contains(&html, "<body>");
        assert_contains(&html, "<p>Hello</p>");
        assert_contains(&html, "/hrml.js");
    }

    #[test]
    fn render_fragment_no_wrap() {
        let env = TestEnv::new("doc_fragment");
        env.write("pages/test.hrml", r#"<p>Fragment</p>"#);
        let html = env.render_fragment("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<html");
        assert_not_contains(&html, "<head>");
        assert_contains(&html, "<p>Fragment</p>");
    }

    #[test]
    fn document_with_layout_load() {
        let env = TestEnv::new("doc_layout");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/site.hrml"?>
<?block slot="content"?>
<main><h1>Home</h1></main>
</?block?>"#,
        );
        env.write(
            "layouts/site.hrml",
            r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<title>Site</title>
</head>
<body>
<?slot id="content"?>Default</?slot?>
</body>
</html>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<!DOCTYPE html>");
        assert_contains(&html, "<h1>Home</h1>");
        assert_not_contains(&html, "Default");
    }

    #[test]
    fn document_with_site_config() {
        let env = TestEnv::new("doc_config");
        env.write("pages/test.hrml", r#"<h1><?get id="site_name"?></h1>"#);
        let html = env
            .engine_with_config("My Site", Some("Description"), Some("/icon.ico"))
            .render("pages/test.hrml", &json!({}))
            .unwrap();
        assert_contains(&html, "<h1>My Site</h1>");
    }

    #[test]
    fn wrapped_document_includes_hrml_js() {
        let env = TestEnv::new("doc_hrml_js");
        env.write("pages/test.hrml", r#"<div>Content</div>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "/hrml.js");
        assert_contains(&html, "<script src=\"/hrml.js\"></script>");
    }

    #[test]
    fn wrapped_document_includes_default_css() {
        let env = TestEnv::new("doc_css");
        env.write("pages/test.hrml", r#"<div>Content</div>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "/static/css/style.css");
    }

    #[test]
    fn wrapped_document_includes_favicon() {
        let env = TestEnv::new("doc_favicon");
        env.write("pages/test.hrml", r#"<div>Content</div>"#);
        let html = env
            .engine_with_config("Site", None, Some("/favicon.ico"))
            .render("pages/test.hrml", &json!({}))
            .unwrap();
        assert_contains(&html, "rel=\"icon\"");
        assert_contains(&html, "href=\"/favicon.ico\"");
    }

    #[test]
    fn wrapped_document_includes_description() {
        let env = TestEnv::new("doc_description");
        env.write("pages/test.hrml", r#"<div>Content</div>"#);
        let html = env
            .engine_with_config("Site", Some("A description"), None)
            .render("pages/test.hrml", &json!({}))
            .unwrap();
        assert_contains(&html, "name=\"description\"");
        assert_contains(&html, "content=\"A description\"");
    }
}

// ============================================================
// 19. CHAIN / PIPELINE TESTS
// ============================================================

mod chain_tests {
    use super::*;

    #[test]
    fn full_page_pipeline() {
        let env = TestEnv::new("chain_full_page");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/base.hrml"?>
<?block slot="head"?><?load file="components/meta.hrml"?></?block?>
<?block slot="content"?>
<?load file="components/header.hrml"?>
<main>
<?load file="sections/hero.hrml"?>
<?load file="sections/features.hrml"?>
</main>
<?load file="components/footer.hrml"?>
</?block?>"#,
        );
        env.write(
            "layouts/base.hrml",
            r#"<!DOCTYPE html>
<html>
<head>
<?slot id="head"?>
<title>Default</title>
</?slot?>
</head>
<body <?slot id="body_attrs"?>class="default"</?slot?>>
<?slot id="content"?>
</body>
</html>"#,
        );
        env.write(
            "components/meta.hrml",
            r#"<?charset?>
<?viewport?>
<?title value="My Site"?>
<?description content="A great site"?>"#,
        );
        env.write("components/header.hrml", "<header>Nav</header>");
        env.write(
            "sections/hero.hrml",
            "<section class=\"hero\">Hero</section>",
        );
        env.write(
            "sections/features.hrml",
            "<section class=\"features\">Features</section>",
        );
        env.write("components/footer.hrml", "<footer>Footer</footer>");

        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<!DOCTYPE html>");
        assert_contains(&html, "<meta charset=\"UTF-8\">");
        assert_contains(&html, "name=\"viewport\"");
        assert_contains(&html, "<title>My Site</title>");
        assert_contains(&html, "<header>Nav</header>");
        assert_contains(&html, "<section class=\"hero\">Hero</section>");
        assert_contains(&html, "<section class=\"features\">Features</section>");
        assert_contains(&html, "<footer>Footer</footer>");
    }

    #[test]
    fn blog_post_pipeline() {
        let env = TestEnv::new("chain_blog");
        env.write(
            "pages/post.hrml",
            r#"<?load file="layouts/blog.hrml"?>
<?block slot="content"?>
<?markdownfm file="content/hello.md" as="post"?>
<article>
<h1><?get id="post.title"?></h1>
<?get id="post.html"?>
</article>
</?block?>"#,
        );
        env.write(
            "layouts/blog.hrml",
            r#"<html>
<head><?charset?><?title value="Blog"?></head>
<body><?slot id="content"?></body>
</html>"#,
        );
        env.write(
            "content/hello.md",
            "---\ntitle = \"Hello World\"\n---\n\nThis is the post body.",
        );

        let html = env.render("pages/post.hrml").unwrap();
        assert_contains(&html, "<h1>Hello World</h1>");
        assert_contains(&html, "This is the post body");
    }

    #[test]
    fn component_chain_with_data_flow() {
        let env = TestEnv::new("chain_data_flow");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/base.hrml"?>
<?block slot="content"?>
<?set id="page_title"?>Dashboard</?set?>
<?component id="panel"?>
<div class="panel">
  <h2><?slot id="title"?></h2>
  <div><?slot id="body"?></div>
</div>
</?component?>
<?use id="panel"?>
<?block slot="title"?><?get id="page_title"?></?block?>
<?block slot="body"?>
<ul>
<?for in="item in data.items"?>
<li><?get id="item"?></li>
</?for?>
</ul>
</?block?>
</?use?>
</?block?>"#,
        );
        env.write(
            "layouts/base.hrml",
            r#"<html><body><?slot id="content"?></body></html>"#,
        );

        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({
                    "data": {
                        "items": ["Alpha", "Beta", "Gamma"]
                    }
                }),
            )
            .unwrap();
        assert_contains(&html, "<h2>Dashboard</h2>");
        assert_contains(&html, "<li>Alpha</li>");
        assert_contains(&html, "<li>Beta</li>");
        assert_contains(&html, "<li>Gamma</li>");
    }
}

// ============================================================
// 20. EDGE CASE AND ERROR TESTS
// ============================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn missing_page_returns_error() {
        let env = TestEnv::new("edge_missing_page");
        let result = env.render("pages/nonexistent.hrml");
        assert!(result.is_err());
    }

    #[test]
    fn deeply_nested_loads() {
        let env = TestEnv::new("edge_deep_loads");
        env.write("pages/test.hrml", r#"<?load file="l1.hrml"?>"#);
        for i in 1..50 {
            let next = i + 1;
            env.write(
                &format!("l{}.hrml", i),
                &format!(r#"L{}<?load file="l{}.hrml"?>"#, i, next),
            );
        }
        env.write("l50.hrml", "L50");
        let html = env.render("pages/test.hrml").unwrap();
        for i in 1..=50 {
            assert_contains(&html, &format!("L{}", i));
        }
    }

    #[test]
    fn special_chars_in_attributes() {
        let env = TestEnv::new("edge_attr_chars");
        env.write(
            "pages/test.hrml",
            r#"<div data-json='{"key":"value"}'>test</div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "data-json");
    }

    #[test]
    fn empty_slot_renders_empty() {
        let env = TestEnv::new("edge_empty_slot");
        env.write(
            "pages/test.hrml",
            r#"<div><?slot id="empty"?></?slot?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<div></div>");
    }

    #[test]
    fn empty_block_does_not_crash() {
        let env = TestEnv::new("edge_empty_block");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/base.hrml"?>
<?block slot="content"?></?block?>"#,
        );
        env.write(
            "layouts/base.hrml",
            r#"<div><?slot id="content"?>default</?slot?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<div>");
    }

    #[test]
    fn self_referencing_block() {
        let env = TestEnv::new("edge_self_ref");
        env.write(
            "pages/test.hrml",
            r#"<?set id="x"?>value</?set?>
<?if cond="x=='value'"?>
<p>Match</p>
</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>Match</p>");
    }

    #[test]
    fn for_with_missing_data_field() {
        let env = TestEnv::new("edge_for_missing");
        env.write(
            "pages/test.hrml",
            r#"<?for in="item in nonexistent"?>
<p><?get id="item"?></p>
</?for?>
<p>After</p>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>After</p>");
    }

    #[test]
    fn if_with_complex_condition() {
        let env = TestEnv::new("edge_if_complex");
        env.write(
            "pages/test.hrml",
            r#"<?set id="role"?>admin</?set?>
<?if cond="role=='admin'"?>
<p>Admin Access</p>
</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>Admin Access</p>");
    }

    #[test]
    fn html_entities_in_text() {
        let env = TestEnv::new("edge_entities");
        env.write("pages/test.hrml", "5 &lt; 10 &amp; 10 &gt; 5");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "5 &lt; 10 &amp; 10 &gt; 5");
    }

    #[test]
    fn large_template() {
        let env = TestEnv::new("edge_large");
        let rows: Vec<String> = (0..100)
            .map(|i| format!("<tr><td>{}</td><td>Row {}</td></tr>", i, i))
            .collect();
        let table = format!("<table>{}</table>", rows.join("\n"));
        env.write("pages/test.hrml", &table);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<td>0</td>");
        assert_contains(&html, "<td>99</td>");
        assert_contains(&html, "<td>Row 50</td>");
    }

    #[test]
    fn multiple_set_get_interleaved() {
        let env = TestEnv::new("edge_interleaved");
        env.write(
            "pages/test.hrml",
            r#"<?set id="a"?>1</?set?>
<?set id="b"?>2</?set?>
<?set id="c"?>3</?set?>
<?get id="a"?>-<?get id="b"?>-<?get id="c"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "1-2-3");
    }

    #[test]
    fn component_use_without_blocks() {
        let env = TestEnv::new("edge_component_no_blocks");
        env.write(
            "pages/test.hrml",
            r#"<?component id="simple"?>
<div>Simple Component</div>
</?component?>
<?use id="simple"?></?use?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<div>Simple Component</div>");
    }

    #[test]
    fn bind_with_missing_from_returns_empty() {
        let env = TestEnv::new("edge_bind_missing_from");
        env.write(
            "pages/test.hrml",
            r#"<?bind var="x" from="nonexistent.path"?>
[<?get id="x"?>]
</?bind?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "[]");
    }

    #[test]
    fn nested_compose() {
        let env = TestEnv::new("edge_nested_compose");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?>
<?compose op="sum"?><span>A</span><?then?><span>B</span></?compose?>
<?then?>
<?compose op="sum"?><span>C</span><?then?><span>D</span></?compose?>
</?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<span>A</span>");
        assert_contains(&html, "<span>B</span>");
        assert_contains(&html, "<span>C</span>");
        assert_contains(&html, "<span>D</span>");
    }

    #[test]
    fn template_with_only_directives() {
        let env = TestEnv::new("edge_only_directives");
        env.write(
            "pages/test.hrml",
            r#"<?charset?>
<?viewport?>
<?title value="Only Directives"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<meta charset=\"UTF-8\">");
        assert_contains(&html, "name=\"viewport\"");
        assert_contains(&html, "<title>Only Directives</title>");
    }

    #[test]
    fn template_with_only_html() {
        let env = TestEnv::new("edge_only_html");
        env.write(
            "pages/test.hrml",
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Pure HTML</title>
</head>
<body>
<h1>Hello</h1>
<p>No directives here</p>
</body>
</html>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_eq!(html.matches("<!DOCTYPE html>").count(), 1);
        assert_contains(&html, "<h1>Hello</h1>");
        assert_contains(&html, "<p>No directives here</p>");
    }
}

// ============================================================
// 21. UNSUSPICIOUS.ORG SIMULATION TESTS
// ============================================================

mod unsuspicious_tests {
    use super::*;

    #[test]
    fn unsuspicious_index_page() {
        let env = TestEnv::new("unsuspicious_index");
        env.write(
            "pages/index.hrml",
            r#"<?load file="layouts/site.hrml"?>
<?block slot="pre_head"?><?load file="content/heads/index.hrml"?></?block?>
<?block slot="body_attrs"?>class="antialiased bg-page text-default tracking-tight"</?block?>
<?block slot="content"?><?load file="content/bodies/index.hrml"?></?block?>"#,
        );
        env.write(
            "layouts/site.hrml",
            r##"<!DOCTYPE html>
<html class="2xl:text-[20px]" dir="ltr" lang="en">
<head>
<?slot id="pre_head"?></?slot?>
<title><?get id="hm_title"?></title>
<meta name="description" content="<?get id="hm_desc"?>">
<?slot id="extra_head"?></?slot?>
</head>
<body <?slot id="body_attrs"?>class="tracking-tight antialiased bg-page text-default"</?slot?>>
<?slot id="content"?></?slot?>
</body>
</html>"##,
        );
        env.write(
            "content/heads/index.hrml",
            r##"<?set id="hm_title"?>USI</?set?>
<?set id="hm_desc"?>Unsuspicious Industries is a research collective</?set?>"##,
        );
        env.write(
            "content/bodies/index.hrml",
            r#"<?compose op="sum"?>
<?load file="components/chrome/header.hrml"?>
<?then?>
<?load file="content/sections/index.hrml"?>
<?then?>
<?load file="components/chrome/footer.hrml"?>
</?compose?>"#,
        );
        env.write("components/chrome/header.hrml", "<header>Header</header>");
        env.write(
            "content/sections/index.hrml",
            "<main><h1>Hero Section</h1></main>",
        );
        env.write("components/chrome/footer.hrml", "<footer>Footer</footer>");

        let html = env.render("pages/index.hrml").unwrap();

        assert_contains(&html, "<!DOCTYPE html>");
        assert_contains(&html, "<title>USI</title>");
        assert_contains(&html, "Unsuspicious Industries is a research collective");
        assert_contains(&html, "<header>Header</header>");
        assert_contains(&html, "<main><h1>Hero Section</h1></main>");
        assert_contains(&html, "<footer>Footer</footer>");
        assert_contains(
            &html,
            "class=\"antialiased bg-page text-default tracking-tight\"",
        );
        assert_not_contains(&html, "<?load");
        assert_not_contains(&html, "<?block");
        assert_not_contains(&html, "<?slot");
        assert_not_contains(&html, "<?component");
        assert_not_contains(&html, "<?use");
        assert_not_contains(&html, "<?compose");
    }

    #[test]
    fn unsuspicious_about_page() {
        let env = TestEnv::new("unsuspicious_about");
        env.write(
            "pages/about.hrml",
            r#"<?load file="layouts/site.hrml"?>
<?block slot="content"?>
<main>
<h1>About Us</h1>
<p>We are Unsuspicious Industries.</p>
</main>
</?block?>"#,
        );
        env.write(
            "layouts/site.hrml",
            r#"<!DOCTYPE html>
<html>
<head><title>About</title></head>
<body>
<?slot id="content"?>Default content</?slot?>
</body>
</html>"#,
        );

        let html = env.render("pages/about.hrml").unwrap();
        assert_contains(&html, "<h1>About Us</h1>");
        assert_contains(&html, "<p>We are Unsuspicious Industries.</p>");
    }
}

// ============================================================
// 22. REGRESSION TESTS
// ============================================================

mod regression_tests {
    use super::*;

    #[test]
    fn slot_tag_not_in_output() {
        let env = TestEnv::new("reg_slot_clean");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/base.hrml"?>
<?block slot="content"?>Done</?block?>"#,
        );
        env.write(
            "layouts/base.hrml",
            r#"<div><?slot id="content"?></?slot?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?slot");
        assert_not_contains(&html, "</?slot?>");
        assert_not_contains(&html, "slot id=");
    }

    #[test]
    fn block_tag_not_in_output() {
        let env = TestEnv::new("reg_block_clean");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/base.hrml"?>
<?block slot="content"?>Done</?block?>"#,
        );
        env.write(
            "layouts/base.hrml",
            r#"<div><?slot id="content"?></?slot?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?block");
        assert_not_contains(&html, "</?block?>");
    }

    #[test]
    fn load_tag_not_in_output() {
        let env = TestEnv::new("reg_load_clean");
        env.write("pages/test.hrml", r#"<?load file="components/x.hrml"?>"#);
        env.write("components/x.hrml", "<div>Loaded</div>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?load");
        assert_contains(&html, "<div>Loaded</div>");
    }

    #[test]
    fn component_tag_not_in_output() {
        let env = TestEnv::new("reg_component_clean");
        env.write(
            "pages/test.hrml",
            r#"<?component id="x"?><span>X</span></?component?>
<?use id="x"?></?use?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?component");
        assert_not_contains(&html, "<?use");
        assert_contains(&html, "<span>X</span>");
    }

    #[test]
    fn set_tag_not_in_output() {
        let env = TestEnv::new("reg_set_clean");
        env.write(
            "pages/test.hrml",
            r#"<?set id="x"?>val</?set?><?get id="x"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?set");
        assert_not_contains(&html, "<?get");
        assert_contains(&html, "val");
    }

    #[test]
    fn compose_tag_not_in_output() {
        let env = TestEnv::new("reg_compose_clean");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?><p>A</p><?then?><p>B</p></?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?compose");
        assert_not_contains(&html, "<?then");
        assert_contains(&html, "<p>A</p><p>B</p>");
    }

    #[test]
    fn if_tag_not_in_output() {
        let env = TestEnv::new("reg_if_clean");
        env.write(
            "pages/test.hrml",
            r#"<?set id="show"?>yes</?set?>
<?if cond="show"?>
<p>Visible</p>
</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?if");
        assert_not_contains(&html, "<?else");
        assert_contains(&html, "<p>Visible</p>");
    }

    #[test]
    fn for_tag_not_in_output() {
        let env = TestEnv::new("reg_for_clean");
        env.write(
            "pages/test.hrml",
            r#"<?for in="x in items"?>
<li><?get id="x"?></li>
</?for?>"#,
        );
        let html = env
            .render_with_data("pages/test.hrml", &json!({ "items": ["a"] }))
            .unwrap();
        assert_not_contains(&html, "<?for");
        assert_contains(&html, "<li>a</li>");
    }

    #[test]
    fn btn_tag_not_in_output() {
        let env = TestEnv::new("reg_btn_clean");
        env.write("pages/test.hrml", r#"<?btn post="/api/x"?>Go</?btn?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?btn");
        assert_contains(&html, "<button");
        assert_contains(&html, "data-post");
    }

    #[test]
    fn form_tag_not_in_output() {
        let env = TestEnv::new("reg_form_clean");
        env.write("pages/test.hrml", r#"<?form post="/api/x"?>input</?form?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?form");
        assert_contains(&html, "<form");
        assert_contains(&html, "data-post");
    }

    #[test]
    fn wasm_tag_not_in_output() {
        let env = TestEnv::new("reg_wasm_clean");
        env.write("pages/test.hrml", r#"<?wasm module="/app.wasm"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?wasm");
        assert_contains(&html, "data-wasm-module");
    }

    #[test]
    fn markdown_tag_not_in_output() {
        let env = TestEnv::new("reg_md_clean");
        env.write("content/test.md", "# Test");
        env.write("pages/test.hrml", r#"<?markdown file="content/test.md"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?markdown");
        assert_contains(&html, "<h1>Test</h1>");
    }

    #[test]
    fn latex_tag_not_in_output() {
        let env = TestEnv::new("reg_latex_clean");
        env.write("pages/test.hrml", r#"<?latex formula="x^2"?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?latex");
        assert_contains(&html, "math-inline");
    }

    #[test]
    fn no_double_html_wrap() {
        let env = TestEnv::new("reg_no_double_wrap");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/full.hrml"?>
<?block slot="content"?>Hello</?block?>"#,
        );
        env.write(
            "layouts/full.hrml",
            r#"<!DOCTYPE html>
<html><head><title>T</title></head>
<body><?slot id="content"?></body></html>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_eq!(html.matches("<!DOCTYPE html>").count(), 1);
        assert_eq!(html.matches("<html").count(), 1);
        assert_eq!(html.matches("<body").count(), 1);
    }

    #[test]
    fn no_unprocessed_php_style_tags() {
        let env = TestEnv::new("reg_no_php_tags");
        env.write(
            "pages/test.hrml",
            r#"<?load file="components/a.hrml"?>
<?set id="x"?>val</?set?>
<?get id="x"?>
<?if cond="x"?>yes<?else?>no</?if?>
<?compose op="sum"?>a<?then?>b</?compose?>"#,
        );
        env.write("components/a.hrml", "<div>A</div>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?");
        assert_not_contains(&html, "?>");
    }
}
