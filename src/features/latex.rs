fn escape_attr(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn render_latex_inline(formula: &str) -> String {
    format!(
        r#"<span class="math-inline" data-math="{}">\({}\)</span>"#,
        escape_attr(formula),
        formula
    )
}

pub fn render_latex_block(formula: &str) -> String {
    format!(
        r#"<div class="math-block" data-math="{}">\[{ }\]</div>"#,
        escape_attr(formula),
        formula
    )
}

pub fn render_math_delimiters(text: &str) -> String {
    // Very small utility: $$...$$ then $...$
    let mut out = String::new();
    let mut i = 0;
    let bytes = text.as_bytes();

    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'$' && bytes[i + 1] == b'$' {
            let start = i + 2;
            if let Some(end_rel) = text[start..].find("$$") {
                let expr = &text[start..start + end_rel];
                out.push_str(&render_latex_block(expr));
                i = start + end_rel + 2;
                continue;
            }
        }
        if bytes[i] == b'$' {
            let start = i + 1;
            if let Some(end_rel) = text[start..].find('$') {
                let expr = &text[start..start + end_rel];
                out.push_str(&render_latex_inline(expr));
                i = start + end_rel + 1;
                continue;
            }
        }

        out.push(bytes[i] as char);
        i += 1;
    }

    out
}
