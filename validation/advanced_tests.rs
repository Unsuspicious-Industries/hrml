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
        let dir = format!("/tmp/hrml_adv_{}_{}", name, now);
        let dirs = [
            "layouts",
            "components",
            "pages",
            "content",
            "sections",
            "chrome",
            "partials",
            "static",
            "endpoints/api",
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

fn assert_count(html: &str, needle: &str, expected: usize) {
    let count = html.matches(needle).count();
    assert_eq!(
        count, expected,
        "\nExpected {} occurrences of '{}', found {}\n\nActual output:\n{}\n",
        expected, needle, count, html
    );
}

// ============================================================
// 1. DYNAMIC COMPONENT TESTS
// ============================================================

mod dynamic_components {
    use super::*;

    #[test]
    fn component_with_dynamic_slot_content() {
        let env = TestEnv::new("dyn_comp_slot");
        env.write(
            "pages/test.hrml",
            r#"<?component id="card"?>
<div class="card">
  <div class="card-header"><?slot id="header"?></div>
  <div class="card-body"><?slot id="body"?></div>
  <div class="card-footer"><?slot id="footer"?></div>
</div>
</?component?>

<?use id="card"?>
<?block slot="header"?><h3>Dynamic Header</h3></?block?>
<?block slot="body"?>
<p>Body paragraph 1</p>
<p>Body paragraph 2</p>
</?block?>
<?block slot="footer"?><small>Footer text</small></?block?>
</?use?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<div class=\"card\">");
        assert_contains(
            &html,
            "<div class=\"card-header\"><h3>Dynamic Header</h3></div>",
        );
        assert_contains(&html, "<div class=\"card-body\">");
        assert_contains(&html, "<p>Body paragraph 1</p>");
        assert_contains(&html, "<p>Body paragraph 2</p>");
        assert_contains(
            &html,
            "<div class=\"card-footer\"><small>Footer text</small></div>",
        );
    }

    #[test]
    fn component_with_variable_injection() {
        let env = TestEnv::new("dyn_comp_var");
        env.write(
            "pages/test.hrml",
            r#"<?component id="greeting"?>
<div class="greet">
  <h2>Hello, <?slot id="name"?>World</?slot?>!</h2>
  <p>Welcome to <?get id="site_name"?></p>
</div>
</?component?>

<?use id="greeting"?>
<?block slot="name"?>Alice</?block?>
</?use?>"#,
        );
        let html = env
            .engine_with_config("MathLab", None, None)
            .render("pages/test.hrml", &json!({}))
            .unwrap();
        assert_contains(&html, "<h2>Hello, Alice!</h2>");
        assert_contains(&html, "<p>Welcome to MathLab</p>");
    }

    #[test]
    fn nested_components() {
        let env = TestEnv::new("dyn_comp_nested");
        env.write(
            "pages/test.hrml",
            r#"<?component id="outer"?>
<section class="outer">
  <h1><?slot id="title"?></h1>
  <?slot id="content"?>
</section>
</?component?>

<?component id="inner"?>
<article class="inner">
  <h2><?slot id="subtitle"?></h2>
  <p><?slot id="text"?></p>
</article>
</?component?>

<?use id="outer"?>
<?block slot="title"?>Outer Title</?block?>
<?block slot="content"?>
<?use id="inner"?>
<?block slot="subtitle"?>Inner Subtitle</?block?>
<?block slot="text"?>Inner text content</?block?>
</?use?>
</?block?>
</?use?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<section class=\"outer\">");
        assert_contains(&html, "<h1>Outer Title</h1>");
        assert_contains(&html, "<article class=\"inner\">");
        assert_contains(&html, "<h2>Inner Subtitle</h2>");
        assert_contains(&html, "<p>Inner text content</p>");
    }

    #[test]
    fn component_reuse_multiple_times() {
        let env = TestEnv::new("dyn_comp_reuse");
        env.write(
            "pages/test.hrml",
            r#"<?component id="item"?>
<li class="item"><?slot id="label"?></li>
</?component?>

<ul>
<?use id="item"?><?block slot="label"?>First</?block?></?use?>
<?use id="item"?><?block slot="label"?>Second</?block?></?use?>
<?use id="item"?><?block slot="label"?>Third</?block?></?use?>
</ul>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<li class=\"item\">First</li>");
        assert_contains(&html, "<li class=\"item\">Second</li>");
        assert_contains(&html, "<li class=\"item\">Third</li>");
    }

    #[test]
    fn component_with_conditional_content() {
        let env = TestEnv::new("dyn_comp_cond");
        env.write(
            "pages/test.hrml",
            r#"<?component id="alert"?>
<div class="alert">
  <?if cond="type=='error'"?>
  <span class="icon">X</span>
  <?else?>
  <?if cond="type=='success'"?>
  <span class="icon">OK</span>
  <?else?>
  <span class="icon">i</span>
  </?if?>
  </?if?>
  <span class="msg"><?slot id="message"?></span>
</div>
</?component?>

<?set id="type"?>error</?set?>
<?use id="alert"?>
<?block slot="message"?>Something went wrong</?block?>
</?use?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<span class=\"icon\">X</span>");
        assert_contains(&html, "<span class=\"msg\">Something went wrong</span>");
        assert_not_contains(&html, "OK</span>");
    }

    #[test]
    fn component_with_for_loop_content() {
        let env = TestEnv::new("dyn_comp_for");
        env.write(
            "pages/test.hrml",
            r#"<?component id="list"?>
<ul class="list">
<?slot id="items"?>
</ul>
</?component?>

<?use id="list"?>
<?block slot="items"?>
<?for in="item in data.fruits"?>
<li class="fruit"><?get id="item"?></li>
</?for?>
</?block?>
</?use?>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({ "data": { "fruits": ["apple", "banana", "cherry"] } }),
            )
            .unwrap();
        assert_contains(&html, "<li class=\"fruit\">apple</li>");
        assert_contains(&html, "<li class=\"fruit\">banana</li>");
        assert_contains(&html, "<li class=\"fruit\">cherry</li>");
    }

    #[test]
    fn component_with_compose_in_body() {
        let env = TestEnv::new("dyn_comp_compose");
        env.write(
            "pages/test.hrml",
            r#"<?component id="panel"?>
<div class="panel">
  <?slot id="content"?>
</div>
</?component?>

<?use id="panel"?>
<?block slot="content"?>
<?compose op="sum"?>
<header>Panel Header</header>
<?then?>
<main>Panel Main</main>
<?then?>
<footer>Panel Footer</footer>
</?compose?>
</?block?>
</?use?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<div class=\"panel\">");
        assert_contains(&html, "<header>Panel Header</header>");
        assert_contains(&html, "<main>Panel Main</main>");
        assert_contains(&html, "<footer>Panel Footer</footer>");
    }

    #[test]
    fn component_default_slots_preserved() {
        let env = TestEnv::new("dyn_comp_defaults");
        env.write(
            "pages/test.hrml",
            r#"<?component id="card"?>
<div class="card">
  <h2><?slot id="title"?>Default Title</?slot?></h2>
  <p><?slot id="body"?>Default body text</?slot?></p>
</div>
</?component?>

<?use id="card"?>
<?block slot="title"?>Custom Title</?block?>
</?use?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<h2>Custom Title</h2>");
        assert_contains(&html, "<p>Default body text</p>");
    }

    #[test]
    fn component_with_math_content() {
        let env = TestEnv::new("dyn_comp_math");
        env.write(
            "pages/test.hrml",
            r#"<?component id="equation"?>
<div class="equation">
  <span class="eq-label"><?slot id="label"?></span>
  <span class="eq-body"><?slot id="body"?></span>
</div>
</?component?>

<?use id="equation"?>
<?block slot="label"?>(1)</?block?>
<?block slot="body"?>E = mc^2</?block?>
</?use?>

<?use id="equation"?>
<?block slot="label"?>(2)</?block?>
<?block slot="body"?>a^2 + b^2 = c^2</?block?>
</?use?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<span class=\"eq-label\">(1)</span>");
        assert_contains(&html, "<span class=\"eq-body\">E = mc^2</span>");
        assert_contains(&html, "<span class=\"eq-label\">(2)</span>");
        assert_contains(&html, "<span class=\"eq-body\">a^2 + b^2 = c^2</span>");
    }
}

// ============================================================
// 2. DYNAMIC VARIABLE & DATA FLOW TESTS
// ============================================================

mod dynamic_data_flow {
    use super::*;

    #[test]
    fn set_get_chain() {
        let env = TestEnv::new("data_chain");
        env.write(
            "pages/test.hrml",
            r#"<?set id="a"?>1</?set?>
<?set id="b"?><?get id="a"?></?set?>
<?set id="c"?><?get id="b"?></?set?>
<?set id="d"?><?get id="c"?></?set?>
<?get id="d"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "1");
    }

    #[test]
    fn variable_override_in_nested_scope() {
        let env = TestEnv::new("data_scope");
        env.write(
            "pages/test.hrml",
            r#"<?set id="x"?>outer</?set?>
<p><?get id="x"?></p>
<?bind var="x" value="inner"?>
<p><?get id="x"?></p>
</?bind?>
<p><?get id="x"?></p>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        let parts: Vec<&str> = html.split("<p>").collect();
        let values: Vec<&str> = parts
            .iter()
            .skip(1)
            .filter_map(|p| p.split("</p>").next())
            .collect();
        assert_eq!(values, vec!["outer", "inner", "outer"]);
    }

    #[test]
    fn json_data_nested_access() {
        let env = TestEnv::new("data_nested");
        env.write(
            "pages/test.hrml",
            r#"<h1><?get id="user.profile.name"?></h1>
<p><?get id="user.profile.email"?></p>
<span><?get id="user.settings.theme"?></span>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({
                    "user": {
                        "profile": {
                            "name": "Alice",
                            "email": "alice@example.com"
                        },
                        "settings": {
                            "theme": "dark"
                        }
                    }
                }),
            )
            .unwrap();
        assert_contains(&html, "<h1>Alice</h1>");
        assert_contains(&html, "<p>alice@example.com</p>");
        assert_contains(&html, "<span>dark</span>");
    }

    #[test]
    fn for_loop_with_nested_data() {
        let env = TestEnv::new("data_for_nested");
        env.write(
            "pages/test.hrml",
            r#"<table>
<?for in="row in data.matrix"?>
<tr>
<?for in="cell in row.cells"?>
<td><?get id="cell"?></td>
</?for?>
</tr>
</?for?>
</table>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({
                    "data": {
                        "matrix": [
                            { "cells": [1, 2, 3] },
                            { "cells": [4, 5, 6] },
                            { "cells": [7, 8, 9] }
                        ]
                    }
                }),
            )
            .unwrap();
        assert_contains(&html, "<td>1</td>");
        assert_contains(&html, "<td>5</td>");
        assert_contains(&html, "<td>9</td>");
        assert_count(&html, "<tr>", 3);
        assert_count(&html, "<td>", 9);
    }

    #[test]
    fn conditional_with_json_data() {
        let env = TestEnv::new("data_cond_json");
        env.write(
            "pages/test.hrml",
            r#"<?if cond="user.role=='admin'"?>
<div class="admin">Admin Panel</div>
<?else?>
<?if cond="user.role=='editor'"?>
<div class="editor">Editor Panel</div>
<?else?>
<div class="viewer">Viewer Panel</div>
</?if?>
</?if?>"#,
        );

        let html_admin = env
            .render_with_data("pages/test.hrml", &json!({ "user": { "role": "admin" } }))
            .unwrap();
        assert_contains(&html_admin, "<div class=\"admin\">Admin Panel</div>");
        assert_not_contains(&html_admin, "Editor");
        assert_not_contains(&html_admin, "Viewer");

        let html_editor = env
            .render_with_data("pages/test.hrml", &json!({ "user": { "role": "editor" } }))
            .unwrap();
        assert_contains(&html_editor, "<div class=\"editor\">Editor Panel</div>");

        let html_viewer = env
            .render_with_data("pages/test.hrml", &json!({ "user": { "role": "viewer" } }))
            .unwrap();
        assert_contains(&html_viewer, "<div class=\"viewer\">Viewer Panel</div>");
    }

    #[test]
    fn bind_from_json_with_component() {
        let env = TestEnv::new("data_bind_comp");
        env.write(
            "pages/test.hrml",
            r#"<?component id="profile"?>
<div class="profile">
  <h2><?slot id="name"?></h2>
  <p><?slot id="bio"?></p>
</div>
</?component?>

<?for in="person in data.people"?>
<?bind var="pname" from="person.name"?>
<?use id="profile"?>
<?block slot="name"?><?get id="pname"?></?block?>
<?block slot="bio"?><?get id="person.bio"?></?block?>
</?use?>
</?bind?>
</?for?>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({
                    "data": {
                        "people": [
                            { "name": "Alice", "bio": "Mathematician" },
                            { "name": "Bob", "bio": "Physicist" }
                        ]
                    }
                }),
            )
            .unwrap();
        assert_contains(&html, "<h2>Alice</h2>");
        assert_contains(&html, "<p>Mathematician</p>");
        assert_contains(&html, "<h2>Bob</h2>");
        assert_contains(&html, "<p>Physicist</p>");
    }

    #[test]
    fn variable_with_math_expression_content() {
        let env = TestEnv::new("data_math_var");
        env.write(
            "pages/test.hrml",
            r#"<?set id="formula"?>E = mc^2</?set?>
<?set id="theorem"?>a^2 + b^2 = c^2</?set?>
<div class="formula"><?get id="formula"?></div>
<div class="theorem"><?get id="theorem"?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<div class=\"formula\">E = mc^2</div>");
        assert_contains(&html, "<div class=\"theorem\">a^2 + b^2 = c^2</div>");
    }
}

// ============================================================
// 3. COMPOSITION & ALGEBRA TESTS
// ============================================================

mod composition_algebra {
    use super::*;

    #[test]
    fn compose_sum_multiple_segments() {
        let env = TestEnv::new("comp_sum_multi");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?>
<span>A</span>
<?then?>
<span>B</span>
<?then?>
<span>C</span>
<?then?>
<span>D</span>
<?then?>
<span>E</span>
</?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(
            &html,
            "<span>A</span><span>B</span><span>C</span><span>D</span><span>E</span>",
        );
    }

    #[test]
    fn compose_product_wraps_in_div() {
        let env = TestEnv::new("comp_product");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="product"?>
<div class="left">L</div>
<?then?>
<div class="right">R</div>
</?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "class=\"hrml-product\"");
        assert_contains(&html, "class=\"hrml-factor\"");
        assert_contains(&html, "<div class=\"left\">L</div>");
        assert_contains(&html, "<div class=\"right\">R</div>");
    }

    #[test]
    fn compose_nested_sum_and_product() {
        let env = TestEnv::new("comp_nested");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?>
<?compose op="product"?><span>A</span><?then?><span>B</span></?compose?>
<?then?>
<?compose op="sum"?><span>C</span><?then?><span>D</span></?compose?>
</?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "class=\"hrml-product\"");
        assert_contains(&html, "<span>A</span>");
        assert_contains(&html, "<span>B</span>");
        assert_contains(&html, "<span>C</span><span>D</span>");
    }

    #[test]
    fn compose_with_load_directives() {
        let env = TestEnv::new("comp_load");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?>
<?load file="sections/alpha.hrml"?>
<?then?>
<?load file="sections/beta.hrml"?>
<?then?>
<?load file="sections/gamma.hrml"?>
</?compose?>"#,
        );
        env.write(
            "sections/alpha.hrml",
            "<section id=\"alpha\">Alpha</section>",
        );
        env.write("sections/beta.hrml", "<section id=\"beta\">Beta</section>");
        env.write(
            "sections/gamma.hrml",
            "<section id=\"gamma\">Gamma</section>",
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<section id=\"alpha\">Alpha</section>");
        assert_contains(&html, "<section id=\"beta\">Beta</section>");
        assert_contains(&html, "<section id=\"gamma\">Gamma</section>");
    }

    #[test]
    fn compose_with_if_conditionals() {
        let env = TestEnv::new("comp_if");
        env.write(
            "pages/test.hrml",
            r#"<?set id="mode"?>extended</?set?>
<?compose op="sum"?>
<header>Header</header>
<?then?>
<?if cond="mode=='extended'"?>
<main>Extended Content</main>
<?else?>
<main>Basic Content</main>
</?if?>
<?then?>
<footer>Footer</footer>
</?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<header>Header</header>");
        assert_contains(&html, "<main>Extended Content</main>");
        assert_contains(&html, "<footer>Footer</footer>");
        assert_not_contains(&html, "Basic Content");
    }

    #[test]
    fn compose_with_for_loop() {
        let env = TestEnv::new("comp_for");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?>
<ul>
<?for in="item in data.items"?>
<li><?get id="item"?></li>
</?for?>
</ul>
<?then?>
<p>Total: <?get id="data.count"?> items</p>
</?compose?>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({
                    "data": {
                        "items": ["x", "y", "z"],
                        "count": 3
                    }
                }),
            )
            .unwrap();
        assert_contains(&html, "<li>x</li>");
        assert_contains(&html, "<li>y</li>");
        assert_contains(&html, "<li>z</li>");
        assert_contains(&html, "<p>Total: 3 items</p>");
    }

    #[test]
    fn compose_empty_segments_ignored() {
        let env = TestEnv::new("comp_empty");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?>
<?then?>
<span>Only This</span>
<?then?>
</?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<span>Only This</span>");
    }

    #[test]
    fn compose_product_with_complex_html() {
        let env = TestEnv::new("comp_product_complex");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="product"?>
<div class="lhs">
  <h2>Left Side</h2>
  <p>Some content</p>
</div>
<?then?>
<div class="rhs">
  <h2>Right Side</h2>
  <p>Other content</p>
</div>
</?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "class=\"hrml-product\"");
        assert_contains(&html, "<h2>Left Side</h2>");
        assert_contains(&html, "<h2>Right Side</h2>");
    }
}

// ============================================================
// 4. API RUNTIME TESTS
// ============================================================

mod api_runtime {
    use super::*;

    #[test]
    fn api_hrml_endpoint() {
        let env = TestEnv::new("api_hrml");
        env.write(
            "endpoints/api/hello.hrml",
            r#"<div class="api-response">Hello from API</div>"#,
        );
        let html = env.render_fragment("endpoints/api/hello.hrml").unwrap();
        assert_contains(&html, "<div class=\"api-response\">Hello from API</div>");
    }

    #[test]
    fn api_hrml_with_data() {
        let env = TestEnv::new("api_hrml_data");
        env.write(
            "endpoints/api/greet.hrml",
            r#"<p>Hello, <?get id="data.name"?>!</p>"#,
        );
        let html = env
            .render_fragment_with_data("endpoints/api/greet.hrml", &json!({ "name": "World" }))
            .unwrap();
        assert_contains(&html, "<p>Hello, World!</p>");
    }

    #[test]
    fn api_endpoint_with_id_and_action() {
        let env = TestEnv::new("api_id_action");
        env.write("endpoints/api/users.hrml", r#"<div>Users list</div>"#);
        env.write(
            "endpoints/api/users/delete.hrml",
            r#"<div>User deleted</div>"#,
        );
        let html_list = env.render_fragment("endpoints/api/users.hrml").unwrap();
        assert_contains(&html_list, "<div>Users list</div>");
        let html_delete = env
            .render_fragment("endpoints/api/users/delete.hrml")
            .unwrap();
        assert_contains(&html_delete, "<div>User deleted</div>");
    }

    #[test]
    fn api_endpoint_with_context_data() {
        let env = TestEnv::new("api_context");
        env.write(
            "endpoints/api/status.hrml",
            r#"<div class="status">
  <span>Module: <?get id="id"?></span>
  <span>Action: <?get id="action"?></span>
</div>"#,
        );
        let html = env.render_fragment("endpoints/api/status.hrml").unwrap();
        assert_contains(&html, "<span>Module: </span>");
        assert_contains(&html, "<span>Action: </span>");
    }

    #[test]
    fn api_endpoint_with_conditional_response() {
        let env = TestEnv::new("api_cond");
        env.write(
            "endpoints/api/check.hrml",
            r#"<?if cond="data.value=='yes'"?>
<div class="result">Yes!</div>
<?else?>
<div class="result">No.</div>
</?if?>"#,
        );
        let html_yes = env
            .render_fragment_with_data("endpoints/api/check.hrml", &json!({ "value": "yes" }))
            .unwrap();
        assert_contains(&html_yes, "<div class=\"result\">Yes!</div>");

        let html_no = env
            .render_fragment_with_data("endpoints/api/check.hrml", &json!({ "value": "no" }))
            .unwrap();
        assert_contains(&html_no, "<div class=\"result\">No.</div>");
    }

    #[test]
    fn api_endpoint_with_for_loop() {
        let env = TestEnv::new("api_for");
        env.write(
            "endpoints/api/list.hrml",
            r#"<ul>
<?for in="item in data.items"?>
<li><?get id="item"?></li>
</?for?>
</ul>"#,
        );
        let html = env
            .render_fragment_with_data(
                "endpoints/api/list.hrml",
                &json!({ "items": ["alpha", "beta", "gamma"] }),
            )
            .unwrap();
        assert_contains(&html, "<li>alpha</li>");
        assert_contains(&html, "<li>beta</li>");
        assert_contains(&html, "<li>gamma</li>");
    }

    #[test]
    fn api_endpoint_with_compose() {
        let env = TestEnv::new("api_compose");
        env.write(
            "endpoints/api/report.hrml",
            r#"<?compose op="sum"?>
<div class="header">Report Header</div>
<?then?>
<div class="body">
<?for in="row in data.rows"?>
<p><?get id="row"?></p>
</?for?>
</div>
<?then?>
<div class="footer">Report Footer</div>
</?compose?>"#,
        );
        let html = env
            .render_fragment_with_data(
                "endpoints/api/report.hrml",
                &json!({ "rows": ["Row 1", "Row 2"] }),
            )
            .unwrap();
        assert_contains(&html, "<div class=\"header\">Report Header</div>");
        assert_contains(&html, "<p>Row 1</p>");
        assert_contains(&html, "<p>Row 2</p>");
        assert_contains(&html, "<div class=\"footer\">Report Footer</div>");
    }

    #[test]
    fn api_endpoint_with_component() {
        let env = TestEnv::new("api_component");
        env.write(
            "endpoints/api/cards.hrml",
            r#"<?component id="card"?>
<div class="card">
  <h3><?slot id="title"?></h3>
  <p><?slot id="desc"?></p>
</div>
</?component?>

<div class="card-list">
<?for in="card in data.cards"?>
<?use id="card"?>
<?block slot="title"?><?get id="card.title"?></?block?>
<?block slot="desc"?><?get id="card.desc"?></?block?>
</?use?>
</?for?>
</div>"#,
        );
        let html = env
            .render_fragment_with_data(
                "endpoints/api/cards.hrml",
                &json!({
                    "cards": [
                        { "title": "Card 1", "desc": "Description 1" },
                        { "title": "Card 2", "desc": "Description 2" }
                    ]
                }),
            )
            .unwrap();
        assert_contains(&html, "<h3>Card 1</h3>");
        assert_contains(&html, "<p>Description 1</p>");
        assert_contains(&html, "<h3>Card 2</h3>");
        assert_contains(&html, "<p>Description 2</p>");
    }
}

// ============================================================
// 5. MATHEMATICAL CONTENT TESTS
// ============================================================

mod mathematical_content {
    use super::*;

    #[test]
    fn latex_inline_and_block() {
        let env = TestEnv::new("math_latex");
        env.write(
            "pages/test.hrml",
            r#"<p>The formula <?latex formula="E = mc^2"?> is famous.</p>
<?latex mode="block" formula="\int_0^1 x^2 dx = \frac{1}{3}"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "math-inline");
        assert_contains(&html, "E = mc^2");
        assert_contains(&html, "math-block");
        assert_contains(&html, "\\[");
    }

    #[test]
    fn markdown_with_math_delimiters() {
        let env = TestEnv::new("math_md");
        env.write(
            "content/equations.md",
            "# Equations\n\nInline: $f(x) = x^2 + 2x + 1$\n\nBlock: $$\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}$$",
        );
        env.write(
            "pages/test.hrml",
            r#"<?markdown file="content/equations.md"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "math-inline");
        assert_contains(&html, "math-block");
        assert_contains(&html, "f(x) = x^2 + 2x + 1");
    }

    #[test]
    fn math_table_with_latex() {
        let env = TestEnv::new("math_table");
        env.write(
            "pages/test.hrml",
            r#"<table class="math-table">
<thead>
<tr><th>Symbol</th><th>Meaning</th></tr>
</thead>
<tbody>
<tr><td><?latex formula="\sum"?></td><td>Summation</td></tr>
<tr><td><?latex formula="\prod"?></td><td>Product</td></tr>
<tr><td><?latex formula="\int"?></td><td>Integral</td></tr>
<tr><td><?latex formula="\partial"?></td><td>Partial derivative</td></tr>
</tbody>
</table>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<th>Symbol</th>");
        assert_contains(&html, "<th>Meaning</th>");
        assert_contains(&html, "<td>");
        assert_contains(&html, "math-inline");
        assert_contains(&html, "Summation");
        assert_contains(&html, "Integral");
    }

    #[test]
    fn math_with_for_loop() {
        let env = TestEnv::new("math_for");
        env.write(
            "pages/test.hrml",
            r#"<div class="powers">
<?for in="n in data.numbers"?>
<p><?get id="n"?>^2 = <?get id="n"?> * <?get id="n"?></p>
</?for?>
</div>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({ "data": { "numbers": [2, 3, 5, 7] } }),
            )
            .unwrap();
        assert_contains(&html, "<p>2^2 = 2 * 2</p>");
        assert_contains(&html, "<p>3^2 = 3 * 3</p>");
        assert_contains(&html, "<p>5^2 = 5 * 5</p>");
        assert_contains(&html, "<p>7^2 = 7 * 7</p>");
    }

    #[test]
    fn math_with_conditional_display() {
        let env = TestEnv::new("math_cond");
        env.write(
            "pages/test.hrml",
            r#"<?set id="level"?>advanced</?set?>
<?if cond="level=='basic'"?>
<p>x + y = z</p>
<?else?>
<?if cond="level=='advanced'"?>
<?latex mode="block" formula="\nabla \times \vec{E} = -\frac{\partial \vec{B}}{\partial t}"?>
<?else?>
<p>No content</p>
</?if?>
</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "math-block");
        assert_contains(&html, "\\nabla");
    }

    #[test]
    fn math_with_compose() {
        let env = TestEnv::new("math_compose");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?>
<h2>Theorem</h2>
<?then?>
<p>For all a, b in R: (a + b)^2 = a^2 + 2ab + b^2</p>
<?then?>
<?latex mode="block" formula="(a + b)^2 = a^2 + 2ab + b^2"?>
</?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<h2>Theorem</h2>");
        assert_contains(&html, "(a + b)^2 = a^2 + 2ab + b^2");
        assert_contains(&html, "math-block");
    }

    #[test]
    fn math_with_component() {
        let env = TestEnv::new("math_comp");
        env.write(
            "pages/test.hrml",
            r#"<?component id="theorem"?>
<div class="theorem">
  <h3><?slot id="name"?></h3>
  <div class="statement"><?slot id="statement"?></div>
</div>
</?component?>

<?use id="theorem"?>
<?block slot="name"?>Pythagorean Theorem</?block?>
<?block slot="statement"?><?latex formula="a^2 + b^2 = c^2"?></?block?>
</?use?>

<?use id="theorem"?>
<?block slot="name"?>Euler's Identity</?block?>
<?block slot="statement"?><?latex formula="e^{i\pi} + 1 = 0"?></?block?>
</?use?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<h3>Pythagorean Theorem</h3>");
        assert_contains(&html, "a^2 + b^2 = c^2");
        assert_contains(&html, "<h3>Euler's Identity</h3>");
        assert_contains(&html, "e^{i\\pi} + 1 = 0");
    }
}

// ============================================================
// 6. COMPLEX TEMPLATE CHAINS
// ============================================================

mod complex_chains {
    use super::*;

    #[test]
    fn multi_level_layout_inheritance() {
        let env = TestEnv::new("chain_inherit");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/page.hrml"?>
<?block slot="page_content"?>
<h1>Page Title</h1>
<p>Page body</p>
</?block?>"#,
        );
        env.write(
            "layouts/page.hrml",
            r#"<?load file="layouts/base.hrml"?>
<?block slot="base_content"?>
<main><?slot id="page_content"?></main>
</?block?>"#,
        );
        env.write(
            "layouts/base.hrml",
            r#"<!DOCTYPE html>
<html>
<head><title>Base</title></head>
<body>
<header>Site Header</header>
<?slot id="base_content"?>
<footer>Site Footer</footer>
</body>
</html>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<!DOCTYPE html>");
        assert_contains(&html, "<header>Site Header</header>");
        assert_contains(&html, "<h1>Page Title</h1>");
        assert_contains(&html, "<p>Page body</p>");
        assert_contains(&html, "<footer>Site Footer</footer>");
    }

    #[test]
    fn component_chain_with_data_propagation() {
        let env = TestEnv::new("chain_data_prop");
        env.write(
            "pages/test.hrml",
            r#"<?set id="site"?>MathWorld</?set?>
<?load file="layouts/app.hrml"?>
<?block slot="app_body"?>
<?component id="section"?>
<section class="section">
  <h2><?slot id="title"?></h2>
  <div><?slot id="content"?></div>
</section>
</?component?>

<?use id="section"?>
<?block slot="title"?>Algebra</?block?>
<?block slot="content"?>
<?for in="topic in data.algebra"?>
<p><?get id="topic"?></p>
</?for?>
</?block?>
</?use?>

<?use id="section"?>
<?block slot="title"?>Calculus</?block?>
<?block slot="content"?>
<?for in="topic in data.calculus"?>
<p><?get id="topic"?></p>
</?for?>
</?block?>
</?use?>
</?block?>"#,
        );
        env.write(
            "layouts/app.hrml",
            r#"<html>
<head><title><?get id="site"?></title></head>
<body>
<nav>Navigation</nav>
<?slot id="app_body"?>
</body>
</html>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({
                    "data": {
                        "algebra": ["Groups", "Rings", "Fields"],
                        "calculus": ["Limits", "Derivatives", "Integrals"]
                    }
                }),
            )
            .unwrap();
        assert_contains(&html, "<title>MathWorld</title>");
        assert_contains(&html, "<h2>Algebra</h2>");
        assert_contains(&html, "<p>Groups</p>");
        assert_contains(&html, "<p>Rings</p>");
        assert_contains(&html, "<p>Fields</p>");
        assert_contains(&html, "<h2>Calculus</h2>");
        assert_contains(&html, "<p>Limits</p>");
        assert_contains(&html, "<p>Derivatives</p>");
        assert_contains(&html, "<p>Integrals</p>");
    }

    #[test]
    fn deeply_nested_composition_chain() {
        let env = TestEnv::new("chain_deep_comp");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?>
<?load file="sections/head.hrml"?>
<?then?>
<?compose op="product"?>
<?load file="sections/left.hrml"?>
<?then?>
<?load file="sections/right.hrml"?>
</?compose?>
<?then?>
<?load file="sections/foot.hrml"?>
</?compose?>"#,
        );
        env.write("sections/head.hrml", "<header>Head</header>");
        env.write("sections/left.hrml", "<aside>Left</aside>");
        env.write("sections/right.hrml", "<main>Right</main>");
        env.write("sections/foot.hrml", "<footer>Foot</footer>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<header>Head</header>");
        assert_contains(&html, "class=\"hrml-product\"");
        assert_contains(&html, "<aside>Left</aside>");
        assert_contains(&html, "<main>Right</main>");
        assert_contains(&html, "<footer>Foot</footer>");
    }

    #[test]
    fn conditional_layout_selection() {
        let env = TestEnv::new("chain_cond_layout");
        env.write(
            "pages/test.hrml",
            r#"<?if cond="data.layout=='fullwidth'"?>
<?load file="layouts/full.hrml"?>
<?block slot="fw_content"?><h1>Full Width</h1></?block?>
</?if?>
<?if cond="data.layout=='sidebar'"?>
<?load file="layouts/sidebar.hrml"?>
<?block slot="sb_content"?><h1>Sidebar</h1></?block?>
</?if?>"#,
        );
        env.write(
            "layouts/full.hrml",
            r#"<html><body class="fullwidth"><?slot id="fw_content"?></body></html>"#,
        );
        env.write(
            "layouts/sidebar.hrml",
            r#"<html><body class="sidebar"><nav>Nav</nav><?slot id="sb_content"?></body></html>"#,
        );

        let html_fw = env
            .render_with_data(
                "pages/test.hrml",
                &json!({ "data": { "layout": "fullwidth" } }),
            )
            .unwrap();
        assert_contains(&html_fw, "class=\"fullwidth\"");
        assert_contains(&html_fw, "<h1>Full Width</h1>");
        assert_not_contains(&html_fw, "sidebar");

        let html_sb = env
            .render_with_data(
                "pages/test.hrml",
                &json!({ "data": { "layout": "sidebar" } }),
            )
            .unwrap();
        assert_contains(&html_sb, "class=\"sidebar\"");
        assert_contains(&html_sb, "<h1>Sidebar</h1>");
        assert_contains(&html_sb, "<nav>Nav</nav>");
    }
}

// ============================================================
// 7. STRESS & EDGE CASE TESTS
// ============================================================

mod stress_edge {
    use super::*;

    #[test]
    fn twenty_component_uses() {
        let env = TestEnv::new("stress_20");
        let mut template = String::from(
            "<?component id=\"item\"?><li><?slot id=\"v\"?></li></?component?>\n<ul>\n",
        );
        for i in 0..20 {
            template.push_str(&format!(
                "<?use id=\"item\"?><?block slot=\"v\"?>Item {}</?block?></?use?>\n",
                i
            ));
        }
        template.push_str("</ul>");
        env.write("pages/test.hrml", &template);
        let html = env.render("pages/test.hrml").unwrap();
        for i in 0..20 {
            assert_contains(&html, &format!("<li>Item {}</li>", i));
        }
    }

    #[test]
    fn deeply_nested_if_chain() {
        let env = TestEnv::new("stress_if_chain");
        let mut template = String::new();
        for i in 0..5 {
            template.push_str(&format!(
                "<?set id=\"level{}\"?>{}</?set?>\n",
                i, i
            ));
        }
        template.push_str("<?if cond=\"level0=='0'?>\n");
        for i in 1..5 {
            template.push_str(&format!(
                "<?if cond=\"level{}=='{}'?>\n",
                i, i
            ));
        }
        template.push_str("<p>Deep match</p>\n");
        for _ in 0..5 {
            template.push_str("</?if?>\n");
        }
        env.write("pages/test.hrml", &template);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>Deep match</p>");
    }
        template.push_str("<?if cond=\"level0=='0'?>\n");
        for i in 1..10 {
            template.push_str(&format!("<?if cond=\"level{}=='{}'?>\n", i, i));
        }
        template.push_str("<p>Deep match</p>\n");
        for _ in 0..10 {
            template.push_str("</?if?>\n");
        }
        env.write("pages/test.hrml", &template);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>Deep match</p>");
    }

    #[test]
    fn compose_with_15_segments() {
        let env = TestEnv::new("stress_comp_15");
        let mut template = String::from("<?compose op=\"sum\"?>\n");
        for i in 0..15 {
            template.push_str(&format!("<span>S{}</span>\n<?then?>\n", i));
        }
        template.push_str("</?compose?>");
        env.write("pages/test.hrml", &template);
        let html = env.render("pages/test.hrml").unwrap();
        for i in 0..15 {
            assert_contains(&html, &format!("<span>S{}</span>", i));
        }
    }

    #[test]
    fn large_for_loop_with_components() {
        let env = TestEnv::new("stress_for_comp");
        env.write(
            "pages/test.hrml",
            r#"<?component id="row"?>
<tr><td><?slot id="id"?></td><td><?slot id="name"?></td></tr>
</?component?>
<table>
<?for in="r in data.rows"?>
<?use id="row"?>
<?block slot="id"?><?get id="r.id"?></?block?>
<?block slot="name"?><?get id="r.name"?></?block?>
</?use?>
</?for?>
</table>"#,
        );
        let rows: Vec<serde_json::Value> = (0..20)
            .map(|i| json!({ "id": i, "name": format!("Row {}", i) }))
            .collect();
        let html = env
            .render_with_data("pages/test.hrml", &json!({ "data": { "rows": rows } }))
            .unwrap();
        assert_contains(&html, "<td>0</td>");
        assert_contains(&html, "<td>Row 0</td>");
        assert_contains(&html, "<td>19</td>");
        assert_contains(&html, "<td>Row 19</td>");
        assert_count(&html, "<tr>", 20);
    }

    #[test]
    fn many_set_get_variables() {
        let env = TestEnv::new("stress_vars");
        let mut template = String::new();
        for i in 0..30 {
            template.push_str(&format!("<?set id=\"v{}\"?>value{}</?set?>\n", i, i));
        }
        for i in 0..30 {
            template.push_str(&format!("<p><?get id=\"v{}\"?></p>\n", i));
        }
        env.write("pages/test.hrml", &template);
        let html = env.render("pages/test.hrml").unwrap();
        for i in 0..30 {
            assert_contains(&html, &format!("<p>value{}</p>", i));
        }
    }

    #[test]
    fn empty_for_loop_graceful() {
        let env = TestEnv::new("stress_empty_for");
        env.write(
            "pages/test.hrml",
            r#"<ul>
<?for in="x in data.empty"?>
<li><?get id="x"?></li>
</?for?>
</ul>
<p>After</p>"#,
        );
        let html = env
            .render_with_data("pages/test.hrml", &json!({ "data": { "empty": [] } }))
            .unwrap();
        assert_contains(&html, "<ul>");
        assert_contains(&html, "</ul>");
        assert_contains(&html, "<p>After</p>");
        assert_not_contains(&html, "<li>");
    }

    #[test]
    fn bind_scope_isolation_in_for() {
        let env = TestEnv::new("stress_bind_isolate");
        env.write(
            "pages/test.hrml",
            r#"<?set id="outer"?>OUTER</?set?>
<?for in="item in data.items"?>
<?bind var="outer" from="item"?>
<p><?get id="outer"?></p>
</?bind?>
</?for?>
<p><?get id="outer"?></p>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({ "data": { "items": ["A", "B", "C"] } }),
            )
            .unwrap();
        let parts: Vec<&str> = html.split("<p>").collect();
        let values: Vec<&str> = parts
            .iter()
            .skip(1)
            .filter_map(|p| p.split("</p>").next())
            .collect();
        assert_eq!(values, vec!["A", "B", "C", "OUTER"]);
    }

    #[test]
    fn special_characters_in_attributes() {
        let env = TestEnv::new("stress_special_attr");
        env.write(
            "pages/test.hrml",
            r##"<?btn post="/api/test?a=1&b=2" target="#result" swap="innerHTML"?>
Click &amp; Go
</?btn?>"##,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "data-post=\"/api/test?a=1&b=2\"");
        assert_contains(&html, "Click");
    }

    #[test]
    fn unicode_in_all_positions() {
        let env = TestEnv::new("stress_unicode");
        env.write(
            "pages/test.hrml",
            r#"<?set id="greeting"?>こんにちは</?set?>
<h1><?get id="greeting"?></h1>
<p>Ελληνικά: αβγδε</p>
<p>العربية: مرحبا</p>
<p>Русский: привет</p>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "こんにちは");
        assert_contains(&html, "Ελληνικά");
        assert_contains(&html, "العربية");
        assert_contains(&html, "Русский");
    }

    #[test]
    fn mixed_directives_in_single_template() {
        let env = TestEnv::new("stress_mixed");
        env.write(
            "pages/test.hrml",
            r#"<?charset?>
<?viewport?>
<?title value="Mixed Test"?>
<?set id="x"?>42</?set?>
<?set id="y"?>hello</?set?>
<?if cond="x=='42'"?>
<p><?get id="y"?></p>
<?else?>
<p>wrong</p>
</?if?>
<?for in="i in data.nums"?>
<span><?get id="i"?></span>
</?for?>
<?compose op="sum"?>
<a>First</a>
<?then?>
<a>Second</a>
</?compose?>
<?btn post="/api/go"?>Go</?btn?>
<?wasm module="/app.wasm"?>"#,
        );
        let html = env
            .render_with_data("pages/test.hrml", &json!({ "data": { "nums": [1, 2, 3] } }))
            .unwrap();
        assert_contains(&html, "<meta charset=\"UTF-8\">");
        assert_contains(&html, "name=\"viewport\"");
        assert_contains(&html, "<title>Mixed Test</title>");
        assert_contains(&html, "<p>hello</p>");
        assert_contains(&html, "<span>1</span>");
        assert_contains(&html, "<span>2</span>");
        assert_contains(&html, "<span>3</span>");
        assert_contains(&html, "<a>First</a><a>Second</a>");
        assert_contains(&html, "data-post=\"/api/go\"");
        assert_contains(&html, "data-wasm-module=\"/app.wasm\"");
    }
}

// ============================================================
// 8. REGRESSION: NO DIRECTIVE LEAKAGE
// ============================================================

mod regression_no_leak {
    use super::*;

    #[test]
    fn no_php_style_tags_in_output() {
        let env = TestEnv::new("reg_no_leak");
        env.write(
            "pages/test.hrml",
            r##"<?load file="components/x.hrml"?>
<?set id="x"?>val</?set?>
<?get id="x"?>
<?if cond="x"?>yes<?else?>no</?if?>
<?compose op="sum"?>a<?then?>b</?compose?>
<?for in="i in data.items"?><?get id="i"?></?for?>
<?btn post="/api/x"?>Go</?btn?>
<?form post="/api/x"?>input</?form?>
<?wasm module="/app.wasm"?>
<?link get="/page" target="#out"?>Link</?link?>"##,
        );
        env.write("components/x.hrml", "<div>X</div>");
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({ "data": { "items": ["a", "b"] } }),
            )
            .unwrap();
        assert_not_contains(&html, "<?");
        assert_not_contains(&html, "?>");
    }

    #[test]
    fn no_slot_or_block_tags_in_output() {
        let env = TestEnv::new("reg_no_slot");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/base.hrml"?>
<?block slot="content"?>Done</?block?>"#,
        );
        env.write(
            "layouts/base.hrml",
            r#"<div><?slot id="content"?>Default</?slot?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?slot");
        assert_not_contains(&html, "<?block");
        assert_not_contains(&html, "</?slot");
        assert_not_contains(&html, "</?block");
    }

    #[test]
    fn no_component_definition_in_output() {
        let env = TestEnv::new("reg_no_comp_def");
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
    fn no_double_html_wrap() {
        let env = TestEnv::new("reg_no_double");
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
        assert_eq!(html.matches("<!DOCTYPE html").count(), 1);
        assert_eq!(html.matches("<html").count(), 1);
        assert_eq!(html.matches("<body").count(), 1);
    }
}
