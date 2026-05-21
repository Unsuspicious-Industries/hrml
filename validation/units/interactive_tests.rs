use super::shared::TestEnv;

#[test]
fn btn_renders_button_with_data_attrs() {
    let env = TestEnv::new("unit_btn");
    env.write(
        "pages/test.hrml",
        r##"<?btn post="/api/x" target="#out" swap="innerHTML"?>Click</?btn?>"##,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<button") && out.contains("data-post=\"/api/x\""));
}

#[test]
fn form_renders_form_tag() {
    let env = TestEnv::new("unit_form");
    env.write(
        "pages/test.hrml",
        r##"<?form post="/api/submit"?>
<input name="e">
</?form?>"##,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<form") && out.contains("data-post"));
}
