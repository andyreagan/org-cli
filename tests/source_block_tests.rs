use org_cli::html::render_html;
use org_cli::parser::parse_org_document;
use std::collections::HashMap;

// ==================== Source Blocks ====================

#[test]
fn test_render_src_block_basic() {
    let input = "* Code\n#+BEGIN_SRC python\ndef hello():\n    print(\"hi\")\n#+END_SRC\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<pre>"), "Expected <pre>, got:\n{}", html);
    assert!(html.contains("<code"), "Expected <code>");
    assert!(html.contains("def hello():"));
    assert!(html.contains("print"));
    assert!(html.contains("</code>"));
    assert!(html.contains("</pre>"));
}

#[test]
fn test_render_src_block_language_class() {
    let input = "* Code\n#+BEGIN_SRC rust\nfn main() {}\n#+END_SRC\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(
        html.contains("language-rust") || html.contains("src-rust"),
        "Expected language class, got:\n{}",
        html
    );
}

#[test]
fn test_render_src_block_no_language() {
    let input = "* Code\n#+BEGIN_SRC\nsome code\n#+END_SRC\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<pre>"));
    assert!(html.contains("some code"));
}

#[test]
fn test_render_src_block_html_escaped() {
    let input = "* Code\n#+BEGIN_SRC html\n<div>test</div>\n#+END_SRC\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("&lt;div&gt;"));
    assert!(!html.contains("<div>test</div>"));
}

#[test]
fn test_render_src_block_preserves_whitespace() {
    let input = "* Code\n#+BEGIN_SRC python\nif True:\n    x = 1\n    y = 2\n#+END_SRC\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    // Whitespace should be preserved inside <pre>
    assert!(html.contains("    x = 1"));
}

// ==================== Example Blocks ====================

#[test]
fn test_render_example_block() {
    let input = "* Example\n#+BEGIN_EXAMPLE\nSome example text\nwith multiple lines\n#+END_EXAMPLE\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<pre"), "Expected <pre>, got:\n{}", html);
    assert!(html.contains("Some example text"));
    assert!(html.contains("with multiple lines"));
}

#[test]
fn test_render_example_block_html_escaped() {
    let input = "* Ex\n#+BEGIN_EXAMPLE\n<script>alert(1)</script>\n#+END_EXAMPLE\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;"));
}

// ==================== Fixed-Width Lines ====================

#[test]
fn test_render_fixed_width_line() {
    let input = "* Heading\n: This is fixed-width\n: Another line\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<pre"), "Expected <pre> for fixed-width, got:\n{}", html);
    assert!(html.contains("This is fixed-width"));
    assert!(html.contains("Another line"));
}

// ==================== Case Insensitivity ====================

#[test]
fn test_render_src_block_case_insensitive() {
    let input = "* Code\n#+begin_src python\nprint(1)\n#+end_src\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<pre>"), "Expected <pre>, got:\n{}", html);
    assert!(html.contains("print(1)"));
}

// ==================== Mixed with Other Content ====================

#[test]
fn test_render_src_block_with_surrounding_text() {
    let input = "* Heading\nHere is some code:\n#+BEGIN_SRC python\nx = 1\n#+END_SRC\nAnd some text after.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<p>"));
    assert!(html.contains("Here is some code:"));
    assert!(html.contains("<pre>"));
    assert!(html.contains("x = 1"));
    assert!(html.contains("And some text after."));
}

// ==================== Multiple Blocks ====================

#[test]
fn test_render_multiple_src_blocks() {
    let input = "* Code\n#+BEGIN_SRC python\na = 1\n#+END_SRC\n#+BEGIN_SRC rust\nlet b = 2;\n#+END_SRC\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    let pre_count = html.matches("<pre>").count();
    assert!(pre_count >= 2, "Expected at least 2 <pre> blocks, got {}", pre_count);
    assert!(html.contains("a = 1"));
    assert!(html.contains("let b = 2;"));
}

// ==================== Regression ====================

#[test]
fn test_render_body_without_blocks_still_works() {
    let input = "* Heading\nJust normal text.\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<p>"));
    assert!(html.contains("Just normal text."));
    assert!(!html.contains("<pre>"));
}

#[test]
fn test_render_empty_src_block() {
    let input = "* Code\n#+BEGIN_SRC python\n#+END_SRC\n";
    let doc = parse_org_document(input).unwrap();
    let html = render_html(&doc, &HashMap::new());
    assert!(html.contains("<pre>"));
    assert!(html.contains("</pre>"));
}
