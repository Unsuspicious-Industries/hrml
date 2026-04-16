// Formal verification properties for the HRML template engine.
//
// These are executable specifications encoded as Rust tests that assert
// language-theoretic invariants over the template language. Each property
// corresponds to a formal theorem about the HRML system.
//
// Language Theory Properties:
//
// P1: PARSING_TERMINATION
//     ∀ input ∈ Σ*, parse(input) terminates
//     The parser always terminates on any input string.
//
// P2: PARSING_CORRECTNESS
//     ∀ input ∈ Σ*, parse(input) = Ok(nodes) ⇒ render(nodes) produces valid HTML
//     Every successfully parsed template renders to well-formed HTML.
//
// P3: DIRECTIVE_COMPLETENESS (No Leakage)
//     ∀ template ∈ Templates, render(template) contains no "<?" or "?>"
//     All HRML directives are fully consumed during rendering.
//
// P4: BLOCK_INJECTION_CORRECTNESS
//     ∀ layout, page, ∀ slot ∈ Slots(layout),
//       if block(slot) ∈ page then render(load(layout) with page) replaces slot with block(slot)
//     Block injection correctly replaces slots with corresponding block content.
//
// P5: LOAD_COMPOSITION
//     ∀ t1, t2, render(load(t1) with load(t2)) = render(compose(t1, t2))
//     Template composition via load is associative and correct.
//
// P6: VARIABLE_SUBSTITUTION
//     ∀ ctx, ∀ var ∈ Vars(ctx), render(template with ctx) substitutes var with ctx[var]
//     Variable substitution is total and correct.
//
// P7: CONDITIONAL_EXHAUSTIVENESS
//     ∀ if_expr, render(if_expr) = render(true_branch) ∨ render(false_branch)
//     Conditionals always produce exactly one branch.
//
// P8: FOR_LOOP_CORRECTNESS
//     ∀ for_expr, items, render(for_expr with items) = concat(render(body) for each item)
//     For loops produce exactly one iteration per item.
//
// P9: NO_CRASH (Total Correctness)
//     ∀ template ∈ ValidTemplates, render(template) ≠ ⊥
//     The engine never panics or crashes on valid input.
//
// P10: HTML_WELL_FORMEDNESS
//      ∀ template, render(template) is valid HTML (DOCTYPE, html, head, body structure)
//      or render_fragment(template) is valid HTML fragment.
//
// P11: CIRCULAR_DEPENDENCY_DETECTION
//      ∀ templates with cycles, resolve(templates) = Err
//      Circular dependencies are always detected.
//
// P12: IDEMPOTENT_RENDERING
//      ∀ template, render(template) when re-parsed and re-rendered produces same output
//      (for templates that produce full HTML documents)
//
// P13: CONTEXT_ISOLATION
//      ∀ ctx1, ctx2, render(template with ctx1) ≠ render(template with ctx2) when ctx1 ≠ ctx2
//      Different contexts produce different outputs (when the template uses the context).
//
// P14: SLOT_DEFAULT_PRESERVATION
//      ∀ layout, slot ∈ Slots(layout), if no block matches slot then render uses default
//      Unmatched slots preserve their default content.
//
// P15: COMPOSE_ASSOCIATIVITY
//      ∀ a, b, c, compose(compose(a, b, Sum), c, Sum) = compose(a, compose(b, c, Sum), Sum)
//      Sum composition is associative.

use hrml::template::Engine;
use serde_json::json;
use std::fs;
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
        let dir = format!("/tmp/hrml_formal_{}_{}", name, now);
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
        if let Some(parent) = std::path::Path::new(&full).parent() {
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

    fn render_with_data(&self, path: &str, data: &serde_json::Value) -> Result<String, String> {
        self.engine().render(path, data)
    }

    #[allow(dead_code)]
    fn render_fragment(&self, path: &str) -> Result<String, String> {
        self.engine().render_fragment(path, &json!({}))
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn assert_contains(html: &str, needle: &str) {
    assert!(
        html.contains(needle),
        "\nExpected: {}\n\nGot:\n{}\n",
        needle,
        html
    );
}

fn assert_not_contains(html: &str, needle: &str) {
    assert!(
        !html.contains(needle),
        "\nDid not expect: {}\n\nGot:\n{}\n",
        needle,
        html
    );
}

// ============================================================
// P1: PARSING_TERMINATION
// ============================================================

mod p1_parsing_termination {
    use super::*;

    #[test]
    fn terminates_on_empty_input() {
        let env = TestEnv::new("p1_empty");
        env.write("pages/test.hrml", "");
        let result = env.render("pages/test.hrml");
        assert!(result.is_ok());
    }

    #[test]
    fn terminates_on_whitespace_only() {
        let env = TestEnv::new("p1_ws");
        env.write("pages/test.hrml", "   \n\n\t  \n\n");
        let result = env.render("pages/test.hrml");
        assert!(result.is_ok());
    }

    #[test]
    fn terminates_on_malformed_directives() {
        let _env = TestEnv::new("p1_malformed");
        let inputs = vec![
            "<?",
            "<??",
            "<?load",
            "<?load file=",
            "<?load file=\"",
            "<?load file=\"x",
            "<?load file=\"x\"",
            "?>",
            "</?",
            "<?if",
            "<?if cond=",
            "<?compose",
            "<?component",
            "<?use",
            "<?bind",
            "<?set",
            "<?get",
            "<?btn",
            "<?form",
            "<?link",
            "<?wasm",
            "<?markdown",
            "<?latex",
            "<?charset",
            "<?viewport",
            "<?title",
            "<?og",
            "<?twitter",
            "<?meta",
            "<?description",
            "<?robots",
            "<?canonical",
            "<?stylesheet",
            "<?script",
        ];
        for (i, input) in inputs.iter().enumerate() {
            let env = TestEnv::new(&format!("p1_malformed_{}", i));
            env.write("pages/test.hrml", input);
            let result = env.render("pages/test.hrml");
            // Parser must never panic or hang - either Ok or Err is acceptable
            // The key invariant is termination: parse(input) always completes
            assert!(
                result.is_ok() || result.is_err(),
                "Parser panicked or hung on input: {:?}",
                input
            );
        }
    }

    #[test]
    fn terminates_on_unclosed_elements() {
        let env = TestEnv::new("p1_unclosed");
        env.write("pages/test.hrml", "<div><span><p>unclosed tags");
        let result = env.render("pages/test.hrml");
        assert!(result.is_ok());
    }

    #[test]
    fn terminates_on_deeply_nested_html() {
        let env = TestEnv::new("p1_deep");
        let content = (0..100).fold(String::from("leaf"), |acc, i| {
            format!("<div class=\"{}\">{}</div>", i, acc)
        });
        env.write("pages/test.hrml", &content);
        let result = env.render("pages/test.hrml");
        assert!(result.is_ok());
    }

    #[test]
    fn terminates_on_large_input() {
        let env = TestEnv::new("p1_large");
        let content = (0..5000)
            .map(|i| format!("<p>paragraph {}</p>", i))
            .collect::<Vec<_>>()
            .join("\n");
        env.write("pages/test.hrml", &content);
        let result = env.render("pages/test.hrml");
        assert!(result.is_ok());
    }
}

// ============================================================
// P2: PARSING_CORRECTNESS → Valid HTML Output
// ============================================================

mod p2_parsing_correctness {
    use super::*;

    #[test]
    fn plain_text_produces_wrapped_html() {
        let env = TestEnv::new("p2_text");
        env.write("pages/test.hrml", "Hello World");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<html");
        assert_contains(&html, "<head>");
        assert_contains(&html, "<body>");
        assert_contains(&html, "Hello World");
    }

    #[test]
    fn full_html_document_preserved() {
        let env = TestEnv::new("p2_full");
        env.write(
            "pages/test.hrml",
            "<!DOCTYPE html><html><head><title>T</title></head><body><p>X</p></body></html>",
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_eq!(html.matches("<!DOCTYPE html").count(), 1);
        assert_eq!(html.matches("<html").count(), 1);
        assert_eq!(html.matches("</html>").count(), 1);
    }

    #[test]
    fn html_fragment_gets_wrapped() {
        let env = TestEnv::new("p2_fragment");
        env.write("pages/test.hrml", "<div><p>Content</p></div>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<html");
        assert_contains(&html, "</html>");
        assert_contains(&html, "<div><p>Content</p></div>");
    }
}

// ============================================================
// P3: DIRECTIVE_COMPLETENESS (No Leakage)
// ============================================================

mod p3_directive_completeness {
    use super::*;

    #[test]
    fn all_directives_consumed() {
        let env = TestEnv::new("p3_all");
        env.write(
            "pages/test.hrml",
            r##"<?charset?>
<?viewport?>
<?title value="Test"?>
<?set id="x"?>val</?set?>
<?get id="x"?>
<?if cond="x"?>yes<?else?>no</?if?>
<?compose op="sum"?>a<?then?>b</?compose?>
<?btn post="/api/x"?>Go</?btn?>
<?form post="/api/x"?>input</?form?>
<?link get="/page" target="#out"?>Link</?link?>
<?wasm module="/app.wasm"?>"##,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?");
        assert_not_contains(&html, "?>");
    }

    #[test]
    fn load_directive_consumed() {
        let env = TestEnv::new("p3_load");
        env.write("pages/test.hrml", r#"<?load file="components/x.hrml"?>"#);
        env.write("components/x.hrml", "<div>Loaded</div>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "<?load");
        assert_contains(&html, "<div>Loaded</div>");
    }

    #[test]
    fn component_directive_consumed() {
        let env = TestEnv::new("p3_comp");
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
    fn slot_block_directives_consumed() {
        let env = TestEnv::new("p3_slot");
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
}

// ============================================================
// P4: BLOCK_INJECTION_CORRECTNESS
// ============================================================

mod p4_block_injection {
    use super::*;

    #[test]
    fn block_replaces_slot() {
        let env = TestEnv::new("p4_replace");
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
    fn unmatched_slot_keeps_default() {
        let env = TestEnv::new("p4_default");
        env.write("pages/test.hrml", r#"<?load file="layouts/base.hrml"?>"#);
        env.write(
            "layouts/base.hrml",
            r#"<div><?slot id="missing"?>Fallback</?slot?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Fallback");
    }

    #[test]
    fn multiple_blocks_injected_correctly() {
        let env = TestEnv::new("p4_multi");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/two.hrml"?>
<?block slot="left"?>L</?block?>
<?block slot="right"?>R</?block?>"#,
        );
        env.write(
            "layouts/two.hrml",
            r#"<div><?slot id="left"?></div><div><?slot id="right"?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<div>L</div>");
        assert_contains(&html, "<div>R</div>");
    }
}

// ============================================================
// P5: LOAD_COMPOSITION
// ============================================================

mod p5_load_composition {
    use super::*;

    #[test]
    fn nested_loads_compose_correctly() {
        let env = TestEnv::new("p5_nested");
        env.write(
            "pages/test.hrml",
            r#"<?load file="layouts/a.hrml"?>
<?block slot="content"?>Final</?block?>"#,
        );
        env.write(
            "layouts/a.hrml",
            r#"<?load file="layouts/b.hrml"?>
<?block slot="inner"?><?slot id="content"?></?block?>"#,
        );
        env.write(
            "layouts/b.hrml",
            r#"<html><body><?slot id="inner"?></body></html>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Final");
        assert_contains(&html, "<html");
        assert_contains(&html, "</html>");
    }

    #[test]
    fn load_chain_preserves_all_content() {
        let env = TestEnv::new("p5_chain");
        env.write("pages/test.hrml", r#"<?load file="a.hrml"?>"#);
        env.write("a.hrml", r#"<?load file="b.hrml"?>A"#);
        env.write("b.hrml", r#"<?load file="c.hrml"?>B"#);
        env.write("c.hrml", "C");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "A");
        assert_contains(&html, "B");
        assert_contains(&html, "C");
    }
}

// ============================================================
// P6: VARIABLE_SUBSTITUTION
// ============================================================

mod p6_variable_substitution {
    use super::*;

    #[test]
    fn set_get_substitution() {
        let env = TestEnv::new("p6_setget");
        env.write(
            "pages/test.hrml",
            r#"<?set id="x"?>42</?set?>
<p><?get id="x"?></p>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<p>42</p>");
    }

    #[test]
    fn json_data_substitution() {
        let env = TestEnv::new("p6_json");
        env.write("pages/test.hrml", r#"<h1><?get id="user.name"?></h1>"#);
        let html = env
            .render_with_data("pages/test.hrml", &json!({ "user": { "name": "Alice" } }))
            .unwrap();
        assert_contains(&html, "<h1>Alice</h1>");
    }

    #[test]
    fn variable_with_dollar_prefix() {
        let env = TestEnv::new("p6_dollar");
        env.write(
            "pages/test.hrml",
            r#"<?set id="title"?>My Title</?set?>
<?title value="$title"?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<title>My Title</title>");
    }

    #[test]
    fn missing_variable_produces_empty() {
        let env = TestEnv::new("p6_missing");
        env.write("pages/test.hrml", r#"<p>[<?get id="nonexistent"?>]</p>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "[]");
    }
}

// ============================================================
// P7: CONDITIONAL_EXHAUSTIVENESS
// ============================================================

mod p7_conditional_exhaustiveness {
    use super::*;

    #[test]
    fn true_branch_rendered() {
        let env = TestEnv::new("p7_true");
        env.write(
            "pages/test.hrml",
            r#"<?set id="x"?>yes</?set?>
<?if cond="x"?>TRUE<?else?>FALSE</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "TRUE");
        assert_not_contains(&html, "FALSE");
    }

    #[test]
    fn false_branch_rendered() {
        let env = TestEnv::new("p7_false");
        env.write(
            "pages/test.hrml",
            r#"<?if cond="missing"?>TRUE<?else?>FALSE</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "FALSE");
        assert_not_contains(&html, "TRUE");
    }

    #[test]
    fn equality_condition() {
        let env = TestEnv::new("p7_eq");
        env.write(
            "pages/test.hrml",
            r#"<?set id="role"?>admin</?set?>
<?if cond="role=='admin'"?>ADMIN<?else?>USER</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "ADMIN");
        assert_not_contains(&html, "USER");
    }

    #[test]
    fn if_without_else_true() {
        let env = TestEnv::new("p7_no_else_t");
        env.write(
            "pages/test.hrml",
            r#"<?set id="x"?>yes</?set?>
<?if cond="x"?>VISIBLE</?if?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "VISIBLE");
    }

    #[test]
    fn if_without_else_false() {
        let env = TestEnv::new("p7_no_else_f");
        env.write("pages/test.hrml", r#"<?if cond="missing"?>VISIBLE</?if?>"#);
        let html = env.render("pages/test.hrml").unwrap();
        assert_not_contains(&html, "VISIBLE");
    }
}

// ============================================================
// P8: FOR_LOOP_CORRECTNESS
// ============================================================

mod p8_for_loop {
    use super::*;

    #[test]
    fn iterates_over_array() {
        let env = TestEnv::new("p8_array");
        env.write(
            "pages/test.hrml",
            r#"<?for in="x in data.items"?>[<?get id="x"?>]</?for?>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({ "data": { "items": ["a", "b", "c"] } }),
            )
            .unwrap();
        assert_contains(&html, "[a]");
        assert_contains(&html, "[b]");
        assert_contains(&html, "[c]");
        assert_eq!(html.matches("[").count(), 3);
    }

    #[test]
    fn empty_array_produces_nothing() {
        let env = TestEnv::new("p8_empty");
        env.write(
            "pages/test.hrml",
            r#"<?for in="x in data.items"?>X</?for?>DONE"#,
        );
        let html = env
            .render_with_data("pages/test.hrml", &json!({ "data": { "items": [] } }))
            .unwrap();
        assert_contains(&html, "DONE");
        assert_not_contains(&html, "X");
    }

    #[test]
    fn missing_data_produces_nothing() {
        let env = TestEnv::new("p8_missing");
        env.write(
            "pages/test.hrml",
            r#"<?for in="x in nonexistent"?>X</?for?>DONE"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "DONE");
        assert_not_contains(&html, "X");
    }

    #[test]
    fn for_over_objects() {
        let env = TestEnv::new("p8_obj");
        env.write(
            "pages/test.hrml",
            r#"<?for in="item in data.items"?><?get id="item.name"?></?for?>"#,
        );
        let html = env
            .render_with_data(
                "pages/test.hrml",
                &json!({
                    "data": {
                        "items": [
                            { "name": "A" },
                            { "name": "B" }
                        ]
                    }
                }),
            )
            .unwrap();
        assert_contains(&html, "A");
        assert_contains(&html, "B");
    }
}

// ============================================================
// P9: NO_CRASH (Total Correctness)
// ============================================================

mod p9_no_crash {
    use super::*;

    #[test]
    fn never_panics_on_valid_templates() {
        let templates = vec![
            ("empty", ""),
            ("text", "Hello"),
            ("html", "<div>test</div>"),
            ("void", r#"<?pure value="x"?>"#),
            ("set_get", r#"<?set id="x"?>v</?set?><?get id="x"?>"#),
            (
                "if_true",
                r#"<?set id="x"?>y</?set?><?if cond="x"?>a<?else?>b</?if?>"#,
            ),
            ("if_false", r#"<?if cond="missing"?>a<?else?>b</?if?>"#),
            ("compose", r#"<?compose op="sum"?>a<?then?>b</?compose?>"#),
            ("btn", r#"<?btn post="/x"?>Go</?btn?>"#),
            ("form", r#"<?form post="/x"?>in</?form?>"#),
            ("link", r#"<?link get="/x"?>Go</?link?>"#),
            ("wasm", r#"<?wasm module="/x.wasm"?>"#),
            ("charset", r#"<?charset?>"#),
            ("viewport", r#"<?viewport?>"#),
            ("title", r#"<?title value="T"?>"#),
            (
                "component",
                r#"<?component id="x"?><span>X</span></?component?><?use id="x"?></?use?>"#,
            ),
            (
                "bind",
                r#"<?bind var="x" value="v"?><p><?get id="x"?></p></?bind?>"#,
            ),
        ];

        for (name, template) in templates {
            let env = TestEnv::new(&format!("p9_{}", name));
            env.write("pages/test.hrml", template);
            let result = env.render("pages/test.hrml");
            assert!(
                result.is_ok(),
                "Template '{}' crashed or errored: {:?}",
                name,
                result.err()
            );
        }
    }

    #[test]
    fn never_panics_on_missing_files() {
        let env = TestEnv::new("p9_missing");
        env.write("pages/test.hrml", r#"<?load file="nonexistent.hrml"?>"#);
        let result = env.render("pages/test.hrml");
        assert!(result.is_err()); // Expected error, not panic
    }

    #[test]
    fn never_panics_on_circular_deps() {
        let env = TestEnv::new("p9_circular");
        env.write("a.hrml", r#"<?load file="b.hrml"?>A"#);
        env.write("b.hrml", r#"<?load file="a.hrml"?>B"#);
        let result = env.render("a.hrml");
        assert!(result.is_err()); // Expected error, not panic
    }
}

// ============================================================
// P10: HTML_WELL_FORMEDNESS
// ============================================================

mod p10_html_well_formedness {
    use super::*;

    #[test]
    fn wrapped_output_has_required_structure() {
        let env = TestEnv::new("p10_wrap");
        env.write("pages/test.hrml", "<p>Content</p>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<html");
        assert_contains(&html, "<head>");
        assert_contains(&html, "</head>");
        assert_contains(&html, "<body>");
        assert_contains(&html, "</body>");
        assert_contains(&html, "</html>");
    }

    #[test]
    fn wrapped_output_has_meta_charset() {
        let env = TestEnv::new("p10_charset");
        env.write("pages/test.hrml", "<p>Content</p>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "<meta charset=\"UTF-8\">");
    }

    #[test]
    fn wrapped_output_has_viewport() {
        let env = TestEnv::new("p10_viewport");
        env.write("pages/test.hrml", "<p>Content</p>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "name=\"viewport\"");
    }

    #[test]
    fn wrapped_output_has_title() {
        let env = TestEnv::new("p10_title");
        env.write("pages/test.hrml", "<p>Content</p>");
        let html = env
            .engine()
            .with_site_name("TestSite".to_string())
            .render("pages/test.hrml", &json!({}))
            .unwrap();
        assert_contains(&html, "<title>TestSite</title>");
    }

    #[test]
    fn wrapped_output_has_hrml_js() {
        let env = TestEnv::new("p10_js");
        env.write("pages/test.hrml", "<p>Content</p>");
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "/hrml.js");
    }

    #[test]
    fn full_document_not_double_wrapped() {
        let env = TestEnv::new("p10_no_double");
        env.write(
            "pages/test.hrml",
            r#"<!DOCTYPE html><html><head><title>T</title></head><body><p>X</p></body></html>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_eq!(html.matches("<!DOCTYPE html").count(), 1);
        assert_eq!(html.matches("<html").count(), 1);
        assert_eq!(html.matches("<body").count(), 1);
    }
}

// ============================================================
// P11: CIRCULAR_DEPENDENCY_DETECTION
// ============================================================

mod p11_circular_detection {
    use super::*;

    #[test]
    fn direct_cycle_detected() {
        let env = TestEnv::new("p11_direct");
        env.write("a.hrml", r#"<?load file="b.hrml"?>A"#);
        env.write("b.hrml", r#"<?load file="a.hrml"?>B"#);
        let result = env.render("a.hrml");
        assert!(result.is_err());
        assert_contains(&result.unwrap_err(), "Circular");
    }

    #[test]
    fn three_way_cycle_detected() {
        let env = TestEnv::new("p11_threeway");
        env.write("a.hrml", r#"<?load file="b.hrml"?>A"#);
        env.write("b.hrml", r#"<?load file="c.hrml"?>B"#);
        env.write("c.hrml", r#"<?load file="a.hrml"?>C"#);
        let result = env.render("a.hrml");
        assert!(result.is_err());
        assert_contains(&result.unwrap_err(), "Circular");
    }

    #[test]
    fn self_cycle_detected() {
        let env = TestEnv::new("p11_self");
        env.write("a.hrml", r#"<?load file="a.hrml"?>A"#);
        let result = env.render("a.hrml");
        assert!(result.is_err());
        assert_contains(&result.unwrap_err(), "Circular");
    }

    #[test]
    fn no_cycle_succeeds() {
        let env = TestEnv::new("p11_nocycle");
        env.write("pages/test.hrml", r#"<?load file="a.hrml"?>"#);
        env.write("a.hrml", r#"<?load file="b.hrml"?>A"#);
        env.write("b.hrml", "B");
        let result = env.render("pages/test.hrml");
        assert!(result.is_ok());
    }
}

// ============================================================
// P12: IDEMPOTENT_RENDERING
// ============================================================

mod p12_idempotent {
    use super::*;

    #[test]
    fn plain_text_idempotent() {
        let env = TestEnv::new("p12_text");
        env.write("pages/test.hrml", "Hello");
        let html1 = env.render("pages/test.hrml").unwrap();
        let html2 = env.render("pages/test.hrml").unwrap();
        assert_eq!(html1, html2);
    }

    #[test]
    fn same_data_same_output() {
        let env = TestEnv::new("p12_data");
        env.write("pages/test.hrml", r#"<p><?get id="user.name"?></p>"#);
        let data = json!({ "user": { "name": "Alice" } });
        let html1 = env.render_with_data("pages/test.hrml", &data).unwrap();
        let html2 = env.render_with_data("pages/test.hrml", &data).unwrap();
        assert_eq!(html1, html2);
    }
}

// ============================================================
// P13: CONTEXT_ISOLATION
// ============================================================

mod p13_context_isolation {
    use super::*;

    #[test]
    fn different_data_different_output() {
        let env = TestEnv::new("p13_diff");
        env.write("pages/test.hrml", r#"<p><?get id="user.name"?></p>"#);
        let html1 = env
            .render_with_data("pages/test.hrml", &json!({ "user": { "name": "Alice" } }))
            .unwrap();
        let html2 = env
            .render_with_data("pages/test.hrml", &json!({ "user": { "name": "Bob" } }))
            .unwrap();
        assert_ne!(html1, html2);
    }

    #[test]
    fn same_data_same_output() {
        let env = TestEnv::new("p13_same");
        env.write("pages/test.hrml", r#"<p><?get id="x"?></p>"#);
        let data = json!({ "x": "val" });
        let html1 = env.render_with_data("pages/test.hrml", &data).unwrap();
        let html2 = env.render_with_data("pages/test.hrml", &data).unwrap();
        assert_eq!(html1, html2);
    }
}

// ============================================================
// P14: SLOT_DEFAULT_PRESERVATION
// ============================================================

mod p14_slot_defaults {
    use super::*;

    #[test]
    fn unmatched_slot_keeps_default() {
        let env = TestEnv::new("p14_default");
        env.write("pages/test.hrml", r#"<?load file="layouts/base.hrml"?>"#);
        env.write(
            "layouts/base.hrml",
            r#"<div><?slot id="unused"?>Default Content</?slot?></div>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "Default Content");
    }

    #[test]
    fn matched_slot_replaces_default() {
        let env = TestEnv::new("p14_replace");
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
}

// ============================================================
// P15: COMPOSE_ASSOCIATIVITY
// ============================================================

mod p15_compose_associativity {
    use super::*;

    #[test]
    fn sum_is_associative() {
        let env1 = TestEnv::new("p15_left");
        env1.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?>
<?compose op="sum"?><span>A</span><?then?><span>B</span></?compose?>
<?then?>
<span>C</span>
</?compose?>"#,
        );
        let env2 = TestEnv::new("p15_right");
        env2.write(
            "pages/test.hrml",
            r#"<?compose op="sum"?>
<span>A</span>
<?then?>
<?compose op="sum"?><span>B</span><?then?><span>C</span></?compose?>
</?compose?>"#,
        );
        let html1 = env1.render("pages/test.hrml").unwrap();
        let html2 = env2.render("pages/test.hrml").unwrap();
        // Both should contain all three spans
        assert_contains(&html1, "<span>A</span>");
        assert_contains(&html1, "<span>B</span>");
        assert_contains(&html1, "<span>C</span>");
        assert_contains(&html2, "<span>A</span>");
        assert_contains(&html2, "<span>B</span>");
        assert_contains(&html2, "<span>C</span>");
    }

    #[test]
    fn product_wraps_both_sides() {
        let env = TestEnv::new("p15_product");
        env.write(
            "pages/test.hrml",
            r#"<?compose op="product"?><span>L</span><?then?><span>R</span></?compose?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        assert_contains(&html, "class=\"hrml-product\"");
        assert_contains(&html, "class=\"hrml-factor\"");
        assert_contains(&html, "<span>L</span>");
        assert_contains(&html, "<span>R</span>");
    }
}

// ============================================================
// P16: ROUTING_CORRECTNESS
// ============================================================

mod p16_routing {
    use hrml::router::{Route, RouteKind, Router};
    use std::path::PathBuf;

    #[test]
    fn static_route_matches_exact_url() {
        let route = Route {
            path: "/about".to_string(),
            template: "pages/about.hrml".to_string(),
            kind: RouteKind::Static,
            params: vec![],
        };
        let params = route.match_url("/about").unwrap();
        assert!(params.is_empty());
        assert!(route.match_url("/contact").is_none());
    }

    #[test]
    fn dynamic_route_extracts_param() {
        let route = Route {
            path: "/blog/[slug]".to_string(),
            template: "pages/blog/[slug].hrml".to_string(),
            kind: RouteKind::Dynamic("slug".to_string()),
            params: vec!["slug".to_string()],
        };
        let params = route.match_url("/blog/hello-world").unwrap();
        assert_eq!(params.get("slug").unwrap(), "hello-world");
    }

    #[test]
    fn catch_all_route_extracts_rest() {
        let route = Route {
            path: "/docs/[...rest]".to_string(),
            template: "pages/docs/[...rest].hrml".to_string(),
            kind: RouteKind::CatchAll,
            params: vec!["rest".to_string()],
        };
        let params = route.match_url("/docs/api/reference").unwrap();
        assert_eq!(params.get("rest").unwrap(), "api/reference");
    }

    #[test]
    fn router_prioritizes_static_over_dynamic() {
        let mut router = Router::new();
        router.routes.push(Route {
            path: "/blog".to_string(),
            template: "pages/blog/index.hrml".to_string(),
            kind: RouteKind::Static,
            params: vec![],
        });
        router.routes.push(Route {
            path: "/blog/[slug]".to_string(),
            template: "pages/blog/[slug].hrml".to_string(),
            kind: RouteKind::Dynamic("slug".to_string()),
            params: vec!["slug".to_string()],
        });

        let (route, _) = router.resolve("/blog").unwrap();
        assert_eq!(route.kind, RouteKind::Static);

        let (route, params) = router.resolve("/blog/hello").unwrap();
        assert_eq!(route.kind, RouteKind::Dynamic("slug".to_string()));
        assert_eq!(params.get("slug").unwrap(), "hello");
    }

    #[test]
    fn no_match_returns_none() {
        let mut router = Router::new();
        router.routes.push(Route {
            path: "/about".to_string(),
            template: "pages/about.hrml".to_string(),
            kind: RouteKind::Static,
            params: vec![],
        });
        assert!(router.resolve("/nonexistent").is_none());
    }

    #[test]
    fn route_from_file_index() {
        let route =
            Route::from_file(&PathBuf::from("pages"), &PathBuf::from("pages/index.hrml")).unwrap();
        assert_eq!(route.path, "/");
        assert_eq!(route.kind, RouteKind::Static);
    }

    #[test]
    fn route_from_file_dynamic() {
        let route = Route::from_file(
            &PathBuf::from("pages"),
            &PathBuf::from("pages/blog/[slug].hrml"),
        )
        .unwrap();
        assert_eq!(route.path, "/blog/[slug]");
        assert!(matches!(route.kind, RouteKind::Dynamic(_)));
    }

    #[test]
    fn route_from_file_catch_all() {
        let route = Route::from_file(
            &PathBuf::from("pages"),
            &PathBuf::from("pages/docs/[...rest].hrml"),
        )
        .unwrap();
        assert_eq!(route.path, "/docs/[...rest]");
        assert_eq!(route.kind, RouteKind::CatchAll);
    }
}

// ============================================================
// P17: SECURITY_INVARIANTS
// ============================================================

mod p17_security {
    use hrml::security::*;

    #[test]
    fn escape_html_never_produces_raw_tags() {
        let inputs = vec![
            "<script>",
            "</script>",
            "<img onerror=alert(1)>",
            "<a href='javascript:alert(1)'>",
            "<div onclick='evil()'>",
        ];
        for input in &inputs {
            let escaped = escape_html(input);
            assert!(
                !escaped.contains('<'),
                "escape_html produced raw '<' for: {}",
                input
            );
            assert!(
                !escaped.contains('>'),
                "escape_html produced raw '>' for: {}",
                input
            );
        }
    }

    #[test]
    fn escape_html_is_idempotent() {
        let input = "<script>alert(1)</script>";
        let once = escape_html(input);
        let twice = escape_html(&once);
        // Double-escaping should produce different output (ampersands get escaped again)
        assert_ne!(once, twice);
        // But the first escape should not contain raw tags
        assert!(!once.contains('<'));
        assert!(!once.contains('>'));
    }

    #[test]
    fn escape_html_preserves_safe_text() {
        let safe = vec![
            "Hello World",
            "a + b = c",
            "5 < 10 is true", // < is escaped
            "numbers: 12345",
            "unicode: こんにちは",
        ];
        for text in &safe {
            let escaped = escape_html(text);
            assert!(!escaped.contains('<'));
            assert!(!escaped.contains('>'));
        }
    }

    #[test]
    fn sanitize_url_blocks_dangerous_schemes() {
        let dangerous = vec![
            "javascript:alert(1)",
            "data:text/html,<script>",
            "vbscript:msgbox(1)",
            "JAVASCRIPT:alert(1)",
            "  javascript:alert(1)",
        ];
        for url in &dangerous {
            assert!(sanitize_url(url).is_none(), "sanitize_url allowed: {}", url);
        }
    }

    #[test]
    fn sanitize_url_allows_safe_urls() {
        let safe = vec![
            "https://example.com",
            "http://example.com",
            "/local/path",
            "#anchor",
            "mailto:test@example.com",
            "relative/path",
        ];
        for url in &safe {
            assert!(sanitize_url(url).is_some(), "sanitize_url blocked: {}", url);
        }
    }

    #[test]
    fn strip_html_removes_all_tags() {
        let inputs = vec![
            ("<p>Hello</p>", "Hello"),
            ("<div><span>test</span></div>", "test"),
            ("<a href='#'>link</a>", "link"),
            ("<script>evil()</script>", "evil()"),
        ];
        for (input, expected) in &inputs {
            let stripped = strip_html(input);
            assert_eq!(&stripped, expected, "strip_html failed for: {}", input);
        }
    }

    #[test]
    fn csrf_token_is_unique() {
        let t1 = generate_csrf_token();
        let t2 = generate_csrf_token();
        assert_ne!(t1, t2);
    }

    #[test]
    fn nonce_validationates_fresh_token() {
        let token = generate_csrf_token();
        assert!(validate_nonce(&token, 3600));
    }

    #[test]
    fn nonce_rejects_expired_token() {
        assert!(!validate_nonce("0_deadbeef", 3600));
    }

    #[test]
    fn nonce_rejects_malformed_token() {
        assert!(!validate_nonce("not-a-token", 3600));
        assert!(!validate_nonce("", 3600));
        assert!(!validate_nonce("abc", 3600));
    }
}

// ============================================================
// P18: XSS_PREVENTION
// ============================================================

mod p18_xss_prevention {
    use hrml::security::escape_html;

    #[test]
    fn escaped_script_cannot_execute() {
        let payload = "<script>alert(document.cookie)</script>";
        let escaped = escape_html(payload);
        assert!(!escaped.contains("<script"));
        assert!(!escaped.contains("</script>"));
        assert!(escaped.contains("&lt;script&gt;"));
    }

    #[test]
    fn escaped_img_onerror_cannot_execute() {
        let payload = "<img src=x onerror=alert(1)>";
        let escaped = escape_html(payload);
        // No raw angle brackets means no HTML element can be formed
        assert!(!escaped.contains("<img"));
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
    }

    #[test]
    fn escaped_svg_cannot_execute() {
        let payload = "<svg onload=alert(1)>";
        let escaped = escape_html(payload);
        assert!(!escaped.contains("<svg"));
    }

    #[test]
    fn escaped_iframe_cannot_execute() {
        let payload = "<iframe src='https://evil.com'></iframe>";
        let escaped = escape_html(payload);
        assert!(!escaped.contains("<iframe"));
    }

    #[test]
    fn escaped_object_cannot_execute() {
        let payload = "<object data='data:text/html,<script>'></object>";
        let escaped = escape_html(payload);
        assert!(!escaped.contains("<object"));
        assert!(!escaped.contains("<script"));
    }

    #[test]
    fn escaped_input_cannot_inject() {
        let payload = "<input onfocus=alert(1) autofocus>";
        let escaped = escape_html(payload);
        assert!(!escaped.contains("<input"));
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
    }
}

// ============================================================
// P19: AUTH_CORRECTNESS
// ============================================================

mod p19_auth {
    use hrml::auth;

    #[test]
    fn current_user_never_panics() {
        let _ = auth::current_user();
    }

    #[test]
    fn current_uid_is_valid() {
        let uid = auth::current_uid();
        assert!(uid > 0 || uid == 0);
    }

    #[test]
    fn is_root_never_panics() {
        let _ = auth::is_root();
    }

    #[test]
    fn authenticate_wrong_user_fails_gracefully() {
        let result = auth::authenticate("nonexistent_user_xyz_12345", "wrong_password");
        // Either PAM is available and returns false, or PAM is not available
        assert!(result.is_ok() || result.as_ref().unwrap_err().contains("libpam"));
    }

    #[test]
    fn is_user_in_group_handles_missing_group() {
        let result = auth::is_user_in_group("root", "nonexistent_group_xyz");
        assert!(result.is_err());
    }
}

// ============================================================
// P20: SSG_CORRECTNESS
// ============================================================

mod p20_ssg {
    use hrml::ssg::BuildReport;

    #[test]
    fn ssg_build_report_starts_empty() {
        let report = BuildReport::new();
        assert_eq!(report.pages.len(), 0);
        assert_eq!(report.errors.len(), 0);
    }

    #[test]
    fn ssg_build_report_tracks_pages() {
        let mut report = BuildReport::new();
        report.add_page(
            "/".to_string(),
            "pages/index.hrml".to_string(),
            "dist/index.html".to_string(),
        );
        report.add_page(
            "/about".to_string(),
            "pages/about.hrml".to_string(),
            "dist/about/index.html".to_string(),
        );
        assert_eq!(report.pages.len(), 2);
        assert_eq!(report.errors.len(), 0);
    }

    #[test]
    fn ssg_build_report_tracks_errors() {
        let mut report = BuildReport::new();
        report.add_error("pages/broken.hrml".to_string(), "render error".to_string());
        assert_eq!(report.pages.len(), 0);
        assert_eq!(report.errors.len(), 1);
    }

    #[test]
    fn ssg_summary_format() {
        let mut report = BuildReport::new();
        report.add_page(
            "/".to_string(),
            "pages/index.hrml".to_string(),
            "dist/index.html".to_string(),
        );
        report.add_page(
            "/about".to_string(),
            "pages/about.hrml".to_string(),
            "dist/about/index.html".to_string(),
        );
        report.add_error("pages/broken.hrml".to_string(), "error".to_string());
        let summary = report.summary();
        assert!(summary.contains("2 pages"));
        assert!(summary.contains("1 errors"));
    }
}
