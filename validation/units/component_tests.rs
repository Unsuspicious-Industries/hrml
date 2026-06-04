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
<a href="$href"><h3><?get id="title"?></h3></a>
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
fn undefined_component_use_is_detected() {
    use xrml::config::Config;
    use xrml::project::Project;

    let env = TestEnv::new("unit_undefined_use");
    env.write(
        "components/card.hrml",
        r#"<?component id="card"?><div class="card"><?slot id="content"?></div><?/component?>"#,
    );
    env.write(
        "pages/ok.hrml",
        r#"<?use id="card"?><?block slot="content"?>x<?/block?></?use?>"#,
    );
    env.write(
        "pages/bad.hrml",
        r#"<?use id="crad"?></?use?>"#, // typo
    );

    let mut project = Project::new(Config::default()).with_base_path(&env.dir());
    for f in ["components/card.hrml", "pages/ok.hrml", "pages/bad.hrml"] {
        let src = std::fs::read_to_string(env.dir().join(f)).unwrap();
        project.add_file(f.to_string(), src);
    }
    project.parse_all().unwrap();

    let undefined = project.undefined_component_uses();
    assert_eq!(
        undefined,
        vec![("pages/bad.hrml".to_string(), "crad".to_string())],
        "expected only the typo'd use to be flagged, got {:?}",
        undefined
    );
}

#[test]
fn use_attributes_are_props_and_enable_instance_style_override() {
    let env = TestEnv::new("unit_use_attr_override");
    env.write(
        "layouts/base.hrml",
        r#"<head><?styles?></head><body><?slot id="content"?></?slot?></body>"#,
    );
    // The component exposes a CSS-variable hook via a `style` prop. When
    // set it renders an inline style; otherwise just the class.
    env.write(
        "components/badge.hrml",
        r#"<?component id="badge"?>
<?style?>.badge { color: var(--badge-accent, #111); }<?/style?>
<?if cond="$style"?>
<span class="badge" style="$style"><?slot id="content"?></span>
<?else?>
<span class="badge"><?slot id="content"?></span>
<?/if?>
<?/component?>"#,
    );
    env.write(
        "pages/test.hrml",
        r#"<?use id="badge" style="--badge-accent: #c00"?><?block slot="content"?>A<?/block?></?use?>
<?use id="badge"?><?block slot="content"?>B<?/block?></?use?>"#,
    );

    let out = env
        .engine()
        .with_default_layout(Some("layouts/base.hrml".to_string()))
        .render("pages/test.hrml", &serde_json::json!({}))
        .unwrap();

    assert!(
        out.contains(r#"<span class="badge" style="--badge-accent: #c00">A</span>"#),
        "instance override not applied: {}",
        out
    );
    assert!(
        out.contains(r#"<span class="badge">B</span>"#),
        "un-overridden instance should have no style attr: {}",
        out
    );
    assert!(
        out.contains("var(--badge-accent, #111)"),
        "base style missing: {}",
        out
    );
}

#[test]
fn component_styles_are_hoisted_and_tree_shaken() {
    let env = TestEnv::new("unit_component_styles");
    env.write(
        "layouts/base.hrml",
        r#"<head><?styles?></head><body><?slot id="content"?></?slot?></body>"#,
    );
    env.write(
        "components/card.hrml",
        r#"<?component id="card"?>
<?style?>.card { color: $globals.ink; }<?/style?>
<div class="card"><?slot id="content"?></div>
<?/component?>"#,
    );
    // Defined but never used — its CSS must NOT ship.
    env.write(
        "components/banner.hrml",
        r#"<?component id="banner"?>
<?style?>.banner { color: hotpink; }<?/style?>
<div class="banner"></div>
<?/component?>"#,
    );
    env.write(
        "pages/test.hrml",
        r#"<?use id="card"?><?block slot="content"?>hi<?/block?></?use?>"#,
    );

    let out = env
        .engine()
        .with_default_layout(Some("layouts/base.hrml".to_string()))
        .with_globals(serde_json::json!({ "ink": "#111" }))
        .render("pages/test.hrml", &serde_json::json!({}))
        .unwrap();

    assert!(out.contains("<style>"), "no hoisted style block: {}", out);
    assert!(
        out.contains(".card { color: #111; }"),
        "card css / token unresolved: {}",
        out
    );
    assert!(
        !out.contains("hotpink"),
        "unused component css leaked (no tree-shaking): {}",
        out
    );
    // The <?style?> must not render where it was written (only in the head sink).
    assert_eq!(
        out.matches("<style>").count(),
        1,
        "style not hoisted to single sink: {}",
        out
    );
}

#[test]
fn prop_default_fills_only_when_unset() {
    let env = TestEnv::new("unit_prop_default");
    env.write(
        "components/grid.hrml",
        r#"<?component id="grid"?>
<?bind var="cols" default="3"/?>
<div class="grid-$cols"><?slot id="content"?></div>
<?/component?>"#,
    );
    env.write(
        "pages/test.hrml",
        r#"<?load file="components/grid.hrml"?>
<?use id="grid"?><?block slot="content"?>A<?/block?></?use?>
<?use id="grid"?><?cols?>2<?/cols?><?block slot="content"?>B<?/block?></?use?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(
        out.contains(r#"<div class="grid-3">A</div>"#),
        "default not used: {}",
        out
    );
    assert!(
        out.contains(r#"<div class="grid-2">B</div>"#),
        "explicit prop ignored: {}",
        out
    );
}

#[test]
fn named_tag_props_bind_like_explicit_bind() {
    // `<?title?>…<?/title?>` inside `<?use?>` is sugar for `<?bind var="title">`.
    let env = TestEnv::new("unit_comp_named_props");
    env.write(
        "components/card.hrml",
        r#"<?component id="card"?>
<?bind var="href"/?>
<?bind var="title"/?>
<?if cond="$href"?>
<a href="$href"><h3>$title</h3></a>
<?else?>
<div><h3>$title</h3></div>
<?/if?>
</?component?>"#,
    );
    env.write(
        "pages/test.hrml",
        r#"<?load file="components/card.hrml"?>
<?use id="card"?>
<?href?>/product<?/href?>
<?title?>Hello<?/title?>
</?use?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(
        out.contains("<a href=\"/product\"><h3>Hello</h3></a>"),
        "named-tag props failed: {}",
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
<a href="$href" class="card-link">
    <div class="card">
        <?if cond="$icon"?>
        <div class="card-icon"><svg class="icon-sm"><use href="#icon-$icon"/></svg></div>
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
    <div class="card-icon"><svg class="icon-sm"><use href="#icon-$icon"/></svg></div>
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
fn default_layout_wraps_a_page_with_no_loads() {
    // A page that declares no <?load?> is wrapped in the configured layout,
    // with the auto-imports loaded ahead of it — so the author writes only blocks.
    let env = TestEnv::new("unit_default_layout");
    env.write(
        "layouts/base.hrml",
        r#"<main><?slot id="content"?></?slot?></main>"#,
    );
    env.write(
        "components/hi.hrml",
        r#"<?component id="hi"?><p class="hi">Hello</p></?component?>"#,
    );
    env.write("_imports.hrml", r#"<?load file="components/hi.hrml"?>"#);
    // No <?load?> here — just the content block and a component use.
    env.write(
        "pages/test.hrml",
        r#"<?block slot="content"?><?use id="hi"?></?use?><span>body</span></?block?>"#,
    );

    let out = env
        .engine()
        .with_default_layout(Some("layouts/base.hrml".to_string()))
        .with_auto_imports(vec!["_imports.hrml".to_string()])
        .render("pages/test.hrml", &serde_json::json!({}))
        .unwrap();

    assert!(out.contains("<main>"), "layout not applied: {}", out);
    assert!(
        out.contains("class=\"hi\">Hello"),
        "auto-import missing: {}",
        out
    );
    assert!(
        out.contains("<span>body</span>"),
        "page body missing: {}",
        out
    );
}

#[test]
fn implicit_content_block_and_auto_discovery() {
    // A page that names no block: its whole body IS the content. And the
    // component is discovered from components/ with no import file at all.
    let env = TestEnv::new("unit_implicit_block");
    env.write(
        "layouts/base.hrml",
        r#"<main><?slot id="content"?></?slot?></main>"#,
    );
    env.write(
        "components/hi.hrml",
        r#"<?component id="hi"?><p class="hi">Hi $who</p></?component?>"#,
    );
    // No block wrapper, no <?load?>, no _imports — just data + body.
    env.write(
        "pages/test.hrml",
        r#"<?set id="who" value="world"/?>
<?use id="hi"?></?use?>
<span>tail</span>"#,
    );

    let out = env
        .engine()
        .with_default_layout(Some("layouts/base.hrml".to_string()))
        .render("pages/test.hrml", &serde_json::json!({}))
        .unwrap();

    assert!(out.contains("<main>"), "layout not applied: {}", out);
    assert!(
        out.contains("class=\"hi\">Hi world"),
        "auto-discovered component / implicit block failed: {}",
        out
    );
    assert!(
        out.contains("<span>tail</span>"),
        "page body missing: {}",
        out
    );
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
<a href="$href" class="card-link">
    <div class="card">
        <?if cond="$icon"?>
        <div class="card-icon"><svg class="icon-sm"><use href="#icon-$icon"/></svg></div>
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
fn named_tag_props_through_layout_slot() {
    let env = TestEnv::new("unit_named_props_layout");
    env.write(
        "layouts/base.hrml",
        r#"<body><?slot id="content"?></?slot?></body>"#,
    );
    env.write(
        "components/card.hrml",
        r##"<?component id="card-cmp"?>
<?bind var="title"/?>
<?if cond="$title"?><h3 class="card-title">$title</h3><?/if?>
<?/component?>"##,
    );
    env.write("_imports.hrml", r#"<?load file="components/card.hrml"?>"#);
    env.write(
        "pages/test.hrml",
        r#"<?load file="layouts/base.hrml"?>
<?load file="_imports.hrml"?>
<?block slot="content"?>
<?use id="card-cmp"?>
    <?title?>Dynamic Formal Systems<?/title?>
</?use?>
</?block?>"#,
    );
    let out = env.render("pages/test.hrml").unwrap();
    assert!(
        out.contains("class=\"card-title\">Dynamic Formal Systems</h3>"),
        "named-tag prop lost through layout slot: {}",
        out
    );
}

#[test]
fn named_tag_props_through_auto_layout() {
    let env = TestEnv::new("unit_named_props_auto_layout");
    env.write(
        "layouts/base.hrml",
        r#"<main><?slot id="content"?></?slot?></main>"#,
    );
    env.write(
        "components/card.hrml",
        r##"<?component id="card-cmp"?>
<?bind var="title"/?>
<?if cond="$title"?><h3 class="card-title">$title</h3><?/if?>
<?/component?>"##,
    );
    env.write("_imports.hrml", r#"<?load file="components/card.hrml"?>"#);
    // No <?load?> — auto-layout wraps it.
    env.write(
        "pages/test.hrml",
        r#"<?block slot="content"?><?use id="card-cmp"?><?title?>Dynamic Formal Systems<?/title?></?use?></?block?>"#,
    );
    let out = env
        .engine()
        .with_default_layout(Some("layouts/base.hrml".to_string()))
        .with_auto_imports(vec!["_imports.hrml".to_string()])
        .render("pages/test.hrml", &serde_json::json!({}))
        .unwrap();
    assert!(
        out.contains("class=\"card-title\">Dynamic Formal Systems</h3>"),
        "named-tag prop lost through auto-layout: {}",
        out
    );
}

#[test]
fn real_usi_index_renders_imported_cards() {
    let env = TestEnv::new("unit_real_usi_index");

    // Copy the whole template tree so the test tracks the live component set
    // rather than a brittle hand-maintained subset.
    for entry in walkdir(std::path::Path::new("usi/templates")) {
        if entry.ends_with(".hrml") {
            let rel = entry.strip_prefix("usi/templates/").unwrap_or(&entry);
            env.write(rel, &fs::read_to_string(&entry).unwrap());
        }
    }

    let out = env
        .engine()
        .with_default_layout(Some("layouts/base.hrml".to_string()))
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
        out.contains("class=\"card-title\">Dynamic Formal Systems lab</h3>"),
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
    use std::path::Path;
    use xrml::config::Config;
    use xrml::project::Project;

    let config = Config {
        default_layout: Some("layouts/base.hrml".to_string()),
        ..Config::default()
    };
    let mut project = Project::new(config);
    let templates_root = Path::new("usi/templates");

    for entry in walkdir(templates_root) {
        if entry.ends_with(".hrml") {
            let rel = entry.strip_prefix("usi/templates/").unwrap_or(&entry);
            let source = std::fs::read_to_string(&entry).unwrap();
            project.add_file(rel.to_string(), source);
        }
    }

    project.parse_all().unwrap();

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
        out.contains("class=\"card-title\">Dynamic Formal Systems lab</h3>"),
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
    let engine = xrml::template::Engine::new("usi/templates")
        .with_site_name("Unsuspicious Industries".to_string())
        .with_default_layout(Some("layouts/base.hrml".to_string()))
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
        out.contains("class=\"card-title\">Dynamic Formal Systems lab</h3>"),
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

#[test]
fn blog_and_jobs_pages_render() {
    use std::fs;
    use std::path::Path;
    use xrml::config::Config;
    use xrml::project::Project;

    let templates_root = Path::new("usi/templates");
    let project_root = Path::new("usi");

    let config = Config {
        site_name: "USI".to_string(),
        globals: serde_json::json!({
            "primary":"#1f2937","secondary":"#1f2937","accent":"#3730a3",
            "text_heading":"#3730a3","text_default":"#1f2937",
            "text_muted":"#6b7280","border":"#e5e7eb",
            "bg_page":"#ffffff","bg_section":"#f8fafc",
            "brand_accent_1":"#54005b","brand_accent_2":"#06b6d4",
            "font_mono":"mono","font_serif":"serif","font_typewriter":"tw"
        }),
        default_layout: Some("layouts/base.hrml".to_string()),
        ..Config::default()
    };

    let mut project = Project::new(config).with_base_path(project_root);

    for entry in walkdir(templates_root) {
        if entry.ends_with(".hrml") {
            let rel = entry.strip_prefix("usi/templates/").unwrap_or(&entry);
            let source = fs::read_to_string(&entry).unwrap();
            project.add_file(rel.to_string(), source);
        }
    }
    project.parse_all().unwrap();

    // The listing pages render with no params.
    let blog_out = project
        .render("pages/blog.hrml", &serde_json::json!({}))
        .unwrap();
    assert!(
        blog_out.contains("completing-regex")
            || blog_out.contains("proposition-7")
            || blog_out.contains("post-card"),
        "blog page missing post content: {}",
        &blog_out[..blog_out.len().min(500)]
    );

    let jobs_out = project
        .render("pages/jobs.hrml", &serde_json::json!({}))
        .unwrap();
    assert!(
        jobs_out.contains("ethics")
            || jobs_out.contains("polymath")
            || jobs_out.contains("job-card"),
        "jobs page missing job content: {}",
        &jobs_out[..jobs_out.len().min(500)]
    );

    // Dynamic [slug] pages: the generic path system expands the route over its
    // collection, then each concrete page is rendered with its slug bound.
    for (page, base) in &[
        ("pages/blog/[slug].hrml", "data/posts"),
        ("pages/jobs/[slug].hrml", "data/jobs"),
    ] {
        let nodes = project
            .get_file(page)
            .unwrap()
            .tree
            .as_ref()
            .unwrap()
            .nodes
            .clone();
        let bindings = xrml::paths::expand(&xrml::paths::route_params(page), &nodes, project_root);
        assert!(
            !bindings.is_empty(),
            "{} expanded to no pages (collection {} empty?)",
            page,
            base
        );
        for binding in bindings {
            let data = serde_json::to_value(&binding).unwrap();
            let result = project.render(page, &data);
            assert!(
                result.is_ok(),
                "page {} {:?} failed: {}",
                page,
                binding,
                result.err().unwrap()
            );
            assert!(
                result.unwrap().contains("hero-title"),
                "{} {:?} missing title",
                page,
                binding
            );
        }
    }
}

fn walkdir(dir: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walkdir(&p));
            } else {
                out.push(p.to_string_lossy().to_string());
            }
        }
    }
    out
}
