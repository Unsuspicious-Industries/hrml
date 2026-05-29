use xrml::features::{doc, oxml_tags, ONode};

#[test]
fn test_oxml_typed_content_elements() {
    let node = ONode::content(oxml_tags::SECTION)
        .class("hero")
        .id("top")
        .child(ONode::content(oxml_tags::H1).text("Hello OXML").build())
        .build();

    let html = node.render();
    assert!(html.contains("<section"));
    assert!(html.contains("class=\"hero\""));
    assert!(html.contains("id=\"top\""));
    assert!(html.contains("<h1>Hello OXML</h1>"));
}

#[test]
fn test_oxml_typed_void_elements() {
    let meta = ONode::void(oxml_tags::META)
        .attr("charset", "UTF-8")
        .build()
        .render();
    let link = ONode::void(oxml_tags::LINK)
        .attr("rel", "stylesheet")
        .attr("href", "/a.css")
        .build()
        .render();

    assert_eq!(meta, "<meta charset=\"UTF-8\">");
    assert!(link.contains("<link"));
    assert!(link.contains("rel=\"stylesheet\""));
    assert!(link.contains("href=\"/a.css\""));
}

#[test]
fn test_oxml_algebraic_cat_and_add() {
    let a = ONode::content(oxml_tags::SPAN).text("A").build();
    let b = ONode::content(oxml_tags::SPAN).text("B").build();
    let c = ONode::content(oxml_tags::SPAN).text("C").build();

    let composed = a.cat2(b) + c;
    let html = composed.render();
    assert!(html.contains("<span>A</span><span>B</span><span>C</span>"));
}

#[test]
fn test_oxml_cat_flattens_nested() {
    let a = ONode::content(oxml_tags::SPAN).text("A").build();
    let b = ONode::content(oxml_tags::SPAN).text("B").build();
    let c = ONode::content(oxml_tags::SPAN).text("C").build();
    let inner = a.cat2(b);
    let outer = inner.cat2(c);

    match outer {
        ONode::Cat(nodes) => {
            assert_eq!(nodes.len(), 3);
        }
        _ => panic!("Expected Cat node"),
    }
}

#[test]
fn test_oxml_map_text_and_pretty_debug() {
    let node = ONode::content(oxml_tags::P).text("hello world").build();
    let upper = node.map_text(|s| s.to_uppercase());
    assert!(upper.render().contains("HELLO WORLD"));

    let pretty = upper.render_pretty();
    assert!(pretty.contains("<p>"));
    assert!(pretty.contains("HELLO WORLD"));
}

#[test]
fn test_oxml_compact_removes_empty() {
    let a = ONode::content(oxml_tags::SPAN).text("A").build();
    let composed = ONode::Empty.cat2(a.clone()).cat2(ONode::Empty).cat2(a);
    let compacted = composed.compact();

    match compacted {
        ONode::Cat(nodes) => {
            assert_eq!(nodes.len(), 2);
        }
        _ => panic!("Expected Cat node after compact"),
    }
}

#[test]
fn test_oxml_insert_at() {
    let a = ONode::content(oxml_tags::SPAN).text("A").build();
    let b = ONode::content(oxml_tags::SPAN).text("B").build();
    let c = ONode::content(oxml_tags::SPAN).text("C").build();

    let seq = a.cat2(b);
    let inserted = seq.insert_at(1, c);

    let html = inserted.render();
    assert!(html.contains("<span>A</span><span>C</span><span>B</span>"));
}

#[test]
fn test_oxml_document_builder() {
    let head = ONode::content(oxml_tags::TITLE).text("Doc").build();
    let body = ONode::content(oxml_tags::MAIN)
        .child(ONode::content(oxml_tags::H1).text("Hi").build())
        .build();
    let html = doc(head, body);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<title>Doc</title>"));
    assert!(html.contains("<h1>Hi</h1>"));
}

#[test]
fn test_oxml_shorthand_constructors() {
    use xrml::features::{div, link, meta, span};

    let d = div().id("app").text("Hello").build().render();
    assert!(d.contains("<div id=\"app\">Hello</div>"));

    let s = span().class("x").text("Y").build().render();
    assert!(s.contains("<span class=\"x\">Y</span>"));

    let m = meta().attr("charset", "UTF-8").build().render();
    assert_eq!(m, "<meta charset=\"UTF-8\">");

    let l = link()
        .attr("rel", "stylesheet")
        .attr("href", "/a.css")
        .build()
        .render();
    assert!(l.contains("<link"));
    assert!(l.contains("rel=\"stylesheet\""));
}

#[test]
fn test_oxml_wrap() {
    let inner = ONode::content(oxml_tags::SPAN).text("inner").build();
    let wrapped = inner.wrap(oxml_tags::DIV);

    let html = wrapped.render();
    assert!(html.contains("<div><span>inner</span></div>"));
}

#[test]
fn test_oxml_add_assign() {
    let mut acc = ONode::content(oxml_tags::DIV).text("A").build();
    acc += ONode::content(oxml_tags::DIV).text("B").build();
    acc += ONode::content(oxml_tags::DIV).text("C").build();

    let html = acc.render();
    assert!(html.contains("<div>A</div><div>B</div><div>C</div>"));
}

#[test]
fn test_oxml_invariant_void_elements_cannot_have_children() {
    let void_node = ONode::void(oxml_tags::INPUT)
        .attr("type", "text")
        .attr("name", "q")
        .build();

    match &void_node {
        ONode::Void(v) => {
            assert!(v.tag.is_void());
            assert_eq!(v.tag.name, "input");
        }
        _ => panic!("Expected Void node"),
    }

    let html = void_node.render();
    assert!(html.contains("<input"));
    assert!(html.contains("name=\"q\""));
    assert!(html.contains("type=\"text\""));
}

#[test]
fn test_oxml_invariant_content_elements_always_have_closing_tags() {
    let content_node = ONode::content(oxml_tags::DIV).build();
    let html = content_node.render();
    assert_eq!(html, "<div></div>");

    let with_text = ONode::content(oxml_tags::P).text("hello").build();
    let html2 = with_text.render();
    assert_eq!(html2, "<p>hello</p>");
}

#[test]
fn test_oxml_elem_tag_classification() {
    assert!(oxml_tags::META.is_void());
    assert!(oxml_tags::LINK.is_void());
    assert!(oxml_tags::IMG.is_void());
    assert!(oxml_tags::INPUT.is_void());
    assert!(oxml_tags::BR.is_void());
    assert!(oxml_tags::HR.is_void());

    assert!(!oxml_tags::DIV.is_void());
    assert!(!oxml_tags::SPAN.is_void());
    assert!(!oxml_tags::H1.is_void());
    assert!(!oxml_tags::P.is_void());
    assert!(!oxml_tags::SECTION.is_void());
    assert!(!oxml_tags::MAIN.is_void());
}

#[test]
fn test_oxml_composition_is_closed() {
    let a = ONode::content(oxml_tags::DIV).text("a").build();
    let b = ONode::void(oxml_tags::BR).build();
    let c = ONode::content(oxml_tags::SPAN).text("c").build();
    let d = ONode::Empty;
    let e = ONode::raw("<custom>");

    let composed = a.cat2(b).cat2(c).cat2(d).cat2(e);
    let html = composed.render();

    assert!(html.contains("<div>a</div>"));
    assert!(html.contains("<br>"));
    assert!(html.contains("<span>c</span>"));
    assert!(html.contains("<custom>"));
}

#[test]
fn test_oxml_attr_if_conditional() {
    let node = ONode::content(oxml_tags::SCRIPT)
        .attr("src", "/app.js")
        .attr_if(true, "defer", "defer")
        .attr_if(false, "async", "async")
        .build()
        .render();

    assert!(node.contains("defer"));
    assert!(!node.contains("async"));
}
