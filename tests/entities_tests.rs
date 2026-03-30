use org_cli::html::render_html;
use org_cli::parser::parse_org_document;
use std::collections::HashMap;

// ==================== Special Strings ====================

#[test]
fn test_render_em_dash() {
    let input = "* Heading\nThis is important---very important.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("—") || html.contains("&mdash;"),
        "--- should become em-dash, got:\n{}",
        html
    );
}

#[test]
fn test_render_en_dash() {
    let input = "* Heading\nPages 10--20.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("–") || html.contains("&ndash;"),
        "-- should become en-dash, got:\n{}",
        html
    );
}

#[test]
fn test_render_ellipsis() {
    let input = "* Heading\nAnd then...\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("…") || html.contains("&hellip;"),
        "... should become ellipsis, got:\n{}",
        html
    );
}

// ==================== Entities ====================

#[test]
fn test_render_entity_alpha() {
    let input = "* Heading\nThe value \\alpha is small.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("α") || html.contains("&alpha;"),
        "\\alpha should render as α, got:\n{}",
        html
    );
}

#[test]
fn test_render_entity_to() {
    let input = "* Heading\nA \\to B.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("→") || html.contains("&rarr;"),
        "\\to should render as →, got:\n{}",
        html
    );
}

#[test]
fn test_render_entity_nbsp() {
    let input = "* Heading\nWord\\nbsp{}word.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("&nbsp;") || html.contains("\u{00a0}"),
        "\\nbsp should render as nbsp, got:\n{}",
        html
    );
}

// ==================== LaTeX Fragments ====================

#[test]
fn test_render_inline_latex() {
    let input = "* Math\nThe formula \\(E = mc^2\\) is famous.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("\\(E = mc^2\\)") || html.contains("E = mc^2"),
        "LaTeX inline should be preserved, got:\n{}",
        html
    );
}

#[test]
fn test_render_display_latex() {
    let input = "* Math\n\\[x = \\frac{-b}{2a}\\]\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("\\[x = \\frac{-b}{2a}\\]") || html.contains("frac"),
        "LaTeX display should be preserved, got:\n{}",
        html
    );
}

#[test]
fn test_render_latex_includes_mathjax() {
    let input = "* Math\n\\(x^2\\)\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        html.contains("mathjax") || html.contains("MathJax"),
        "Should include MathJax when LaTeX is present, got head:\n{}",
        &html[..500.min(html.len())]
    );
}

#[test]
fn test_render_no_mathjax_without_latex() {
    let input = "* Heading\nNo math here.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    assert!(
        !html.contains("MathJax") && !html.contains("mathjax"),
        "Should not include MathJax without LaTeX"
    );
}

#[test]
fn test_render_special_strings_not_in_code() {
    let input = "* Heading\n~a---b~ and normal---text.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new(), None);
    // Inside code, --- should NOT become em-dash
    assert!(html.contains("<code>a---b</code>"));
}
