use hrml::template::Engine;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Test harness for HRML validation suite.
/// Provides utilities to load fixtures, render templates, and compare against expected HTML.
pub struct ValidationHarness {
    pub fixture_root: PathBuf,
    pub expected_root: PathBuf,
    pub temp_dir: PathBuf,
}

impl ValidationHarness {
    pub fn new() -> Self {
        let fixture_root = PathBuf::from("validation/fixtures");
        let expected_root = PathBuf::from("validation/expected");
        let temp_dir = PathBuf::from(format!("/tmp/hrml_validation_{}", std::process::id()));

        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        Self {
            fixture_root,
            expected_root,
            temp_dir,
        }
    }

    /// Setup a test environment by copying fixtures into an isolated temp directory.
    /// Returns the path to the isolated test directory and an Engine configured for it.
    pub fn setup_test(&self, test_name: &str) -> (PathBuf, Engine) {
        let test_dir = self.temp_dir.join(test_name);
        let _ = fs::remove_dir_all(&test_dir);

        // Copy all fixture subdirectories
        let subdirs = [
            "layouts",
            "components",
            "pages",
            "content",
            "sections",
            "chrome",
            "static",
        ];
        for subdir in &subdirs {
            let src = self.fixture_root.join(subdir);
            let dst = test_dir.join(subdir);
            if src.exists() {
                copy_dir_all(&src, &dst).expect(&format!("Failed to copy {}", subdir));
            }
        }

        // Create missing directories
        for subdir in &subdirs {
            let _ = fs::create_dir_all(test_dir.join(subdir));
        }

        let engine = Engine::new(&test_dir.to_string_lossy());
        (test_dir, engine)
    }

    /// Setup test with custom site configuration.
    pub fn setup_test_with_config(
        &self,
        test_name: &str,
        site_name: &str,
        description: Option<&str>,
        favicon: Option<&str>,
    ) -> (PathBuf, Engine) {
        let (test_dir, engine) = self.setup_test(test_name);
        let engine = engine.with_site_name(site_name.to_string());
        let engine = match description {
            Some(d) => engine.with_description(Some(d.to_string())),
            None => engine,
        };
        let engine = match favicon {
            Some(f) => engine.with_favicon(Some(f.to_string())),
            None => engine,
        };
        (test_dir, engine)
    }

    /// Render a template and compare against expected HTML file.
    /// The expected file should be at `validation/expected/{category}/{test_name}.html`.
    pub fn assert_render(
        &self,
        test_name: &str,
        template_path: &str,
        category: &str,
        data: &Value,
    ) -> String {
        let (_test_dir, engine) = self.setup_test(test_name);

        let result = engine.render(template_path, data);

        match result {
            Ok(html) => {
                let expected_path = self
                    .expected_root
                    .join(category)
                    .join(format!("{}.html", test_name));
                if expected_path.exists() {
                    let expected = fs::read_to_string(&expected_path).expect(&format!(
                        "Failed to read expected file: {}",
                        expected_path.display()
                    ));

                    assert_eq!(
                        normalize_html(&html),
                        normalize_html(&expected),
                        "\n\n=== RENDER MISMATCH: {} ===\n\n--- Expected ---\n{}\n\n--- Got ---\n{}\n",
                        test_name,
                        expected,
                        html
                    );
                }
                html
            }
            Err(e) => panic!("Template '{}' failed to render: {}", template_path, e),
        }
    }

    /// Render a template and return the HTML without comparing to expected.
    pub fn render(&self, test_name: &str, template_path: &str, data: &Value) -> String {
        let (_test_dir, engine) = self.setup_test(test_name);
        engine
            .render(template_path, data)
            .expect(&format!("Template '{}' failed to render", template_path))
    }

    /// Render with custom engine configuration.
    pub fn render_with_config(
        &self,
        test_name: &str,
        template_path: &str,
        data: &Value,
        site_name: &str,
        description: Option<&str>,
        favicon: Option<&str>,
    ) -> String {
        let (_test_dir, engine) =
            self.setup_test_with_config(test_name, site_name, description, favicon);
        engine
            .render(template_path, data)
            .expect(&format!("Template '{}' failed to render", template_path))
    }

    /// Write additional test-specific templates.
    pub fn write_template(&self, test_dir: &Path, path: &str, content: &str) {
        let full_path = test_dir.join(path);
        if let Some(parent) = full_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&full_path, content).expect(&format!("Failed to write template: {}", path));
    }

    /// Clean up all temporary test directories.
    pub fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

impl Default for ValidationHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Chain test: renders a sequence of templates and verifies the final output.
pub struct ChainTest {
    harness: ValidationHarness,
    test_name: String,
    test_dir: PathBuf,
    engine: Engine,
}

impl ChainTest {
    pub fn new(harness: ValidationHarness, test_name: &str) -> Self {
        let (test_dir, engine) = harness.setup_test(test_name);
        Self {
            harness,
            test_name: test_name.to_string(),
            test_dir,
            engine,
        }
    }

    /// Add a template to the chain.
    pub fn add_template(&mut self, path: &str, content: &str) -> &mut Self {
        self.harness.write_template(&self.test_dir, path, content);
        self
    }

    /// Render the entry template and return the result.
    pub fn render(&self, entry: &str, data: &Value) -> Result<String, String> {
        self.engine.render(entry, data)
    }

    /// Render and assert against expected output.
    pub fn assert(&self, entry: &str, data: &Value, category: &str) -> String {
        let result = self.render(entry, data).expect(&format!(
            "Chain test '{}' failed to render '{}'",
            self.test_name, entry
        ));

        let expected_path = self
            .harness
            .expected_root
            .join(category)
            .join(format!("{}.html", &self.test_name));
        if expected_path.exists() {
            let expected = fs::read_to_string(&expected_path).expect(&format!(
                "Failed to read expected: {}",
                expected_path.display()
            ));

            assert_eq!(
                normalize_html(&result),
                normalize_html(&expected),
                "\n\n=== CHAIN TEST MISMATCH: {} ===\n\n--- Expected ---\n{}\n\n--- Got ---\n{}\n",
                self.test_name,
                expected,
                result
            );
        }

        result
    }

    /// Render and assert that the output contains specific strings.
    pub fn assert_contains(&self, entry: &str, data: &Value, expected_strings: &[&str]) -> String {
        let result = self.render(entry, data).expect(&format!(
            "Chain test '{}' failed to render '{}'",
            self.test_name, entry
        ));

        for s in expected_strings {
            assert!(
                result.contains(s),
                "Chain test '{}': expected output to contain '{}'\n\n--- Output ---\n{}\n",
                self.test_name,
                s,
                result
            );
        }

        result
    }

    /// Render and assert that the output does NOT contain specific strings.
    pub fn assert_not_contains(
        &self,
        entry: &str,
        data: &Value,
        forbidden_strings: &[&str],
    ) -> String {
        let result = self.render(entry, data).expect(&format!(
            "Chain test '{}' failed to render '{}'",
            self.test_name, entry
        ));

        for s in forbidden_strings {
            assert!(
                !result.contains(s),
                "Chain test '{}': expected output NOT to contain '{}'\n\n--- Output ---\n{}\n",
                self.test_name,
                s,
                result
            );
        }

        result
    }
}

/// Normalize HTML for comparison: collapse whitespace, trim lines, remove empty lines.
pub fn normalize_html(html: &str) -> String {
    html.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Recursively copy a directory.
pub fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

/// Write a fixture file to the fixtures directory.
pub fn write_fixture(relative_path: &str, content: &str) {
    let path = PathBuf::from("validation/fixtures").join(relative_path);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, content).expect(&format!("Failed to write fixture: {}", path.display()));
}

/// Write an expected output file.
pub fn write_expected(category: &str, test_name: &str, content: &str) {
    let path = PathBuf::from("validation/expected")
        .join(category)
        .join(format!("{}.html", test_name));
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, content).expect(&format!("Failed to write expected: {}", path.display()));
}

/// Bulk test runner: loads all .hrml files from a fixture subdirectory,
/// renders each, and compares against corresponding .html in expected/{category}/.
pub fn run_bulk_tests(fixture_subdir: &str, category: &str) {
    let harness = ValidationHarness::new();
    let fixture_path = harness.fixture_root.join(fixture_subdir);

    if !fixture_path.exists() {
        eprintln!("Fixture directory not found: {}", fixture_path.display());
        return;
    }

    let mut passed = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    for entry in walk_hrml_files(&fixture_path) {
        let stem = entry.file_stem().unwrap().to_string_lossy().to_string();
        let rel_path = entry
            .strip_prefix(&harness.fixture_root)
            .unwrap()
            .to_string_lossy()
            .to_string();

        let test_name = format!("{}_{}", category, stem);
        let (test_dir, engine) = harness.setup_test(&test_name);

        // Also copy the specific file if it's in a custom location
        if let Some(parent) = entry.parent() {
            let rel_parent = parent.strip_prefix(&harness.fixture_root).unwrap();
            let dst_parent = test_dir.join(rel_parent);
            let _ = fs::create_dir_all(&dst_parent);
            let _ = fs::copy(&entry, dst_parent.join(entry.file_name().unwrap()));
        }

        match engine.render(&rel_path, &serde_json::json!({})) {
            Ok(html) => {
                let expected_path = harness
                    .expected_root
                    .join(category)
                    .join(format!("{}.html", stem));
                if expected_path.exists() {
                    let expected = fs::read_to_string(&expected_path).unwrap();
                    if normalize_html(&html) == normalize_html(&expected) {
                        passed += 1;
                    } else {
                        failed += 1;
                        errors.push(format!(
                            "MISMATCH: {}\nExpected:\n{}\nGot:\n{}",
                            rel_path, expected, html
                        ));
                    }
                } else {
                    // No expected file, just check it renders without error
                    passed += 1;
                }
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("ERROR: {} - {}", rel_path, e));
            }
        }
    }

    println!("\n=== Bulk Test Results: {} ===", category);
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);

    if !errors.is_empty() {
        println!("\nFailures:");
        for err in &errors {
            println!("  {}", err);
        }
    }

    assert_eq!(
        failed, 0,
        "{} tests failed in category '{}'",
        failed, category
    );
}

fn walk_hrml_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_hrml_files(&path));
            } else if path.extension().map(|e| e == "hrml").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    files
}

/// Test result summary for reporting.
pub struct TestReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub details: Vec<(String, bool, String)>,
}

impl TestReport {
    pub fn new() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            details: Vec::new(),
        }
    }

    pub fn record(&mut self, name: &str, success: bool, message: &str) {
        self.total += 1;
        if success {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
        self.details
            .push((name.to_string(), success, message.to_string()));
    }

    pub fn summary(&self) -> String {
        format!(
            "\n=== Test Report ===\nTotal: {}\nPassed: {}\nFailed: {}\n",
            self.total, self.passed, self.failed
        )
    }
}

impl Default for TestReport {
    fn default() -> Self {
        Self::new()
    }
}
