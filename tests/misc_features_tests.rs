use org_cli::html::render_html;
use org_cli::parser::parse_org_document;
use std::collections::HashMap;

// ==================== Horizontal Rules ====================

#[test]
fn test_render_horizontal_rule() {
    let input = "* Heading\nAbove.\n-----\nBelow.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<hr"), "Expected <hr>, got:\n{}", html);
    assert!(html.contains("Above."));
    assert!(html.contains("Below."));
}

#[test]
fn test_render_long_horizontal_rule() {
    let input = "* Heading\n----------\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<hr"));
}

#[test]
fn test_render_four_dashes_is_not_rule() {
    let input = "* Heading\n----\nText.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(!html.contains("<hr"), "4 dashes should not be a rule");
}

// ==================== Line Breaks ====================

#[test]
fn test_render_line_break() {
    let input = "* Heading\nLine one \\\\\nLine two\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<br"), "Expected <br> for \\\\, got:\n{}", html);
}

// ==================== Comment Lines ====================

#[test]
fn test_comment_lines_excluded() {
    let input = "* Heading\n# This is a comment\nVisible text.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(!html.contains("This is a comment"), "Comment should be excluded");
    assert!(html.contains("Visible text."));
}

#[test]
fn test_comment_block_excluded() {
    let input = "* Heading\n#+BEGIN_COMMENT\nHidden text.\n#+END_COMMENT\nVisible.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(!html.contains("Hidden text."), "Comment block should be excluded");
    assert!(html.contains("Visible."));
}

#[test]
fn test_noexport_tag_excluded() {
    let input = "* Public heading\nVisible.\n* Secret heading :noexport:\nHidden.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("Public heading"));
    assert!(html.contains("Visible."));
    assert!(!html.contains("Secret heading"), "noexport should be excluded");
    assert!(!html.contains("Hidden."));
}

// ==================== Images ====================

#[test]
fn test_render_image_link_inline() {
    let input = "* Heading\n[[./img/photo.jpg]]\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<img"), "Image link without desc should inline, got:\n{}", html);
    assert!(html.contains("src=\"./img/photo.jpg\"") || html.contains("./img/photo.jpg"));
}

#[test]
fn test_render_image_link_png() {
    let input = "* Heading\n[[./diagram.png]]\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<img"));
}

#[test]
fn test_render_image_link_with_description_is_link() {
    let input = "* Heading\n[[./photo.jpg][Click here]]\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    // With description, should be a link not inline image
    assert!(html.contains("<a"));
    assert!(html.contains("Click here"));
}

#[test]
fn test_render_http_image_inline() {
    let input = "* Heading\n[[https://example.com/photo.png]]\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<img"), "Remote image should inline");
}

#[test]
fn test_render_non_image_link_not_inlined() {
    let input = "* Heading\n[[https://example.com/page]]\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(!html.contains("<img"), "Non-image link should not be <img>");
    assert!(html.contains("<a"));
}
