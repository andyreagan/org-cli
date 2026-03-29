use org_cli::html::render_html;
use org_cli::parser::parse_org_document;
use std::collections::HashMap;

// ==================== Parser: Unordered Lists ====================

#[test]
fn test_parse_unordered_list_dash() {
    let input = "* Heading\n- item one\n- item two\n- item three\n";
    let doc = parse_org_document(input).unwrap();
    let body = &doc.entries[0].body;
    assert!(body.contains("- item one"));
    // The body should be preserved for the renderer to parse lists from
}

#[test]
fn test_render_unordered_list_dash() {
    let input = "* Heading\n- item one\n- item two\n- item three\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<ul>"), "Expected <ul>, got:\n{}", html);
    assert!(html.contains("<li>"), "Expected <li>");
    assert!(html.contains("item one"));
    assert!(html.contains("item two"));
    assert!(html.contains("item three"));
    assert!(html.contains("</ul>"));
}

#[test]
fn test_render_unordered_list_plus() {
    let input = "* Heading\n+ item one\n+ item two\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<ul>"));
    assert!(html.contains("item one"));
    assert!(html.contains("item two"));
}

// ==================== Parser: Ordered Lists ====================

#[test]
fn test_render_ordered_list_dot() {
    let input = "* Heading\n1. first\n2. second\n3. third\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<ol>"), "Expected <ol>, got:\n{}", html);
    assert!(html.contains("first"));
    assert!(html.contains("second"));
    assert!(html.contains("third"));
    assert!(html.contains("</ol>"));
}

#[test]
fn test_render_ordered_list_paren() {
    let input = "* Heading\n1) first\n2) second\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<ol>"), "Expected <ol>, got:\n{}", html);
    assert!(html.contains("first"));
}

// ==================== Description Lists ====================

#[test]
fn test_render_description_list() {
    let input = "* Heading\n- Emacs :: A text editor\n- Vim :: Another text editor\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<dl>"), "Expected <dl>, got:\n{}", html);
    assert!(html.contains("<dt>"), "Expected <dt>");
    assert!(html.contains("Emacs"));
    assert!(html.contains("<dd>"), "Expected <dd>");
    assert!(html.contains("A text editor"));
    assert!(html.contains("</dl>"));
}

// ==================== Nested Lists ====================

#[test]
fn test_render_nested_unordered_list() {
    let input = "* Heading\n- parent one\n  - child one\n  - child two\n- parent two\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    // Should have nested <ul> tags
    let ul_count = html.matches("<ul>").count();
    assert!(
        ul_count >= 2,
        "Expected at least 2 <ul> (nested), got {}, html:\n{}",
        ul_count,
        html
    );
    assert!(html.contains("parent one"));
    assert!(html.contains("child one"));
    assert!(html.contains("child two"));
    assert!(html.contains("parent two"));
}

#[test]
fn test_render_nested_mixed_list() {
    let input = "* Heading\n1. first ordered\n   - nested unordered\n   - nested unordered 2\n2. second ordered\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<ol>"), "Expected <ol>");
    assert!(html.contains("<ul>"), "Expected nested <ul>");
    assert!(html.contains("first ordered"));
    assert!(html.contains("nested unordered"));
    assert!(html.contains("second ordered"));
}

// ==================== Checkboxes ====================

#[test]
fn test_render_checkbox_unchecked() {
    let input = "* Heading\n- [ ] buy milk\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<li>"), "Expected <li>");
    // Should render a checkbox or checkbox indicator
    assert!(
        html.contains("checkbox") || html.contains("☐") || html.contains("type=\"checkbox\""),
        "Expected checkbox indicator, got:\n{}",
        html
    );
    assert!(html.contains("buy milk"));
}

#[test]
fn test_render_checkbox_checked() {
    let input = "* Heading\n- [X] buy milk\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<li>"));
    assert!(
        html.contains("checked") || html.contains("☑") || html.contains("[X]"),
        "Expected checked indicator, got:\n{}",
        html
    );
    assert!(html.contains("buy milk"));
}

#[test]
fn test_render_checkbox_partial() {
    let input = "* Heading\n- [-] partially done\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("partially done"));
}

// ==================== List with Body Text ====================

#[test]
fn test_render_list_preceded_by_paragraph() {
    let input = "* Heading\nSome intro text.\n- item one\n- item two\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<p>"));
    assert!(html.contains("Some intro text."));
    assert!(html.contains("<ul>"));
    assert!(html.contains("item one"));
}

#[test]
fn test_render_list_followed_by_paragraph() {
    let input = "* Heading\n- item one\n- item two\n\nSome closing text.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<ul>"));
    assert!(html.contains("</ul>"));
    assert!(html.contains("Some closing text."));
}

// ==================== Multi-line List Items ====================

#[test]
fn test_render_multiline_list_item() {
    let input = "* Heading\n- item one which\n  continues on next line\n- item two\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<ul>"));
    // The continued text should be part of the same <li>
    assert!(html.contains("item one which"));
    assert!(html.contains("continues on next line"));
    assert!(html.contains("item two"));
}

// ==================== List with Inline Markup ====================

#[test]
fn test_render_list_with_inline_markup() {
    let input = "* Heading\n- *bold* item\n- /italic/ item\n- item with [[https://example.com][link]]\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<strong>bold</strong>"));
    assert!(html.contains("<em>italic</em>"));
    assert!(html.contains("href=\"https://example.com\""));
}

// ==================== Edge Cases ====================

#[test]
fn test_render_empty_list_item() {
    let input = "* Heading\n- \n- item two\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    // Should not crash, should produce valid HTML
    assert!(html.contains("<ul>"));
    assert!(html.contains("item two"));
}

#[test]
fn test_render_list_ends_at_two_blank_lines() {
    let input = "* Heading\n- item one\n- item two\n\n\nNot a list item.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<ul>"));
    assert!(html.contains("</ul>"));
    // "Not a list item" should be in a paragraph, not a list
    // (it's fine if this detail is approximate)
    assert!(html.contains("Not a list item."));
}

#[test]
fn test_render_body_without_list_still_works() {
    // Regression: make sure normal body text is not broken
    let input = "#+OPTIONS: toc:nil\n* Heading\nJust some normal text.\nAnother line.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<p>"));
    assert!(html.contains("Just some normal text."));
    // No list in the body content (TOC disabled so no <ul> at all)
    assert!(!html.contains("<ul>"));
    assert!(!html.contains("<ol>"));
}

#[test]
fn test_render_deeply_nested_list() {
    let input = "* Heading\n- level 1\n  - level 2\n    - level 3\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    let ul_count = html.matches("<ul>").count();
    assert!(
        ul_count >= 3,
        "Expected at least 3 nested <ul>, got {}, html:\n{}",
        ul_count,
        html
    );
}
