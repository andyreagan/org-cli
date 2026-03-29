use org_cli::html::render_html;
use org_cli::parser::parse_org_document;
use std::collections::HashMap;

#[test]
fn test_render_quote_block() {
    let input = "* Heading\n#+BEGIN_QUOTE\nTo be or not to be.\n#+END_QUOTE\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<blockquote>"), "Expected <blockquote>, got:\n{}", html);
    assert!(html.contains("To be or not to be."));
    assert!(html.contains("</blockquote>"));
}

#[test]
fn test_render_quote_block_multiline() {
    let input = "* Heading\n#+BEGIN_QUOTE\nLine one.\nLine two.\n#+END_QUOTE\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<blockquote>"));
    assert!(html.contains("Line one."));
    assert!(html.contains("Line two."));
}

#[test]
fn test_render_verse_block() {
    let input = "* Poem\n#+BEGIN_VERSE\nGreat clouds overhead\nTiny black birds rise and fall\n#+END_VERSE\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    // Verse should preserve line breaks
    assert!(html.contains("Great clouds overhead"), "Expected verse content");
    assert!(
        html.contains("<br") || html.contains("white-space: pre") || html.contains("<pre"),
        "Verse should preserve line breaks, got:\n{}", html
    );
}

#[test]
fn test_render_center_block() {
    let input = "* Heading\n#+BEGIN_CENTER\nCentered text here.\n#+END_CENTER\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("Centered text here."));
    assert!(
        html.contains("text-align: center") || html.contains("center"),
        "Expected centered styling, got:\n{}", html
    );
}

#[test]
fn test_render_export_html_block() {
    let input = "* Heading\n#+BEGIN_EXPORT html\n<div class=\"custom\">raw html</div>\n#+END_EXPORT\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    // Raw HTML should be passed through without escaping
    assert!(
        html.contains("<div class=\"custom\">raw html</div>"),
        "Expected raw HTML passthrough, got:\n{}", html
    );
}

#[test]
fn test_render_export_html_block_not_escaped() {
    let input = "* Heading\n#+BEGIN_EXPORT html\n<b>bold</b> & <i>italic</i>\n#+END_EXPORT\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<b>bold</b>"));
    assert!(html.contains("<i>italic</i>"));
}

#[test]
fn test_render_generic_block() {
    let input = "* Heading\n#+BEGIN_warning\nThis is a warning.\n#+END_warning\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(
        html.contains("class=\"warning\"") || html.contains("warning"),
        "Expected div with class, got:\n{}", html
    );
    assert!(html.contains("This is a warning."));
}

#[test]
fn test_render_quote_block_case_insensitive() {
    let input = "* Heading\n#+begin_quote\nQuoted text.\n#+end_quote\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<blockquote>"));
    assert!(html.contains("Quoted text."));
}

#[test]
fn test_render_block_with_surrounding_text() {
    let input = "* Heading\nBefore.\n#+BEGIN_QUOTE\nQuote.\n#+END_QUOTE\nAfter.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("Before."));
    assert!(html.contains("<blockquote>"));
    assert!(html.contains("After."));
}

#[test]
fn test_render_no_block_regression() {
    let input = "* Heading\nPlain text only.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(!html.contains("<blockquote>"));
    assert!(html.contains("Plain text only."));
}
