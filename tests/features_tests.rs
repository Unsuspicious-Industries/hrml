use xrml::features::{excerpt, render_latex_block, render_latex_inline, render_markdown, slugify};

#[test]
fn test_markdown_renders_headings_and_lists() {
    let src = "# Title\n\n- a\n- b\n";
    let html = render_markdown(src);
    assert!(html.contains("<h1>Title</h1>"));
    assert!(html.contains("<li>a</li>"));
    assert!(html.contains("<li>b</li>"));
}

#[test]
fn test_latex_helpers_emit_wrappers() {
    let inline = render_latex_inline("x+y");
    let block = render_latex_block("\\int_0^1 x dx");
    assert!(inline.contains("math-inline"));
    assert!(inline.contains("\\(x+y\\)"));
    assert!(block.contains("math-block"));
    assert!(block.contains("\\[\\int_0^1 x dx\\]"));
}

#[test]
fn test_text_helpers_slugify_and_excerpt() {
    assert_eq!(slugify("Hello, HRML Framework!"), "hello-hrml-framework");
    assert_eq!(excerpt("one two three four", 9), "one two...");
}
