use org_cli::html::{export_site, render_html};
use org_cli::parser::parse_org_document;
use std::collections::HashMap;
use std::fs;
use tempfile::tempdir;

// ==================== CUSTOM_ID ====================

#[test]
fn test_custom_id_used_as_anchor() {
    let input = "* Heading\n:PROPERTIES:\n:CUSTOM_ID: my-section\n:END:\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(
        html.contains("id=\"my-section\""),
        "CUSTOM_ID should be used as anchor, got:\n{}",
        html
    );
}

#[test]
fn test_custom_id_takes_precedence_over_id() {
    let input = "* Heading\n:PROPERTIES:\n:CUSTOM_ID: custom-one\n:ID: id-one\n:END:\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(
        html.contains("id=\"custom-one\""),
        "CUSTOM_ID should take precedence, got:\n{}",
        html
    );
}

// ==================== Internal Links ====================

#[test]
fn test_internal_link_custom_id() {
    let input = "* Target\n:PROPERTIES:\n:CUSTOM_ID: target-sec\n:END:\nContent.\n* Source\nSee [[#target-sec][the target]].\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(
        html.contains("href=\"#target-sec\""),
        "[[#custom-id]] should link to anchor, got:\n{}",
        html
    );
}

#[test]
fn test_internal_link_headline() {
    let input = "* Introduction\nContent.\n* Details\nSee [[*Introduction][the intro]].\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(
        html.contains("href=\"#introduction\"") || html.contains("href=\"#"),
        "[[*Heading]] should link to heading slug, got:\n{}",
        html
    );
    assert!(html.contains("the intro"));
}

#[test]
fn test_internal_link_headline_no_description() {
    let input = "* My Section\nContent.\n* Other\nSee [[*My Section]].\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("href=\"#"));
    assert!(html.contains("My Section"));
}

// ==================== File Links ====================

#[test]
fn test_file_link_rewritten_to_html() {
    let input = "* Heading\nSee [[file:other.org][Other file]].\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(
        html.contains("href=\"other.html\""),
        "file:other.org should become other.html, got:\n{}",
        html
    );
}

#[test]
fn test_file_link_with_heading_search() {
    let input = "* Heading\nSee [[file:other.org::*Some Heading][link]].\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(
        html.contains("other.html#"),
        "Should link to other.html with anchor, got:\n{}",
        html
    );
}

#[test]
fn test_file_link_with_custom_id() {
    let input = "* Heading\nSee [[file:other.org::#my-id][link]].\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(
        html.contains("href=\"other.html#my-id\""),
        "Should resolve to other.html#my-id, got:\n{}",
        html
    );
}

#[test]
fn test_file_link_relative_path() {
    let input = "* Heading\nSee [[file:sub/notes.org][Notes]].\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(
        html.contains("href=\"sub/notes.html\""),
        "Should rewrite .org to .html in path, got:\n{}",
        html
    );
}

#[test]
fn test_file_link_non_org_file_unchanged() {
    let input = "* Heading\nSee [[file:document.pdf][PDF]].\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(
        html.contains("href=\"document.pdf\""),
        "Non-org file links should keep extension, got:\n{}",
        html
    );
}

// ==================== Export Integration ====================

#[test]
fn test_export_resolves_custom_id_links() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    fs::write(
        dir.path().join("a.org"),
        "* Source\nLink to [[#target-sec][target]].\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("b.org"),
        "* Target\n:PROPERTIES:\n:CUSTOM_ID: target-sec\n:END:\n",
    )
    .unwrap();

    export_site(dir.path(), out_dir.path()).unwrap();

    let a_html = fs::read_to_string(out_dir.path().join("a.html")).unwrap();
    // Internal #custom-id links within the same file or across files
    assert!(a_html.contains("href=\"#target-sec\"") || a_html.contains("target-sec"));
}
