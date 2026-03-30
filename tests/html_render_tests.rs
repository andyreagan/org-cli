use org_cli::html::*;
use org_cli::parser::*;
use std::collections::HashMap;

// ==================== Basic HTML Rendering ====================

#[test]
fn test_render_simple_heading() {
    let input = "* Hello World\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<h1"));
    assert!(html.contains("Hello World"));
    assert!(html.contains("</h1>"));
}

#[test]
fn test_render_heading_levels() {
    let input = "* H1\n** H2\n*** H3\n**** H4\n***** H5\n****** H6\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<h1"));
    assert!(html.contains("<h2"));
    assert!(html.contains("<h3"));
    assert!(html.contains("<h4"));
    assert!(html.contains("<h5"));
    assert!(html.contains("<h6"));
}

#[test]
fn test_render_heading_deeper_than_6_clamps_to_h6() {
    let input = "******* Very deep heading\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<h6"));
}

#[test]
fn test_render_body_text_as_paragraph() {
    let input = "* Heading\nSome body text here.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<p>"));
    assert!(html.contains("Some body text here."));
}

#[test]
fn test_render_multiple_body_paragraphs() {
    let input = "* Heading\nFirst paragraph.\n\nSecond paragraph.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // Blank line should separate paragraphs
    let p_count = html.matches("<p>").count();
    assert!(p_count >= 2, "Expected at least 2 paragraphs, got {}", p_count);
}

// ==================== TODO Keywords ====================

#[test]
fn test_render_todo_keyword() {
    let input = "* TODO Buy groceries\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("TODO"));
    assert!(html.contains("todo-keyword"));
    assert!(html.contains("Buy groceries"));
}

#[test]
fn test_render_done_keyword() {
    let input = "* DONE Completed task\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("DONE"));
    assert!(html.contains("done-keyword") || html.contains("todo-keyword"));
}

// ==================== Tags ====================

#[test]
fn test_render_tags() {
    let input = "* Heading :work:urgent:\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("work"));
    assert!(html.contains("urgent"));
    assert!(html.contains("tag") || html.contains("org-tag"));
}

// ==================== Priority ====================

#[test]
fn test_render_priority() {
    let input = "* TODO [#A] Important task\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("[#A]") || html.contains("priority"));
    assert!(html.contains("Important task"));
}

// ==================== Links ====================

#[test]
fn test_render_url_link() {
    let input = "* Heading\nVisit [[https://example.com][Example Site]] now.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<a"));
    assert!(html.contains("href=\"https://example.com\""));
    assert!(html.contains("Example Site"));
    assert!(html.contains("</a>"));
}

#[test]
fn test_render_url_link_without_description() {
    let input = "* Heading\nVisit [[https://example.com]] now.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("href=\"https://example.com\""));
    assert!(html.contains("https://example.com"));
}

#[test]
fn test_render_id_link_resolved() {
    let input = "* Heading\nSee [[id:target-uuid][Target Entry]] for info.\n";
    let doc = parse_org_document(input).unwrap();

    // Provide an id_map that resolves target-uuid -> notes.html#target-uuid
    let mut id_map: HashMap<String, String> = HashMap::new();
    id_map.insert(
        "target-uuid".to_string(),
        "notes.html#target-uuid".to_string(),
    );

    let html = render_html(&doc, &id_map, None);
    assert!(html.contains("href=\"notes.html#target-uuid\""));
    assert!(html.contains("Target Entry"));
}

#[test]
fn test_render_id_link_unresolved() {
    let input = "* Heading\nSee [[id:missing-uuid][Missing]] for info.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // Unresolved id link should render as a broken/placeholder link
    assert!(html.contains("Missing"));
    // Should indicate it's broken or at least not crash
    assert!(html.contains("<a") || html.contains("<span"));
}

// ==================== Timestamps ====================

#[test]
fn test_render_scheduled_timestamp() {
    let input = "* TODO Task\nSCHEDULED: <2026-03-21 Sat>\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("2026-03-21"));
    assert!(html.contains("scheduled") || html.contains("SCHEDULED"));
}

#[test]
fn test_render_deadline_timestamp() {
    let input = "* TODO Task\nDEADLINE: <2026-03-25 Wed>\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("2026-03-25"));
    assert!(html.contains("deadline") || html.contains("DEADLINE"));
}

// ==================== Inline Markup Rendering ====================

#[test]
fn test_render_bold() {
    let input = "* Heading\nThis is *bold* text.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<strong>bold</strong>") || html.contains("<b>bold</b>"));
}

#[test]
fn test_render_italic() {
    let input = "* Heading\nThis is /italic/ text.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<em>italic</em>") || html.contains("<i>italic</i>"));
}

#[test]
fn test_render_code() {
    let input = "* Heading\nThis is ~code~ text.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<code>code</code>"));
}

#[test]
fn test_render_verbatim() {
    let input = "* Heading\nThis is =verbatim= text.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<code>verbatim</code>") || html.contains("<samp>verbatim</samp>"));
}

#[test]
fn test_render_strikethrough() {
    let input = "* Heading\nThis is +deleted+ text.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<del>deleted</del>") || html.contains("<s>deleted</s>"));
}

#[test]
fn test_render_underline() {
    let input = "* Heading\nThis is _underlined_ text.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<u>underlined</u>") || html.contains("text-decoration: underline"));
}

// ==================== Document Structure ====================

#[test]
fn test_render_html_document_structure() {
    let input = "#+TITLE: Test Doc\n* Heading\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<!DOCTYPE html>") || html.contains("<!doctype html>"));
    assert!(html.contains("<html"));
    assert!(html.contains("<head>"));
    assert!(html.contains("<body>"));
    assert!(html.contains("</html>"));
}

#[test]
fn test_render_title_in_head() {
    let input = "#+TITLE: My Document Title\n* Heading\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<title>My Document Title</title>"));
}

#[test]
fn test_render_title_in_body_as_h1() {
    let input = "#+TITLE: My Document Title\n* Heading\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // The title should appear as an h1 or header in the body too
    assert!(
        html.contains("My Document Title"),
        "Document title should appear in body"
    );
}

#[test]
fn test_render_includes_css() {
    let input = "* Heading\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // Should include either inline CSS or a link to a stylesheet
    assert!(
        html.contains("<style") || html.contains("stylesheet"),
        "HTML should include CSS styling"
    );
}

// ==================== Anchor IDs ====================

#[test]
fn test_render_heading_with_id_anchor() {
    let input = "* Heading\n:PROPERTIES:\n:ID: my-heading-id\n:END:\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("id=\"my-heading-id\""),
        "Heading with :ID: should have an HTML id attribute"
    );
}

#[test]
fn test_render_heading_without_id_gets_slug_anchor() {
    let input = "* Some Heading Text\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // Should generate a slug-based anchor for navigation
    assert!(
        html.contains("id=\"") || html.contains("name=\""),
        "Headings should have anchors for navigation"
    );
}

// ==================== HTML Escaping ====================

#[test]
fn test_render_escapes_html_entities() {
    let input = "* Heading with <angle> & \"quotes\"\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("&lt;angle&gt;"));
    assert!(html.contains("&amp;"));
    assert!(html.contains("&quot;quotes&quot;") || html.contains("&#34;"));
}

#[test]
fn test_render_escapes_body_html_entities() {
    let input = "* Heading\nBody with <script>alert('xss')</script>\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;"));
}

// ==================== Properties ====================

#[test]
fn test_render_properties_hidden_by_default() {
    let input = "* Heading\n:PROPERTIES:\n:ID: abc\n:CUSTOM: value\n:END:\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // Properties should not be rendered as visible text by default
    // (the :ID: is used for anchors, but :CUSTOM: shouldn't show)
    assert!(
        !html.contains(":PROPERTIES:"),
        "Raw :PROPERTIES: should not appear in HTML"
    );
}

// ==================== Preamble ====================

#[test]
fn test_render_preamble_not_shown_as_raw() {
    let input = "#+TITLE: My Doc\n#+AUTHOR: Jane\n* Heading\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        !html.contains("#+TITLE:"),
        "Raw #+TITLE: should not appear in HTML body"
    );
    assert!(
        !html.contains("#+AUTHOR:"),
        "Raw #+AUTHOR: should not appear in HTML body"
    );
}

#[test]
fn test_preamble_body_rendered() {
    // Text before any heading (doc.preamble) must appear in the HTML output.
    // This was the bug causing all.org (a flat list of [[file:…]] links) to render empty.
    let input = "#+TITLE: Sitemap\n\n- [[file:about.org][About]]\n- [[file:blog.org][Blog]]\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("About"), "preamble list item 'About' missing");
    assert!(html.contains("Blog"),  "preamble list item 'Blog' missing");
    assert!(html.contains("<ul>") || html.contains("<li>"), "expected list markup");
}

#[test]
fn test_bare_heading_link() {
    // [[File over app]] with no prefix should resolve to #file-over-app (same-page heading link).
    let input = "* Notes\nSee [[File over app]] for details.\n\n* File over app\nContent here.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("href=\"#file-over-app\""),
        "bare [[Heading]] should resolve to #slug, got:\n{}",
        &html[html.find("See").unwrap_or(0)..][..200.min(html.len())]
    );
}
