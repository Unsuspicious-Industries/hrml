use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use xrml::template::Engine;

fn temp_dir(prefix: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = format!("/tmp/hrml_var_res_{}_{}", prefix, now);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn dollar_var_in_text_resolves_against_context() {
    let dir = temp_dir("text");
    fs::create_dir_all(format!("{}/pages", dir)).unwrap();
    fs::write(
        format!("{}/pages/index.hrml", dir),
        r#"<a href="/blog/$slug" class="row">$title</a>"#,
    )
    .unwrap();

    let engine = Engine::new(&dir);
    let html = engine
        .render_fragment(
            "pages/index.hrml",
            &json!({"slug": "intro", "title": "Hello"}),
        )
        .unwrap();
    assert!(
        html.contains("/blog/intro"),
        "expected slug substitution, got: {html}"
    );
    assert!(
        html.contains(">Hello<"),
        "expected title substitution, got: {html}"
    );
}

#[test]
fn dollar_var_dot_path_resolves() {
    let dir = temp_dir("dot");
    fs::create_dir_all(format!("{}/pages", dir)).unwrap();
    fs::write(
        format!("{}/pages/index.hrml", dir),
        r#"<p>$post.title by $post.author</p>"#,
    )
    .unwrap();

    let engine = Engine::new(&dir);
    let html = engine
        .render_fragment(
            "pages/index.hrml",
            &json!({"post": {"title": "Foo", "author": "Ada"}}),
        )
        .unwrap();
    assert!(html.contains("Foo by Ada"), "got: {html}");
}

#[test]
fn dollar_var_keeps_unmatched_dot_suffix_as_literal() {
    let dir = temp_dir("suffix");
    fs::create_dir_all(format!("{}/pages", dir)).unwrap();
    fs::write(
        format!("{}/pages/index.hrml", dir),
        r#"<p>posts/$slug.mdx</p>"#,
    )
    .unwrap();

    let engine = Engine::new(&dir);
    let html = engine
        .render_fragment("pages/index.hrml", &json!({"slug": "intro"}))
        .unwrap();
    assert!(html.contains("posts/intro.mdx"), "got: {html}");
}

#[test]
fn no_template_file_contains_nested_quote_directive() {
    // Putting code in quotes is illegal. This catches re-introductions of
    // patterns like `href="<?get id="x"?>"` across the repo's templates.
    use std::collections::VecDeque;
    let mut queue: VecDeque<std::path::PathBuf> = VecDeque::new();
    queue.push_back(std::path::PathBuf::from("usi/templates"));
    queue.push_back(std::path::PathBuf::from("validation/cases"));

    let mut offenders = Vec::new();
    while let Some(p) = queue.pop_front() {
        if !p.exists() {
            continue;
        }
        for entry in fs::read_dir(&p).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                queue.push_back(path);
                continue;
            }
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(ext, "hrml" | "trml" | "toml") {
                continue;
            }
            let body = fs::read_to_string(&path).unwrap_or_default();
            scan_for_nested_quote(&path, &body, &mut offenders);
        }
    }
    assert!(
        offenders.is_empty(),
        "nested-quote directives in template files (lift to $var):\n{}",
        offenders.join("\n")
    );
}

fn scan_for_nested_quote(path: &Path, body: &str, out: &mut Vec<String>) {
    for (i, line) in body.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut j = 0;
        while j + 1 < bytes.len() {
            if bytes[j] == b'=' && (bytes[j + 1] == b'"' || bytes[j + 1] == b'\'') {
                let quote = bytes[j + 1];
                let mut k = j + 2;
                while k + 1 < bytes.len() && bytes[k] != quote {
                    if bytes[k] == b'<' && bytes[k + 1] == b'?' {
                        out.push(format!(
                            "{}:{}: nested `<?` inside attribute value",
                            path.display(),
                            i + 1
                        ));
                        break;
                    }
                    k += 1;
                }
                j = k + 1;
            } else {
                j += 1;
            }
        }
    }
}
