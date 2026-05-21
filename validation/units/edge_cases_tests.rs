use super::shared::TestEnv;

#[test]
fn parser_does_not_hang_on_long_searches() {
    let env = TestEnv::new("parser_long_search");
    let long_body: String = std::iter::repeat('a').take(800).collect();
    let content = format!("<?bind var=\"name\" value=\"x\"/?>{}", long_body);
    env.write("pages/test.hrml", &content);

    let out = env.render("pages/test.hrml");
    assert!(out.is_ok(), "parser hung or errored: {:?}", out);
}

#[test]
fn nested_components_bind_and_slots() {
    let env = TestEnv::new("nested_components");
    env.write(
        "pages/test.hrml",
        r#"<?component id="shell"?><section><?slot id="body"?></?slot?></section></?component?>
<?component id="card"?><article><?slot id="body"?></?slot?></article></?component?>
<?bind var="title" value="Nested"/?>
<?use id="shell"?><?block slot="body"?><?use id="card"?><?block slot="body"?><h2><?get id="title"?></h2></?block?></?use?></?block?></?use?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<section><article><h2>Nested</h2></article></section>"));
}

#[test]
fn component_slots_and_blocks_injection() {
    let env = TestEnv::new("component_slots");
    // Define a component that expects a slot 'body'
    env.write(
        "pages/test.hrml",
        r#"<?component id="card"?><div class="card"><?slot id="body"?></?slot?></div></?component?>
<?use id="card"?><?block slot="body"?><p>Inside</p></?block?></?use?>"#,
    );

    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("class=\"card\""));
    assert!(out.contains("Inside"), "slot content not injected: {}", out);
}

#[test]
fn circular_loads_report_error() {
    let env = TestEnv::new("circular_loads");
    env.write("pages/a.hrml", r#"<?load file="pages/b.hrml"?>"#);
    env.write("pages/b.hrml", r#"<?load file="pages/a.hrml"?>"#);

    let res = env.render("pages/a.hrml");
    assert!(
        res.is_err(),
        "expected circular dependency error but render succeeded"
    );
    let err = res.err().unwrap();
    assert!(
        err.contains("Circular template dependency") || err.contains("circular"),
        "unexpected error: {}",
        err
    );
}

#[test]
fn render_fragment_vs_full_document() {
    let env = TestEnv::new("fragment_vs_full");
    env.write("pages/test.hrml", "FragmentOnly");

    let frag = env.render_fragment("pages/test.hrml").unwrap();
    assert_eq!(frag, "FragmentOnly");

    let full = env.render("pages/test.hrml").unwrap();
    assert!(
        full.contains("<!DOCTYPE html") || full.contains("<html"),
        "full document wrapper missing: {}",
        full
    );
}

#[test]
fn markdown_frontmatter_sets_context_value() {
    let env = TestEnv::new("md_frontmatter");
    env.write("doc.md", "---\ntitle = \"Edge Title\"\n---\nHello from md");

    env.write(
        "pages/test.hrml",
        r#"<?markdownfm file="doc.md" as="meta"?><?get id="meta"?>"#,
    );

    let out = env.render("pages/test.hrml").unwrap();
    assert!(
        out.contains("Edge Title"),
        "frontmatter title not found in output: {}",
        out
    );
}
