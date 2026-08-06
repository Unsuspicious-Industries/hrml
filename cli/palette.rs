use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A named token-to-value substitution table, loaded from a TOML file.
///
/// Design tokens are written into templates as `USI_<ROLE>` names (see
/// docs/style.md) and resolved here at build/serve time against a palette the
/// operator supplies - the nix config passes one derived from the fleet's
/// design system. The engine itself does not know any palette; it only knows
/// the contract: every `USI_<UPPERCASE>` run in a template or static asset
/// must be resolvable, and a leftover is a hard error.
#[derive(Clone)]
pub struct Palette {
    tokens: HashMap<String, String>,
}

impl Palette {
    pub fn load(path: &Path) -> Result<Palette, String> {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read palette {}: {}", path.display(), e))?;

        let value: toml::Value = toml::from_str(&text)
            .map_err(|e| format!("Failed to parse palette {}: {}", path.display(), e))?;

        let table = value
            .as_table()
            .ok_or_else(|| format!("Palette {} must be a TOML table", path.display()))?;

        let mut tokens = HashMap::new();
        for (key, value) in table {
            if !is_token_name(key) {
                return Err(format!(
                    "Palette {}: invalid token name {key:?} - expected USI_<ROLE> with \
                     uppercase letters, digits and underscores",
                    path.display()
                ));
            }
            let hex = value.as_str().ok_or_else(|| {
                format!(
                    "Palette {}: token {key} must be a string value",
                    path.display()
                )
            })?;
            tokens.insert(key.clone(), hex.to_string());
        }

        if tokens.is_empty() {
            return Err(format!(
                "Palette {} defines no tokens",
                path.display()
            ));
        }

        Ok(Palette { tokens })
    }

    /// Substitute every palette token in `text` with its value.
    ///
    /// Longest tokens are applied first so that `USI_INK_DIM` resolves before
    /// `USI_INK` can swallow it. Fails if any token-shaped text remains, which
    /// means the palette does not cover this project.
    pub fn apply(&self, text: &str) -> Result<String, String> {
        let mut keys: Vec<&String> = self.tokens.keys().collect();
        keys.sort_by_key(|key| std::cmp::Reverse(key.len()));

        let mut out = text.to_string();
        for key in keys {
            out = out.replace(key.as_str(), self.tokens[key].as_str());
        }

        if let Some(leftover) = first_leftover(&out) {
            return Err(format!(
                "unresolved palette token {leftover}: not defined in the palette"
            ));
        }

        Ok(out)
    }
}

fn is_token_name(key: &str) -> bool {
    key.len() > 4
        && key.starts_with("USI_")
        && key[4..]
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Find the first `USI_<UPPERCASE...>` run in `text`.
fn first_leftover(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if bytes[i..].starts_with(b"USI_") {
            let mut j = i + 4;
            while j < bytes.len()
                && (bytes[j].is_ascii_uppercase() || bytes[j].is_ascii_digit() || bytes[j] == b'_')
            {
                j += 1;
            }
            if j > i + 4 {
                return Some(text[i..j].to_string());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    None
}

/// Read every file under `root` (relative to `root`) as bytes.
///
/// Text files whose content mentions `USI_` tokens are passed through the
/// palette so static CSS, SVG and JS participate in the design system too.
/// Binary files are copied verbatim. Fails on the first unresolved token.
pub fn scan_static_dir(root: &Path, palette: Option<&Palette>) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    scan_dir(root, root, palette, &mut out)?;
    Ok(out)
}

fn scan_dir(
    base: &Path,
    dir: &Path,
    palette: Option<&Palette>,
    out: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read {}: {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read {}: {}", dir.display(), e))?;
        let path = entry.path();

        if path.is_dir() {
            scan_dir(base, &path, palette, out)?;
            continue;
        }

        let rel = path
            .strip_prefix(base)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .to_string();

        let bytes = fs::read(&path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let bytes = apply_if_text(&bytes, palette, &rel)?;
        out.push((rel, bytes));
    }

    Ok(())
}

fn apply_if_text(bytes: &[u8], palette: Option<&Palette>, rel: &str) -> Result<Vec<u8>, String> {
    if let Ok(text) = std::str::from_utf8(bytes) {
        if text.contains("USI_") {
            if let Some(palette) = palette {
                return palette
                    .apply(text)
                    .map(String::into_bytes)
                    .map_err(|e| format!("static/{rel}: {e}"));
            }
        }
    }
    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette_with(entries: &[(&str, &str)]) -> Palette {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("palette.toml");
        let mut content = String::new();
        for (key, value) in entries {
            content.push_str(&format!("{key} = \"{value}\"\n"));
        }
        fs::write(&path, content).unwrap();
        Palette::load(&path).unwrap()
    }

    #[test]
    fn longest_token_resolves_first() {
        let palette = palette_with(&[("USI_INK", "#111111"), ("USI_INK_DIM", "#222222")]);
        assert_eq!(palette.apply("a: USI_INK_DIM").unwrap(), "a: #222222");
        assert_eq!(palette.apply("a: USI_INK").unwrap(), "a: #111111");
        assert_eq!(
            palette.apply("a: USI_INK_DIM b: USI_INK").unwrap(),
            "a: #222222 b: #111111"
        );
    }

    #[test]
    fn unresolved_token_is_an_error() {
        let palette = palette_with(&[("USI_INK", "#111111")]);
        let err = palette.apply("color: USI_GROUND").unwrap_err();
        assert!(err.contains("USI_GROUND"), "got: {err}");
    }

    #[test]
    fn plain_text_is_untouched() {
        let palette = palette_with(&[("USI_INK", "#111111")]);
        assert_eq!(palette.apply("no tokens here").unwrap(), "no tokens here");
    }

    #[test]
    fn token_names_must_match_the_contract() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("palette.toml");

        fs::write(&path, "USI_lower = \"#fff\"\n").unwrap();
        assert!(Palette::load(&path).is_err());

        fs::write(&path, "COLOR = \"#fff\"\n").unwrap();
        assert!(Palette::load(&path).is_err());

        fs::write(&path, "USI_ = \"#fff\"\n").unwrap();
        assert!(Palette::load(&path).is_err());
    }

    #[test]
    fn empty_palette_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("palette.toml");
        fs::write(&path, "").unwrap();
        assert!(Palette::load(&path).is_err());
    }

    #[test]
    fn static_scan_transforms_text_and_passes_binary_through() {
        let dir = tempfile::tempdir().unwrap();
        let css_dir = dir.path().join("css");
        fs::create_dir_all(&css_dir).unwrap();
        fs::write(
            css_dir.join("style.css"),
            "body { color: USI_INK; }\n",
        )
        .unwrap();
        fs::write(dir.path().join("img.bin"), [0xFF, 0xFE, 0x00, 0x01]).unwrap();

        let palette = palette_with(&[("USI_INK", "#123456")]);
        let entries = scan_static_dir(dir.path(), Some(&palette)).unwrap();
        assert_eq!(entries.len(), 2);

        for (rel, bytes) in entries {
            if rel == "css/style.css" {
                assert_eq!(String::from_utf8(bytes).unwrap(), "body { color: #123456; }\n");
            } else {
                assert_eq!(rel, "img.bin");
                assert_eq!(bytes, [0xFF, 0xFE, 0x00, 0x01]);
            }
        }
    }

    #[test]
    fn static_scan_reports_unresolved_tokens() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("style.css"), "color: USI_NOPE;\n").unwrap();

        let palette = palette_with(&[("USI_INK", "#123456")]);
        let err = scan_static_dir(dir.path(), Some(&palette)).unwrap_err();
        assert!(err.contains("USI_NOPE"), "got: {err}");
    }

    #[test]
    fn scan_without_palette_is_verbatim() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("style.css"), "color: USI_INK;\n").unwrap();

        let entries = scan_static_dir(dir.path(), None).unwrap();
        assert_eq!(String::from_utf8(entries[0].1.clone()).unwrap(), "color: USI_INK;\n");
    }
}
