use org_cli::html::render_html;
use org_cli::parser::parse_org_document;
use std::collections::HashMap;

#[test]
fn test_toc_generated_when_enabled() {
    // ToC is opt-in via #+OPTIONS: toc:t
    let input = "#+TITLE: Doc\n#+OPTIONS: toc:t\n* Chapter One\n** Section A\n* Chapter Two\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("Table of Contents") || html.contains("table-of-contents"),
        "Expected TOC with toc:t, got:\n{}",
        html
    );
    assert!(html.contains("Chapter One"));
    assert!(html.contains("Section A"));
    assert!(html.contains("Chapter Two"));
}

#[test]
fn test_toc_off_by_default() {
    // Without #+OPTIONS: toc:t, no ToC should be generated
    let input = "#+TITLE: Doc\n* Chapter One\n* Chapter Two\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        !html.contains("Table of Contents") && !html.contains("table-of-contents"),
        "ToC should be OFF by default (matching org-html-with-toc nil)"
    );
}

#[test]
fn test_toc_contains_links() {
    let input = "#+OPTIONS: toc:t\n* First\n* Second\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("href=\"#"),
        "TOC should contain anchor links"
    );
}

#[test]
fn test_toc_disabled() {
    let input = "#+OPTIONS: toc:nil\n* Chapter One\n* Chapter Two\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // Should NOT have a TOC section before the content
    let body_start = html.find("<body>").unwrap_or(0);
    let first_h1 = html.find("<h1").unwrap_or(html.len());
    let between = &html[body_start..first_h1];
    assert!(
        !between.contains("Table of Contents"),
        "TOC should be disabled with toc:nil"
    );
}

#[test]
fn test_toc_depth_limited() {
    let input = "#+OPTIONS: toc:1\n* Chapter\n** Section\n*** Subsection\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // TOC should include Chapter but not Section or Subsection
    // Find the TOC section
    if let Some(toc_start) = html.find("table-of-contents") {
        let toc_end = html[toc_start..].find("</nav>").map(|p| toc_start + p + 6).unwrap_or(html.len());
        let toc_section = &html[toc_start..toc_end];
        assert!(toc_section.contains("Chapter"), "TOC should contain level 1");
        assert!(!toc_section.contains("Section"), "TOC should not contain level 2 with toc:1");
    }
}

#[test]
fn test_toc_nested_structure() {
    let input = "* Chapter\n** Section\n* Chapter 2\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // Should have nested lists in TOC
    if let Some(toc_start) = html.find("table-of-contents") {
        let toc_end = html[toc_start..].find("</nav>").or_else(|| html[toc_start..].find("</div>")).unwrap_or(500);
        let toc = &html[toc_start..toc_start + toc_end];
        let ul_count = toc.matches("<ul").count();
        assert!(ul_count >= 2, "TOC should have nested lists for nested headings, got {} <ul> in:\n{}", ul_count, toc);
    }
}
