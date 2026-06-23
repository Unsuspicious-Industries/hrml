//! The utility algebra — Tailwind-flavoured single-purpose classes, derived
//! from the design tokens and tree-shaken to the classes a page actually
//! writes.
//!
//! A utility is a *partial function* `class → rule`: the grammar below either
//! recognises a class name and yields exactly one CSS rule, or yields nothing
//! (the name belongs to some component's own stylesheet — the two vocabularies
//! coexist by construction). Three families:
//!
//! - **Atoms** — a closed set of layout/typography one-liners (`flex`,
//!   `hidden`, `uppercase`, …).
//! - **Token images** — the `[globals]` token set mapped through the functors
//!   `text-` (color), `bg-` (background), `border-` (border-color) and
//!   `font-` (family). `text-muted` resolves the token `text_muted`,
//!   `text-accent` the token `accent`: the prefixed spelling is tried first,
//!   then the bare one. The utility vocabulary therefore grows with the
//!   project's tokens and with nothing else — there is no built-in palette.
//! - **The spacing lattice** — `ℕ → length`, `n ↦ n × 0.25rem`, addressed by
//!   `m`/`p` with optional axis or side suffix, and `gap` (`p-4` = `1rem`,
//!   `mx-2` = horizontal `0.5rem`, Tailwind's scale).
//!
//! Generation happens at build time inside the same tree-shake pass as
//! component `<?style?>` blocks, so the emitted sheet is exactly the set of
//! utilities reachable from the page — no scanner config, no purge step.

use serde_json::Value;
use std::collections::BTreeSet;

/// One rule per recognised utility class, in sorted class order (utilities
/// are order-independent: each touches a distinct declaration).
pub fn rules(classes: &BTreeSet<String>, globals: &Value) -> Vec<String> {
    classes
        .iter()
        .filter(|class| {
            !class.is_empty()
                && class
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
        .filter_map(|class| {
            declaration(class, globals).map(|decl| format!(".{} {{ {}; }}", class, decl))
        })
        .collect()
}

fn declaration(class: &str, globals: &Value) -> Option<String> {
    if let Some(atom) = atom(class) {
        return Some(atom.to_string());
    }

    let (prefix, arg) = class.split_once('-')?;
    if let Some(n) = parse_scale(arg) {
        return spacing(prefix, &n);
    }
    match prefix {
        "text" => Some(format!("color: {}", token(globals, "text", arg)?)),
        "bg" => Some(format!("background: {}", token(globals, "bg", arg)?)),
        "border" => Some(format!("border-color: {}", token(globals, "border", arg)?)),
        "font" => Some(format!("font-family: {}", token(globals, "font", arg)?)),
        "items" => {
            let v = flex_keyword(arg)?;
            Some(format!("align-items: {}", v))
        }
        "justify" => {
            let v = match arg {
                "between" => "space-between",
                "around" => "space-around",
                other => flex_keyword(other)?,
            };
            Some(format!("justify-content: {}", v))
        }
        _ => None,
    }
}

/// The closed set of argument-free utilities.
fn atom(class: &str) -> Option<&'static str> {
    Some(match class {
        "flex" => "display: flex",
        "grid" => "display: grid",
        "block" => "display: block",
        "inline-block" => "display: inline-block",
        "hidden" => "display: none",
        "flex-col" => "flex-direction: column",
        "flex-wrap" => "flex-wrap: wrap",
        "w-full" => "width: 100%",
        "uppercase" => "text-transform: uppercase",
        "italic" => "font-style: italic",
        "text-center" => "text-align: center",
        "text-right" => "text-align: right",
        "rounded" => "border-radius: 4px",
        "rounded-full" => "border-radius: 999px",
        _ => return None,
    })
}

/// Resolve a utility argument against the tokens: the prefixed spelling
/// (`text-muted` → `text_muted`) wins over the bare one (`text-accent` →
/// `accent`), and the value is always the token's custom property, so
/// utilities and `<?style?>` blocks can never disagree.
fn token(globals: &Value, family: &str, arg: &str) -> Option<String> {
    let map = globals.as_object()?;
    let name = arg.replace('-', "_");
    let prefixed = format!("{}_{}", family, name);
    let key = if map.contains_key(&prefixed) {
        prefixed
    } else if map.contains_key(&name) {
        name
    } else {
        return None;
    };
    Some(format!("var(--{})", key.replace('_', "-")))
}

fn flex_keyword(arg: &str) -> Option<&'static str> {
    Some(match arg {
        "start" => "flex-start",
        "end" => "flex-end",
        "center" => "center",
        "baseline" => "baseline",
        "stretch" => "stretch",
        _ => return None,
    })
}

/// `n ↦ n × 0.25rem`, rendered without trailing zeros.
fn parse_scale(arg: &str) -> Option<String> {
    let n: u32 = arg.parse().ok()?;
    let quarters = n as f64 * 0.25;
    Some(if quarters == 0.0 {
        "0".to_string()
    } else {
        format!("{}rem", quarters)
    })
}

fn spacing(prefix: &str, length: &str) -> Option<String> {
    let one = |prop: &str| Some(format!("{}: {}", prop, length));
    let two = |a: &str, b: &str| Some(format!("{}: {len}; {}: {len}", a, b, len = length));
    match prefix {
        "gap" => one("gap"),
        "m" => one("margin"),
        "p" => one("padding"),
        "mt" => one("margin-top"),
        "mr" => one("margin-right"),
        "mb" => one("margin-bottom"),
        "ml" => one("margin-left"),
        "pt" => one("padding-top"),
        "pr" => one("padding-right"),
        "pb" => one("padding-bottom"),
        "pl" => one("padding-left"),
        "mx" => two("margin-left", "margin-right"),
        "my" => two("margin-top", "margin-bottom"),
        "px" => two("padding-left", "padding-right"),
        "py" => two("padding-top", "padding-bottom"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn classes(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn atoms_and_spacing() {
        let out = rules(&classes(&["flex", "items-center", "gap-2", "px-4", "p-0"]), &json!({}));
        assert_eq!(
            out,
            vec![
                ".flex { display: flex; }",
                ".gap-2 { gap: 0.5rem; }",
                ".items-center { align-items: center; }",
                ".p-0 { padding: 0; }",
                ".px-4 { padding-left: 1rem; padding-right: 1rem; }",
            ]
        );
    }

    #[test]
    fn token_images_prefer_the_prefixed_spelling() {
        let globals = json!({ "accent": "#37a", "text_muted": "#6b7", "font_mono": "Hack" });
        let out = rules(
            &classes(&["text-muted", "text-accent", "bg-accent", "font-mono"]),
            &globals,
        );
        assert_eq!(
            out,
            vec![
                ".bg-accent { background: var(--accent); }",
                ".font-mono { font-family: var(--font-mono); }",
                ".text-accent { color: var(--accent); }",
                ".text-muted { color: var(--text-muted); }",
            ]
        );
    }

    #[test]
    fn unknown_classes_yield_nothing() {
        let globals = json!({ "accent": "#37a" });
        let out = rules(
            &classes(&["site-nav", "fn-note", "text-vermilion", "Weird$name", "p-x"]),
            &globals,
        );
        assert!(out.is_empty());
    }
}
