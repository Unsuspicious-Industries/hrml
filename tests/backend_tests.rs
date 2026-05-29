use xrml::backend::Runtime;
use serde_json::json;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn setup_backend_dir(test_name: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let test_dir = format!("/tmp/hrml_test_backend_{}_{}", test_name, now);
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(format!("{}/api/todos", test_dir)).unwrap();
    test_dir
}

#[test]
fn endpoint_hrml_template_supports_action_and_context() {
    let dir = setup_backend_dir("hrml_action");
    fs::write(
        format!("{}/api/todos/create.hrml", dir),
        "<div><?get id=\"action\"?>-<?get id=\"data.title\"?></div>",
    )
    .unwrap();

    let rt = Runtime::new(&dir);
    let value = rt
        .call_endpoint("/api/todos/create", &json!({"title": "Ship"}))
        .unwrap();

    assert_eq!(value.as_str().unwrap(), "<div>create-Ship</div>");
}

#[test]
fn endpoint_html_file_returns_raw_string() {
    let dir = setup_backend_dir("html_raw");
    fs::write(format!("{}/api/todos.html", dir), "<p>ok</p>").unwrap();

    let rt = Runtime::new(&dir);
    let value = rt.call_endpoint("/api/todos", &json!({})).unwrap();
    assert_eq!(value.as_str().unwrap(), "<p>ok</p>");
}

#[test]
fn endpoint_json_file_returns_json_value() {
    let dir = setup_backend_dir("json_value");
    fs::write(format!("{}/api/todos.json", dir), "{\"ok\":true}").unwrap();

    let rt = Runtime::new(&dir);
    let value = rt.call_endpoint("/api/todos", &json!({})).unwrap();
    assert_eq!(value["ok"], json!(true));
}

#[test]
fn endpoint_missing_returns_error() {
    let dir = setup_backend_dir("missing");
    let rt = Runtime::new(&dir);
    let err = rt.call_endpoint("/api/unknown", &json!({})).unwrap_err();
    assert!(err.contains("Endpoint not found"));
}

#[test]
fn endpoint_flat_fallback_supports_module_action_files() {
    let dir = setup_backend_dir("flat_fallback");
    fs::write(
        format!("{}/api/todos_publish.hrml", dir),
        "<div><?get id=\"action\"?>-<?get id=\"data.state\"?></div>",
    )
    .unwrap();

    let rt = Runtime::new(&dir);
    let value = rt
        .call_endpoint("/api/todos/publish", &json!({"state": "ready"}))
        .unwrap();

    assert_eq!(value.as_str().unwrap(), "<div>publish-ready</div>");
}

#[test]
fn endpoint_rejects_non_api_paths() {
    let dir = setup_backend_dir("invalid_path");
    let rt = Runtime::new(&dir);
    let err = rt.call_endpoint("/pages/home", &json!({})).unwrap_err();
    assert!(err.contains("Invalid endpoint path"));
}
