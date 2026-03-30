use org_cli::html::render_html;
use org_cli::parser::parse_org_document;
use std::collections::HashMap;

#[test]
fn test_render_footnote_reference() {
    let input = "* Heading\nSome text[fn:1] here.\n\n[fn:1] This is footnote one.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("Some text"), "Body text present");
    // Should render a superscript link to the footnote
    assert!(
        html.contains("<sup>") || html.contains("footnote-ref") || html.contains("fn-"),
        "Expected footnote reference, got:\n{}", html
    );
}

#[test]
fn test_render_footnote_definition_section() {
    let input = "* Heading\nText[fn:1].\n\n[fn:1] Footnote content.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // Should have a footnotes section at the bottom
    assert!(
        html.contains("Footnote") || html.contains("footnote"),
        "Expected footnote section, got:\n{}", html
    );
    assert!(html.contains("Footnote content."));
}

#[test]
fn test_render_multiple_footnotes() {
    let input = "* Heading\nFirst[fn:1] and second[fn:2].\n\n[fn:1] Note one.\n[fn:2] Note two.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("Note one."));
    assert!(html.contains("Note two."));
}

#[test]
fn test_render_named_footnote() {
    let input = "* Heading\nSee[fn:myname] for details.\n\n[fn:myname] Named footnote.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("Named footnote."));
}

#[test]
fn test_render_inline_footnote() {
    let input = "* Heading\nSome text[fn:: This is inline] here.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("This is inline"));
}

#[test]
fn test_render_named_inline_footnote() {
    let input = "* Heading\nSome text[fn:myfoot: Inline named] here.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("Inline named"));
}

#[test]
fn test_render_footnote_links_back() {
    let input = "* Heading\nText[fn:1].\n\n[fn:1] A footnote.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // The reference should link to the definition and vice versa
    assert!(html.contains("href=\"#"), "Expected anchor links in footnotes");
}

#[test]
fn test_render_no_footnote_regression() {
    let input = "* Heading\nJust plain text without footnotes.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(!html.contains("Footnotes"));
    assert!(html.contains("Just plain text"));
}
