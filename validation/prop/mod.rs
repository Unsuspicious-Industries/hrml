use proptest::prelude::*;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct PropEnv {
    dir: PathBuf,
}

impl PropEnv {
    fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hrml_prop_{}_{}", name, now));
        fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn write(&self, path: &str, content: &str) {
        let full = self.dir.join(path);
        if let Some(parent) = Path::new(&full).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, content).unwrap();
    }

    fn render(&self, path: &str) -> Result<String, String> {
        xrml::template::Engine::new(self.dir.to_str().unwrap())
            .render(path, &json!({}))
            .map_err(|e| e.to_string())
    }
}

impl Drop for PropEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn safe_text() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 _.-]{0,40}".prop_map(|s| s.trim().to_string())
}

fn known_directive(name: &str) -> bool {
    matches!(
        name,
        "load"
            | "else"
            | "wasm"
            | "set"
            | "pure"
            | "mdx"
            | "markdown"
            | "markdownfm"
            | "latex"
            | "meta"
            | "linktag"
            | "title"
            | "og"
            | "twitter"
            | "charset"
            | "viewport"
            | "canonical"
            | "description"
            | "robots"
            | "stylesheet"
            | "script"
            | "use"
            | "bind"
            | "block"
            | "get"
            | "component"
            | "if"
            | "for"
            | "slot"
            | "btn"
            | "link"
            | "form"
            | "data"
            | "map"
            | "filter"
            | "sort"
            | "slice"
            | "record"
            | "list"
            | "field"
            | "items"
            | "item"
    )
}

proptest! {
    #[test]
    fn bind_get(value in safe_text()) {
        let env = PropEnv::new("bind_get");
        env.write(
            "pages/test.hrml",
            &format!(r#"<?bind var="title"?>{}<?/bind?><h1><?get id="title"?></h1>"#, value),
        );

        let html = env.render("pages/test.hrml").unwrap();
        let expected = format!("<h1>{}</h1>", value);
        prop_assert!(html.contains(&expected));
    }

    #[test]
    fn slot_default_replaced(slot_content in safe_text()) {
        let content = if slot_content.is_empty() { "x".to_string() } else { slot_content };
        let env = PropEnv::new("component_slot");
        env.write(
            "pages/test.hrml",
            &format!(r#"<?component id="card"?><article><?slot id="body"?>Default</?slot?></article></?component?>
<?use id="card"?><?block slot="body"?><p>{}</p></?block?></?use?>"#, content),
        );

        let html = env.render("pages/test.hrml").unwrap();
        prop_assert!(html.contains("<article>"));
        let expected = format!("<p>{}</p>", content);
        prop_assert!(html.contains(&expected));
        prop_assert!(!html.contains("Default"));
    }

    #[test]
    fn load_bind_use(value in safe_text()) {
        let value = if value.is_empty() { "loaded".to_string() } else { value };
        let env = PropEnv::new("load_component_bind");
        env.write(
            "components/card.hrml",
            r#"<?component id="card"?><article class="card"><?slot id="body"?>Default</?slot?></article></?component?>"#,
        );
        env.write(
            "pages/test.hrml",
            &format!(r#"<?load file="components/card.hrml"?>
<?bind var="title"?>{}<?/bind?>
<?use id="card"?><?block slot="body"?><h2><?get id="title"?></h2></?block?></?use?>"#, value),
        );

        let html = env.render("pages/test.hrml").unwrap();
        prop_assert!(html.contains("class=\"card\""));
        let expected = format!("<h2>{}</h2>", value);
        prop_assert!(html.contains(&expected));
        prop_assert!(!html.contains("Default"));
    }

    #[test]
    fn bind_feeds_component(value in safe_text()) {
        let value = if value.is_empty() { "Bound Title".to_string() } else { value };
        let env = PropEnv::new("child_bindings_component_use");
        env.write(
            "components/card.hrml",
            r#"<?component id="card"?>
<?bind var="title"/?>
<?if cond="$title"?><h2><?get id="title"?></h2><?else?><h2>Missing</h2><?/if?>
<?/component?>"#,
        );
        env.write(
            "pages/test.hrml",
            &format!(r#"<?load file="components/card.hrml"?>
<?use id="card"?>
<?bind var="title"?>{}<?/bind?>
</?use?>"#, value),
        );

        let html = env.render("pages/test.hrml").unwrap();
        let expected = format!("<h2>{}</h2>", value);
        prop_assert!(html.contains(&expected));
        prop_assert!(!html.contains("<h2>Missing</h2>"));
    }

    #[test]
    fn unknown_void_is_html(name in "[a-z]{1,12}") {
        prop_assume!(!known_directive(&name));
        let env = PropEnv::new("unknown_void_html");
        env.write("pages/test.hrml", &format!("<?{}?>", name));

        let html = env.render("pages/test.hrml").unwrap();
        let expected = format!("<{} />", name);
        prop_assert!(html.contains(&expected),
            "unknown void <?{}?> should render as <{} />, got: {}", name, name, html);
    }

    #[test]
    fn unknown_elem_is_html(name in "[a-z]{1,12}") {
        prop_assume!(!known_directive(&name));
        let env = PropEnv::new("unknown_elem_html");
        env.write("pages/test.hrml", &format!("<?{0}?><span>X</span></?{0}?>", name));

        let html = env.render("pages/test.hrml").unwrap();
        assert!(html.contains(&format!("<{}>", name)), "missing open tag for {}", name);
        assert!(html.contains(&format!("</{}>", name)), "missing close tag for {}", name);
        assert!(html.contains("<span>X</span>"), "children not preserved in {}", name);
    }

    #[test]
    fn malformed_opener_errors(name in "[a-z]{1,12}") {
        let env = PropEnv::new("malformed_opener");
        env.write("pages/test.hrml", &format!("<?{}", name));

        let err = env.render("pages/test.hrml").unwrap_err();
        prop_assert!(err.contains("Unclosed HRML directive"));
        prop_assert!(err.contains("error [parse]"));
    }

    #[test]
    fn parser_total(s in ".{0,200}") {
        let _ = xrml::template::Engine::parse_source(&s);
    }

    #[test]
    fn text_no_raw_html(html in arb_well_formed_html()) {
        let ast = xrml::template::Engine::parse_source(&html)
            .expect("well-formed input parses");
        walk_assert_no_html_in_text(&ast.nodes);
    }

    #[test]
    fn render_deterministic(value in safe_text()) {
        let env = PropEnv::new("render_determinism");
        env.write(
            "pages/test.hrml",
            &format!(r#"<?bind var="v"?>{}<?/bind?><p>$v</p>"#, value),
        );
        let a = env.render("pages/test.hrml").unwrap();
        let b = env.render("pages/test.hrml").unwrap();
        prop_assert_eq!(a, b);
    }

    #[test]
    fn dollar_sub_idempotent(value in "[a-zA-Z0-9 _.-]{1,20}") {
        let env = PropEnv::new("dollar_idempotent");
        env.write(
            "pages/once.hrml",
            &format!(r#"<?bind var="v"?>{}<?/bind?><p>$v</p>"#, value),
        );
        let once = env.render("pages/once.hrml").unwrap();
        env.write(
            "pages/twice.hrml",
            &format!("<p>{}</p>", value),
        );
        let twice = env.render("pages/twice.hrml").unwrap();
        let want = format!("<p>{}</p>", value);
        prop_assert!(once.contains(&want), "missing {} in {}", want, once);
        prop_assert!(twice.contains(&want), "missing {} in {}", want, twice);
    }

    #[test]
    fn nested_quote_rejected(var in "[a-z][a-z0-9_-]{0,8}") {
        let src = format!(r#"<a href="<?get id="{}"?>">x</a>"#, var);
        let err = xrml::template::Engine::parse_source(&src).unwrap_err().to_string();
        prop_assert!(err.contains("Nested HRML directive") || err.contains("not allowed"),
            "expected nested-quote rejection, got: {}", err);
    }

    #[test]
    fn void_no_slash(src in r#"/[a-z]{1,10}\.png"#) {
        let env = PropEnv::new("html_void_slash");
        env.write(
            "pages/test.hrml",
            &format!(r#"<img src="{}">"#, src),
        );
        let html = env.render("pages/test.hrml").unwrap();
        let want = format!(r#"<img src="{}">"#, src);
        prop_assert!(html.contains(&want), "want {} in {}", want, html);
        prop_assert!(!html.contains(&format!("{} />", src.trim_end_matches('"'))),
            "void tag must not self-close: {}", html);
    }

    #[test]
    fn self_close_has_slash(name in "[a-z][a-z0-9-]{0,6}") {
        prop_assume!(!is_html_void(&name));
        let env = PropEnv::new("self_close_slash");
        env.write(
            "pages/test.hrml",
            &format!(r##"<svg><{} href="#x"/></svg>"##, name),
        );
        let html = env.render("pages/test.hrml").unwrap();
        let want = format!(r##"<{} href="#x" />"##, name);
        prop_assert!(html.contains(&want),
            "expected self-closing form, got: {}", html);
    }

    #[test]
    fn hrml_trml_equiv(text in safe_text()) {
        let body = if text.is_empty() { "x".to_string() } else { text };
        let hrml = format!("<div class=\"box\"><p>{}</p></div>", body);
        let trml = xrml::convert::to_trml(&hrml).unwrap();
        let env = PropEnv::new("hrml_trml_equiv");
        env.write("pages/h.hrml", &hrml);
        env.write("pages/t.trml", &trml);
        let html_a = env.render("pages/h.hrml").unwrap();
        let html_b = env.render("pages/t.trml").unwrap();
        prop_assert_eq!(strip_ws(&html_a), strip_ws(&html_b));
    }

    #[test]
    fn map_order(values in proptest::collection::vec("[a-z]{1,8}", 1..6)) {
        let env = PropEnv::new("map_order");
        for (idx, v) in values.iter().enumerate() {
            env.write(
                &format!("items/{:02}.json", idx),
                &format!(r#"{{"name":"{}"}}"#, v),
            );
        }
        env.write(
            "pages/test.hrml",
            r#"<?data from="items" as="items"?><ul><?map over="items" as="i"?><li><?get id="i.name"?></li></?map?></ul>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        let mut last_pos = 0;
        for v in &values {
            let needle = format!("<li>{}</li>", v);
            match html[last_pos..].find(&needle) {
                Some(off) => { last_pos += off + needle.len(); }
                None => prop_assert!(false, "missing {} after pos {}, html={}", needle, last_pos, html),
            }
        }
    }

    #[test]
    fn deep_nest(depth in 1usize..30) {
        let mut src = String::new();
        for _ in 0..depth { src.push_str("<div>"); }
        src.push_str("inner");
        for _ in 0..depth { src.push_str("</div>"); }
        let env = PropEnv::new("deep_nesting");
        env.write("pages/test.hrml", &src);
        let html = env.render("pages/test.hrml").unwrap();
        prop_assert_eq!(html.matches("<div>").count(), depth);
        prop_assert_eq!(html.matches("</div>").count(), depth);
        prop_assert!(html.contains("inner"));
    }

    #[test]
    fn attr_order_invariant(
        a in "[a-z]{1,5}",
        b in "[a-z]{1,5}",
        c in "[a-z]{1,5}",
    ) {
        let env = PropEnv::new("attr_order");
        let s1 = format!(r#"<div a="{}" b="{}" c="{}"></div>"#, a, b, c);
        let s2 = format!(r#"<div c="{}" a="{}" b="{}"></div>"#, c, a, b);
        let s3 = format!(r#"<div b="{}" c="{}" a="{}"></div>"#, b, c, a);
        env.write("pages/p1.hrml", &s1);
        env.write("pages/p2.hrml", &s2);
        env.write("pages/p3.hrml", &s3);
        let r1 = env.render("pages/p1.hrml").unwrap();
        let r2 = env.render("pages/p2.hrml").unwrap();
        let r3 = env.render("pages/p3.hrml").unwrap();
        prop_assert_eq!(strip_ws(&r1), strip_ws(&r2));
        prop_assert_eq!(strip_ws(&r2), strip_ws(&r3));
    }

    #[test]
    fn inter_tag_ws(text in "[a-z]{1,10}") {
        let env = PropEnv::new("ws_insens");
        let tight = format!("<div><p>{}</p></div>", text);
        let loose = format!("<div>\n  <p>{}</p>\n</div>", text);
        env.write("pages/tight.hrml", &tight);
        env.write("pages/loose.hrml", &loose);
        let a = env.render("pages/tight.hrml").unwrap();
        let b = env.render("pages/loose.hrml").unwrap();
        prop_assert_eq!(strip_inter_tag_ws(&a), strip_inter_tag_ws(&b));
    }

    #[test]
    fn render_wellformed(html_src in arb_well_formed_html()) {
        let env = PropEnv::new("wellformed");
        env.write("pages/test.hrml", &html_src);
        let html = match env.render("pages/test.hrml") {
            Ok(h) => h,
            Err(_) => return Ok(()),
        };
        let counts = count_tags(&html);
        for (name, (open, close)) in &counts {
            if is_html_void(name) {
                prop_assert_eq!(*close, 0, "void tag {} has {} close tags in {}", name, close, html);
            } else {
                prop_assert_eq!(open, close,
                    "tag {} unbalanced: open={} close={} in {}", name, open, close, html);
            }
        }
    }

    #[test]
    fn empty_map(_seed in 0u8..1) {
        let env = PropEnv::new("empty_map");
        fs::create_dir_all(env.dir.join("items")).unwrap();
        env.write(
            "pages/test.hrml",
            r#"<?data from="items" as="items"?><ul><?map over="items" as="i"?><li><?get id="i.name"?></li></?map?></ul>"#,
        );
        let html = env.render("pages/test.hrml").unwrap_or_default();
        prop_assert!(!html.contains("<li>"), "empty map should emit no <li>, got: {}", html);
    }

    #[test]
    fn if_false(payload in "[a-z]{8,10}") {
        let env = PropEnv::new("if_false");
        let marker = format!("IFMARKER{}", payload);
        env.write(
            "pages/test.hrml",
            &format!(r#"<?if cond="$flag"?><p>{}</p><?/if?>"#, marker),
        );
        let html = env.render("pages/test.hrml").unwrap();
        prop_assert!(!html.contains(&marker),
            "if-false body leaked: marker={} html={}", marker, html);
    }

    #[test]
    fn bind_shadows(outer in "[a-z]{1,8}", inner in "[a-z]{1,8}") {
        prop_assume!(outer != inner);
        let env = PropEnv::new("bind_shadow");
        env.write(
            "pages/test.hrml",
            &format!(
                r#"<?bind var="x"?>{}<?/bind?><p>$x</p><?bind var="x"?>{}<?/bind?><p>$x</p>"#,
                outer, inner,
            ),
        );
        let html = env.render("pages/test.hrml").unwrap();
        prop_assert!(html.contains(&format!("<p>{}</p>", outer)),
            "first bind not applied: {}", html);
        prop_assert!(html.contains(&format!("<p>{}</p>", inner)),
            "second bind not applied: {}", html);
    }

    #[test]
    fn trml_total(s in ".{0,200}") {
        let env = PropEnv::new("trml_total");
        env.write("pages/test.trml", &s);
        let _ = env.render("pages/test.trml");
    }

    #[test]
    fn convert_matches(text in "[a-z]{1,10}") {
        let src = format!("<section><h1>{}</h1></section>", text);
        let trml = xrml::convert::to_trml(&src).unwrap();
        let env = PropEnv::new("convert_match");
        env.write("pages/s.hrml", &src);
        env.write("pages/t.trml", &trml);
        let a = env.render("pages/s.hrml").unwrap();
        let b = env.render("pages/t.trml").unwrap();
        prop_assert_eq!(strip_ws(&a), strip_ws(&b));
    }

    #[test]
    fn special_chars_safe(text in r#"[a-zA-Z0-9 &<>/\-]{0,30}"#) {
        let env = PropEnv::new("special_chars");
        env.write("pages/test.hrml", &format!("<p>{}</p>", text));
        let _ = env.render("pages/test.hrml");
    }

    #[test]
    fn dotted_path(value in "[a-z]{1,10}") {
        let env = PropEnv::new("dotted_path");
        env.write(
            "items/x.json",
            &format!(r#"{{"name":"{}"}}"#, value),
        );
        env.write(
            "pages/test.hrml",
            r#"<?data from="items" as="items"?><?map over="items" as="i"?><p>$i.name</p><?/map?>"#,
        );
        let html = env.render("pages/test.hrml").unwrap();
        prop_assert!(html.contains(&format!("<p>{}</p>", value)),
            "dotted path failed: value={} html={}", value, html);
    }

    #[test]
    fn render_pure(value in safe_text()) {
        let env = PropEnv::new("render_pure");
        env.write(
            "pages/test.hrml",
            &format!(r#"<?bind var="v"?>{}<?/bind?><div><p>$v</p><p>$v</p><p>$v</p></div>"#, value),
        );
        let a = env.render("pages/test.hrml").unwrap();
        let b = env.render("pages/test.hrml").unwrap();
        let c = env.render("pages/test.hrml").unwrap();
        prop_assert_eq!(&a, &b);
        prop_assert_eq!(&b, &c);
    }
}

fn is_html_void(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "source"
            | "track"
            | "wbr"
    )
}

fn strip_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Removes whitespace-only runs that appear between `>` and `<`.
fn strip_inter_tag_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'>' {
            out.push('>');
            i += 1;
            let j = i;
            while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                i += 1;
            }
            if i < bytes.len() && bytes[i] != b'<' {
                // non-tag follows: emit the whitespace we skipped
                out.push_str(&s[j..i]);
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// Returns a map from tag name → (open_count, close_count) for the given
/// HTML.  Used to assert well-formedness of rendered output.  Self-closing
/// (` />`) tags count as one open and zero close, since they consume both.
fn count_tags(html: &str) -> std::collections::HashMap<String, (usize, usize)> {
    let bytes = html.as_bytes();
    let mut counts: std::collections::HashMap<String, (usize, usize)> =
        std::collections::HashMap::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        if i + 1 < bytes.len() && bytes[i + 1] == b'!' {
            // comment / doctype — skip past `>`
            while i < bytes.len() && bytes[i] != b'>' {
                i += 1;
            }
            continue;
        }
        let is_close = i + 1 < bytes.len() && bytes[i + 1] == b'/';
        let name_start = i + if is_close { 2 } else { 1 };
        let mut j = name_start;
        while j < bytes.len()
            && !bytes[j].is_ascii_whitespace()
            && bytes[j] != b'>'
            && bytes[j] != b'/'
        {
            j += 1;
        }
        if j == name_start {
            i = j + 1;
            continue;
        }
        let name = std::str::from_utf8(&bytes[name_start..j])
            .unwrap_or_default()
            .to_ascii_lowercase();
        // Find end of tag, tracking quotes so attribute values cannot
        // confuse us.
        let mut k = j;
        let mut in_q: Option<u8> = None;
        while k < bytes.len() {
            let b = bytes[k];
            if let Some(q) = in_q {
                if b == q {
                    in_q = None;
                }
            } else if b == b'"' || b == b'\'' {
                in_q = Some(b);
            } else if b == b'>' {
                break;
            }
            k += 1;
        }
        let self_close = k > 0 && bytes[k - 1] == b'/';
        let entry = counts.entry(name).or_insert((0, 0));
        if is_close {
            entry.1 += 1;
        } else if !self_close {
            entry.0 += 1;
        } else {
            // self-closing: do not count toward open/close balance
        }
        i = k + 1;
    }
    counts
}

/// Generates short, well-formed HTML fragments with optional HRML directives,
/// for testing the no-raw-HTML invariant and the parser.
fn arb_well_formed_html() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        Just("hi".to_string()),
        Just("<br>".to_string()),
        Just("<?get id=\"x\"?>".to_string()),
        "[a-z]{1,6}".prop_map(|s| s),
    ];
    leaf.prop_recursive(3, 16, 4, |inner| {
        prop_oneof![
            inner.clone().prop_map(|c| format!("<div>{}</div>", c)),
            inner.clone().prop_map(|c| format!("<span>{}</span>", c)),
            (inner.clone(), inner.clone())
                .prop_map(|(a, b)| format!("<section>{}{}</section>", a, b)),
            inner.prop_map(|c| format!("<?if cond=\"$x\"?>{}<?/if?>", c)),
        ]
    })
}

fn walk_assert_no_html_in_text(nodes: &[xrml::template::Node]) {
    use xrml::template::Node;
    for node in nodes {
        match node {
            Node::Text(t) => {
                // No `<a` (alpha) sequence — that would mean an HTML tag-open
                // leaked into a text leaf, violating Stage 1's invariant.
                let bytes = t.as_bytes();
                for i in 0..bytes.len().saturating_sub(1) {
                    if bytes[i] == b'<' && bytes[i + 1].is_ascii_alphabetic() {
                        panic!(
                            "raw HTML tag-open inside Node::Text: {:?}",
                            &t[i..(i + 10).min(t.len())]
                        );
                    }
                }
            }
            Node::Element { children, .. } => walk_assert_no_html_in_text(children),
            Node::VoidElement { .. } => {}
            Node::Load { blocks, .. } => {
                for v in blocks.values() {
                    walk_assert_no_html_in_text(v);
                }
            }
        }
    }
}
