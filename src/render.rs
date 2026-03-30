//! HTML rendering — inline markup, body element parsing/rendering,
//! full-page rendering, CSS, slugify, footnotes, ToC, entry rendering.
//!
//! The public API is:
//!   - `RenderOptions` — preamble / head / head_extra
//!   - `render_html(doc, id_map, preamble)` — convenience wrapper
//!   - `render_html_opts(doc, id_map, opts)` — full control
//!   - `resolve_page_title(doc)` — used by site.rs for index generation

use crate::parser::parse_inline_markup;
use crate::types::*;
use std::collections::HashMap;

// ==================== HTML Escaping ====================

pub(crate) fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ==================== Footnotes ====================

/// Extract footnote definitions from body text.
/// Returns (cleaned body without definitions, Vec of (name, definition_text))
fn extract_footnote_definitions(body: &str) -> (String, Vec<(String, String)>) {
    let mut cleaned = String::new();
    let mut definitions: Vec<(String, String)> = Vec::new();
    let mut in_footnote = false;
    let mut current_name = String::new();
    let mut current_def = String::new();

    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[fn:") {
            if let Some(close) = trimmed.find(']') {
                let marker = &trimmed[4..close];
                let rest = trimmed[close + 1..].trim();
                if !marker.contains(':') {
                    if in_footnote {
                        definitions.push((current_name.clone(), current_def.trim().to_string()));
                    }
                    current_name = marker.to_string();
                    current_def = rest.to_string();
                    in_footnote = true;
                    continue;
                }
            }
        }
        if in_footnote && !trimmed.is_empty() && (line.starts_with(' ') || line.starts_with('\t')) {
            current_def.push(' ');
            current_def.push_str(trimmed);
            continue;
        }
        if in_footnote {
            definitions.push((current_name.clone(), current_def.trim().to_string()));
            in_footnote = false;
            current_name.clear();
            current_def.clear();
        }
        cleaned.push_str(line);
        cleaned.push('\n');
    }
    if in_footnote {
        definitions.push((current_name, current_def.trim().to_string()));
    }
    (cleaned, definitions)
}

fn replace_footnote_refs(
    text: &str,
    footnote_counter: &mut usize,
    inline_defs: &mut Vec<(String, String)>,
) -> String {
    let mut result = String::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("[fn:") {
        result.push_str(&remaining[..start]);
        let after = &remaining[start + 4..];

        if let Some(close) = after.find(']') {
            let inner = &after[..close];

            if inner.starts_with(": ") || inner.starts_with(':') {
                *footnote_counter += 1;
                let num = *footnote_counter;
                let def_text = inner.trim_start_matches(':').trim().to_string();
                inline_defs.push((format!("_inline_{}", num), def_text));
                result.push_str(&format!(
                    "<sup><a id=\"fnr-{}\" href=\"#fn-{}\" class=\"footnote-ref\">{}</a></sup>",
                    num, num, num
                ));
            } else if inner.contains(": ") {
                let colon_pos = inner.find(": ").unwrap();
                let name = &inner[..colon_pos];
                let def_text = inner[colon_pos + 2..].trim().to_string();
                *footnote_counter += 1;
                let num = *footnote_counter;
                inline_defs.push((name.to_string(), def_text));
                result.push_str(&format!(
                    "<sup><a id=\"fnr-{}\" href=\"#fn-{}\" class=\"footnote-ref\">{}</a></sup>",
                    num, num, num
                ));
            } else {
                *footnote_counter += 1;
                let num = *footnote_counter;
                result.push_str(&format!(
                    "<sup><a id=\"fnr-{}\" href=\"#fn-{}\" class=\"footnote-ref\">{}</a></sup>",
                    num, num, num
                ));
            }
            remaining = &after[close + 1..];
        } else {
            result.push_str("[fn:");
            remaining = after;
        }
    }
    result.push_str(remaining);
    result
}

// ==================== Inline Markup ====================

pub(crate) fn render_inline_html(text: &str, id_map: &HashMap<String, String>) -> String {
    let processed = text.replace("\\\\", "\x00LINEBREAK\x00");
    let fragments = parse_inline_markup(&processed);
    let mut html = String::new();

    for fragment in fragments {
        match fragment {
            InlineFragment::Text(s) => {
                html.push_str(&process_special_strings(&escape_html(&s)));
            }
            InlineFragment::Bold(s) => {
                html.push_str("<strong>");
                html.push_str(&escape_html(&s));
                html.push_str("</strong>");
            }
            InlineFragment::Italic(s) => {
                html.push_str("<em>");
                html.push_str(&escape_html(&s));
                html.push_str("</em>");
            }
            InlineFragment::Code(s) | InlineFragment::Verbatim(s) => {
                html.push_str("<code>");
                html.push_str(&escape_html(&s));
                html.push_str("</code>");
            }
            InlineFragment::Strikethrough(s) => {
                html.push_str("<del>");
                html.push_str(&escape_html(&s));
                html.push_str("</del>");
            }
            InlineFragment::Underline(s) => {
                html.push_str("<u>");
                html.push_str(&escape_html(&s));
                html.push_str("</u>");
            }
            InlineFragment::Link(link) => {
                render_link(&mut html, &link, id_map);
            }
        }
    }

    html.replace("\x00LINEBREAK\x00", "<br>")
}

fn render_link(html: &mut String, link: &Link, id_map: &HashMap<String, String>) {
    if link.is_id_link() {
        let id = link.id_value().unwrap_or("");
        let desc = link.description.as_deref().unwrap_or(&link.url);
        if let Some(target) = id_map.get(id) {
            html.push_str(&format!(
                "<a href=\"{}\">{}</a>",
                escape_html(target),
                escape_html(desc)
            ));
        } else {
            html.push_str(&format!(
                "<span class=\"broken-link\" title=\"Unresolved ID: {}\">{}</span>",
                escape_html(id),
                escape_html(desc)
            ));
        }
    } else if link.url.starts_with('#') {
        let desc = link.description.as_deref().unwrap_or(&link.url[1..]);
        html.push_str(&format!(
            "<a href=\"{}\">{}</a>",
            escape_html(&link.url),
            escape_html(desc)
        ));
    } else if link.url.starts_with('*') {
        let heading_text = &link.url[1..];
        let desc = link.description.as_deref().unwrap_or(heading_text);
        html.push_str(&format!(
            "<a href=\"#{}\">{}</a>",
            escape_html(&slugify(heading_text)),
            escape_html(desc)
        ));
    } else if link.url.starts_with("file:") {
        let file_path = &link.url[5..];
        let (path_part, search_part) = if let Some(sep) = file_path.find("::") {
            (&file_path[..sep], Some(&file_path[sep + 2..]))
        } else {
            (file_path, None)
        };
        let html_path = if path_part.ends_with(".org") {
            format!("{}.html", &path_part[..path_part.len() - 4])
        } else {
            path_part.to_string()
        };
        let anchor = search_part.map(|s| {
            if s.starts_with('#') {
                s.to_string()
            } else if s.starts_with('*') {
                format!("#{}", slugify(&s[1..]))
            } else {
                format!("#{}", slugify(s))
            }
        });
        let full_href = match anchor {
            Some(a) => format!("{}{}", html_path, a),
            None => html_path,
        };
        let desc = link.description.as_deref().unwrap_or(&link.url);
        html.push_str(&format!(
            "<a href=\"{}\">{}</a>",
            escape_html(&full_href),
            escape_html(desc)
        ));
    } else if link.description.is_none() && is_image_url(&link.url) {
        html.push_str(&format!(
            "<img src=\"{}\" alt=\"{}\">",
            escape_html(&link.url),
            escape_html(&link.url)
        ));
    } else if !link.url.contains(':') && !link.url.contains('/') && !link.url.contains('.') {
        // Bare [[Text]] with no scheme/path — treat as an internal heading link
        let desc = link.description.as_deref().unwrap_or(&link.url);
        html.push_str(&format!(
            "<a href=\"#{}\">{}</a>",
            escape_html(&slugify(&link.url)),
            escape_html(desc)
        ));
    } else {
        let desc = link.description.as_deref().unwrap_or(&link.url);
        html.push_str(&format!(
            "<a href=\"{}\">{}</a>",
            escape_html(&link.url),
            escape_html(desc)
        ));
    }
}

fn is_image_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".svg")
        || lower.ends_with(".webp")
        || lower.ends_with(".bmp")
}

// ==================== Special strings / entities ====================

fn process_special_strings(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Org entity: \name or \name{}
        if chars[i] == '\\' && i + 1 < len && chars[i + 1].is_alphabetic() {
            let start = i + 1;
            let mut end = start;
            while end < len && chars[end].is_alphabetic() {
                end += 1;
            }
            let name = &text[start..end];
            let final_pos = if end + 1 < len && chars[end] == '{' && chars[end + 1] == '}' {
                end + 2
            } else {
                end
            };
            if let Some(entity) = lookup_entity(name) {
                result.push_str(entity);
                i = final_pos;
                continue;
            }
        }
        // Em-dash ---
        if i + 2 < len && chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '-' {
            result.push_str("&mdash;");
            i += 3;
            continue;
        }
        // En-dash --
        if i + 1 < len && chars[i] == '-' && chars[i + 1] == '-' {
            result.push_str("&ndash;");
            i += 2;
            continue;
        }
        // Ellipsis ...
        if i + 2 < len && chars[i] == '.' && chars[i + 1] == '.' && chars[i + 2] == '.' {
            result.push_str("&hellip;");
            i += 3;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

fn lookup_entity(name: &str) -> Option<&'static str> {
    match name {
        "alpha" => Some("&alpha;"),
        "beta" => Some("&beta;"),
        "gamma" => Some("&gamma;"),
        "delta" => Some("&delta;"),
        "epsilon" => Some("&epsilon;"),
        "zeta" => Some("&zeta;"),
        "eta" => Some("&eta;"),
        "theta" => Some("&theta;"),
        "iota" => Some("&iota;"),
        "kappa" => Some("&kappa;"),
        "lambda" => Some("&lambda;"),
        "mu" => Some("&mu;"),
        "nu" => Some("&nu;"),
        "xi" => Some("&xi;"),
        "pi" => Some("&pi;"),
        "rho" => Some("&rho;"),
        "sigma" => Some("&sigma;"),
        "tau" => Some("&tau;"),
        "upsilon" => Some("&upsilon;"),
        "phi" => Some("&phi;"),
        "chi" => Some("&chi;"),
        "psi" => Some("&psi;"),
        "omega" => Some("&omega;"),
        "Alpha" => Some("&Alpha;"),
        "Beta" => Some("&Beta;"),
        "Gamma" => Some("&Gamma;"),
        "Delta" => Some("&Delta;"),
        "Theta" => Some("&Theta;"),
        "Lambda" => Some("&Lambda;"),
        "Pi" => Some("&Pi;"),
        "Sigma" => Some("&Sigma;"),
        "Phi" => Some("&Phi;"),
        "Psi" => Some("&Psi;"),
        "Omega" => Some("&Omega;"),
        "to" | "rarr" => Some("&rarr;"),
        "larr" | "leftarrow" => Some("&larr;"),
        "uarr" => Some("&uarr;"),
        "darr" => Some("&darr;"),
        "harr" => Some("&harr;"),
        "rArr" | "Rightarrow" => Some("&rArr;"),
        "lArr" | "Leftarrow" => Some("&lArr;"),
        "hArr" => Some("&hArr;"),
        "nbsp" => Some("&nbsp;"),
        "ensp" => Some("&ensp;"),
        "emsp" => Some("&emsp;"),
        "thinsp" => Some("&thinsp;"),
        "mdash" => Some("&mdash;"),
        "ndash" => Some("&ndash;"),
        "hellip" => Some("&hellip;"),
        "laquo" => Some("&laquo;"),
        "raquo" => Some("&raquo;"),
        "lsquo" => Some("&lsquo;"),
        "rsquo" => Some("&rsquo;"),
        "ldquo" => Some("&ldquo;"),
        "rdquo" => Some("&rdquo;"),
        "times" => Some("&times;"),
        "divide" => Some("&divide;"),
        "plusmn" | "pm" => Some("&plusmn;"),
        "infty" | "infin" => Some("&infin;"),
        "ne" => Some("&ne;"),
        "le" => Some("&le;"),
        "ge" => Some("&ge;"),
        "approx" => Some("&asymp;"),
        "sum" => Some("&sum;"),
        "prod" => Some("&prod;"),
        "int" => Some("&int;"),
        "partial" => Some("&part;"),
        "nabla" => Some("&nabla;"),
        "forall" => Some("&forall;"),
        "exists" => Some("&exist;"),
        "empty" => Some("&empty;"),
        "in" => Some("&isin;"),
        "notin" => Some("&notin;"),
        "sub" => Some("&sub;"),
        "sup" => Some("&sup;"),
        "cap" => Some("&cap;"),
        "cup" => Some("&cup;"),
        "and" => Some("&and;"),
        "or" => Some("&or;"),
        "not" => Some("&not;"),
        "deg" => Some("&deg;"),
        "prime" => Some("&prime;"),
        "Prime" => Some("&Prime;"),
        "star" => Some("&lowast;"),
        "bullet" => Some("&bull;"),
        "dagger" => Some("&dagger;"),
        "Dagger" => Some("&Dagger;"),
        "dollar" | "USD" => Some("$"),
        "amp" => Some("&amp;"),
        "copy" => Some("&copy;"),
        "reg" => Some("&reg;"),
        "trade" => Some("&trade;"),
        "sect" => Some("&sect;"),
        "para" => Some("&para;"),
        _ => None,
    }
}

// ==================== CSS ====================

pub(crate) fn default_css() -> &'static str {
    r#"
body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
  max-width: 800px;
  margin: 0 auto;
  padding: 2em;
  line-height: 1.6;
  color: #333;
}
h1, h2, h3, h4, h5, h6 { margin-top: 1.5em; margin-bottom: 0.5em; }
h1 { border-bottom: 2px solid #eee; padding-bottom: 0.3em; }
.todo-keyword { font-weight: bold; padding: 0.1em 0.3em; border-radius: 3px; font-size: 0.85em; }
.todo-keyword.TODO { background: #fff3cd; color: #856404; }
.todo-keyword.DONE { background: #d4edda; color: #155724; }
.todo-keyword.NEXT { background: #cce5ff; color: #004085; }
.todo-keyword.WAITING { background: #e2d5f1; color: #563d7c; }
.todo-keyword.CANCELLED { background: #f8d7da; color: #721c24; text-decoration: line-through; }
.todo-keyword.IN-PROGRESS { background: #d1ecf1; color: #0c5460; }
.done-keyword { background: #d4edda; color: #155724; }
.org-tag { display: inline-block; background: #e9ecef; color: #495057; padding: 0.1em 0.4em; border-radius: 3px; font-size: 0.8em; margin-left: 0.3em; }
.priority { font-weight: bold; margin-right: 0.3em; }
.priority-A { color: #dc3545; }
.priority-B { color: #ffc107; }
.priority-C { color: #17a2b8; }
.planning { color: #6c757d; font-size: 0.9em; margin-bottom: 0.5em; }
.planning .label { font-weight: bold; }
.broken-link { color: #dc3545; text-decoration: underline wavy; }
a { color: #007bff; text-decoration: none; }
a:hover { text-decoration: underline; }
code { background: #f4f4f4; padding: 0.15em 0.3em; border-radius: 3px; font-size: 0.9em; }
del { color: #999; }
img { max-width: 100%; height: auto; }
.backlinks { font-size: 0.85em; color: #6c757d; margin-bottom: 0.5em; }
.backlinks-label { font-weight: bold; }
.backlinks a { color: #6c757d; }
.document-title { font-size: 2em; font-weight: bold; margin-bottom: 0.5em; }
.document-meta { color: #6c757d; margin-bottom: 2em; }
.index-list { list-style: none; padding: 0; }
.index-list li { margin: 0.5em 0; }
.index-list a { font-size: 1.1em; }
"#
}

// ==================== Helpers ====================

fn format_timestamp_html(ts: &Timestamp) -> String {
    let mut s = format!("{:04}-{:02}-{:02}", ts.date.year, ts.date.month, ts.date.day);
    if let Some(ref wd) = ts.date.weekday {
        s.push(' ');
        s.push_str(wd);
    }
    if let Some(ref t) = ts.time {
        s.push_str(&format!(" {:02}:{:02}", t.hour, t.minute));
        if let Some(ref et) = ts.end_time {
            s.push_str(&format!("-{:02}:{:02}", et.hour, et.minute));
        }
    }
    s
}

pub(crate) fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn has_latex_fragments(doc: &OrgDocument) -> bool {
    doc.entries.iter().any(|e| {
        let b = &e.body;
        b.contains("\\(") || b.contains("\\[") || b.contains("$$") || b.contains("\\begin{")
    })
}

fn inject_attr_html(html_fragment: &str, attrs: &str) -> String {
    let attr_pairs = parse_attr_html(attrs);
    let mut result = html_fragment.to_string();
    if let Some(tag_end) = result.find('>') {
        if !result[..tag_end].contains('/') || result[..tag_end].contains(' ') {
            let attr_str: String = attr_pairs
                .iter()
                .map(|(k, v)| format!(" {}=\"{}\"", k, escape_html(v)))
                .collect();
            result.insert_str(tag_end, &attr_str);
        }
    }
    result
}

fn parse_attr_html(attrs: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let parts: Vec<&str> = attrs.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        if parts[i].starts_with(':') {
            let key = &parts[i][1..];
            if i + 1 < parts.len() && !parts[i + 1].starts_with(':') {
                pairs.push((key.to_string(), parts[i + 1].to_string()));
                i += 2;
            } else {
                pairs.push((key.to_string(), String::new()));
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    pairs
}

// ==================== Export options (from #+OPTIONS) ====================

struct ExportOptions {
    show_todo: bool,
    show_tags: bool,
    show_priority: bool,
    show_planning: bool,
}

impl ExportOptions {
    fn from_document(doc: &OrgDocument) -> Self {
        ExportOptions {
            show_todo: doc.option_value("todo").as_deref() != Some("nil"),
            show_tags: doc.option_value("tags").as_deref() != Some("nil"),
            show_priority: doc.option_value("pri").as_deref() != Some("nil"),
            show_planning: doc.option_value("p").as_deref() != Some("nil"),
        }
    }
}

// ==================== Page title resolution ====================

/// Resolve the page title with fallback priority:
///   1. #+TITLE: keyword
///   2. First top-level heading (level == 1)
///   3. "Untitled"
pub fn resolve_page_title<'a>(doc: &'a OrgDocument) -> &'a str {
    if let Some(t) = doc.title() {
        return t;
    }
    if let Some(entry) = doc.entries.iter().find(|e| e.level == 1) {
        return &entry.title;
    }
    "Untitled"
}

// ==================== Public render API ====================

/// Options for rendering an `OrgDocument` to a complete HTML page.
pub struct RenderOptions<'a> {
    /// Raw HTML injected immediately after `<body>` (e.g. nav header).
    pub preamble: Option<&'a str>,
    /// Raw HTML that replaces the built-in `<style>` block inside `<head>`.
    pub head: Option<&'a str>,
    /// Raw HTML appended at the end of `<head>` (favicon, manifests, …).
    pub head_extra: Option<&'a str>,
}

impl<'a> RenderOptions<'a> {
    /// All options absent — produces the default built-in stylesheet.
    pub fn none() -> Self {
        RenderOptions { preamble: None, head: None, head_extra: None }
    }
}

/// Render an `OrgDocument` to a complete HTML page string.
///
/// `id_map` maps org-id values to their resolved HTML paths.
/// `preamble` is optional raw HTML injected after `<body>`.
pub fn render_html(
    doc: &OrgDocument,
    id_map: &HashMap<String, String>,
    preamble: Option<&str>,
) -> String {
    render_html_opts(doc, id_map, &RenderOptions { preamble, head: None, head_extra: None })
}

/// Like `render_html` but with full control over head and preamble content.
pub fn render_html_opts(
    doc: &OrgDocument,
    id_map: &HashMap<String, String>,
    opts: &RenderOptions,
) -> String {
    let mut html = String::new();
    let title = resolve_page_title(doc);

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str(&format!("<title>{}</title>\n", escape_html(title)));

    match opts.head {
        Some(h) if !h.is_empty() => html.push_str(h),
        _ => {
            html.push_str("<style>");
            html.push_str(default_css());
            html.push_str("</style>\n");
        }
    }
    if has_latex_fragments(doc) {
        html.push_str("<script src=\"https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js\" async></script>\n");
    }
    if let Some(he) = opts.head_extra {
        if !he.is_empty() {
            html.push_str(he);
            if !he.ends_with('\n') { html.push('\n'); }
        }
    }
    html.push_str("</head>\n<body>\n");

    if let Some(pre) = opts.preamble {
        if !pre.is_empty() {
            html.push_str(pre);
            if !pre.ends_with('\n') { html.push('\n'); }
        }
    }

    if let Some(t) = doc.title() {
        html.push_str(&format!("<div class=\"document-title\">{}</div>\n", escape_html(t)));
    }
    if let Some(a) = doc.author() {
        html.push_str(&format!("<div class=\"document-meta\">By {}</div>\n", escape_html(a)));
    }

    // Render document preamble (text before the first heading)
    if !doc.preamble.is_empty() {
        render_body(&mut html, &doc.preamble, id_map);
    }

    let export_opts = ExportOptions::from_document(doc);

    // Table of Contents — off by default, enabled by #+OPTIONS: toc:t or toc:<depth>
    let toc_option = doc.option_value("toc");
    let toc_enabled = matches!(toc_option.as_deref(), Some(v) if v != "nil" && v != "");
    let toc_depth: usize = toc_option
        .as_deref()
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);
    if toc_enabled && !doc.entries.is_empty() {
        render_toc(&mut html, &doc.entries, toc_depth);
    }

    // Determine noexport entries
    let mut skip_entries = vec![false; doc.entries.len()];
    let mut noexport_level: Option<usize> = None;
    for (idx, entry) in doc.entries.iter().enumerate() {
        if let Some(level) = noexport_level {
            if entry.level > level {
                skip_entries[idx] = true;
                continue;
            } else {
                noexport_level = None;
            }
        }
        if entry.tags.iter().any(|t| t == "noexport") {
            skip_entries[idx] = true;
            noexport_level = Some(entry.level);
        }
    }

    // Extract footnote definitions from all bodies upfront
    let mut all_footnote_defs: Vec<(String, String)> = Vec::new();
    let mut cleaned_bodies: Vec<String> = Vec::new();
    for (idx, entry) in doc.entries.iter().enumerate() {
        if skip_entries[idx] {
            cleaned_bodies.push(String::new());
            continue;
        }
        let (cleaned, defs) = extract_footnote_definitions(&entry.body);
        cleaned_bodies.push(cleaned);
        all_footnote_defs.extend(defs);
    }

    let mut footnote_counter: usize = 0;
    let mut inline_defs: Vec<(String, String)> = Vec::new();
    for (idx, entry) in doc.entries.iter().enumerate() {
        if !skip_entries[idx] {
            render_entry(&mut html, entry, id_map, &cleaned_bodies[idx],
                         &mut footnote_counter, &mut inline_defs, &export_opts);
        }
    }
    all_footnote_defs.extend(inline_defs);

    if !all_footnote_defs.is_empty() {
        html.push_str("<div class=\"footnotes\">\n<h2>Footnotes</h2>\n<ol>\n");
        for i in 0..footnote_counter {
            let num = i + 1;
            let def_text = all_footnote_defs.get(i).map(|d| d.1.as_str()).unwrap_or("");
            html.push_str(&format!(
                "<li id=\"fn-{}\">{} <a href=\"#fnr-{}\">↩</a></li>\n",
                num,
                render_inline_html(def_text, id_map),
                num
            ));
        }
        html.push_str("</ol>\n</div>\n");
    }

    html.push_str("</body>\n</html>\n");
    html
}

// ==================== ToC ====================

fn render_toc(html: &mut String, entries: &[OrgEntry], max_depth: usize) {
    let toc_entries: Vec<&OrgEntry> = entries
        .iter()
        .filter(|e| e.level <= max_depth && !e.tags.iter().any(|t| t == "noexport"))
        .collect();
    if toc_entries.is_empty() {
        return;
    }
    html.push_str("<nav id=\"table-of-contents\">\n");
    html.push_str("<h2>Table of Contents</h2>\n");
    let mut current_level = 0;
    for entry in &toc_entries {
        let level = entry.level;
        while current_level < level { html.push_str("<ul>\n"); current_level += 1; }
        while current_level > level { html.push_str("</ul>\n"); current_level -= 1; }
        let anchor = entry.properties.get("CUSTOM_ID")
            .or_else(|| entry.properties.get("ID"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| slugify(&entry.title));
        html.push_str(&format!(
            "<li><a href=\"#{}\">{}</a></li>\n",
            escape_html(&anchor),
            escape_html(&entry.title)
        ));
    }
    while current_level > 0 { html.push_str("</ul>\n"); current_level -= 1; }
    html.push_str("</nav>\n");
}

// ==================== Entry rendering ====================

fn render_entry(
    html: &mut String,
    entry: &OrgEntry,
    id_map: &HashMap<String, String>,
    cleaned_body: &str,
    footnote_counter: &mut usize,
    inline_defs: &mut Vec<(String, String)>,
    opts: &ExportOptions,
) {
    let level = entry.level.min(6);
    let anchor_id = entry.properties.get("CUSTOM_ID")
        .or_else(|| entry.properties.get("ID"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| slugify(&entry.title));

    html.push_str(&format!("<h{} id=\"{}\">", level, escape_html(&anchor_id)));

    if opts.show_todo {
        if let Some(ref kw) = entry.keyword {
            let kw_str = kw.as_str();
            let extra = if matches!(kw, Keyword::Done) { " done-keyword" } else { "" };
            html.push_str(&format!(
                "<span class=\"todo-keyword {}{}\">{}</span> ",
                kw_str, extra, kw_str
            ));
        }
    }
    if opts.show_priority {
        if let Some(ref prio) = entry.priority {
            let c = prio.as_char();
            html.push_str(&format!("<span class=\"priority priority-{}\">[#{}]</span> ", c, c));
        }
    }

    html.push_str(&render_inline_html(&entry.title, id_map));

    if opts.show_tags {
        for tag in &entry.tags {
            html.push_str(&format!(" <span class=\"org-tag\">{}</span>", escape_html(tag)));
        }
    }
    html.push_str(&format!("</h{}>\n", level));

    // Planning
    let has_planning = opts.show_planning
        && (entry.scheduled.is_some() || entry.deadline.is_some() || entry.closed.is_some());
    if has_planning {
        html.push_str("<div class=\"planning\">");
        if let Some(ref c) = entry.closed {
            html.push_str(&format!("<span class=\"label\">CLOSED:</span> {} ", format_timestamp_html(c)));
        }
        if let Some(ref s) = entry.scheduled {
            html.push_str(&format!("<span class=\"label scheduled\">SCHEDULED:</span> {} ", format_timestamp_html(s)));
        }
        if let Some(ref d) = entry.deadline {
            html.push_str(&format!("<span class=\"label deadline\">DEADLINE:</span> {} ", format_timestamp_html(d)));
        }
        html.push_str("</div>\n");
    }

    // Backlinks
    if let Some(ref raw) = entry.backlinks_raw {
        let inner = raw.trim().trim_start_matches('/').trim_end_matches('/').trim();
        let links_text = inner
            .strip_prefix("Backlinks: ")
            .or_else(|| inner.strip_prefix("Backlinks:"))
            .unwrap_or(inner);
        if !links_text.is_empty() {
            html.push_str("<div class=\"backlinks\"><span class=\"backlinks-label\">Backlinks:</span> ");
            html.push_str(&render_inline_html(links_text, id_map));
            html.push_str("</div>\n");
        }
    }

    // Body
    if !cleaned_body.is_empty() {
        let processed = replace_footnote_refs(cleaned_body, footnote_counter, inline_defs);
        render_body_raw(html, &processed, id_map);
    }
}

// ==================== Body element parsing ====================

#[derive(Debug)]
enum ListKind { Unordered, Ordered, Description }

#[derive(Debug)]
enum Checkbox { Unchecked, Checked, Partial }

#[derive(Debug)]
struct ListItem {
    indent: usize,
    kind: ListKind,
    checkbox: Option<Checkbox>,
    term: Option<String>,
    text: String,
}

fn parse_list_line(line: &str) -> Option<ListItem> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();

    if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("+ ")) {
        return Some(parse_list_item_content(indent, ListKind::Unordered, rest));
    }
    if indent > 0 {
        if let Some(rest) = trimmed.strip_prefix("* ") {
            return Some(parse_list_item_content(indent, ListKind::Unordered, rest));
        }
    }
    let mut i = 0;
    let bytes = trimmed.as_bytes();
    while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
    if i > 0 && i < bytes.len() {
        let after = bytes[i];
        if (after == b'.' || after == b')') && i + 1 < bytes.len() && bytes[i + 1] == b' ' {
            return Some(parse_list_item_content(indent, ListKind::Ordered, &trimmed[i + 2..]));
        }
    }
    None
}

fn parse_list_item_content(indent: usize, kind: ListKind, rest: &str) -> ListItem {
    let (checkbox, text) = if rest.starts_with("[ ] ") {
        (Some(Checkbox::Unchecked), &rest[4..])
    } else if rest.starts_with("[X] ") || rest.starts_with("[x] ") {
        (Some(Checkbox::Checked), &rest[4..])
    } else if rest.starts_with("[-] ") {
        (Some(Checkbox::Partial), &rest[4..])
    } else {
        (None, rest)
    };

    if matches!(kind, ListKind::Unordered) && checkbox.is_none() {
        if let Some(sep) = text.find(" :: ") {
            return ListItem {
                indent,
                kind: ListKind::Description,
                checkbox: None,
                term: Some(text[..sep].to_string()),
                text: text[sep + 4..].to_string(),
            };
        }
    }
    ListItem { indent, kind, checkbox, term: None, text: text.to_string() }
}

fn is_continuation_line(line: &str, item_indent: usize) -> bool {
    if line.trim().is_empty() { return false; }
    let line_indent = line.len() - line.trim_start().len();
    line_indent > item_indent && parse_list_line(line).is_none()
}

#[derive(Debug)]
enum BodyElement {
    Paragraph(String),
    List(Vec<ListItem>, Vec<Vec<BodyElement>>),
    HorizontalRule,
    SrcBlock { language: Option<String>, content: String },
    ExampleBlock(String),
    FixedWidth(String),
    Table { has_header: bool, header: Vec<Vec<String>>, body: Vec<Vec<String>> },
    GenericBlock { name: String, arg: String, content: String },
    CaptionedElement { caption: String, attr_html: Option<String>, inner: Box<BodyElement> },
}

fn is_table_separator(row: &str) -> bool {
    let inner = row.trim().trim_start_matches('|').trim_end_matches('|');
    !inner.is_empty() && inner.chars().all(|c| c == '-' || c == '+' || c == ' ')
}

fn parse_table_row(row: &str) -> Vec<String> {
    let inner = row.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn push_element(
    elements: &mut Vec<BodyElement>,
    elem: BodyElement,
    pending_caption: &mut Option<String>,
    pending_attr_html: &mut Option<String>,
) {
    if pending_caption.is_some() || pending_attr_html.is_some() {
        elements.push(BodyElement::CaptionedElement {
            caption: pending_caption.take().unwrap_or_default(),
            attr_html: pending_attr_html.take(),
            inner: Box::new(elem),
        });
    } else {
        elements.push(elem);
    }
}

fn parse_body_elements(body: &str) -> Vec<BodyElement> {
    let lines: Vec<&str> = body.lines().collect();
    let mut elements = Vec::new();
    let mut i = 0;
    let mut pending_caption: Option<String> = None;
    let mut pending_attr_html: Option<String> = None;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let trimmed_up = trimmed.to_uppercase();

        if trimmed_up.starts_with("#+CAPTION:") {
            pending_caption = Some(trimmed[10..].trim().to_string());
            i += 1; continue;
        }
        if trimmed_up.starts_with("#+ATTR_HTML:") {
            pending_attr_html = Some(trimmed[12..].trim().to_string());
            i += 1; continue;
        }
        if trimmed_up.starts_with("#+HTML:") {
            let raw = trimmed[7..].trim().to_string();
            push_element(&mut elements,
                BodyElement::GenericBlock { name: "EXPORT".into(), arg: "html".into(), content: format!("{}\n", raw) },
                &mut pending_caption, &mut pending_attr_html);
            i += 1; continue;
        }
        if trimmed_up.starts_with("#+TBLFM:") {
            i += 1; continue;
        }
        if trimmed.starts_with("# ") || trimmed == "#" {
            i += 1; continue;
        }
        if trimmed.len() >= 5 && trimmed.chars().all(|c| c == '-') {
            push_element(&mut elements, BodyElement::HorizontalRule, &mut pending_caption, &mut pending_attr_html);
            i += 1; continue;
        }

        if trimmed_up.starts_with("#+BEGIN_SRC") {
            let lang = trimmed[11..].trim().split_whitespace().next()
                .filter(|s| !s.is_empty()).map(|s| s.to_string());
            i += 1;
            let mut content = String::new();
            while i < lines.len() {
                if lines[i].trim().to_uppercase().starts_with("#+END_SRC") { i += 1; break; }
                content.push_str(lines[i]); content.push('\n'); i += 1;
            }
            push_element(&mut elements, BodyElement::SrcBlock { language: lang, content }, &mut pending_caption, &mut pending_attr_html);
            continue;
        }
        if trimmed_up.starts_with("#+BEGIN_EXAMPLE") {
            i += 1;
            let mut content = String::new();
            while i < lines.len() {
                if lines[i].trim().to_uppercase().starts_with("#+END_EXAMPLE") { i += 1; break; }
                content.push_str(lines[i]); content.push('\n'); i += 1;
            }
            push_element(&mut elements, BodyElement::ExampleBlock(content), &mut pending_caption, &mut pending_attr_html);
            continue;
        }
        if trimmed_up.starts_with("#+BEGIN_") {
            let block_name_part = &trimmed[8..];
            let block_name = block_name_part.split_whitespace().next().unwrap_or("").to_string();
            let block_arg = block_name_part[block_name.len()..].trim().to_string();
            let block_name_upper = block_name.to_uppercase();
            let end_tag = format!("#+END_{}", block_name_upper);
            i += 1;
            let mut content = String::new();
            while i < lines.len() {
                if lines[i].trim().to_uppercase().starts_with(&end_tag) { i += 1; break; }
                content.push_str(lines[i]); content.push('\n'); i += 1;
            }
            push_element(&mut elements,
                BodyElement::GenericBlock { name: block_name_upper, arg: block_arg, content },
                &mut pending_caption, &mut pending_attr_html);
            continue;
        }
        if trimmed.starts_with(": ") || trimmed == ":" {
            let mut content = String::new();
            while i < lines.len() {
                let lt = lines[i].trim();
                if lt.starts_with(": ") { content.push_str(&lt[2..]); content.push('\n'); i += 1; }
                else if lt == ":" { content.push('\n'); i += 1; }
                else { break; }
            }
            push_element(&mut elements, BodyElement::FixedWidth(content), &mut pending_caption, &mut pending_attr_html);
            continue;
        }
        if trimmed.starts_with('|') {
            let mut rows = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with('|') {
                rows.push(lines[i].trim()); i += 1;
            }
            let mut header_rows: Vec<Vec<String>> = Vec::new();
            let mut body_rows: Vec<Vec<String>> = Vec::new();
            let mut found_sep = false;
            for row in &rows {
                if is_table_separator(row) { found_sep = true; continue; }
                let cells = parse_table_row(row);
                if !found_sep { header_rows.push(cells); } else { body_rows.push(cells); }
            }
            let elem = if found_sep {
                BodyElement::Table { has_header: true, header: header_rows, body: body_rows }
            } else {
                BodyElement::Table { has_header: false, header: Vec::new(), body: header_rows }
            };
            push_element(&mut elements, elem, &mut pending_caption, &mut pending_attr_html);
            continue;
        }
        if let Some(mut item) = parse_list_line(line) {
            let list_indent = item.indent;
            let mut items: Vec<ListItem> = Vec::new();
            let mut children: Vec<Vec<BodyElement>> = Vec::new();
            loop {
                i += 1;
                let mut child_lines = String::new();
                while i < lines.len() {
                    let next = lines[i];
                    if let Some(ni) = parse_list_line(next) {
                        if ni.indent <= list_indent { break; }
                        child_lines.push_str(next); child_lines.push('\n'); i += 1; continue;
                    }
                    if next.trim().is_empty() {
                        if i + 1 < lines.len() && lines[i + 1].trim().is_empty() { break; }
                        let mut peek = i + 1;
                        while peek < lines.len() && lines[peek].trim().is_empty() { peek += 1; }
                        if peek < lines.len() {
                            if let Some(pi) = parse_list_line(lines[peek]) {
                                if pi.indent == list_indent {
                                    child_lines.push('\n'); i += 1; continue;
                                }
                            }
                        }
                        break;
                    }
                    let next_indent = next.len() - next.trim_start().len();
                    if next_indent > list_indent {
                        if is_continuation_line(next, list_indent) {
                            item.text.push(' '); item.text.push_str(next.trim());
                        } else {
                            child_lines.push_str(next); child_lines.push('\n');
                        }
                        i += 1;
                    } else { break; }
                }
                let child_elems = if child_lines.is_empty() { Vec::new() }
                                  else { parse_body_elements(&child_lines) };
                items.push(item);
                children.push(child_elems);
                if i < lines.len() {
                    if let Some(ni) = parse_list_line(lines[i]) {
                        if ni.indent == list_indent { item = ni; continue; }
                    }
                }
                break;
            }
            push_element(&mut elements, BodyElement::List(items, children), &mut pending_caption, &mut pending_attr_html);
        } else if line.trim().is_empty() {
            i += 1;
        } else {
            // Regular paragraph
            let mut para = String::new();
            while i < lines.len() {
                let l = lines[i];
                let lt = l.trim();
                if lt.is_empty() || parse_list_line(l).is_some() { break; }
                let ltu = lt.to_uppercase();
                if ltu.starts_with("#+BEGIN_") || lt.starts_with(": ") || lt == ":"
                    || lt.starts_with('|') || (lt.len() >= 5 && lt.chars().all(|c| c == '-'))
                    || lt.starts_with("# ") || lt == "#" { break; }
                if ltu.starts_with("#+") {
                    if para.is_empty() { i += 1; }
                    break;
                }
                if !para.is_empty() { para.push(' '); }
                para.push_str(lt);
                i += 1;
            }
            if !para.is_empty() {
                push_element(&mut elements, BodyElement::Paragraph(para), &mut pending_caption, &mut pending_attr_html);
            }
        }
    }
    elements
}

// ==================== Body element rendering ====================

fn render_body(html: &mut String, body: &str, id_map: &HashMap<String, String>) {
    for elem in &parse_body_elements(body) {
        render_body_element(html, elem, id_map);
    }
}

/// Render body text that already has footnote `<sup>` tags embedded.
/// We parse and render normally, then unescape the `<sup>` tags that
/// `render_inline_html` double-escaped.
fn render_body_raw(html: &mut String, body: &str, id_map: &HashMap<String, String>) {
    let mut temp = String::new();
    render_body(&mut temp, body, id_map);
    // Unescape the footnote ref HTML that got double-escaped
    let unescaped = temp
        .replace("&lt;sup&gt;&lt;a id=&quot;fnr-", "<sup><a id=\"fnr-")
        .replace("&quot; href=&quot;#fn-", "\" href=\"#fn-")
        .replace("&quot; class=&quot;footnote-ref&quot;&gt;", "\" class=\"footnote-ref\">")
        .replace("&lt;/a&gt;&lt;/sup&gt;", "</a></sup>");
    html.push_str(&unescaped);
}

fn render_body_element(html: &mut String, elem: &BodyElement, id_map: &HashMap<String, String>) {
    match elem {
        BodyElement::Paragraph(text) => {
            html.push_str("<p>");
            html.push_str(&render_inline_html(text, id_map));
            html.push_str("</p>\n");
        }
        BodyElement::HorizontalRule => html.push_str("<hr>\n"),
        BodyElement::SrcBlock { language, content } => {
            let lang_class = language.as_ref()
                .map(|l| format!(" class=\"language-{}\"", escape_html(l)))
                .unwrap_or_default();
            html.push_str(&format!("<pre><code{}>{}</code></pre>\n", lang_class, escape_html(content)));
        }
        BodyElement::ExampleBlock(content) => {
            html.push_str(&format!("<pre class=\"example\">{}</pre>\n", escape_html(content)));
        }
        BodyElement::FixedWidth(content) => {
            html.push_str(&format!("<pre class=\"fixed-width\">{}</pre>\n", escape_html(content)));
        }
        BodyElement::GenericBlock { name, arg, content } => {
            render_generic_block(html, name, arg, content, id_map);
        }
        BodyElement::Table { has_header, header, body } => {
            render_table(html, *has_header, header, body, id_map);
        }
        BodyElement::List(items, children) => {
            render_list(html, items, children, id_map);
        }
        BodyElement::CaptionedElement { caption, attr_html, inner } => {
            render_captioned(html, caption, attr_html.as_deref(), inner, id_map);
        }
    }
}

fn render_generic_block(
    html: &mut String, name: &str, arg: &str, content: &str,
    id_map: &HashMap<String, String>,
) {
    match name {
        "COMMENT" => {}
        "QUOTE" => {
            html.push_str("<blockquote>\n");
            for line in content.lines() {
                if !line.trim().is_empty() {
                    html.push_str("<p>");
                    html.push_str(&render_inline_html(line.trim(), id_map));
                    html.push_str("</p>\n");
                }
            }
            html.push_str("</blockquote>\n");
        }
        "VERSE" => {
            html.push_str("<p class=\"verse\" style=\"white-space: pre-line;\">\n");
            for line in content.lines() {
                html.push_str(&render_inline_html(line, id_map));
                html.push_str("<br>\n");
            }
            html.push_str("</p>\n");
        }
        "CENTER" => {
            html.push_str("<div style=\"text-align: center;\">\n");
            for line in content.lines() {
                if !line.trim().is_empty() {
                    html.push_str("<p>");
                    html.push_str(&render_inline_html(line.trim(), id_map));
                    html.push_str("</p>\n");
                }
            }
            html.push_str("</div>\n");
        }
        "EXPORT" if arg.eq_ignore_ascii_case("html") => {
            html.push_str(content);
        }
        _ => {
            let class = name.to_lowercase();
            html.push_str(&format!("<div class=\"{}\">\n", escape_html(&class)));
            for line in content.lines() {
                if !line.trim().is_empty() {
                    html.push_str("<p>");
                    html.push_str(&render_inline_html(line.trim(), id_map));
                    html.push_str("</p>\n");
                }
            }
            html.push_str("</div>\n");
        }
    }
}

fn render_table(
    html: &mut String,
    has_header: bool,
    header: &[Vec<String>],
    body: &[Vec<String>],
    id_map: &HashMap<String, String>,
) {
    html.push_str("<table>\n");
    if has_header && !header.is_empty() {
        html.push_str("<thead>\n");
        for row in header {
            html.push_str("<tr>");
            for cell in row {
                html.push_str("<th>");
                html.push_str(&render_inline_html(cell, id_map));
                html.push_str("</th>");
            }
            html.push_str("</tr>\n");
        }
        html.push_str("</thead>\n");
    }
    html.push_str("<tbody>\n");
    for row in body {
        html.push_str("<tr>");
        for cell in row {
            html.push_str("<td>");
            html.push_str(&render_inline_html(cell, id_map));
            html.push_str("</td>");
        }
        html.push_str("</tr>\n");
    }
    html.push_str("</tbody>\n</table>\n");
}

fn render_list(
    html: &mut String,
    items: &[ListItem],
    children: &[Vec<BodyElement>],
    id_map: &HashMap<String, String>,
) {
    if items.is_empty() { return; }
    let tag = match items[0].kind {
        ListKind::Unordered => "ul",
        ListKind::Ordered => "ol",
        ListKind::Description => "dl",
    };
    html.push_str(&format!("<{}>\n", tag));
    for (idx, item) in items.iter().enumerate() {
        match item.kind {
            ListKind::Description => {
                html.push_str("<dt>");
                if let Some(ref term) = item.term {
                    html.push_str(&render_inline_html(term, id_map));
                }
                html.push_str("</dt>\n<dd>");
                html.push_str(&render_inline_html(&item.text, id_map));
                if let Some(c) = children.get(idx) {
                    for e in c { render_body_element(html, e, id_map); }
                }
                html.push_str("</dd>\n");
            }
            _ => {
                html.push_str("<li>");
                if let Some(ref cb) = item.checkbox {
                    html.push_str(match cb {
                        Checkbox::Unchecked => "<input type=\"checkbox\" disabled> ",
                        Checkbox::Checked   => "<input type=\"checkbox\" checked disabled> ",
                        Checkbox::Partial   => "<input type=\"checkbox\" class=\"checkbox-partial\" disabled> ",
                    });
                }
                html.push_str(&render_inline_html(&item.text, id_map));
                if let Some(c) = children.get(idx) {
                    for e in c { html.push('\n'); render_body_element(html, e, id_map); }
                }
                html.push_str("</li>\n");
            }
        }
    }
    html.push_str(&format!("</{}>\n", tag));
}

fn render_captioned(
    html: &mut String,
    caption: &str,
    attr_html: Option<&str>,
    inner: &BodyElement,
    id_map: &HashMap<String, String>,
) {
    // Image or table with caption/attrs → wrap in <figure>
    let is_figurable = matches!(inner, BodyElement::Paragraph(_) | BodyElement::Table { .. });
    if is_figurable && (attr_html.is_some() || !caption.is_empty()) {
        html.push_str("<figure>\n");
        if let Some(attrs) = attr_html {
            let mut inner_html = String::new();
            render_body_element(&mut inner_html, inner, id_map);
            html.push_str(&inject_attr_html(&inner_html, attrs));
        } else {
            render_body_element(html, inner, id_map);
        }
        if !caption.is_empty() {
            html.push_str(&format!(
                "<figcaption>{}</figcaption>\n",
                render_inline_html(caption, id_map)
            ));
        }
        html.push_str("</figure>\n");
    } else {
        html.push_str("<figure>\n");
        if let Some(attrs) = attr_html {
            let mut inner_html = String::new();
            render_body_element(&mut inner_html, inner, id_map);
            html.push_str(&inject_attr_html(&inner_html, attrs));
        } else {
            render_body_element(html, inner, id_map);
        }
        if !caption.is_empty() {
            html.push_str(&format!(
                "<figcaption>{}</figcaption>\n",
                render_inline_html(caption, id_map)
            ));
        }
        html.push_str("</figure>\n");
    }
}
