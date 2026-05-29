use crate::ast_log;
use crate::project::load_project;
use std::fs;
use std::path::Path;

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

    let mut rendered_count = 0;
    for path in project.pages() {
        if !path.starts_with("pages/") {
            continue;
        }

        let html = match project.render(path, &serde_json::json!({})) {
            Ok(html) => html,
            Err(e) => {
                return Err(format!("Failed to render {}: {}", path, e));
            }
        };

        let out_name = if path.starts_with("pages/") {
            let name = path.strip_prefix("pages/").unwrap().to_string();
            if let Some(stem) = name.strip_suffix(".hrml").or_else(|| name.strip_suffix(".trml")) {
                stem.to_string() + ".html"
            } else {
                name
            }
        } else if let Some(stem) = path.strip_suffix(".hrml").or_else(|| path.strip_suffix(".trml")) {
            stem.to_string() + ".html"
        } else {
            path.to_string()
        };

        let out_path = dist_path.join(&out_name);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
        }

        fs::write(&out_path, &html).map_err(|e| format!("Failed to write {}: {}", out_name, e))?;
        rendered_count += 1;
    }

    let static_src = project_path.join(&project.config.static_path);
    copy_dir_recursive(&static_src, &dist_path)?;

    println!("Done! {} pages rendered to dist/", rendered_count);
    println!("Run 'hrml serve' to preview the built site.");

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
