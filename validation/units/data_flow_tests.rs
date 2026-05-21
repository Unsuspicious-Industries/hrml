use super::shared::TestEnv;
use serde_json::json;

#[test]
fn get_from_json_context() {
    let env = TestEnv::new("unit_data_json");
    env.write("pages/test.hrml", "<h1><?get id=\"user.name\"?></h1>");
    let out = env
        .render_with_data("pages/test.hrml", &json!({ "user": { "name": "Zoe" } }))
        .unwrap();
    assert!(out.contains("<h1>Zoe</h1>"));
}

#[test]
fn set_and_get_override_precedence() {
    let env = TestEnv::new("unit_data_setget");
    env.write(
        "pages/test.hrml",
        r#"<?set id="title"?>Local</?set?>
<h1><?get id="title"?></h1>"#,
    );
    let out = env
        .render_with_data("pages/test.hrml", &json!({ "title": "Remote" }))
        .unwrap();
    assert!(out.contains("<h1>Local</h1>"));
}

#[test]
fn bind_from_children_sets_var() {
    let env = TestEnv::new("unit_data_bind_children");
    env.write(
        "pages/test.hrml",
        r#"<?bind var="title"?>Local</?bind?><h1><?get id="title"?></h1>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<h1>Local</h1>"), "bind from children failed: {}", out);
}

#[test]
fn markdownfm_inside_bind_sets_meta_and_body() {
    let env = TestEnv::new("unit_data_markdownfm_bind");
    env.write(
        "posts/test.mdx",
        "---\ntitle: Bound Post\nexcerpt: Bound Excerpt\nauthor: Tester\npublishDate: 2025-01-01\nstatus: published\n---\n\n# Hello\n\nBody copy.",
    );
    env.write(
        "pages/test.hrml",
        r#"<?bind var="post_body"?><?markdownfm file="posts/test.mdx" as="post"?><?/bind?>
<h1><?get id="post.title"?></h1>
<p><?get id="post.excerpt"?></p>
<?get id="post_body"?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<h1>Bound Post</h1>"), "missing title: {}", out);
    assert!(out.contains("<p>Bound Excerpt</p>"), "missing excerpt: {}", out);
    assert!(out.contains("<h1>Hello</h1>"), "missing markdown body: {}", out);
    assert!(out.contains("<p>Body copy.</p>"), "missing body paragraph: {}", out);
}
