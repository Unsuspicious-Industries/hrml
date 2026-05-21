use super::shared::TestEnv;

#[test]
fn parses_basic_text_and_html() {
    let env = TestEnv::new("unit_parser_basic");
    env.write("pages/a.hrml", "Hello Unit");
    env.write("pages/b.hrml", "<div><p>Block</p></div>");
    let ha = env.render("pages/a.hrml").unwrap();
    let hb = env.render("pages/b.hrml").unwrap();
    assert!(ha.contains("Hello Unit"));
    assert!(hb.contains("<div><p>Block</p></div>"));
}

#[test]
fn parses_attributes_and_voids() {
    let env = TestEnv::new("unit_parser_attrs");
    env.write("pages/test.hrml", r#"<img src="/x.png"> <input required>"#);
    let html = env.render("pages/test.hrml").unwrap();
    assert!(html.contains("<img"));
    assert!(html.contains("required"));
}
