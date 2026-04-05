// HRML Static Site Generator
//
// Unix philosophy: one job, do it well.
// Reads .hrml templates from a directory, renders each to HTML, writes to output.
// No magic, no config files, just files in → files out.

use crate::template::Engine;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SSG {
    pub pages_dir: PathBuf,
    pub output_dir: PathBuf,
    pub data: Value,
    pub engine: Engine,
}

impl SSG {
    pub fn new(pages_dir: &str, output_dir: &str) -> Self {
        Self {
            pages_dir: PathBuf::from(pages_dir),
            output_dir: PathBuf::from(output_dir),
            data: Value::Null,
            engine: Engine::new(pages_dir),
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    /// Build all pages to static HTML
    pub fn build(&self) -> Result<BuildReport, String> {
        let mut report = BuildReport::new();

        // Create output directory
        fs::create_dir_all(&self.output_dir)
            .map_err(|e| format!("Failed to create output dir: {}", e))?;

        // Copy static assets if they exist
        let static_dir = self.pages_dir.join("static");
        if static_dir.exists() {
            copy_dir(&static_dir, &self.output_dir.join("static"))
                .map_err(|e| format!("Failed to copy static assets: {}", e))?;
        }

        // Render each page
        self.render_pages(&self.pages_dir, &self.output_dir, &mut report)?;

        // Generate sitemap
        self.generate_sitemap(&report)?;

        // Generate 404 page if not already rendered
        if !report.pages.iter().any(|p| p.url == "/404") {
            self.generate_404(&mut report)?;
        }

        Ok(report)
    }

    fn render_pages(
        &self,
        src_dir: &Path,
        out_dir: &Path,
        report: &mut BuildReport,
    ) -> Result<(), String> {
        let entries = fs::read_dir(src_dir)
            .map_err(|e| format!("Failed to read {}: {}", src_dir.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_dir() {
                // Skip static directory (handled separately)
                if path.file_name().map(|n| n == "static").unwrap_or(false) {
                    continue;
                }
                let sub_out = out_dir.join(path.file_name().unwrap());
                fs::create_dir_all(&sub_out).map_err(|e| e.to_string())?;
                self.render_pages(&path, &sub_out, report)?;
            } else if path.extension().map(|e| e == "hrml").unwrap_or(false) {
                let stem = path.file_stem().unwrap().to_string_lossy();

                // Skip dynamic routes (contain [ or ])
                if stem.contains('[') || stem.contains(']') {
                    continue;
                }

                let rel_path = path
                    .strip_prefix(&self.pages_dir)
                    .map_err(|e| e.to_string())?;
                let rel_str = rel_path.to_string_lossy().replace('\\', "/");

                // Determine output path
                let out_path = if stem == "index" {
                    out_dir.join("index.html")
                } else {
                    let page_dir = out_dir.join(&*stem);
                    fs::create_dir_all(&page_dir).map_err(|e| e.to_string())?;
                    page_dir.join("index.html")
                };

                // Render
                match self.engine.render(&rel_str, &self.data) {
                    Ok(html) => {
                        fs::write(&out_path, &html).map_err(|e| e.to_string())?;
                        let url = self.path_to_url(&rel_str, &stem);
                        report.add_page(url, rel_str, out_path.to_string_lossy().to_string());
                    }
                    Err(e) => {
                        report.add_error(rel_str, e);
                    }
                }
            }
        }

        Ok(())
    }

    fn path_to_url(&self, rel_path: &str, stem: &str) -> String {
        let path = rel_path
            .trim_end_matches(".hrml")
            .trim_end_matches("/index");

        if path.is_empty() || path == "index" {
            "/".to_string()
        } else if stem == "index" {
            format!("/{}", path.trim_end_matches("/index"))
        } else {
            format!("/{}", path)
        }
    }

    fn generate_sitemap(&self, report: &BuildReport) -> Result<(), String> {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push_str("\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

        for page in &report.pages {
            xml.push_str("  <url>\n");
            xml.push_str(&format!("    <loc>{}</loc>\n", page.url));
            xml.push_str("  </url>\n");
        }

        xml.push_str("</urlset>\n");

        let sitemap_path = self.output_dir.join("sitemap.xml");
        fs::write(&sitemap_path, &xml).map_err(|e| e.to_string())?;

        Ok(())
    }

    fn generate_404(&self, report: &mut BuildReport) -> Result<(), String> {
        // Check for 404.hrml in pages dir
        let not_found = self.pages_dir.join("404.hrml");
        let html = if not_found.exists() {
            self.engine
                .render("404.hrml", &self.data)
                .unwrap_or_else(|_| self.default_404())
        } else {
            self.default_404()
        };

        let out_path = self.output_dir.join("404.html");
        fs::write(&out_path, &html).map_err(|e| e.to_string())?;
        report.add_page(
            "/404".to_string(),
            "404.hrml".to_string(),
            out_path.to_string_lossy().to_string(),
        );

        Ok(())
    }

    fn default_404(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>404 Not Found</title>
</head>
<body>
<h1>404</h1>
<p>Page not found.</p>
</body>
</html>"#
        )
    }
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct BuildReport {
    pub pages: Vec<PageInfo>,
    pub errors: Vec<ErrorInfo>,
}

#[derive(Debug, Clone)]
pub struct PageInfo {
    pub url: String,
    pub template: String,
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct ErrorInfo {
    pub template: String,
    pub error: String,
}

impl BuildReport {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn add_page(&mut self, url: String, template: String, output: String) {
        self.pages.push(PageInfo {
            url,
            template,
            output,
        });
    }

    pub fn add_error(&mut self, template: String, error: String) {
        self.errors.push(ErrorInfo { template, error });
    }

    pub fn summary(&self) -> String {
        format!(
            "Built {} pages, {} errors",
            self.pages.len(),
            self.errors.len()
        )
    }
}

impl Default for BuildReport {
    fn default() -> Self {
        Self::new()
    }
}
