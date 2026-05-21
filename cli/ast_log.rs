use hrml::project::Project;
use std::fs;
use std::path::Path;

pub fn write_ast_log(project_path: &Path, config: &hrml::config::Config) -> Result<(), String> {
    let mut project = Project::new(config.clone());
    let templates_path = project_path.join(&config.templates_path);
    load_dir_into_project(&mut project, &templates_path, &templates_path)?;

    let endpoints_path = project_path.join(&config.endpoints_path);
    load_dir_into_project(&mut project, &endpoints_path, &endpoints_path)?;

    project.parse_all().map_err(|e| e.to_string())?;

    let log_path = project_path.join("ast.log");
    fs::write(&log_path, format_project(&project, project_path))
        .map_err(|e| format!("Failed to write {}: {}", log_path.display(), e))?;

    println!("AST log written to {}", log_path.display());
    Ok(())
}

fn format_project(project: &Project, project_path: &Path) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    let _ = writeln!(out, "Project {{");
    let _ = writeln!(out, "  dependency_order: {:?},", project.dependency_order);
    let _ = writeln!(out, "  files: [");

    let rel_prefix = project_path.to_string_lossy().into_owned();
    for (rel_path, file) in &project.files {
        let _ = writeln!(out, "    HrFile {{");
        let full = project_path.join(rel_path);
        let display = full
            .strip_prefix(&rel_prefix)
            .unwrap_or(&full)
            .to_string_lossy();
        let _ = writeln!(out, "      path: {:?},", display);
        let _ = writeln!(out, "      text: {:?},", file.text);
        let _ = writeln!(out, "      tree: {:#?},", file.tree);
        let _ = writeln!(out, "    }},");
    }

    let _ = writeln!(out, "  ],");
    let _ = writeln!(out, "}}");
    out
}

fn load_dir_into_project(project: &mut Project, base: &Path, dir: &Path) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read dir: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            load_dir_into_project(project, base, &path)?;
        } else if path.extension().map(|e| e == "hrml").unwrap_or(false) {
            let rel_path = path
                .strip_prefix(base)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .to_string();
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
            project.add_file(rel_path, content);
        }
    }
    Ok(())
}
