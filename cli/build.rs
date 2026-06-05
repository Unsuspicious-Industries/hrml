use crate::ast_log;
use crate::project::load_project;
use std::fs;
use std::path::Path;
use xrml::paths;

pub fn build_site(project_path: &Path, log_ast: bool) -> Result<(), String> {
    let mut project = load_project(project_path)?;

    if log_ast {
        ast_log::write_ast_log(project_path, &project.config)?;
    }

    let dist_path = project_path.join("dist");

    if dist_path.exists() {
        fs::remove_dir_all(&dist_path).map_err(|e| format!("Failed to remove dist: {}", e))?;
    }
    fs::create_dir_all(&dist_path).map_err(|e| format!("Failed to create dist: {}", e))?;

    println!("Building static site to 'dist/'...");

    project.parse_all().map_err(|e| e.to_string())?;

    // Fail loud on a `<?use?>` that names no defined component — a typo or a
    // component missing from the shared library — rather than shipping silent
    // empty output.
    let undefined = project.undefined_component_uses();
    if !undefined.is_empty() {
        let mut msg = String::from("Undefined component(s) referenced:\n");
        for (page, id) in &undefined {
            msg.push_str(&format!(
                "  {page}: <?use id=\"{id}\"?> — no such component\n"
            ));
        }
        return Err(msg);
    }

    // Each page expands to one or more concrete routes: a static page is a single
    // route, while a dynamic `[param]` page fans out over the collection it binds
    // (see `xrml::paths`). Render every concrete route and write it under dist/.
    let page_paths: Vec<String> = project.pages().map(String::from).collect();
    let mut rendered_count = 0;
    for path in page_paths {
        if !path.starts_with("pages/") {
            continue;
        }

        let params = paths::route_params(&path);
        let page_nodes = project
            .get_file(&path)
            .and_then(|f| f.tree.as_ref())
            .map(|t| t.nodes.clone())
            .unwrap_or_default();
        let bindings = paths::expand(&params, &page_nodes, project_path);

        for binding in bindings {
            let data = serde_json::to_value(&binding).unwrap_or_else(|_| serde_json::json!({}));
            let html = project
                .render(&path, &data)
                .map_err(|e| format!("Failed to render {}: {}", path, e))?;

            let out_name = paths::output_path(&path, &binding);
            let out_path = dist_path.join(&out_name);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
            }
            // A finished page carries no meaningful surrounding whitespace; emit it
            // tidily (trimmed, single trailing newline) so output is stable however
            // the page was authored — explicit loads or auto-layout.
            let document = format!("{}\n", html.trim());
            fs::write(&out_path, &document)
                .map_err(|e| format!("Failed to write {}: {}", out_name, e))?;
            rendered_count += 1;
        }
    }

    let static_src = project_path.join(&project.config.static_path);
    let static_dst = dist_path.join(&project.config.static_path);
    copy_dir_recursive(&static_src, &static_dst)?;

    println!("Done! {} pages rendered to dist/", rendered_count);
    println!(
        "Run '{} serve' to preview the built site.",
        env!("CARGO_BIN_NAME")
    );

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }

    fs::create_dir_all(dst).map_err(|e| format!("Failed to create dir: {}", e))?;

    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Entry error: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("Failed to copy: {}", e))?;
        }
    }

    Ok(())
}
