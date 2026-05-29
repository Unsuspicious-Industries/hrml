use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use xrml::config::Config;
use xrml::project::Project;
use xrml::template::Engine;

struct Env {
    dir: PathBuf,
}

impl Env {
    fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hrml_trml_{name}_{now}"));
        let _ = fs::create_dir_all(&dir);
        Self { dir }
    }

    fn write(&self, path: &str, content: &str) {
        let full = self.dir.join(path);
        if let Some(parent) = Path::new(&full).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&full, content).unwrap();
    }

    fn render(&self, path: &str) -> Result<String, String> {
        Engine::new(self.dir.to_str().unwrap())
            .render(path, &json!({}))
            .map_err(|e| e.to_string())
    }

    fn render_project(&self, files: &[&str], target: &str) -> Result<String, String> {
        let mut project = Project::new(Config::default()).with_base_path(&self.dir);
        for &path in files {
            let source = fs::read_to_string(self.dir.join(path)).unwrap();
            project.add_file(path.to_string(), source);
        }
        project.parse_all().map_err(|e| e.to_string())?;
        project
            .render(target, &json!({}))
            .map_err(|e| e.to_string())
    }

    fn assert_trml_equivalent(&self, name: &str, hrml: &str, trml: &str) {
        let hp = format!("{name}.hrml");
        let tp = format!("{name}.trml");
        self.write(&hp, hrml);
        self.write(&tp, trml);
        let h = self.render(&hp).unwrap();
        let t = self.render(&tp).unwrap();
        let h = normalize_ws(&h);
        let t = normalize_ws(&t);
        assert_eq!(
            h, t,
            "TRML ≠ HRML for {name}\n=== HRML ===\n{h}\n=== TRML ===\n{t}"
        );
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn normalize_ws(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

// ────────────────────────────────────────────────
//  Void/element directives
// ────────────────────────────────────────────────

#[test]
fn component_and_use() {
    let env = Env::new("c_u");
    env.assert_trml_equivalent(
        "pages/test",
        r#"<?component id="x"?><span>X</span></?component?>
<?use id="x"?></?use?>"#,
        r#"[component id="x"]
  [span]
  X
[use id="x"]"#,
    );
}

#[test]
fn component_slot_and_block() {
    let env = Env::new("c_slot");
    env.assert_trml_equivalent(
        "pages/test",
        r#"<?component id="c"?><article><?slot id="body"?>Default</?slot?></article></?component?>
<?use id="c"?><?block slot="body"?>Custom</?block?></?use?>"#,
        r#"[component id="c"]
  [article]
    [slot id="body"]
    Default
[use id="c"]
  [block slot="body"]
  Custom"#,
    );
}

#[test]
fn bind_value_and_get() {
    let env = Env::new("bvg");
    env.assert_trml_equivalent(
        "pages/test",
        r#"<?bind var="title"?>Hello<?/bind?>
<h1><?get id="title"?></h1>"#,
        r#"[bind]
var = "title"
Hello
[h1]
  [get]
  id = "title""#,
    );
}

#[test]
fn bind_block_content() {
    let env = Env::new("bbc");
    env.assert_trml_equivalent(
        "pages/test",
        r#"<?bind var="x"?>Hello<?/bind?>
<p><?get id="x"?></p>"#,
        r#"[bind]
var = "x"
Hello
[p]
  [get]
  id = "x""#,
    );
}

#[test]
fn if_true_branch() {
    let env = Env::new("if_true");
    env.assert_trml_equivalent(
        "pages/test",
        r#"<?bind var="s"?>1<?/bind?>
<?if cond="$s"?><h1>Yes</h1><?else?><h1>No</h1><?/if?>"#,
        r#"[bind]
var = "s"
1
[if cond="$s"]
  [h1]
  Yes
  [else]
  [h1]
  No"#,
    );
}

#[test]
fn if_false_branch() {
    let env = Env::new("if_false");
    env.assert_trml_equivalent(
        "pages/test",
        r#"<?if cond="$n"?><h1>No</h1><?else?><h1>Yes</h1><?/if?>"#,
        r#"[if cond="$n"]
  [h1]
  No
  [else]
  [h1]
  Yes"#,
    );
}

// ────────────────────────────────────────────────
//  Data pipeline
// ────────────────────────────────────────────────

#[test]
fn data_and_map() {
    let env = Env::new("dm");
    env.write("items/a.json", r#"{"title":"A"}"#);
    env.write("items/b.json", r#"{"title":"B"}"#);
    env.assert_trml_equivalent(
        "pages/test",
        r#"<?data from="items" as="items"?>
<?map over="items" as="i"?><li><?get id="i.title"?></li></?map?>"#,
        r#"[data]
from = "items"
as = "items"
[map over="items" as="i"]
  [li]
    [get]
    id = "i.title""#,
    );
}

#[test]
fn sort_and_filter() {
    let env = Env::new("sf");
    env.write("p/a.json", r#"{"n":"B","prio":"2"}"#);
    env.write("p/b.json", r#"{"n":"A","prio":"1"}"#);
    env.write("p/c.json", r#"{"n":"C","prio":"3"}"#);
    env.assert_trml_equivalent(
        "pages/test",
        r#"<?data from="p" as="items"?>
<?sort over="items" by="prio" as="items"?>
<ul><?map over="items" as="i"?><li><?get id="i.n"?></li></?map?></ul>"#,
        r#"[data]
from = "p"
as = "items"
[sort]
over = "items"
by = "prio"
as = "items"
[ul]
  [map over="items" as="i"]
    [li]
      [get]
      id = "i.n""#,
    );
}

#[test]
fn record_inline() {
    let env = Env::new("rec");
    env.assert_trml_equivalent(
        "pages/test",
        r#"<?record as="r"?>
<?field name="k" value="Key"?>
<?field name="v" value="Val"?>
<?/record?>
<dl><dt><?get id="r.k"?></dt><dd><?get id="r.v"?></dd></dl>"#,
        r#"[record as="r"]
  [field]
  name = "k"
  value = "Key"
  [field]
  name = "v"
  value = "Val"
[dl]
  [dt]
    [get]
    id = "r.k"
  [dd]
    [get]
    id = "r.v""#,
    );
}

// ────────────────────────────────────────────────
//  Component composition (separate files for cross-referencing)
// ────────────────────────────────────────────────

#[test]
fn component_loading_another_component() {
    let env = Env::new("clc");
    env.write("components/btn.hrml", r#"<?component id="btn"?><?bind var="label"/><?bind var="url"/><a href="$url"><?get id="label"?></a><?/component?>"#);
    env.write(
        "components/btn.trml",
        r#"[component id="btn"]
  [bind]
  var = "label"
  [bind]
  var = "url"
  [a]
  href = "/x"
    [get]
    id = "label""#,
    );
    env.write(
        "pages/test.hrml",
        r#"<?load file="components/btn.hrml"?>
<?use id="btn"?><?bind var="label"?>Go<?/bind?><?bind var="url"?>/x<?/bind?></?use?>"#,
    );
    env.write(
        "pages/test.trml",
        r#"[load file="components/btn.trml"]
[use id="btn"]
  [bind]
  var = "label"
  Go
  [bind]
  var = "url"
  /x"#,
    );
    let h = env.render("pages/test.hrml").unwrap();
    let t = env.render("pages/test.trml").unwrap();
    assert_eq!(normalize_ws(&h), normalize_ws(&t));
}

#[test]
fn component_with_slot_defaults() {
    let env = Env::new("cwsd");
    env.write("components/pan.hrml", r#"<?component id="pan"?><?slot id="h"?>DefH<?/slot?><div><?slot id="b"?><p>DefB</p><?/slot?></div><?/component?>"#);
    env.write(
        "components/pan.trml",
        r#"[component id="pan"]
  [slot id="h"]
  DefH
  [div]
    [slot id="b"]
      [p]
      DefB"#,
    );
    env.write("pages/test.hrml", r#"<?load file="components/pan.hrml"?>
<?use id="pan"?><?block slot="h"?><h1>C</h1><?/block?><?block slot="b"?><p>O</p><?/block?></?use?>"#);
    env.write(
        "pages/test.trml",
        r#"[load file="components/pan.trml"]
[use id="pan"]
  [block slot="h"]
    [h1]
    C
  [block slot="b"]
    [p]
    O"#,
    );
    let h = env.render("pages/test.hrml").unwrap();
    let t = env.render("pages/test.trml").unwrap();
    assert_eq!(normalize_ws(&h), normalize_ws(&t));
}

// ────────────────────────────────────────────────
//  Layout with project renderer
// ────────────────────────────────────────────────

#[test]
fn layout_slot_injection() {
    let env = Env::new("lsi");
    env.write(
        "layouts/base.hrml",
        "<html><body><?slot id=\"content\"?><?/slot?></body></html>",
    );
    env.write(
        "layouts/base.trml",
        "[html]\n  [body]\n    [slot id=\"content\"]",
    );
    env.write(
        "pages/p.hrml",
        "<?load file=\"layouts/base.hrml\"?><?block slot=\"content\"?><h1>Hi</h1><?/block?>",
    );
    env.write(
        "pages/p.trml",
        "[load file=\"layouts/base.trml\"]\n[block slot=\"content\"]\n  [h1]\n  Hi",
    );
    let h = env
        .render_project(&["layouts/base.hrml", "pages/p.hrml"], "pages/p.hrml")
        .unwrap();
    let t = env
        .render_project(&["layouts/base.trml", "pages/p.trml"], "pages/p.trml")
        .unwrap();
    assert_eq!(normalize_ws(&h), normalize_ws(&t));
}

#[test]
fn layout_conditional_slot() {
    let env = Env::new("lcs");
    env.write("layouts/cond.hrml", "<html><body><?if cond=\"$show\"?><header><?slot id=\"banner\"?><?/slot?></header><?/if?><main><?slot id=\"body\"?><?/slot?></main></body></html>");
    env.write("layouts/cond.trml", "[html]\n  [body]\n    [if cond=\"$show\"]\n      [header]\n        [slot id=\"banner\"]\n    [main]\n      [slot id=\"body\"]");
    env.write("pages/with.hrml", "<?bind var=\"show\"?>1<?/bind?><?load file=\"layouts/cond.hrml\"?><?block slot=\"banner\"?><h1>P</h1><?/block?><?block slot=\"body\"?><p>M</p><?/block?>");
    env.write("pages/with.trml", "[bind]\nvar = \"show\"\n1\n[load file=\"layouts/cond.trml\"]\n[block slot=\"banner\"]\n  [h1]\n  P\n[block slot=\"body\"]\n  [p]\n  M");
    let h = env
        .render_project(&["layouts/cond.hrml", "pages/with.hrml"], "pages/with.hrml")
        .unwrap();
    let t = env
        .render_project(&["layouts/cond.trml", "pages/with.trml"], "pages/with.trml")
        .unwrap();
    assert_eq!(normalize_ws(&h), normalize_ws(&t));
}

#[test]
fn markdownfm_frontmatter() {
    let env = Env::new("mdfm");
    env.write("notes/p.mdx", "---\ntitle: P\ndate: 2025\n---\n\nBody.");
    env.assert_trml_equivalent(
        "pages/test",
        r#"<?markdownfm file="notes/p.mdx" as="post"?>
<h1><?get id="post.title"?></h1>
<p><?get id="post.body"?></p>"#,
        r#"[markdownfm]
file = "notes/p.mdx"
as = "post"
[h1]
  [get]
  id = "post.title"
[p]
  [get]
  id = "post.body""#,
    );
}

#[test]
fn mixed_html_and_directives() {
    let env = Env::new("mixed");
    env.assert_trml_equivalent(
        "pages/test",
        r#"<?bind var="n"?>Alice<?/bind?>
<div class="card">
  <h2><?get id="n"?></h2>
  <p>Hello world</p>
</div>"#,
        r#"[bind]
var = "n"
Alice
[div class="card"]
  [h2]
    [get]
    id = "n"
  [p]
  Hello world"#,
    );
}
