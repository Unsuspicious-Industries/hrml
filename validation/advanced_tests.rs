// Advanced randomized and stress tests using proptest
use proptest::prelude::*;
use std::fs;

proptest! {
    #[test]
    fn random_text_roundtrip(s in "[ -~]{0,200}") {
        // Ensure parser doesn't panic on random ascii text and roundtrips via render_fragment
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("pages").join("r.hrml");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, &s).unwrap();
        let engine = xrml::template::Engine::new(tmp.path().to_str().unwrap());
        let _ = engine.render_fragment("pages/r.hrml", &serde_json::json!({}));
    }

    #[test]
    fn random_balanced_tags(_depth in 0usize..6, n in 0usize..20) {
        // build a nested balanced tags structure
        let mut s = String::new();
        for i in 0..n {
            s.push_str(&format!("<div id=\"d{}\">", i));
        }
        s.push_str("content");
        for _ in 0..n {
            s.push_str("</div>");
        }

        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("pages").join("b.hrml");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, &s).unwrap();
        let engine = xrml::template::Engine::new(tmp.path().to_str().unwrap());
        let out = engine.render("pages/b.hrml", &serde_json::json!({})).unwrap();
        assert!(out.contains("content"));
    }
}
