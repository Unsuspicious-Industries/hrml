// HRML Validation Harness
//
// This module provides utilities to chain-test hundreds of HRML templates
// against expected HTML results. It supports:
//
// 1. Fixture-based testing: Load .hrml templates from validation/fixtures/,
//    render them, and compare against expected HTML in validation/expected/
//
// 2. Chain testing: Render sequences of templates where output from one
//    becomes input to another, verifying end-to-end pipelines
//
// 3. Bulk validation: Walk entire directory trees of fixtures and validate
//    all templates in a single test run
//
// 4. Property-based assertions: Verify invariants like "no directive leakage",
//    "valid HTML structure", "no crashes", etc.
//
// Usage:
//   - Place .hrml files in validation/fixtures/pages/
//   - Place expected .html files in validation/expected/pages/
//   - Run: cargo test --test validation_harness
//
// The harness automatically discovers all fixtures and validates them.

use hrml::template::Engine;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================
// Test Environment
// ============================================================

pub struct TestEnv {
    pub dir: String,
}

impl TestEnv {
    pub fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = format!("/tmp/hrml_val_{}_{}", name, now);
        let dirs = [
            "layouts",
            "components",
            "pages",
            "content",
            "sections",
            "chrome",
            "partials",
            "static",
            "endpoints/api",
        ];
        for d in &dirs {
            let _ = fs::create_dir_all(format!("{}/{}", dir, d));
        }
        TestEnv { dir }
    }

    pub fn write(&self, path: &str, content: &str) {
        let full = format!("{}/{}", self.dir, path);
        if let Some(parent) = Path::new(&full).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&full, content).unwrap();
    }

    pub fn read(&self, path: &str) -> String {
        fs::read_to_string(format!("{}/{}", self.dir, path)).unwrap()
    }

    pub fn engine(&self) -> Engine {
        Engine::new(&self.dir)
    }

    pub fn engine_with_config(
        &self,
        site_name: &str,
        description: Option<&str>,
        favicon: Option<&str>,
    ) -> Engine {
        let mut e = Engine::new(&self.dir).with_site_name(site_name.to_string());
        if let Some(d) = description {
            e = e.with_description(Some(d.to_string()));
        }
        if let Some(f) = favicon {
            e = e.with_favicon(Some(f.to_string()));
        }
        e
    }

    pub fn render(&self, path: &str) -> Result<String, String> {
        self.engine().render(path, &json!({}))
    }

    pub fn render_with_data(&self, path: &str, data: &Value) -> Result<String, String> {
        self.engine().render(path, data)
    }

    pub fn render_fragment(&self, path: &str) -> Result<String, String> {
        self.engine().render_fragment(path, &json!({}))
    }

    pub fn render_fragment_with_data(&self, path: &str, data: &Value) -> Result<String, String> {
        self.engine().render_fragment(path, data)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

// ============================================================
// Fixture Loader
// ============================================================

pub struct FixtureLoader {
    pub fixtures_dir: PathBuf,
    pub expected_dir: PathBuf,
}

impl FixtureLoader {
    pub fn new() -> Self {
        Self {
            fixtures_dir: PathBuf::from("validation/fixtures"),
            expected_dir: PathBuf::from("validation/expected"),
        }
    }

    /// Load all .hrml files from a subdirectory of fixtures/
    pub fn load_fixtures(&self, subdir: &str) -> Vec<(String, String)> {
        let dir = self.fixtures_dir.join(subdir);
        if !dir.exists() {
            return Vec::new();
        }
        let mut fixtures = Vec::new();
        for entry in fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map(|e| e == "hrml").unwrap_or(false) {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                let content = fs::read_to_string(&path).unwrap();
                fixtures.push((name, content));
            }
        }
        fixtures.sort_by(|a, b| a.0.cmp(&b.0));
        fixtures
    }

    /// Load expected HTML for a fixture
    pub fn load_expected(&self, subdir: &str, name: &str) -> Option<String> {
        let path = self
            .expected_dir
            .join(subdir)
            .join(format!("{}.html", name));
        if path.exists() {
            Some(fs::read_to_string(&path).unwrap())
        } else {
            None
        }
    }

    /// Load all expected HTML files from a subdirectory
    pub fn load_expected_all(&self, subdir: &str) -> HashMap<String, String> {
        let dir = self.expected_dir.join(subdir);
        if !dir.exists() {
            return HashMap::new();
        }
        let mut expected = HashMap::new();
        for entry in fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map(|e| e == "html").unwrap_or(false) {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                let content = fs::read_to_string(&path).unwrap();
                expected.insert(name, content);
            }
        }
        expected
    }
}

impl Default for FixtureLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Chain Tester
// ============================================================

pub struct ChainTest {
    env: TestEnv,
    steps: Vec<(String, String)>,
}

impl ChainTest {
    pub fn new(name: &str) -> Self {
        Self {
            env: TestEnv::new(name),
            steps: Vec::new(),
        }
    }

    /// Add a template to the chain
    pub fn add_template(&mut self, path: &str, content: &str) -> &mut Self {
        self.env.write(path, content);
        self.steps.push((path.to_string(), content.to_string()));
        self
    }

    /// Render the entry point and return the result
    pub fn render(&self, entry: &str) -> Result<String, String> {
        self.env.render(entry)
    }

    /// Render with data
    pub fn render_with_data(&self, entry: &str, data: &Value) -> Result<String, String> {
        self.env.render_with_data(entry, data)
    }

    /// Render and compare against expected HTML
    pub fn assert_output(&self, entry: &str, expected: &str) {
        let html = self.render(entry).unwrap();
        assert_eq!(
            normalize(&html),
            normalize(expected),
            "\nExpected:\n{}\n\nGot:\n{}\n",
            expected,
            html
        );
    }

    /// Render and assert contains/not_contains
    pub fn assert_contains(&self, entry: &str, needles: &[&str]) {
        let html = self.render(entry).unwrap();
        for needle in needles {
            assert!(
                html.contains(needle),
                "\nExpected to contain: {}\n\nGot:\n{}\n",
                needle,
                html
            );
        }
    }

    pub fn assert_not_contains(&self, entry: &str, needles: &[&str]) {
        let html = self.render(entry).unwrap();
        for needle in needles {
            assert!(
                !html.contains(needle),
                "\nExpected NOT to contain: {}\n\nGot:\n{}\n",
                needle,
                html
            );
        }
    }
}

// ============================================================
// Bulk Validator
// ============================================================

pub struct BulkValidator {
    pub results: Vec<(String, bool, String)>,
}

impl BulkValidator {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Validate all fixtures in a subdirectory
    pub fn validate_fixtures(
        &mut self,
        subdir: &str,
        render_fn: impl Fn(&str) -> Result<String, String>,
    ) {
        let loader = FixtureLoader::new();
        let fixtures = loader.load_fixtures(subdir);
        let expected_map = loader.load_expected_all(subdir);

        for (name, content) in &fixtures {
            // Write fixture to temp env
            let env = TestEnv::new(&format!("bulk_{}_{}", subdir, name));
            env.write(&format!("pages/{}.hrml", name), content);

            match render_fn(&format!("pages/{}.hrml", name)) {
                Ok(html) => {
                    if let Some(expected) = expected_map.get(name) {
                        if normalize(&html) == normalize(expected) {
                            self.results.push((name.clone(), true, String::new()));
                        } else {
                            self.results.push((
                                name.clone(),
                                false,
                                format!("Output mismatch\nExpected:\n{}\nGot:\n{}", expected, html),
                            ));
                        }
                    } else {
                        // No expected file, just check it renders without error
                        self.results.push((name.clone(), true, String::new()));
                    }
                }
                Err(e) => {
                    self.results
                        .push((name.clone(), false, format!("Render error: {}", e)));
                }
            }
        }
    }

    /// Print summary
    pub fn summary(&self) -> String {
        let total = self.results.len();
        let passed = self.results.iter().filter(|(_, ok, _)| *ok).count();
        let failed = total - passed;

        let mut out = format!(
            "\n=== Bulk Validation Summary ===\nTotal: {}\nPassed: {}\nFailed: {}\n",
            total, passed, failed
        );

        if failed > 0 {
            out.push_str("\nFailures:\n");
            for (name, ok, msg) in &self.results {
                if !ok {
                    out.push_str(&format!("  - {}: {}\n", name, msg));
                }
            }
        }

        out
    }

    /// Assert all passed
    pub fn assert_all_passed(&self) {
        let failed: Vec<_> = self.results.iter().filter(|(_, ok, _)| !ok).collect();
        assert!(
            failed.is_empty(),
            "{} fixtures failed:\n{}",
            failed.len(),
            self.summary()
        );
    }
}

impl Default for BulkValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Assertion Helpers
// ============================================================

pub fn assert_contains(html: &str, needle: &str) {
    assert!(
        html.contains(needle),
        "\nExpected to contain: {}\n\nGot:\n{}\n",
        needle,
        html
    );
}

pub fn assert_not_contains(html: &str, needle: &str) {
    assert!(
        !html.contains(needle),
        "\nExpected NOT to contain: {}\n\nGot:\n{}\n",
        needle,
        html
    );
}

pub fn assert_count(html: &str, needle: &str, expected: usize) {
    let count = html.matches(needle).count();
    assert_eq!(
        count, expected,
        "\nExpected {} occurrences of '{}', found {}\n\nGot:\n{}\n",
        expected, needle, count, html
    );
}

pub fn normalize(html: &str) -> String {
    html.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// ============================================================
// HTML Validation
// ============================================================

pub fn validate_html_structure(html: &str) -> Result<(), String> {
    // Check for basic HTML structure
    if html.contains("<!DOCTYPE html") {
        if html.matches("<html").count() != 1 {
            return Err("Expected exactly one <html> tag".to_string());
        }
        if html.matches("<head>").count() != 1 {
            return Err("Expected exactly one <head> tag".to_string());
        }
        if html.matches("</head>").count() != 1 {
            return Err("Expected exactly one </head> tag".to_string());
        }
        if html.matches("<body").count() != 1 {
            return Err("Expected exactly one <body> tag".to_string());
        }
        if html.matches("</body>").count() != 1 {
            return Err("Expected exactly one </body> tag".to_string());
        }
        if html.matches("</html>").count() != 1 {
            return Err("Expected exactly one </html> tag".to_string());
        }
    }

    // Check for directive leakage
    if html.contains("<?") {
        return Err("Found unprocessed HRML directive: <?".to_string());
    }
    if html.contains("?>") {
        return Err("Found unprocessed HRML directive: ?>".to_string());
    }

    Ok(())
}

// ============================================================
// Fixture Writers (for generating test data)
// ============================================================

pub fn write_fixture(subdir: &str, name: &str, content: &str) {
    let path = PathBuf::from("validation/fixtures")
        .join(subdir)
        .join(format!("{}.hrml", name));
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, content).unwrap();
}

pub fn write_expected(subdir: &str, name: &str, content: &str) {
    let path = PathBuf::from("validation/expected")
        .join(subdir)
        .join(format!("{}.html", name));
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, content).unwrap();
}

// ============================================================
// Test Report
// ============================================================

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

    pub fn assert_all_passed(&self) {
        assert_eq!(
            self.failed,
            0,
            "{} tests failed:\n{}",
            self.failed,
            self.summary()
        );
    }
}

impl Default for TestReport {
    fn default() -> Self {
        Self::new()
    }
}
