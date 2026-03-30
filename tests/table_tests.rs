use org_cli::html::render_html;
use org_cli::parser::parse_org_document;
use std::collections::HashMap;

#[test]
fn test_render_simple_table() {
    let input = "* Data\n| Name  | Age |\n| Alice | 30  |\n| Bob   | 25  |\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<table"), "Expected <table>, got:\n{}", html);
    assert!(html.contains("Alice"));
    assert!(html.contains("30"));
    assert!(html.contains("Bob"));
    assert!(html.contains("</table>"));
}

#[test]
fn test_render_table_with_header() {
    let input = "* Data\n| Name  | Age |\n|-------+-----|\n| Alice | 30  |\n| Bob   | 25  |\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<thead>"), "Expected <thead>, got:\n{}", html);
    assert!(html.contains("<th>") || html.contains("<th"), "Expected <th>");
    assert!(html.contains("Name"));
    assert!(html.contains("<tbody>"));
    assert!(html.contains("Alice"));
}

#[test]
fn test_render_table_cells_trimmed() {
    let input = "* Data\n|  spaced  |  content  |\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("spaced"));
    assert!(html.contains("content"));
}

#[test]
fn test_render_table_html_escaped() {
    let input = "* Data\n| <script> | a&b |\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("a&amp;b"));
}

#[test]
fn test_render_table_single_column() {
    let input = "* Data\n| item |\n| one  |\n| two  |\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<table"));
    assert!(html.contains("one"));
    assert!(html.contains("two"));
}

#[test]
fn test_render_table_separator_only_rows_ignored() {
    let input = "* Data\n| A | B |\n|---+---|\n| 1 | 2 |\n|---+---|\n| 3 | 4 |\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // Separator rows should not appear as data rows
    assert!(!html.contains("---"));
    assert!(html.contains("1"));
    assert!(html.contains("3"));
}

#[test]
fn test_render_table_with_surrounding_text() {
    let input = "* Heading\nHere is a table:\n| a | b |\n| 1 | 2 |\nAnd some text after.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("Here is a table:"));
    assert!(html.contains("<table"));
    assert!(html.contains("And some text after."));
}

#[test]
fn test_render_multiple_tables() {
    let input = "* Data\n| a |\n| 1 |\n\n| b |\n| 2 |\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    let table_count = html.matches("<table").count();
    assert!(table_count >= 2, "Expected 2 tables, got {}", table_count);
}

#[test]
fn test_render_table_empty_cells() {
    let input = "* Data\n| a |   | c |\n|   | b |   |\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<table"));
    assert!(html.contains("</table>"));
}

#[test]
fn test_render_no_table_regression() {
    let input = "* Heading\nNormal text, no pipes.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(!html.contains("<table"));
    assert!(html.contains("Normal text"));
}

#[test]
fn test_render_table_inline_markup() {
    let input = "* Data\n| *bold* | /italic/ |\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(html.contains("<strong>bold</strong>") || html.contains("bold"));
}
