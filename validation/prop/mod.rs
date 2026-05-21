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
        hrml::template::Engine::new(self.dir.to_str().unwrap())
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
    )
}

proptest! {
    #[test]
    fn bind_and_get_roundtrip(value in safe_text()) {
        let env = PropEnv::new("bind_get");
        env.write(
            "pages/test.hrml",
            &format!(r#"<?bind var="title" value="{}"/?>
<h1><?get id="title"?></h1>"#, value),
        );

        let html = env.render("pages/test.hrml").unwrap();
        let expected = format!("<h1>{}</h1>", value);
        prop_assert!(html.contains(&expected));
    }

    #[test]
    fn component_slot_replaces_default(slot_content in safe_text()) {
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
    fn load_component_bind_and_use_are_connected(value in safe_text()) {
        let value = if value.is_empty() { "loaded".to_string() } else { value };
        let env = PropEnv::new("load_component_bind");
        env.write(
            "components/card.hrml",
            r#"<?component id="card"?><article class="card"><?slot id="body"?>Default</?slot?></article></?component?>"#,
        );
        env.write(
            "pages/test.hrml",
            &format!(r#"<?load file="components/card.hrml"?>
<?bind var="title" value="{}"/?>
<?use id="card"?><?block slot="body"?><h2><?get id="title"?></h2></?block?></?use?>"#, value),
        );

        let html = env.render("pages/test.hrml").unwrap();
        prop_assert!(html.contains("class=\"card\""));
        let expected = format!("<h2>{}</h2>", value);
        prop_assert!(html.contains(&expected));
        prop_assert!(!html.contains("Default"));
    }

    #[test]
    fn child_bindings_feed_component_use(value in safe_text()) {
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
    fn unknown_directives_are_code_errors(name in "[a-z]{1,12}") {
        prop_assume!(!known_directive(&name));
        let env = PropEnv::new("unknown_directive");
        env.write("pages/test.hrml", &format!("<?{}?>", name));

        let err = env.render("pages/test.hrml").unwrap_err();
        prop_assert!(err.contains("Unknown HRML directive"));
        prop_assert!(err.contains("Code"));
    }

    #[test]
    fn malformed_directive_openers_are_errors(name in "[a-z]{1,12}") {
        let env = PropEnv::new("malformed_opener");
        env.write("pages/test.hrml", &format!("<?{}", name));

        let err = env.render("pages/test.hrml").unwrap_err();
        prop_assert!(err.contains("Unclosed HRML directive"));
        prop_assert!(err.contains("Code"));
    }
}
