use xrml::template::Engine;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize, Clone)]
pub struct ValidationCase {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub xrml: String,
    pub expected: String,
    #[serde(default)]
    pub files: Vec<CaseFile>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CaseFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ValidationRecord {
    pub name: String,
    pub kind: String,
    pub matched: bool,
    pub expected: String,
    pub actual: Option<String>,
    pub error: Option<String>,
}

pub fn load_cases(cases_root: &Path) -> Result<Vec<ValidationCase>, String> {
    let mut cases = Vec::new();

    for entry in fs::read_dir(cases_root)
        .map_err(|e| format!("Failed to read {}: {}", cases_root.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Invalid directory entry: {}", e))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }

        let raw = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let case: ValidationCase = toml::from_str(&raw)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        if !matches!(case.kind.as_str(), "structure" | "logic" | "api") {
            return Err(format!(
                "Invalid case type '{}' in {}",
                case.kind,
                path.display()
            ));
        }

        cases.push(case);
    }

    cases.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(cases)
}

pub fn run_cases(cases_root: &Path) -> Result<Vec<ValidationRecord>, String> {
    let cases = load_cases(cases_root)?;
    let mut records = Vec::with_capacity(cases.len());

    for case in cases {
        records.push(run_case(&case));
    }

    Ok(records)
}

pub fn summarize(records: &[ValidationRecord]) -> (usize, usize) {
    let passed = records
        .iter()
        .filter(|record| record.error.is_none() && record.matched)
        .count();
    (passed, records.len().saturating_sub(passed))
}

pub fn failure_message(record: &ValidationRecord) -> String {
    if let Some(error) = &record.error {
        return format!("{} ({}) render error: {}", record.name, record.kind, error);
    }

    format!(
        "{} ({}) mismatch\n  expected: {}\n  got: {}",
        record.name,
        record.kind,
        preview(&sort_tag_attrs(&normalize(&remove_spaces_between_tags(
            &extract_body(&record.expected)
        )))),
        preview(&sort_tag_attrs(&normalize(&remove_spaces_between_tags(
            &extract_body(record.actual.as_deref().unwrap_or_default())
        ))))
    )
}

fn run_case(case: &ValidationCase) -> ValidationRecord {
    let tmp = make_temp_dir(&case.name);
    let result = run_case_in_dir(case, &tmp);
    let _ = fs::remove_dir_all(&tmp);

    match result {
        Ok(actual) => ValidationRecord {
            name: case.name.clone(),
            kind: case.kind.clone(),
            matched: compare_output(&actual, &case.expected),
            expected: case.expected.clone(),
            actual: Some(actual),
            error: None,
        },
        Err(error) => ValidationRecord {
            name: case.name.clone(),
            kind: case.kind.clone(),
            matched: false,
            expected: case.expected.clone(),
            actual: None,
            error: Some(error),
        },
    }
}

fn run_case_in_dir(case: &ValidationCase, root: &Path) -> Result<String, String> {
    let page_path = root.join("pages/test.hrml");
    fs::create_dir_all(page_path.parent().unwrap())
        .map_err(|e| format!("Failed to create pages dir: {}", e))?;
    fs::write(&page_path, &case.xrml).map_err(|e| format!("Failed to write case page: {}", e))?;

    for file in &case.files {
        let file_path = root.join(&file.path);
        fs::create_dir_all(file_path.parent().unwrap())
            .map_err(|e| format!("Failed to create {}: {}", file_path.display(), e))?;
        fs::write(&file_path, &file.content)
            .map_err(|e| format!("Failed to write {}: {}", file_path.display(), e))?;
    }

    let engine = Engine::new(root.to_string_lossy().as_ref());
    engine
        .render("pages/test.hrml", &serde_json::json!({}))
        .map_err(|e| e.to_string())
}

fn make_temp_dir(name: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("hrml_case_{}_{}", name, now));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn compare_output(rendered: &str, expected: &str) -> bool {
    let rendered_body = extract_body(rendered);
    let expected_body = extract_body(expected);

    let expected_pre = remove_spaces_between_tags(expected_body);
    let rendered_pre = remove_spaces_between_tags(rendered_body);

    let expected_norm = sort_tag_attrs(&normalize(&expected_pre));
    let rendered_norm = sort_tag_attrs(&normalize(&rendered_pre));

    rendered_norm.contains(&expected_norm) || expected_norm.contains(&rendered_norm)
}

/// Rewrites HTML tags within `s` so that attributes appear in lexical order.
/// Allows DOM-equivalent comparison without depending on parser insertion order.
fn sort_tag_attrs(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let end = match find_tag_end(bytes, i) {
            Some(e) => e,
            None => {
                out.push('<');
                i += 1;
                continue;
            }
        };
        let tag = std::str::from_utf8(&bytes[i..end + 1]).unwrap_or_default();
        out.push_str(&sort_one_tag(tag));
        i = end + 1;
    }
    out
}

fn find_tag_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut j = start + 1;
    let mut in_quote: Option<u8> = None;
    while j < bytes.len() {
        let b = bytes[j];
        if let Some(q) = in_quote {
            if b == q {
                in_quote = None;
            }
        } else {
            if b == b'"' || b == b'\'' {
                in_quote = Some(b);
            } else if b == b'>' {
                return Some(j);
            }
        }
        j += 1;
    }
    None
}

fn sort_one_tag(tag: &str) -> String {
    let inner = match tag.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        Some(t) => t,
        None => return tag.to_string(),
    };
    let (name_end, _) = match inner.char_indices().find(|(_, c)| c.is_whitespace()) {
        Some(p) => p,
        None => return tag.to_string(),
    };
    let name = &inner[..name_end];
    let rest = &inner[name_end..];
    let self_closing = rest.trim_end().ends_with('/');
    let attr_body = if self_closing {
        let trimmed = rest.trim_end();
        &trimmed[..trimmed.len() - 1]
    } else {
        rest
    };
    let mut attrs = parse_attrs(attr_body);
    attrs.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = String::new();
    out.push('<');
    out.push_str(name);
    for (k, v) in &attrs {
        out.push(' ');
        out.push_str(k);
        if let Some(v) = v {
            out.push('=');
            out.push('"');
            out.push_str(v);
            out.push('"');
        }
    }
    if self_closing {
        out.push_str(" />");
    } else {
        out.push('>');
    }
    out
}

fn parse_attrs(s: &str) -> Vec<(String, Option<String>)> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let key_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'>'
        {
            i += 1;
        }
        let key = std::str::from_utf8(&bytes[key_start..i])
            .unwrap_or_default()
            .to_string();
        if key.is_empty() {
            i += 1;
            continue;
        }
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let q = bytes[i];
                i += 1;
                let val_start = i;
                while i < bytes.len() && bytes[i] != q {
                    i += 1;
                }
                let val = std::str::from_utf8(&bytes[val_start..i])
                    .unwrap_or_default()
                    .to_string();
                if i < bytes.len() {
                    i += 1;
                }
                out.push((key, Some(val)));
            } else {
                let val_start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                    i += 1;
                }
                let val = std::str::from_utf8(&bytes[val_start..i])
                    .unwrap_or_default()
                    .to_string();
                out.push((key, Some(val)));
            }
        } else {
            out.push((key, None));
        }
    }
    out
}

fn extract_body(s: &str) -> &str {
    if let Some(body_pos) = s.find("<body") {
        if let Some(start_tag_end) = s[body_pos..].find('>') {
            let start = body_pos + start_tag_end + 1;
            if let Some(end) = s.find("</body>") {
                return &s[start..end];
            }
            return &s[start..];
        }
    }
    s
}

fn remove_spaces_between_tags(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'>' {
            out.push('>');
            i += 1;

            let mut j = i;
            while j < bytes.len()
                && (bytes[j] == b' ' || bytes[j] == b'\n' || bytes[j] == b'\r' || bytes[j] == b'\t')
            {
                j += 1;
            }

            if j < bytes.len() && bytes[j] == b'<' {
                i = j;
                continue;
            }

            while i < j {
                out.push(bytes[i] as char);
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }

    out
}

fn normalize(s: &str) -> String {
    s.replace('\r', " ")
        .replace('\n', " ")
        .replace('\t', " ")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

fn preview(s: &str) -> String {
    let limit = 600;
    if s.len() <= limit {
        s.to_string()
    } else {
        format!("{}...", &s[..limit])
    }
}
