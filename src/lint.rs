//! Colour-literal linting - making a palette bypass a build error.
//!
//! A project that resolves design tokens (`--palette`) already fails on a
//! *leftover* `USI_<ROLE>`: a typo, or a role the palette lacks. What it
//! cannot see is a *bypass* - `background: white` is perfectly valid CSS, so
//! the token machinery never gets a chance to object, and the page ships a
//! colour that no palette chose. Swap the palette and the hardcoded value
//! stays put; a card whose background is literally `white` is a white box on a
//! dark palette's page.
//!
//! So the rule "ask for a role, never a hex" needs the same treatment the
//! roles themselves got: it has to be checked, not remembered. This module is
//! that check.
//!
//! ```text
//! lint : CSS → [Finding]
//! ```
//!
//! A finding is a colour written as a literal value rather than as a token.
//! Only *values* are scanned, never property names - otherwise `white-space`
//! reads as the colour `white`, which is the obvious way to get a linter
//! switched off. Values that name a colour indirectly (`var(--ink)`,
//! `currentColor`, `transparent`, `inherit`) are the point of the exercise and
//! are always allowed.

use std::collections::BTreeSet;

/// One offending declaration: the property, the literal, and the 1-based line
/// it sits on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub line: usize,
    pub property: String,
    pub literal: String,
}

/// Colour keywords that carry no palette meaning and are always permitted:
/// they either defer to something else or are the absence of a colour.
const ALLOWED_KEYWORDS: &[&str] = &[
    "transparent",
    "currentcolor",
    "inherit",
    "initial",
    "unset",
    "revert",
    "none",
    "auto",
];

/// The CSS named colours. Deliberately the full set rather than a handful:
/// a linter that catches `white` but not `whitesmoke` teaches people which
/// spelling to use to get around it.
const NAMED_COLORS: &[&str] = &[
    "aliceblue", "antiquewhite", "aqua", "aquamarine", "azure", "beige",
    "bisque", "black", "blanchedalmond", "blue", "blueviolet", "brown",
    "burlywood", "cadetblue", "chartreuse", "chocolate", "coral",
    "cornflowerblue", "cornsilk", "crimson", "cyan", "darkblue", "darkcyan",
    "darkgoldenrod", "darkgray", "darkgreen", "darkgrey", "darkkhaki",
    "darkmagenta", "darkolivegreen", "darkorange", "darkorchid", "darkred",
    "darksalmon", "darkseagreen", "darkslateblue", "darkslategray",
    "darkslategrey", "darkturquoise", "darkviolet", "deeppink", "deepskyblue",
    "dimgray", "dimgrey", "dodgerblue", "firebrick", "floralwhite",
    "forestgreen", "fuchsia", "gainsboro", "ghostwhite", "gold", "goldenrod",
    "gray", "green", "greenyellow", "grey", "honeydew", "hotpink",
    "indianred", "indigo", "ivory", "khaki", "lavender", "lavenderblush",
    "lawngreen", "lemonchiffon", "lightblue", "lightcoral", "lightcyan",
    "lightgoldenrodyellow", "lightgray", "lightgreen", "lightgrey",
    "lightpink", "lightsalmon", "lightseagreen", "lightskyblue",
    "lightslategray", "lightslategrey", "lightsteelblue", "lightyellow",
    "lime", "limegreen", "linen", "magenta", "maroon", "mediumaquamarine",
    "mediumblue", "mediumorchid", "mediumpurple", "mediumseagreen",
    "mediumslateblue", "mediumspringgreen", "mediumturquoise",
    "mediumvioletred", "midnightblue", "mintcream", "mistyrose", "moccasin",
    "navajowhite", "navy", "oldlace", "olive", "olivedrab", "orange",
    "orangered", "orchid", "palegoldenrod", "palegreen", "paleturquoise",
    "palevioletred", "papayawhip", "peachpuff", "peru", "pink", "plum",
    "powderblue", "purple", "rebeccapurple", "red", "rosybrown", "royalblue",
    "saddlebrown", "salmon", "sandybrown", "seagreen", "seashell", "sienna",
    "silver", "skyblue", "slateblue", "slategray", "slategrey", "snow",
    "springgreen", "steelblue", "tan", "teal", "thistle", "tomato",
    "turquoise", "violet", "wheat", "white", "whitesmoke", "yellow",
    "yellowgreen",
];

/// Colour-producing functions that take literal channel values. `color-mix`
/// and `var` are absent on purpose: both take their colour from something
/// else, so they carry a token rather than replacing one.
const COLOR_FUNCTIONS: &[&str] = &["rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch"];

/// Every colour literal in `css`, in source order.
///
/// Scans declaration *values* only. The parse is deliberately shallow - CSS
/// that reaches here has already been written by hand into a `<?style?>`
/// block, and a linter that needs a full CSS grammar to say "this is a hex
/// code" is a linter nobody will keep working.
pub fn color_literals(css: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let stripped = strip_comments(css);

    for (index, raw_line) in stripped.lines().enumerate() {
        let line_no = index + 1;

        // A declaration is `property: value`. Anything without a colon is a
        // selector, a brace or an at-rule, and has no value to inspect.
        // Split off the value at the FIRST colon so that `background:
        // url(a:b)` keeps its whole value.
        let Some((property, rest)) = raw_line.split_once(':') else {
            continue;
        };

        let property = property.trim();
        // Pseudo-selectors (`a:hover {`) are not declarations. They are
        // distinguished by ending in a brace rather than a semicolon.
        if property.is_empty() || rest.trim_end().ends_with('{') {
            continue;
        }

        let value = rest.split(';').next().unwrap_or(rest);

        for literal in literals_in_value(value) {
            findings.push(Finding {
                line: line_no,
                property: property.to_string(),
                literal,
            });
        }
    }

    findings
}

/// The colour literals inside one declaration value.
fn literals_in_value(value: &str) -> Vec<String> {
    let mut found = BTreeSet::new();

    // Hex: # followed by 3, 4, 6 or 8 hex digits.
    let bytes: Vec<char> = value.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '#' {
            let digits: String = bytes[i + 1..]
                .iter()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            if matches!(digits.len(), 3 | 4 | 6 | 8) {
                found.insert(format!("#{}", digits));
            }
        }
        i += 1;
    }

    // Functional notation, and bare keywords. Tokenise on anything that
    // cannot appear in an identifier so `1px solid white` yields `white` and
    // `var(--ink)` yields `var` and `--ink`, neither of which is a colour.
    let mut tokens = value
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .filter(|t| !t.is_empty())
        .peekable();

    let lowered: Vec<String> = {
        let mut out = Vec::new();
        while let Some(t) = tokens.next() {
            out.push(t.to_ascii_lowercase());
        }
        out
    };

    for (index, token) in lowered.iter().enumerate() {
        if ALLOWED_KEYWORDS.contains(&token.as_str()) {
            continue;
        }
        // A custom property reference is a token by definition.
        if token.starts_with("--") {
            continue;
        }
        if COLOR_FUNCTIONS.contains(&token.as_str()) {
            // Only a call is a colour; `rgb` as a bare word is not.
            let call = value
                .to_ascii_lowercase()
                .contains(&format!("{}(", token));
            if call {
                found.insert(format!("{}()", token));
            }
            continue;
        }
        if NAMED_COLORS.contains(&token.as_str()) {
            // `--ink-dim` tokenises to `ink`/`dim`; skip a named colour that
            // is really part of a custom property or a longer identifier.
            let preceded_by_custom_prop = index > 0
                && lowered
                    .get(index - 1)
                    .map(|p| p.starts_with("--"))
                    .unwrap_or(false);
            if !preceded_by_custom_prop {
                found.insert(token.clone());
            }
        }
    }

    found.into_iter().collect()
}

/// Remove `/* … */`, preserving newlines so line numbers stay true.
fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();
    let mut in_comment = false;

    while let Some(c) = chars.next() {
        if in_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_comment = false;
            } else if c == '\n' {
                out.push('\n');
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_comment = true;
            continue;
        }
        out.push(c);
    }

    out
}

/// Every `<?style?> … <?/style?>` block in a template, as (offset_line, css).
///
/// Textual rather than AST-based on purpose: the lint wants to report the line
/// an author can go and look at, and a parsed tree has already discarded that.
pub fn style_blocks(source: &str) -> Vec<(usize, String)> {
    const OPEN: &str = "<?style?>";
    const CLOSE: &str = "<?/style?>";

    let mut blocks = Vec::new();
    let mut cursor = 0;

    while let Some(start) = source[cursor..].find(OPEN) {
        let open_at = cursor + start;
        let body_at = open_at + OPEN.len();
        let Some(end) = source[body_at..].find(CLOSE) else {
            break;
        };
        let body = &source[body_at..body_at + end];
        let line_offset = source[..body_at].lines().count();
        blocks.push((line_offset, body.to_string()));
        cursor = body_at + end + CLOSE.len();
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literals(css: &str) -> Vec<String> {
        color_literals(css).into_iter().map(|f| f.literal).collect()
    }

    #[test]
    fn flags_hex_and_named_and_functional() {
        assert_eq!(literals("a { color: #fff; }"), vec!["#fff"]);
        assert_eq!(literals("a { background: white; }"), vec!["white"]);
        assert_eq!(literals("a { color: rgba(0,0,0,.1); }"), vec!["rgba()"]);
    }

    #[test]
    fn accepts_tokens_and_neutral_keywords() {
        assert!(literals("a { color: var(--ink); }").is_empty());
        assert!(literals("a { background: transparent; }").is_empty());
        assert!(literals("a { border-color: currentColor; }").is_empty());
        assert!(literals("a { background: color-mix(in srgb, var(--ink) 15%, transparent); }").is_empty());
    }

    #[test]
    fn ignores_property_names_that_contain_a_colour() {
        // The reason values are scanned and declarations are not: `white-space`
        // is a property, not the colour white.
        assert!(literals("a { white-space: nowrap; }").is_empty());
    }

    #[test]
    fn ignores_selectors_and_at_rules() {
        assert!(literals("a:hover { }").is_empty());
        assert!(literals("@media (max-width: 768px) {").is_empty());
    }

    #[test]
    fn ignores_comments() {
        assert!(literals("/* background: white; */\na { color: var(--ink); }").is_empty());
    }

    #[test]
    fn reports_the_line() {
        let found = color_literals("a {\n  color: #123456;\n}");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 2);
        assert_eq!(found[0].property, "color");
    }

    #[test]
    fn finds_style_blocks_with_offsets() {
        let src = "<p>x</p>\n<?style?>\n.a { color: red; }\n<?/style?>";
        let blocks = style_blocks(src);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].1.contains("color: red"));
    }
}
