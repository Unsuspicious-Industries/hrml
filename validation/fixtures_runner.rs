use hrml::template::Engine;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn copy_recursively(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let from = entry.path();
            let to = dst.join(entry.file_name());
            if file_type.is_dir() {
                copy_recursively(&from, &to)?;
            } else if file_type.is_file() {
                fs::create_dir_all(to.parent().unwrap())?;
                fs::copy(&from, &to)?;
            }
        }
    }
    Ok(())
}

#[test]
fn run_all_fixtures() {
    let fixtures_root = PathBuf::from("validation/fixtures");
    let expected_root = PathBuf::from("validation/expected");
    for entry in fs::read_dir(&fixtures_root).expect("fixtures root") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy();

        // prepare temp engine dir
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("hrml_fixture_{}_{}", name, now));
        let _ = fs::create_dir_all(&tmp);
        copy_recursively(&path, &tmp).expect("copy fixture");

        let engine = Engine::new(tmp.to_str().unwrap());
        // render pages/test.hrml
        let target = "pages/test.hrml";
        let rendered = engine.render(target, &serde_json::json!({}));
        let rendered = match rendered {
            Ok(s) => s,
            Err(e) => panic!("fixture '{}' failed to render: {}", name, e),
        };

        // load expected
        let expected_file = expected_root.join(name.as_ref()).join("expected.txt");
        let expected = fs::read_to_string(&expected_file)
            .unwrap_or_else(|_| panic!("missing expected for fixture '{}'", name));

        assert!(
            rendered.contains(&expected),
            "Fixture '{}' did not contain expected substring. Expected:\n{}\n\nGot:\n{}",
            name,
            expected,
            rendered
        );

        let _ = fs::remove_dir_all(&tmp);
    }
}
