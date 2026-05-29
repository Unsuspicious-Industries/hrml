use super::shared::TestEnv;
use serde_json::json;

#[test]
fn resolve_supports_inline_var_in_path() {
    let env = TestEnv::new("unit_resolve_inline_var");
    env.write(
        "posts/my-post.mdx",
        "---\ntitle: Test Post\nauthor: Test Author\n---\n\nBody text here.",
    );
    env.write(
        "pages/post.hrml",
        r#"<?data from="posts/$slug.mdx" as="post"?><h1><?get id="post.title"?></h1><p><?get id="post.author"?></p>"#,
    );
    let out = env
        .render_with_data("pages/post.hrml", &json!({"slug": "my-post"}))
        .unwrap();
    assert!(
        out.contains("<h1>Test Post</h1>"),
        "expected title in: {}",
        out
    );
}

#[test]
fn data_dir_loads_multiple_posts_with_slugs() {
    let env = TestEnv::new("unit_data_dir_posts");
    env.write(
        "posts/post-a.mdx",
        "---\ntitle: Post A\npublishDate: 2025-01-01\n---\n\nContent A.",
    );
    env.write(
        "posts/post-b.mdx",
        "---\ntitle: Post B\npublishDate: 2025-02-01\n---\n\nContent B.",
    );
    env.write(
        "pages/blog.hrml",
        r#"<?data from="posts" as="posts"?><ul><?map over="posts" as="p"?><li><?get id="p.title"?></li></?map?></ul>"#,
    );
    let out = env.render("pages/blog.hrml").unwrap();
    assert!(out.contains("<li>Post A</li>"), "expected Post A: {}", out);
    assert!(out.contains("<li>Post B</li>"), "expected Post B: {}", out);
}

#[test]
fn dynamic_slug_page_renders_with_variable_data() {
    let env = TestEnv::new("unit_dynamic_slug");
    env.write(
        "posts/my-post.mdx",
        "---\ntitle: Dynamic Page\nexcerpt: Test excerpt\nauthor: Author Name\npublishDate: 2025-03-01\n---\n\n# Heading\n\nBody text.",
    );
    env.write(
        "pages/blog/[slug].hrml",
        r#"<?data from="posts/$slug.mdx" as="post"?><div class="post"><h1><?get id="post.title"?></h1><p class="excerpt"><?get id="post.excerpt"?></p><p class="author"><?get id="post.author"?></p><time><?get id="post.publishDate"?></time><div class="body"><?get id="post.body"?></div></div>"#,
    );
    let out = env
        .render_with_data("pages/blog/[slug].hrml", &json!({"slug": "my-post"}))
        .unwrap();
    assert!(
        out.contains("<h1>Dynamic Page</h1>"),
        "missing title: {}",
        out
    );
    assert!(out.contains("Test excerpt"), "missing excerpt: {}", out);
    assert!(out.contains("Author Name"), "missing author: {}", out);
    assert!(out.contains("2025-03-01"), "missing date: {}", out);
    assert!(
        out.contains("<h1>Heading</h1>"),
        "missing body heading: {}",
        out
    );
    assert!(out.contains("Body text"), "missing body text: {}", out);
}

#[test]
fn resolve_falls_back_to_literal_when_no_dollar() {
    let env = TestEnv::new("unit_resolve_no_dollar");
    env.write(
        "posts/my-post.mdx",
        "---\ntitle: Static Path\n---\n\nContent.",
    );
    env.write(
        "pages/post.hrml",
        r#"<?data from="posts/my-post.mdx" as="post"?><h1><?get id="post.title"?></h1>"#,
    );
    let out = env.render("pages/post.hrml").unwrap();
    assert!(
        out.contains("<h1>Static Path</h1>"),
        "expected title in: {}",
        out
    );
}

#[test]
fn resolve_entire_value_as_var() {
    let env = TestEnv::new("unit_resolve_entire_var");
    env.write("posts/my-post.mdx", "---\ntitle: Test Post\n---\n\nBody.");
    env.write(
        "pages/post.hrml",
        r#"<?data from="$slug" as="post"?><h1><?get id="post.title"?></h1>"#,
    );
    let out = env
        .render_with_data("pages/post.hrml", &json!({"slug": "posts/my-post.mdx"}))
        .unwrap();
    assert!(
        out.contains("<h1>Test Post</h1>"),
        "expected title in: {}",
        out
    );
}

#[test]
fn markdownfm_with_inline_var_in_path() {
    let env = TestEnv::new("unit_markdownfm_inline_var");
    env.write(
        "posts/my-post.mdx",
        "---\ntitle: Bound Post\nexcerpt: Bound Excerpt\nauthor: Tester\npublishDate: 2025-01-01\nstatus: published\n---\n\n# Hello\n\nBody copy.",
    );
    env.write(
        "pages/post.hrml",
        r#"<?bind var="post_body"?><?markdownfm file="posts/$slug.mdx" as="post"?><?/bind?>
<h1><?get id="post.title"?></h1>
<p><?get id="post.excerpt"?></p>
<?get id="post_body"?>"#,
    );
    let out = env
        .render_with_data("pages/post.hrml", &json!({"slug": "my-post"}))
        .unwrap();
    assert!(
        out.contains("<h1>Bound Post</h1>"),
        "missing title: {}",
        out
    );
    assert!(
        out.contains("<p>Bound Excerpt</p>"),
        "missing excerpt: {}",
        out
    );
    assert!(
        out.contains("<h1>Hello</h1>"),
        "missing markdown body: {}",
        out
    );
}

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
    assert!(
        out.contains("<h1>Local</h1>"),
        "bind from children failed: {}",
        out
    );
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
    assert!(
        out.contains("<h1>Bound Post</h1>"),
        "missing title: {}",
        out
    );
    assert!(
        out.contains("<p>Bound Excerpt</p>"),
        "missing excerpt: {}",
        out
    );
    assert!(
        out.contains("<h1>Hello</h1>"),
        "missing markdown body: {}",
        out
    );
    assert!(
        out.contains("<p>Body copy.</p>"),
        "missing body paragraph: {}",
        out
    );
}

#[test]
fn data_loaded_mdx_renders_math_delimiters() {
    let env = TestEnv::new("unit_data_math");
    env.write(
        "posts/math-post.mdx",
        "---\ntitle: Math Post\n---\n\nHere is inline math $x^2 + y^2 = z^2$ and display math $$\\int_0^\\infty e^{-x} dx$$\n\nMore text.",
    );
    env.write(
        "pages/post.hrml",
        r#"<?data from="posts/$slug.mdx" as="post"?><h1><?get id="post.title"?></h1><?get id="post.body"?>"#,
    );
    let out = env
        .render_with_data("pages/post.hrml", &json!({"slug": "math-post"}))
        .unwrap();
    assert!(
        out.contains("class=\"math-inline\""),
        "missing inline math: {}",
        out
    );
    assert!(
        out.contains("class=\"math-block\""),
        "missing block math: {}",
        out
    );
    assert!(out.contains("\\(x^2"), "missing \\( delimiter: {}", out);
    assert!(
        out.contains("\\]</div>"),
        "missing \\[ delimiter close: {}",
        out
    );
}
