use org_cli::parser::*;
use org_cli::types::*;

// ==================== org-id Link Parsing ====================

#[test]
fn test_parse_id_link() {
    let input = "* See [[id:abc-123-def][Some Entry]]\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].links.len(), 1);
    assert_eq!(doc.entries[0].links[0].url, "id:abc-123-def");
    assert_eq!(
        doc.entries[0].links[0].description,
        Some("Some Entry".to_string())
    );
    assert!(doc.entries[0].links[0].is_id_link());
    assert_eq!(doc.entries[0].links[0].id_value(), Some("abc-123-def"));
}

#[test]
fn test_parse_id_link_without_description() {
    let input = "* See [[id:abc-123]]\n";
    let doc = parse_org_document(input).unwrap();
    assert!(doc.entries[0].links[0].is_id_link());
    assert_eq!(doc.entries[0].links[0].id_value(), Some("abc-123"));
}

#[test]
fn test_non_id_link_is_not_id() {
    let input = "* Visit [[https://example.com][Example]]\n";
    let doc = parse_org_document(input).unwrap();
    assert!(!doc.entries[0].links[0].is_id_link());
    assert_eq!(doc.entries[0].links[0].id_value(), None);
}

#[test]
fn test_file_link_is_not_id() {
    let input = "* Open [[file:notes.org]]\n";
    let doc = parse_org_document(input).unwrap();
    assert!(!doc.entries[0].links[0].is_id_link());
}

#[test]
fn test_id_link_in_body() {
    let input = "* Heading\nSee [[id:uuid-456][related note]] for details.\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].links.len(), 1);
    assert!(doc.entries[0].links[0].is_id_link());
}

#[test]
fn test_mixed_id_and_url_links() {
    let input = "* Heading\n[[id:abc][Internal]] and [[https://example.com][External]]\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].links.len(), 2);
    assert!(doc.entries[0].links[0].is_id_link());
    assert!(!doc.entries[0].links[1].is_id_link());
}

// ==================== ID Property Parsing ====================

#[test]
fn test_entry_has_id_property() {
    let input = "* Heading\n:PROPERTIES:\n:ID: abc-123-def\n:END:\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(
        doc.entries[0].properties.get("ID"),
        Some(&"abc-123-def".to_string())
    );
}

#[test]
fn test_entry_id_method() {
    let input = "* Heading\n:PROPERTIES:\n:ID: uuid-here\n:END:\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].id(), Some("uuid-here"));
}

#[test]
fn test_entry_without_id() {
    let input = "* Heading\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].id(), None);
}

// ==================== Inline Markup Parsing ====================

#[test]
fn test_parse_bold_markup() {
    let fragments = parse_inline_markup("This is *bold* text");
    assert!(
        fragments
            .iter()
            .any(|f| matches!(f, InlineFragment::Bold(s) if s == "bold"))
    );
}

#[test]
fn test_parse_italic_markup() {
    let fragments = parse_inline_markup("This is /italic/ text");
    assert!(
        fragments
            .iter()
            .any(|f| matches!(f, InlineFragment::Italic(s) if s == "italic"))
    );
}

#[test]
fn test_parse_code_markup() {
    let fragments = parse_inline_markup("This is ~code~ text");
    assert!(
        fragments
            .iter()
            .any(|f| matches!(f, InlineFragment::Code(s) if s == "code"))
    );
}

#[test]
fn test_parse_verbatim_markup() {
    let fragments = parse_inline_markup("This is =verbatim= text");
    assert!(
        fragments
            .iter()
            .any(|f| matches!(f, InlineFragment::Verbatim(s) if s == "verbatim"))
    );
}

#[test]
fn test_parse_strikethrough_markup() {
    let fragments = parse_inline_markup("This is +deleted+ text");
    assert!(
        fragments
            .iter()
            .any(|f| matches!(f, InlineFragment::Strikethrough(s) if s == "deleted"))
    );
}

#[test]
fn test_parse_underline_markup() {
    let fragments = parse_inline_markup("This is _underlined_ text");
    assert!(
        fragments
            .iter()
            .any(|f| matches!(f, InlineFragment::Underline(s) if s == "underlined"))
    );
}

#[test]
fn test_parse_plain_text_only() {
    let fragments = parse_inline_markup("Just plain text");
    assert_eq!(fragments.len(), 1);
    assert!(matches!(&fragments[0], InlineFragment::Text(s) if s == "Just plain text"));
}

#[test]
fn test_parse_multiple_markup_types() {
    let fragments = parse_inline_markup("*bold* and /italic/ and ~code~");
    let bold_count = fragments
        .iter()
        .filter(|f| matches!(f, InlineFragment::Bold(_)))
        .count();
    let italic_count = fragments
        .iter()
        .filter(|f| matches!(f, InlineFragment::Italic(_)))
        .count();
    let code_count = fragments
        .iter()
        .filter(|f| matches!(f, InlineFragment::Code(_)))
        .count();
    assert_eq!(bold_count, 1);
    assert_eq!(italic_count, 1);
    assert_eq!(code_count, 1);
}

#[test]
fn test_parse_inline_link() {
    let fragments = parse_inline_markup("See [[https://example.com][Example]] here");
    assert!(
        fragments
            .iter()
            .any(|f| matches!(f, InlineFragment::Link(link) if link.url == "https://example.com"))
    );
}

#[test]
fn test_parse_inline_id_link() {
    let fragments = parse_inline_markup("See [[id:abc-123][Related]] here");
    assert!(
        fragments
            .iter()
            .any(|f| matches!(f, InlineFragment::Link(link) if link.is_id_link()))
    );
}

#[test]
fn test_markup_not_in_middle_of_word() {
    // Per org spec, markup markers must be preceded by whitespace/start-of-line
    // and followed by whitespace/punctuation/end-of-line
    let fragments = parse_inline_markup("path/to/file");
    // The /to/ should NOT be parsed as italic since it's inside a path
    let italic_count = fragments
        .iter()
        .filter(|f| matches!(f, InlineFragment::Italic(_)))
        .count();
    assert_eq!(italic_count, 0);
}

#[test]
fn test_unclosed_markup_is_plain_text() {
    let fragments = parse_inline_markup("This *unclosed bold");
    // No bold fragments since it's not closed
    let bold_count = fragments
        .iter()
        .filter(|f| matches!(f, InlineFragment::Bold(_)))
        .count();
    assert_eq!(bold_count, 0);
}

// ==================== Preamble Keyword Parsing ====================

#[test]
fn test_parse_title_keyword() {
    let input = "#+TITLE: My Document\n* Heading\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.title(), Some("My Document"));
}

#[test]
fn test_parse_author_keyword() {
    let input = "#+AUTHOR: John Doe\n* Heading\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.author(), Some("John Doe"));
}

#[test]
fn test_parse_no_title() {
    let input = "* Just a heading\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.title(), None);
}

#[test]
fn test_parse_multiple_keywords() {
    let input = "#+TITLE: My Doc\n#+AUTHOR: Jane\n#+DATE: 2026-03-29\n* Heading\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.title(), Some("My Doc"));
    assert_eq!(doc.author(), Some("Jane"));
}
