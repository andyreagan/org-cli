use org_cli::html::render_html;
use org_cli::parser::parse_org_document;
use std::collections::HashMap;

// ==================== Captions ====================

#[test]
fn test_render_caption_on_image() {
    let input = "* Heading\n#+CAPTION: A nice photo\n[[./photo.jpg]]\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("A nice photo"), "Caption text should appear");
    assert!(
        html.contains("<figure") || html.contains("<figcaption") || html.contains("caption"),
        "Should have figure/caption markup, got:\n{}",
        html
    );
}

#[test]
fn test_render_caption_on_table() {
    let input = "* Heading\n#+CAPTION: Sales data\n| Q1 | 100 |\n| Q2 | 200 |\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("Sales data"));
    assert!(
        html.contains("<caption>") || html.contains("<figcaption"),
        "Table caption should render, got:\n{}",
        html
    );
}

#[test]
fn test_render_caption_on_src_block() {
    let input = "* Heading\n#+CAPTION: Example code\n#+BEGIN_SRC python\nprint(1)\n#+END_SRC\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("Example code"));
}

// ==================== ATTR_HTML ====================

#[test]
fn test_render_attr_html_on_image() {
    let input = "* Heading\n#+ATTR_HTML: :width 300 :alt A cat\n[[./cat.jpg]]\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("width=\"300\"") || html.contains("width: 300"),
        "Should apply width attr, got:\n{}",
        html
    );
}

#[test]
fn test_render_attr_html_class() {
    let input = "* Heading\n#+ATTR_HTML: :class highlight\n#+BEGIN_SRC python\nprint(1)\n#+END_SRC\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("class=\"highlight\"") || html.contains("highlight"),
        "Should apply class attr, got:\n{}",
        html
    );
}

// ==================== Export Options ====================

#[test]
fn test_option_num_nil_no_section_numbers() {
    let input = "#+OPTIONS: num:nil\n* Chapter\n** Section\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // With num:nil, headings should not have section numbers
    // (our default is no numbers anyway, so this should just work)
    assert!(html.contains("Chapter"));
    assert!(!html.contains("1 Chapter") && !html.contains("1.1 Section"));
}

#[test]
fn test_option_todo_nil_hides_keywords() {
    let input = "#+OPTIONS: todo:nil\n* TODO Buy groceries\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("Buy groceries"));
    assert!(
        !html.contains("class=\"todo-keyword"),
        "todo:nil should hide TODO keywords, got:\n{}",
        html
    );
}

#[test]
fn test_option_tags_nil_hides_tags() {
    let input = "#+OPTIONS: tags:nil\n* Heading :work:personal:\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("Heading"));
    // Check that no actual tag spans appear in the body (CSS will have .org-tag)
    assert!(
        !html.contains("<span class=\"org-tag\">"),
        "tags:nil should hide tags, got:\n{}",
        html
    );
}

#[test]
fn test_option_pri_nil_hides_priority() {
    let input = "#+OPTIONS: pri:nil\n* TODO [#A] Important task\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("Important task"));
    assert!(
        !html.contains("<span class=\"priority"),
        "pri:nil should hide priority, got:\n{}",
        html
    );
}

#[test]
fn test_option_p_nil_hides_planning() {
    let input = "#+OPTIONS: p:nil\n* TODO Task\nSCHEDULED: <2026-03-21 Sat>\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("Task"));
    assert!(
        !html.contains("<div class=\"planning\""),
        "p:nil should hide planning, got:\n{}",
        html
    );
}
