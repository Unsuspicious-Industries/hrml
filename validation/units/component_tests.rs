use super::shared::TestEnv;
use std::fs;

#[test]
fn define_and_use_simple_component() {
    let env = TestEnv::new("unit_comp_basic");
    env.write(
        "pages/test.hrml",
        r#"<?component id="x"?><span>X</span></?component?>
<?use id="x"?></?use?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<span>X</span>"));
}

#[test]
fn component_slots_and_defaults() {
    let env = TestEnv::new("unit_comp_slots");
    env.write(
        "pages/test.hrml",
        r#"<?component id="c"?><article><?slot id="body"?>Default</?slot?></article></?component?>
<?use id="c"?><?block slot="body"?>Custom</?block?></?use?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("<article>"));
    assert!(out.contains("Custom"));
    assert!(!out.contains("Default"));
}

#[test]
fn component_use_applies_child_bindings_before_render() {
    let env = TestEnv::new("unit_comp_child_bindings");
    env.write(
        "components/card.hrml",
        r#"<?component id="card"?>
<?bind var="href"/?>
<?bind var="title"/?>
<?if cond="$href"?>
<a href="<?get id="href"?>"><h3><?get id="title"?></h3></a>
<?else?>
<div><h3><?get id="title"?></h3></div>
<?/if?>
</?component?>"#,
    );
    env.write(
        "pages/test.hrml",
        r#"<?load file="components/card.hrml"?>
<?use id="card"?>
<?bind var="href"?>/product<?/bind?>
<?bind var="title"?>Hello<?/bind?>
</?use?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(
        out.contains("<a href=\"/product\"><h3>Hello</h3></a>"),
        "component child bindings failed: {}",
        out
    );
}

#[test]
fn usi_card_component_renders_bound_content() {
    let env = TestEnv::new("unit_usi_card_component");
    env.write(
        "components/card.hrml",
        r##"<?component id="card-cmp"?>
<?bind var="href"/?>
<?bind var="icon"/?>
<?bind var="title"/?>
<?bind var="text"/?>
<?if cond="$href"?>
<a href="<?get id="href"?>" class="card-link">
    <div class="card">
        <?if cond="$icon"?>
        <div class="card-icon"><svg class="icon-sm"><use href="#icon-<?get id="icon"?>"/></svg></div>
        <?/if?>
        <?if cond="$title"?>
        <h3 class="card-title"><?get id="title"?></h3>
        <?/if?>
        <?if cond="$text"?>
        <p class="card-text"><?get id="text"?></p>
        <?/if?>
    </div>
</a>
<?else?>
<div class="card">
    <?if cond="$icon"?>
    <div class="card-icon"><svg class="icon-sm"><use href="#icon-<?get id="icon"?>"/></svg></div>
    <?/if?>
    <?if cond="$title"?>
    <h3 class="card-title"><?get id="title"?></h3>
    <?/if?>
    <?if cond="$text"?>
    <p class="card-text"><?get id="text"?></p>
    <?/if?>
</div>
<?/if?>
<?/component?>"##,
    );
    env.write(
        "pages/test.hrml",
        r#"<?load file="components/card.hrml"?>
<?use id="card-cmp"?>
    <?bind var="href"?>/product<?/bind?>
    <?bind var="icon"?>code<?/bind?>
    <?bind var="title"?>Dynamic Formal Systems<?/bind?>
    <?bind var="text"?>Investigating interaction combinators.<?/bind?>
</?use?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("href=\"/product\""), "missing href: {}", out);
    assert!(
        out.contains("class=\"card-title\">Dynamic Formal Systems</h3>"),
        "missing title: {}",
        out
    );
    assert!(
        out.contains("class=\"card-text\">Investigating interaction combinators.</p>"),
        "missing text: {}",
        out
    );
    assert!(out.contains("#icon-code"), "missing icon ref: {}", out);
}

#[test]
fn imported_component_available_inside_loaded_layout_slot() {
    let env = TestEnv::new("unit_imported_component_layout_slot");
    env.write(
        "layouts/base.hrml",
        r#"<body><?slot id="content"?></?slot?></body>"#,
    );
    env.write(
        "components/card.hrml",
        r##"<?component id="card-cmp"?>
<?bind var="href"/?>
<?bind var="icon"/?>
<?bind var="title"/?>
<?bind var="text"/?>
<?if cond="$href"?>
<a href="<?get id="href"?>" class="card-link">
    <div class="card">
        <?if cond="$icon"?>
        <div class="card-icon"><svg class="icon-sm"><use href="#icon-<?get id="icon"?>"/></svg></div>
        <?/if?>
        <?if cond="$title"?>
        <h3 class="card-title"><?get id="title"?></h3>
        <?/if?>
        <?if cond="$text"?>
        <p class="card-text"><?get id="text"?></p>
        <?/if?>
    </div>
</a>
<?else?>
<div class="card"></div>
<?/if?>
<?/component?>"##,
    );
    env.write("_imports.hrml", r#"<?load file="components/card.hrml"?>"#);
    env.write(
        "pages/test.hrml",
        r#"<?load file="layouts/base.hrml"?>
<?load file="_imports.hrml"?>
<?block slot="content"?>
<?use id="card-cmp"?>
    <?bind var="href"?>/product<?/bind?>
    <?bind var="icon"?>code<?/bind?>
    <?bind var="title"?>Dynamic Formal Systems<?/bind?>
    <?bind var="text"?>Investigating interaction combinators.<?/bind?>
</?use?>
</?block?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(out.contains("href=\"/product\""), "missing href: {}", out);
    assert!(out.contains("#icon-code"), "missing icon ref: {}", out);
    assert!(
        out.contains("class=\"card-title\">Dynamic Formal Systems</h3>"),
        "missing title: {}",
        out
    );
}

#[test]
fn real_usi_index_renders_imported_cards() {
    let env = TestEnv::new("unit_real_usi_index");

    for path in [
        "_imports.hrml",
        "layouts/base.hrml",
        "components/card.hrml",
        "components/stat.hrml",
        "components/nav.hrml",
        "components/footer.hrml",
        "components/post-card.hrml",
        "components/job-card.hrml",
        "components/prose-page.hrml",
        "components/hero.hrml",
        "components/cta.hrml",
        "pages/index.hrml",
    ] {
        let source = fs::read_to_string(format!("usi/templates/{}", path)).unwrap();
        env.write(path, &source);
    }

    let out = env
        .render_with_data(
            "pages/index.hrml",
            &serde_json::json!({
                "globals": {
                    "primary": "#1f2937",
                    "secondary": "#1f2937",
                    "accent": "#3730a3",
                    "text_heading": "#3730a3",
                    "text_default": "#1f2937",
                    "text_muted": "#6b7280",
                    "border": "#e5e7eb",
                    "bg_page": "#ffffff",
                    "bg_section": "#f8fafc",
                    "brand_accent_1": "#54005b",
                    "brand_accent_2": "#06b6d4",
                    "font_mono": "mono",
                    "font_serif": "serif"
                },
                "site_name": "Unsuspicious Industries"
            }),
        )
        .unwrap();

    assert!(out.contains("class=\"site-nav\""), "missing nav: {}", out);
    assert!(
        out.contains("class=\"card-title\">Dynamic Formal Systems</h3>"),
        "missing card title: {}",
        out
    );
    assert!(out.contains("#icon-code"), "missing icon: {}", out);
    assert!(
        out.contains("class=\"stat-item\""),
        "missing stat item: {}",
        out
    );
}

#[test]
fn real_usi_index_renders_imported_cards_via_project_api() {
    use hrml::config::Config;
    use hrml::project::Project;
    use std::path::Path;

    let mut project = Project::new(Config::default());
    let templates_root = Path::new("usi/templates");

    for path in [
        "_imports.hrml",
        "layouts/base.hrml",
        "components/card.hrml",
        "components/stat.hrml",
        "components/nav.hrml",
        "components/footer.hrml",
        "components/post-card.hrml",
        "components/job-card.hrml",
        "components/prose-page.hrml",
        "components/hero.hrml",
        "components/cta.hrml",
        "pages/index.hrml",
    ] {
        let source = std::fs::read_to_string(templates_root.join(path)).unwrap();
        project.add_file(path.to_string(), source);
    }

    project.parse_all().unwrap();

    let order = project.resolve_order().unwrap();
    let idx_pos = order.iter().position(|p| p == "pages/index.hrml").unwrap();
    let imp_pos = order.iter().position(|p| p == "_imports.hrml").unwrap();
    assert!(imp_pos < idx_pos, "_imports should resolve before index");

    let out = project
        .render(
            "pages/index.hrml",
            &serde_json::json!({
                "globals": {
                    "primary": "#1f2937",
                    "secondary": "#1f2937",
                    "accent": "#3730a3",
                    "text_heading": "#3730a3",
                    "text_default": "#1f2937",
                    "text_muted": "#6b7280",
                    "border": "#e5e7eb",
                    "bg_page": "#ffffff",
                    "bg_section": "#f8fafc",
                    "brand_accent_1": "#54005b",
                    "brand_accent_2": "#06b6d4",
                    "font_mono": "mono",
                    "font_serif": "serif"
                },
                "site_name": "Unsuspicious Industries"
            }),
        )
        .unwrap();

    assert!(out.contains("class=\"site-nav\""), "missing nav: {}", out);
    assert!(
        out.contains("class=\"card-title\">Dynamic Formal Systems</h3>"),
        "missing card title: {}",
        out
    );
    assert!(out.contains("#icon-code"), "missing icon: {}", out);
    assert!(
        out.contains("class=\"stat-item\""),
        "missing stat item: {}",
        out
    );
}

#[test]
fn direct_usi_templates_path_renders_cards() {
    let engine = hrml::template::Engine::new("usi/templates")
        .with_site_name("Unsuspicious Industries".to_string())
        .with_globals(serde_json::json!({
            "primary": "#1f2937",
            "secondary": "#1f2937",
            "accent": "#3730a3",
            "text_heading": "#3730a3",
            "text_default": "#1f2937",
            "text_muted": "#6b7280",
            "border": "#e5e7eb",
            "bg_page": "#ffffff",
            "bg_section": "#f8fafc",
            "brand_accent_1": "#54005b",
            "brand_accent_2": "#06b6d4",
            "font_mono": "mono",
            "font_serif": "serif"
        }));

    let out = engine
        .render("pages/index.hrml", &serde_json::json!({}))
        .unwrap();
    assert!(out.contains("class=\"site-nav\""), "missing nav: {}", out);
    assert!(
        out.contains("class=\"card-title\">Dynamic Formal Systems</h3>"),
        "missing card title: {}",
        out
    );
    assert!(out.contains("#icon-code"), "missing icon: {}", out);
}

#[test]
fn prose_page_receives_meta_and_body_from_bound_markdown() {
    let env = TestEnv::new("unit_prose_page_bound_markdown");
    env.write(
        "components/prose-page.hrml",
        r#"<?component id="prose-page"?>
<?bind var="label"/?><?bind var="title"/?><?bind var="description"/?><?bind var="meta"/?>
<section class="hero hero-compact">
    <div class="hero-content">
        <?if cond="$label"?><p class="hero-label"><?get id="label"?></p><?/if?>
        <?if cond="$title"?><h1 class="hero-title"><?get id="title"?></h1><?/if?>
        <?if cond="$description"?><p class="hero-description"><?get id="description"?></p><?/if?>
        <?if cond="$meta"?><p class="page-meta"><?get id="meta"?></p><?/if?>
    </div>
</section>
<section class="section prose-section"><article class="prose-shell"><?slot id="content"?></?slot?></article></section>
<?/component?>"#,
    );
    env.write(
        "posts/test.mdx",
        "---\ntitle: Bound Post\nexcerpt: Bound Excerpt\nauthor: Tester\npublishDate: 2025-01-01\nstatus: published\n---\n\n# Hello\n\nBody copy.",
    );
    env.write(
        "pages/test.hrml",
        r#"<?load file="components/prose-page.hrml"?>
<?bind var="post_body"?><?markdownfm file="posts/test.mdx" as="post"?><?/bind?>
<?use id="prose-page"?>
    <?bind var="label"?>Research Note<?/bind?>
    <?bind var="title"?><?get id="post.title"?><?/bind?>
    <?bind var="description"?><?get id="post.excerpt"?><?/bind?>
    <?bind var="meta"?><?get id="post.author"?> · <?get id="post.publishDate"?> · <?get id="post.status"?><?/bind?>
    <?block slot="content"?><?get id="post_body"?><?/block?>
</?use?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(
        out.contains("<h1 class=\"hero-title\">Bound Post</h1>"),
        "missing prose title: {}",
        out
    );
    assert!(
        out.contains("<p class=\"hero-description\">Bound Excerpt</p>"),
        "missing prose excerpt: {}",
        out
    );
    assert!(
        out.contains("Tester · 2025-01-01 · published"),
        "missing prose meta: {}",
        out
    );
    assert!(
        out.contains("<h1>Hello</h1>"),
        "missing prose body: {}",
        out
    );
}
