use crate::validation;
use std::fs;
use std::io;
use std::path::Path;
use xrml::{assets, config::Config, project::Project, router};

pub fn create_project(name: &str) -> io::Result<()> {
    let project_path = Path::new(name);
    if project_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("Directory '{}' already exists", name),
        ));
    }

    let config = Config {
        site_name: name.to_string(),
        ..Config::default()
    };

    fs::create_dir_all(project_path)?;
    setup_project_files(project_path, &config)
}

pub(crate) fn load_project_config(project_path: &Path) -> Result<Config, String> {
    let config_path = project_path.join("xrml.toml");
    if !config_path.exists() {
        return Err(format!("No xrml.toml found in {}", project_path.display()));
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read {}: {}", config_path.display(), e))?;

    Config::from_toml(&content).map_err(|e| format!("Failed to load config: {}", e))
}

pub fn load_project(path: &Path) -> Result<Project, String> {
    let config = load_project_config(path)?;
    let templates_path = path.join(&config.templates_path);

    let mut project = Project::new(config).with_base_path(path);
    load_dir_into_project(&mut project, &templates_path, &templates_path)?;

    Ok(project)
}

fn load_dir_into_project(project: &mut Project, base: &Path, dir: &Path) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read dir: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_dir() {
            load_dir_into_project(project, base, &path)?;
        } else if path
            .extension()
            .map(|e| e == "hrml" || e == "trml")
            .unwrap_or(false)
        {
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

pub(crate) fn validate_project(path: &Path) -> Result<(), String> {
    let config_path = path.join("xrml.toml");
    if !config_path.exists() {
        return Err(format!("File not found: {}", config_path.display()));
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read {}: {}", config_path.display(), e))?;
    let config =
        Config::from_toml(&content).map_err(|e| format!("Failed to load xrml.toml: {}", e))?;

    let templates_path = path.join(&config.templates_path);
    if !templates_path.exists() {
        return Err(format!(
            "Templates directory not found: {}\nCreate it or update xrml.toml",
            templates_path.display()
        ));
    }

    let base_layout = templates_path.join("layouts/base.hrml");
    if !base_layout.exists() {
        eprintln!(
            "[WARNING] Base layout not found at {}",
            base_layout.display()
        );
    }

    let index_page = templates_path.join("pages/index.hrml");
    if !index_page.exists() {
        eprintln!("[WARNING] Index page not found at {}", index_page.display());
    }

    let static_path = path.join(&config.static_path);
    if !static_path.exists() {
        eprintln!(
            "[WARNING] Static directory not found: {}",
            static_path.display()
        );
    }

    let mut project = Project::new(config.clone());
    load_dir_into_project(&mut project, &templates_path, &templates_path)?;

    match project.render("pages/index.hrml", &serde_json::json!({})) {
        Ok(_) => println!("[OK] Index template renders successfully"),
        Err(e) => eprintln!("[WARNING] Index template failed to render: {}", e),
    }

    println!("[OK] Project validation complete");
    Ok(())
}

pub fn check_project(path: &Path) -> Result<(), String> {
    let config = load_project_config(path)?;
    let templates_path = path.join(&config.templates_path);

    if !templates_path.exists() {
        return Err(format!(
            "Templates dir not found: {}",
            templates_path.display()
        ));
    }

    let pages_dir = templates_path.join("pages");
    let router = router::Router::from_pages_dir(&pages_dir);
    println!("Routes found: {}", router.routes.len());
    for route in &router.routes {
        println!("  {} -> {} ({:?})", route.path, route.template, route.kind);
    }

    let mut project = Project::new(config).with_base_path(path);
    load_dir_into_project(&mut project, &templates_path, &templates_path)?;
    project.parse_all().map_err(|e| e.to_string())?;

    let mut errors = 0;
    let mut ok = 0;

    for (page, id) in project.undefined_component_uses() {
        eprintln!("[ERROR] {page}: <?use id=\"{id}\"?> — no such component");
        errors += 1;
    }

    for route in router.static_routes() {
        let template_path = if route.template.starts_with("pages/") {
            route.template.clone()
        } else {
            format!("pages/{}", route.template)
        };

        match project.render(&template_path, &serde_json::json!({})) {
            Ok(html) => {
                if html.contains("<?") || html.contains("?>") {
                    eprintln!("[WARN] {} has unprocessed directives", route.path);
                }
                if html.contains("<!DOCTYPE") && html.matches("<html").count() != 1 {
                    eprintln!("[WARN] {} has malformed HTML", route.path);
                }
                ok += 1;
            }
            Err(e) => {
                eprintln!("[ERROR] {} failed: {}", route.path, e);
                errors += 1;
            }
        }
    }

    let cases_root = path.join("validation/cases");
    if cases_root.exists() {
        let records = validation::run_cases(&cases_root)?;
        let (_, failed) = validation::summarize(&records);

        if failed > 0 {
            eprintln!("\nValidation case failures:");
            for record in records
                .iter()
                .filter(|record| record.error.is_some() || !record.matched)
            {
                eprintln!("[FAIL] {}", validation::failure_message(record));
            }
            errors += failed;
        } else {
            println!("Validation cases: {} passed", records.len());
        }
    }

    println!("\nCheck complete: {} ok, {} errors", ok, errors);
    if errors > 0 {
        Err(format!("{} templates failed to render", errors))
    } else {
        Ok(())
    }
}

fn setup_project_files(project_path: &Path, config: &Config) -> io::Result<()> {
    fs::create_dir_all(project_path.join(&config.templates_path).join("pages"))?;
    fs::create_dir_all(project_path.join(&config.templates_path).join("layouts"))?;
    fs::create_dir_all(project_path.join(&config.templates_path).join("components"))?;
    fs::create_dir_all(project_path.join(&config.endpoints_path).join("api"))?;
    fs::create_dir_all(project_path.join(&config.static_path).join("css"))?;
    fs::create_dir_all(project_path.join(&config.static_path).join("js"))?;
    fs::create_dir_all(project_path.join(&config.static_path).join("images"))?;

    let config_path = project_path.join("xrml.toml");
    if !config_path.exists() {
        fs::write(&config_path, assets::default_config(&config.site_name))?;
    }

    let templates_dir = project_path.join(&config.templates_path);

    let base_layout_path = templates_dir.join("layouts/base.hrml");
    if !base_layout_path.exists() {
        fs::write(base_layout_path, assets::BASE_LAYOUT)?;
    }

    let nav_path = templates_dir.join("components/nav.hrml");
    if !nav_path.exists() {
        fs::write(nav_path, assets::NAV)?;
    }

    let index_path = templates_dir.join("pages/index.hrml");
    if !index_path.exists() {
        fs::write(index_path, assets::INDEX_PAGE)?;
    }

    let about_path = templates_dir.join("pages/about.hrml");
    if !about_path.exists() {
        fs::write(about_path, assets::ABOUT_PAGE)?;
    }

    let css_path = project_path.join(&config.static_path).join("css/style.css");
    if !css_path.exists() {
        fs::write(css_path, assets::STYLE_CSS)?;
    }

    let hello_path = project_path
        .join(&config.endpoints_path)
        .join("api/hello.hrml");
    if !hello_path.exists() {
        fs::write(hello_path, assets::HELLO_ENDPOINT)?;
    }

    let readme_path = project_path.join("README.md");
    if !readme_path.exists() {
        fs::write(readme_path, assets::readme(&config.site_name))?;
    }

    let gitignore_path = project_path.join(".gitignore");
    if !gitignore_path.exists() {
        fs::write(gitignore_path, assets::GITIGNORE)?;
    }

    Ok(())
}
