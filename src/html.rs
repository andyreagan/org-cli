use crate::commands::find_org_files;
use crate::parser::{parse_inline_markup, parse_org_document};
use crate::types::*;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ==================== HTML Escaping ====================

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ==================== Inline Markup to HTML ====================

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
        // Check for footnote definition: [fn:NAME] text...
        if trimmed.starts_with("[fn:") {
            if let Some(close) = trimmed.find(']') {
                let marker = &trimmed[4..close];
                let rest = trimmed[close + 1..].trim();
                // This is a footnote definition if the marker doesn't contain ':'
                // (inline footnotes have [fn:: ...] or [fn:name: ...] which are in-text)
                if !marker.contains(':') {
                    // Save previous footnote if any
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
        // Continuation of footnote definition (indented or non-empty after def)
        if in_footnote && !trimmed.is_empty() && (line.starts_with(' ') || line.starts_with('\t')) {
            current_def.push(' ');
            current_def.push_str(trimmed);
            continue;
        }
        // End of footnote
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

/// Replace footnote references [fn:NAME], [fn:: inline], [fn:NAME: inline] with HTML
fn replace_footnote_refs(text: &str, footnote_counter: &mut usize, inline_defs: &mut Vec<(String, String)>) -> String {
    let mut result = String::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("[fn:") {
        result.push_str(&remaining[..start]);
        let after = &remaining[start + 4..];

        // Find the closing ]
        if let Some(close) = after.find(']') {
            let inner = &after[..close];

            if inner.starts_with(": ") || inner.starts_with(':') {
                // Anonymous inline: [fn:: definition]
                *footnote_counter += 1;
                let num = *footnote_counter;
                let def_text = inner.trim_start_matches(':').trim().to_string();
                let fn_name = format!("_inline_{}", num);
                inline_defs.push((fn_name.clone(), def_text));
                result.push_str(&format!(
                    "<sup><a id=\"fnr-{}\" href=\"#fn-{}\" class=\"footnote-ref\">{}</a></sup>",
                    num, num, num
                ));
            } else if inner.contains(": ") {
                // Named inline: [fn:name: definition]
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
                // Named reference: [fn:NAME]
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

fn render_inline_html(text: &str, id_map: &HashMap<String, String>) -> String {
    // Handle \\ line breaks: replace \\ with <br> placeholder then restore after rendering
    let processed = text.replace("\\\\", "\x00LINEBREAK\x00");
    let fragments = parse_inline_markup(&processed);
    let mut html = String::new();

    for fragment in fragments {
        match fragment {
            InlineFragment::Text(s) => {
                let escaped = escape_html(&s);
                html.push_str(&process_special_strings(&escaped));
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
            InlineFragment::Code(s) => {
                html.push_str("<code>");
                html.push_str(&escape_html(&s));
                html.push_str("</code>");
            }
            InlineFragment::Verbatim(s) => {
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
                if link.is_id_link() {
                    let id = link.id_value().unwrap_or("");
                    let desc = link
                        .description
                        .as_deref()
                        .unwrap_or(&link.url);
                    if let Some(target) = id_map.get(id) {
                        html.push_str(&format!(
                            "<a href=\"{}\">{}</a>",
                            escape_html(target),
                            escape_html(desc)
                        ));
                    } else {
                        // Unresolved ID link
                        html.push_str(&format!(
                            "<span class=\"broken-link\" title=\"Unresolved ID: {}\">{}</span>",
                            escape_html(id),
                            escape_html(desc)
                        ));
                    }
                } else if link.url.starts_with('#') {
                    // Internal link to CUSTOM_ID: [[#my-id]]
                    let desc = link.description.as_deref().unwrap_or(&link.url[1..]);
                    html.push_str(&format!(
                        "<a href=\"{}\">{}</a>",
                        escape_html(&link.url),
                        escape_html(desc)
                    ));
                } else if link.url.starts_with('*') {
                    // Internal link to heading: [[*Heading Text]]
                    let heading_text = &link.url[1..];
                    let slug = slugify(heading_text);
                    let desc = link.description.as_deref().unwrap_or(heading_text);
                    html.push_str(&format!(
                        "<a href=\"#{}\">{}</a>",
                        escape_html(&slug),
                        escape_html(desc)
                    ));
                } else if link.url.starts_with("file:") {
                    // File link: rewrite .org → .html
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
                } else {
                    // Check for image inlining: no description + image URL
                    if link.description.is_none() && is_image_url(&link.url) {
                        html.push_str(&format!(
                            "<img src=\"{}\" alt=\"{}\">",
                            escape_html(&link.url),
                            escape_html(&link.url)
                        ));
                    } else {
                        let desc = link
                            .description
                            .as_deref()
                            .unwrap_or(&link.url);
                        html.push_str(&format!(
                            "<a href=\"{}\">{}</a>",
                            escape_html(&link.url),
                            escape_html(desc)
                        ));
                    }
                }
            }
        }
    }

    // Restore line break placeholders
    html.replace("\x00LINEBREAK\x00", "<br>")
}

/// Process special strings and org entities in plain text
fn process_special_strings(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Check for org entities: \name or \name{}
        if chars[i] == '\\' && i + 1 < len && chars[i + 1].is_alphabetic() {
            let start = i + 1;
            let mut end = start;
            while end < len && chars[end].is_alphabetic() {
                end += 1;
            }
            let name = &text[start..end];
            // Consume optional {} after entity name
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
            // Not a known entity, keep as-is
        }

        // Em-dash: ---
        if i + 2 < len && chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '-' {
            result.push_str("&mdash;");
            i += 3;
            continue;
        }
        // En-dash: --
        if i + 1 < len && chars[i] == '-' && chars[i + 1] == '-' {
            result.push_str("&ndash;");
            i += 2;
            continue;
        }
        // Ellipsis: ...
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
    // Common org entities (subset of the full list)
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
        "plusmn" => Some("&plusmn;"),
        "pm" => Some("&plusmn;"),
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

/// Check if document contains LaTeX fragments
fn has_latex_fragments(doc: &OrgDocument) -> bool {
    for entry in &doc.entries {
        let body = &entry.body;
        if body.contains("\\(") || body.contains("\\[")
            || body.contains("$$") || body.contains("\\begin{") {
            return true;
        }
    }
    false
}

/// Parse #+ATTR_HTML: :key value :key2 value2 and inject into HTML tag
fn inject_attr_html(html_fragment: &str, attrs: &str) -> String {
    let attr_pairs = parse_attr_html(attrs);
    let mut result = html_fragment.to_string();
    // Find the first HTML tag and inject attributes
    if let Some(tag_end) = result.find('>') {
        // Check it's not a closing tag
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

// ==================== CSS ====================

fn default_css() -> &'static str {
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
.document-title { font-size: 2em; font-weight: bold; margin-bottom: 0.5em; }
.document-meta { color: #6c757d; margin-bottom: 2em; }
.index-list { list-style: none; padding: 0; }
.index-list li { margin: 0.5em 0; }
.index-list a { font-size: 1.1em; }
"#
}

// ==================== Timestamp Formatting ====================

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

// ==================== Slug Generation ====================

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ==================== HTML Rendering ====================

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

/// Render an OrgDocument to a complete HTML page.
/// `id_map` maps org-id values to their resolved HTML paths (e.g., "file.html#id").
pub fn render_html(doc: &OrgDocument, id_map: &HashMap<String, String>) -> String {
    let mut html = String::new();

    let title = doc.title().unwrap_or("Untitled");

    // Document header
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str(&format!("<title>{}</title>\n", escape_html(title)));
    html.push_str("<style>");
    html.push_str(default_css());
    html.push_str("</style>\n");
    if has_latex_fragments(doc) {
        html.push_str("<script src=\"https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js\" async></script>\n");
    }
    html.push_str("</head>\n<body>\n");

    // Document title from preamble
    if let Some(t) = doc.title() {
        html.push_str(&format!(
            "<div class=\"document-title\">{}</div>\n",
            escape_html(t)
        ));
    }
    if let Some(a) = doc.author() {
        html.push_str(&format!(
            "<div class=\"document-meta\">By {}</div>\n",
            escape_html(a)
        ));
    }

    let opts = ExportOptions::from_document(doc);

    // Generate Table of Contents
    let toc_option = doc.option_value("toc");
    let toc_enabled = toc_option.as_deref() != Some("nil");
    let toc_depth: usize = toc_option
        .as_deref()
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);

    if toc_enabled && !doc.entries.is_empty() {
        render_toc(&mut html, &doc.entries, toc_depth);
    }

    // Determine which entries to skip (noexport tag and their children)
    let mut skip_entries: Vec<bool> = vec![false; doc.entries.len()];
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

    // Collect footnote definitions from all entry bodies
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

    // Render entries (using cleaned bodies)
    let mut footnote_counter: usize = 0;
    let mut inline_defs: Vec<(String, String)> = Vec::new();
    for (idx, entry) in doc.entries.iter().enumerate() {
        if skip_entries[idx] {
            continue;
        }
        render_entry_with_footnotes(&mut html, entry, id_map, &cleaned_bodies[idx], &mut footnote_counter, &mut inline_defs, &opts);
    }

    // Merge inline defs into all_footnote_defs
    all_footnote_defs.extend(inline_defs);

    // Render footnotes section if we have any
    if !all_footnote_defs.is_empty() {
        html.push_str("<div class=\"footnotes\">\n<h2>Footnotes</h2>\n<ol>\n");
        for i in 0..footnote_counter {
            let num = i + 1;
            let def_text = if num <= all_footnote_defs.len() {
                &all_footnote_defs[num - 1].1
            } else {
                ""
            };
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

fn render_toc(html: &mut String, entries: &[OrgEntry], max_depth: usize) {
    // Collect entries that should appear in TOC
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
        while current_level < level {
            html.push_str("<ul>\n");
            current_level += 1;
        }
        while current_level > level {
            html.push_str("</ul>\n");
            current_level -= 1;
        }
        let anchor = entry
            .properties
            .get("CUSTOM_ID")
            .or_else(|| entry.properties.get("ID"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| slugify(&entry.title));
        html.push_str(&format!(
            "<li><a href=\"#{}\">{}</a></li>\n",
            escape_html(&anchor),
            escape_html(&entry.title)
        ));
    }
    while current_level > 0 {
        html.push_str("</ul>\n");
        current_level -= 1;
    }

    html.push_str("</nav>\n");
}

fn render_entry_with_footnotes(
    html: &mut String,
    entry: &OrgEntry,
    id_map: &HashMap<String, String>,
    cleaned_body: &str,
    footnote_counter: &mut usize,
    inline_defs: &mut Vec<(String, String)>,
    opts: &ExportOptions,
) {
    render_entry_inner(html, entry, id_map, Some(cleaned_body), Some(footnote_counter), Some(inline_defs), opts);
}

#[allow(dead_code)]
fn render_entry(html: &mut String, entry: &OrgEntry, id_map: &HashMap<String, String>) {
    let default_opts = ExportOptions { show_todo: true, show_tags: true, show_priority: true, show_planning: true };
    render_entry_inner(html, entry, id_map, None, None, None, &default_opts);
}

fn render_entry_inner(
    html: &mut String,
    entry: &OrgEntry,
    id_map: &HashMap<String, String>,
    cleaned_body: Option<&str>,
    footnote_counter: Option<&mut usize>,
    inline_defs: Option<&mut Vec<(String, String)>>,
    opts: &ExportOptions,
) {
    let level = entry.level.min(6);

    // Determine the anchor id: CUSTOM_ID > ID > slugified title
    let anchor_id = entry
        .properties
        .get("CUSTOM_ID")
        .or_else(|| entry.properties.get("ID"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| slugify(&entry.title));

    // Open heading tag
    html.push_str(&format!("<h{} id=\"{}\">", level, escape_html(&anchor_id)));

    // Keyword
    if opts.show_todo {
        if let Some(ref kw) = entry.keyword {
            let kw_str = kw.as_str();
            let extra_class = if matches!(kw, Keyword::Done) {
                " done-keyword"
            } else {
                ""
            };
            html.push_str(&format!(
                "<span class=\"todo-keyword {}{}\">{}</span> ",
                kw_str, extra_class, kw_str
            ));
        }
    }

    // Priority
    if opts.show_priority {
        if let Some(ref prio) = entry.priority {
            let c = prio.as_char();
            html.push_str(&format!(
                "<span class=\"priority priority-{}\">[#{}]</span> ",
                c, c
            ));
        }
    }

    // Title with inline markup
    html.push_str(&render_inline_html(&entry.title, id_map));

    // Tags
    if opts.show_tags {
        for tag in &entry.tags {
            html.push_str(&format!(
                " <span class=\"org-tag\">{}</span>",
                escape_html(tag)
            ));
        }
    }

    html.push_str(&format!("</h{}>\n", level));

    // Planning line
    let has_planning = opts.show_planning && (entry.scheduled.is_some()
        || entry.deadline.is_some()
        || entry.closed.is_some());
    if has_planning {
        html.push_str("<div class=\"planning\">");
        if let Some(ref closed) = entry.closed {
            html.push_str(&format!(
                "<span class=\"label\">CLOSED:</span> {} ",
                format_timestamp_html(closed)
            ));
        }
        if let Some(ref sched) = entry.scheduled {
            html.push_str(&format!(
                "<span class=\"label scheduled\">SCHEDULED:</span> {} ",
                format_timestamp_html(sched)
            ));
        }
        if let Some(ref dl) = entry.deadline {
            html.push_str(&format!(
                "<span class=\"label deadline\">DEADLINE:</span> {} ",
                format_timestamp_html(dl)
            ));
        }
        html.push_str("</div>\n");
    }

    // Body
    let body_text = cleaned_body.unwrap_or(&entry.body);
    if !body_text.is_empty() {
        if let (Some(counter), Some(defs)) = (footnote_counter, inline_defs) {
            // Process footnote refs in the body
            let processed = replace_footnote_refs(body_text, counter, defs);
            render_body_raw(html, &processed, id_map);
        } else {
            render_body(html, body_text, id_map);
        }
    }
}

// ==================== Body Element Parsing ====================

#[derive(Debug)]
enum ListKind {
    Unordered,
    Ordered,
    Description,
}

#[derive(Debug)]
enum Checkbox {
    Unchecked,
    Checked,
    Partial,
}

#[derive(Debug)]
struct ListItem {
    indent: usize,
    kind: ListKind,
    checkbox: Option<Checkbox>,
    /// For description lists: the term before `::`
    term: Option<String>,
    /// The text content of the item (first line after bullet)
    text: String,
}

/// Detect if a line starts a list item. Returns (indent, ListItem) or None.
fn parse_list_line(line: &str) -> Option<ListItem> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();

    // Unordered: `- `, `+ ` (not `* ` which conflicts with headings at col 0)
    if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("+ ")) {
        return Some(parse_list_item_content(indent, ListKind::Unordered, rest));
    }

    // `* ` only when indented (not a heading)
    if indent > 0 {
        if let Some(rest) = trimmed.strip_prefix("* ") {
            return Some(parse_list_item_content(indent, ListKind::Unordered, rest));
        }
    }

    // Ordered: `1. `, `1) `, `12. `, etc.
    let mut i = 0;
    let bytes = trimmed.as_bytes();
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i < bytes.len() {
        let after_num = bytes[i];
        if (after_num == b'.' || after_num == b')') && i + 1 < bytes.len() && bytes[i + 1] == b' '
        {
            let rest = &trimmed[i + 2..];
            return Some(parse_list_item_content(indent, ListKind::Ordered, rest));
        }
    }

    None
}

fn parse_list_item_content(indent: usize, kind: ListKind, rest: &str) -> ListItem {
    // Check for checkbox
    let (checkbox, text_after_cb) = if rest.starts_with("[ ] ") {
        (Some(Checkbox::Unchecked), &rest[4..])
    } else if rest.starts_with("[X] ") || rest.starts_with("[x] ") {
        (Some(Checkbox::Checked), &rest[4..])
    } else if rest.starts_with("[-] ") {
        (Some(Checkbox::Partial), &rest[4..])
    } else {
        (None, rest)
    };

    // Check for description list: `term :: description`
    // Only for unordered lists
    if matches!(kind, ListKind::Unordered) && checkbox.is_none() {
        if let Some(sep_pos) = text_after_cb.find(" :: ") {
            let term = text_after_cb[..sep_pos].to_string();
            let desc = text_after_cb[sep_pos + 4..].to_string();
            return ListItem {
                indent,
                kind: ListKind::Description,
                checkbox: None,
                term: Some(term),
                text: desc,
            };
        }
    }

    ListItem {
        indent,
        kind,
        checkbox,
        term: None,
        text: text_after_cb.to_string(),
    }
}

/// Is this line a continuation of a list item? (indented text, not a new item, not blank)
fn is_continuation_line(line: &str, item_indent: usize) -> bool {
    if line.trim().is_empty() {
        return false;
    }
    let line_indent = line.len() - line.trim_start().len();
    // Continuation if indented more than the bullet
    if line_indent <= item_indent {
        return false;
    }
    // And not itself a new list item
    parse_list_line(line).is_none()
}

#[derive(Debug)]
enum BodyElement {
    Paragraph(String),
    List(Vec<ListItem>, Vec<Vec<BodyElement>>), // items + children per item
    HorizontalRule,
    SrcBlock { language: Option<String>, content: String },
    ExampleBlock(String),
    FixedWidth(String),
    Table { has_header: bool, header: Vec<Vec<String>>, body: Vec<Vec<String>> },
    GenericBlock { name: String, arg: String, content: String },
    CaptionedElement { caption: String, attr_html: Option<String>, inner: Box<BodyElement> },
}

fn is_table_separator(row: &str) -> bool {
    // A separator row looks like |---+---| or |---|
    let inner = row.trim().trim_start_matches('|').trim_end_matches('|');
    !inner.is_empty() && inner.chars().all(|c| c == '-' || c == '+' || c == ' ')
}

fn parse_table_row(row: &str) -> Vec<String> {
    let trimmed = row.trim();
    let inner = trimmed.trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(|cell| cell.trim().to_string()).collect()
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

        // Parse #+CAPTION: and #+ATTR_HTML: directives (attach to next element)
        let trimmed_up = trimmed.to_uppercase();
        if trimmed_up.starts_with("#+CAPTION:") {
            let caption = trimmed[10..].trim().to_string();
            pending_caption = Some(caption);
            i += 1;
            continue;
        }
        if trimmed_up.starts_with("#+ATTR_HTML:") {
            let attrs = trimmed[12..].trim().to_string();
            pending_attr_html = Some(attrs);
            i += 1;
            continue;
        }

        // Skip comment lines (# at start of line, with space or alone)
        if trimmed.starts_with("# ") || trimmed == "#" {
            i += 1;
            continue;
        }

        // Horizontal rule: 5+ dashes only
        if trimmed.len() >= 5 && trimmed.chars().all(|c| c == '-') {
            push_element(&mut elements, BodyElement::HorizontalRule, &mut pending_caption, &mut pending_attr_html);
            i += 1;
            continue;
        }

        // Check for #+BEGIN_SRC / #+BEGIN_EXAMPLE blocks
        let trimmed_upper = trimmed.to_uppercase();
        if trimmed_upper.starts_with("#+BEGIN_SRC") {
            let lang = trimmed[11..].trim().split_whitespace().next()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            i += 1;
            let mut content = String::new();
            while i < lines.len() {
                let l = lines[i];
                if l.trim().to_uppercase().starts_with("#+END_SRC") {
                    i += 1;
                    break;
                }
                content.push_str(l);
                content.push('\n');
                i += 1;
            }
            push_element(&mut elements, BodyElement::SrcBlock { language: lang, content }, &mut pending_caption, &mut pending_attr_html);
            continue;
        }

        if trimmed_upper.starts_with("#+BEGIN_EXAMPLE") {
            i += 1;
            let mut content = String::new();
            while i < lines.len() {
                let l = lines[i];
                if l.trim().to_uppercase().starts_with("#+END_EXAMPLE") {
                    i += 1;
                    break;
                }
                content.push_str(l);
                content.push('\n');
                i += 1;
            }
            push_element(&mut elements, BodyElement::ExampleBlock(content), &mut pending_caption, &mut pending_attr_html);
            continue;
        }

        // Generic #+BEGIN_xxx blocks (QUOTE, VERSE, CENTER, EXPORT, custom)
        if trimmed_upper.starts_with("#+BEGIN_") {
            let block_name_part = &trimmed[8..]; // after "#+BEGIN_"
            let block_name = block_name_part.split_whitespace().next().unwrap_or("").to_string();
            let block_arg = block_name_part[block_name.len()..].trim().to_string();
            let block_name_upper = block_name.to_uppercase();
            let end_tag = format!("#+END_{}", block_name_upper);
            i += 1;
            let mut content = String::new();
            while i < lines.len() {
                let l = lines[i];
                if l.trim().to_uppercase().starts_with(&end_tag) {
                    i += 1;
                    break;
                }
                content.push_str(l);
                content.push('\n');
                i += 1;
            }
            push_element(&mut elements, BodyElement::GenericBlock { name: block_name_upper, arg: block_arg, content }, &mut pending_caption, &mut pending_attr_html);
            continue;
        }

        // Check for fixed-width lines (: text)
        if trimmed.starts_with(": ") || trimmed == ":" {
            let mut content = String::new();
            while i < lines.len() {
                let lt = lines[i].trim();
                if lt.starts_with(": ") {
                    content.push_str(&lt[2..]);
                    content.push('\n');
                    i += 1;
                } else if lt == ":" {
                    content.push('\n');
                    i += 1;
                } else {
                    break;
                }
            }
            push_element(&mut elements, BodyElement::FixedWidth(content), &mut pending_caption, &mut pending_attr_html);
            continue;
        }

        // Check for table rows (lines starting with |)
        if trimmed.starts_with('|') {
            let mut rows: Vec<&str> = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with('|') {
                rows.push(lines[i].trim());
                i += 1;
            }
            // Parse into header/body
            let mut header_rows: Vec<Vec<String>> = Vec::new();
            let mut body_rows: Vec<Vec<String>> = Vec::new();
            let mut found_separator = false;
            for row in &rows {
                if is_table_separator(row) {
                    found_separator = true;
                    continue;
                }
                let cells = parse_table_row(row);
                if !found_separator {
                    header_rows.push(cells);
                } else {
                    body_rows.push(cells);
                }
            }
            if found_separator {
                push_element(&mut elements, BodyElement::Table { has_header: true, header: header_rows, body: body_rows }, &mut pending_caption, &mut pending_attr_html);
            } else {
                // No separator: all rows are body rows
                push_element(&mut elements, BodyElement::Table { has_header: false, header: Vec::new(), body: header_rows }, &mut pending_caption, &mut pending_attr_html);
            }
            continue;
        }

        // Check if this starts a list
        if let Some(mut item) = parse_list_line(line) {
            let list_indent = item.indent;
            let mut items: Vec<ListItem> = Vec::new();
            let mut children: Vec<Vec<BodyElement>> = Vec::new();

            loop {
                // Collect continuation lines for this item
                i += 1;
                let mut child_lines = String::new();
                while i < lines.len() {
                    let next = lines[i];
                    // If it's a new list item at the same or lesser indent, stop
                    if let Some(next_item) = parse_list_line(next) {
                        if next_item.indent <= list_indent {
                            break;
                        }
                        // It's a sub-list item — collect as child content
                        child_lines.push_str(next);
                        child_lines.push('\n');
                        i += 1;
                        continue;
                    }

                    if next.trim().is_empty() {
                        // A single blank line may separate continuation or end the list.
                        // Check if the *next* non-blank line is still part of this list.
                        // Two consecutive blanks end the list.
                        if i + 1 < lines.len() && lines[i + 1].trim().is_empty() {
                            // Two blank lines → end of list
                            break;
                        }
                        // Single blank — check if next non-blank is a list item at list level
                        let mut peek = i + 1;
                        while peek < lines.len() && lines[peek].trim().is_empty() {
                            peek += 1;
                        }
                        if peek < lines.len() {
                            if let Some(peek_item) = parse_list_line(lines[peek]) {
                                if peek_item.indent == list_indent {
                                    // Blank separating list items — skip blank, continue
                                    child_lines.push('\n');
                                    i += 1;
                                    continue;
                                }
                            }
                        }
                        break;
                    }

                    let next_indent = next.len() - next.trim_start().len();
                    if next_indent > list_indent {
                        // Continuation or sub-content
                        if is_continuation_line(next, list_indent) {
                            item.text.push(' ');
                            item.text.push_str(next.trim());
                        } else {
                            child_lines.push_str(next);
                            child_lines.push('\n');
                        }
                        i += 1;
                    } else {
                        break;
                    }
                }

                // Parse child elements recursively
                let child_elems = if child_lines.is_empty() {
                    Vec::new()
                } else {
                    parse_body_elements(&child_lines)
                };

                items.push(item);
                children.push(child_elems);

                // Check if next line is another list item at the same level
                if i < lines.len() {
                    if let Some(next_item) = parse_list_line(lines[i]) {
                        if next_item.indent == list_indent {
                            item = next_item;
                            continue;
                        }
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
                if l.trim().is_empty() {
                    break;
                }
                if parse_list_line(l).is_some() {
                    break;
                }
                // Stop at block beginnings
                let lt = l.trim().to_uppercase();
                if lt.starts_with("#+BEGIN_") {
                    break;
                }
                // Stop at fixed-width lines
                if l.trim().starts_with(": ") || l.trim() == ":" {
                    break;
                }
                // Stop at table rows
                if l.trim().starts_with('|') {
                    break;
                }
                // Stop at horizontal rules
                let lt2 = l.trim();
                if lt2.len() >= 5 && lt2.chars().all(|c| c == '-') {
                    break;
                }
                // Stop at comment lines
                if lt2.starts_with("# ") || lt2 == "#" {
                    break;
                }
                // Stop at org keywords (#+CAPTION:, #+ATTR_HTML:, etc.)
                if lt.starts_with("#+") {
                    break;
                }
                if !para.is_empty() {
                    para.push(' ');
                }
                para.push_str(l.trim());
                i += 1;
            }
            if !para.is_empty() {
                push_element(&mut elements, BodyElement::Paragraph(para), &mut pending_caption, &mut pending_attr_html);
            }
        }
    }

    elements
}

// ==================== Body Rendering ====================

fn render_body(html: &mut String, body: &str, id_map: &HashMap<String, String>) {
    let elements = parse_body_elements(body);
    for elem in &elements {
        render_body_element(html, elem, id_map);
    }
}

/// Like render_body but for text that already has footnote refs replaced as raw HTML.
/// We render inline markup but pass through the <sup> footnote tags.
fn render_body_raw(html: &mut String, body: &str, id_map: &HashMap<String, String>) {
    // The footnote replacement produces <sup>...</sup> tags in the text.
    // We need to render body elements but preserve those tags.
    // Strategy: render normally, then unescape the footnote tags.
    let elements = parse_body_elements(body);
    let mut temp = String::new();
    for elem in &elements {
        render_body_element(&mut temp, elem, id_map);
    }
    // Unescape footnote ref HTML that got escaped by render_inline_html
    let unescaped = temp
        .replace("&lt;sup&gt;&lt;a id=&quot;fnr-", "<sup><a id=\"fnr-")
        .replace("&quot; href=&quot;#fn-", "\" href=\"#fn-")
        .replace("&quot; class=&quot;footnote-ref&quot;&gt;", "\" class=\"footnote-ref\">")
        .replace("&lt;/a&gt;&lt;/sup&gt;", "</a></sup>");
    html.push_str(&unescaped);
}

fn render_body_element(
    html: &mut String,
    elem: &BodyElement,
    id_map: &HashMap<String, String>,
) {
    match elem {
        BodyElement::Paragraph(text) => {
            html.push_str("<p>");
            html.push_str(&render_inline_html(text, id_map));
            html.push_str("</p>\n");
        }
        BodyElement::CaptionedElement { caption, attr_html, inner } => {
            // Handle ATTR_HTML for images specifically
            if let BodyElement::Paragraph(text) = inner.as_ref() {
                // Check if this is a standalone image link that needs attr
                if attr_html.is_some() || !caption.is_empty() {
                    html.push_str("<figure>\n");
                    // Re-render inner with attr_html
                    if let Some(ref attrs) = attr_html {
                        // Try to inject attrs into img tag
                        let mut inner_html = String::new();
                        render_body_element(&mut inner_html, inner, id_map);
                        let injected = inject_attr_html(&inner_html, attrs);
                        html.push_str(&injected);
                    } else {
                        render_body_element(html, inner, id_map);
                    }
                    if !caption.is_empty() {
                        html.push_str(&format!("<figcaption>{}</figcaption>\n", render_inline_html(caption, id_map)));
                    }
                    html.push_str("</figure>\n");
                    return;
                }
            }

            // For tables, use <caption>
            if let BodyElement::Table { .. } = inner.as_ref() {
                if !caption.is_empty() {
                    // Render table with caption
                    html.push_str("<figure>\n");
                    render_body_element(html, inner, id_map);
                    html.push_str(&format!("<figcaption>{}</figcaption>\n", render_inline_html(caption, id_map)));
                    html.push_str("</figure>\n");
                    return;
                }
            }

            // Generic: wrap in figure
            html.push_str("<figure>\n");
            if let Some(ref attrs) = attr_html {
                let mut inner_html = String::new();
                render_body_element(&mut inner_html, inner, id_map);
                html.push_str(&inject_attr_html(&inner_html, attrs));
            } else {
                render_body_element(html, inner, id_map);
            }
            if !caption.is_empty() {
                html.push_str(&format!("<figcaption>{}</figcaption>\n", render_inline_html(caption, id_map)));
            }
            html.push_str("</figure>\n");
        }
        BodyElement::HorizontalRule => {
            html.push_str("<hr>\n");
        }
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
            match name.as_str() {
                "COMMENT" => {
                    // Comment blocks are excluded from export
                }
                "QUOTE" => {
                    html.push_str("<blockquote>\n");
                    // Render content as paragraphs with inline markup
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
                "EXPORT" => {
                    if arg.eq_ignore_ascii_case("html") {
                        // Raw HTML passthrough
                        html.push_str(content);
                    }
                    // Other export formats are silently ignored
                }
                _ => {
                    // Generic block → <div class="name">
                    let class_name = name.to_lowercase();
                    html.push_str(&format!("<div class=\"{}\">\n", escape_html(&class_name)));
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
        BodyElement::Table { has_header, header, body } => {
            html.push_str("<table>\n");
            if *has_header && !header.is_empty() {
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
            html.push_str("</tbody>\n");
            html.push_str("</table>\n");
        }
        BodyElement::List(items, children) => {
            if items.is_empty() {
                return;
            }
            // Determine list type from first item
            let list_tag = match items[0].kind {
                ListKind::Unordered => "ul",
                ListKind::Ordered => "ol",
                ListKind::Description => "dl",
            };

            html.push_str(&format!("<{}>\n", list_tag));

            for (idx, item) in items.iter().enumerate() {
                match item.kind {
                    ListKind::Description => {
                        html.push_str("<dt>");
                        if let Some(ref term) = item.term {
                            html.push_str(&render_inline_html(term, id_map));
                        }
                        html.push_str("</dt>\n<dd>");
                        html.push_str(&render_inline_html(&item.text, id_map));
                        // Render children
                        if let Some(child_elems) = children.get(idx) {
                            for child in child_elems {
                                render_body_element(html, child, id_map);
                            }
                        }
                        html.push_str("</dd>\n");
                    }
                    _ => {
                        html.push_str("<li>");
                        // Render checkbox
                        if let Some(ref cb) = item.checkbox {
                            match cb {
                                Checkbox::Unchecked => {
                                    html.push_str(
                                        "<input type=\"checkbox\" disabled> ",
                                    );
                                }
                                Checkbox::Checked => {
                                    html.push_str(
                                        "<input type=\"checkbox\" checked disabled> ",
                                    );
                                }
                                Checkbox::Partial => {
                                    html.push_str(
                                        "<input type=\"checkbox\" class=\"checkbox-partial\" disabled> ",
                                    );
                                }
                            }
                        }
                        html.push_str(&render_inline_html(&item.text, id_map));
                        // Render children
                        if let Some(child_elems) = children.get(idx) {
                            for child in child_elems {
                                html.push_str("\n");
                                render_body_element(html, child, id_map);
                            }
                        }
                        html.push_str("</li>\n");
                    }
                }
            }

            html.push_str(&format!("</{}>\n", list_tag));
        }
    }
}

// ==================== ID Index Building ====================

/// Build a mapping from org-id values to HTML file paths with anchors.
/// Scans all .org files in the given directory recursively.
/// Returns a map like: { "uuid-123" => "subdir/file.html#uuid-123" }
pub fn build_id_index(root: &Path) -> Result<HashMap<String, String>> {
    let files = find_org_files(root)?;
    let mut index = HashMap::new();

    for file_path in &files {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;
        let doc = parse_org_document(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", file_path.display(), e))?;

        // Compute the relative HTML path from root
        let rel_path = file_path
            .strip_prefix(root)
            .unwrap_or(file_path)
            .with_extension("html");
        let rel_str = rel_path.to_string_lossy().to_string();

        for entry in &doc.entries {
            if let Some(id) = entry.id() {
                let target = format!("{}#{}", rel_str, id);
                index.insert(id.to_string(), target);
            }
        }
    }

    Ok(index)
}

// ==================== Site Export ====================

/// Export all .org files in `src_dir` as HTML files to `out_dir`.
/// Resolves org-id links across the entire site.
/// Also generates an index.html listing all pages.
pub fn export_site(src_dir: &Path, out_dir: &Path) -> Result<()> {
    // Build the global ID index
    let id_map = build_id_index(src_dir)?;

    // Find all org files
    let files = find_org_files(src_dir)?;

    // Track pages for index generation
    let mut pages: Vec<(String, String)> = Vec::new(); // (relative html path, title)

    for file_path in &files {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;
        let doc = parse_org_document(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", file_path.display(), e))?;

        // Compute relative paths
        let rel_org = file_path.strip_prefix(src_dir).unwrap_or(file_path);
        let rel_html = rel_org.with_extension("html");
        let out_file = out_dir.join(&rel_html);

        // Compute the id_map relative to *this* file's location
        let file_dir = rel_html.parent().unwrap_or(Path::new(""));
        let relative_id_map = make_relative_id_map(&id_map, file_dir);

        // Render
        let html = render_html(&doc, &relative_id_map);

        // Ensure output directory exists
        if let Some(parent) = out_file.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&out_file, &html)
            .with_context(|| format!("Failed to write {}", out_file.display()))?;

        let title = doc
            .title()
            .unwrap_or_else(|| {
                rel_org
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
            })
            .to_string();
        pages.push((rel_html.to_string_lossy().to_string(), title));
    }

    // Generate index.html
    generate_index(out_dir, &pages)?;

    Ok(())
}

/// Make ID map paths relative to a given file's directory.
fn make_relative_id_map(
    id_map: &HashMap<String, String>,
    file_dir: &Path,
) -> HashMap<String, String> {
    let mut relative = HashMap::new();
    for (id, target) in id_map {
        let target_path = PathBuf::from(target.split('#').next().unwrap_or(""));
        let anchor = target.split('#').nth(1).unwrap_or("");

        // Compute relative path from file_dir to target
        let rel = make_relative_path(file_dir, &target_path);
        let rel_str = if anchor.is_empty() {
            rel
        } else {
            format!("{}#{}", rel, anchor)
        };
        relative.insert(id.clone(), rel_str);
    }
    relative
}

/// Compute a relative path from `from_dir` to `to_file`.
fn make_relative_path(from_dir: &Path, to_file: &Path) -> String {
    // Both paths are relative to the site root
    let from_components: Vec<_> = from_dir.components().collect();
    let to_components: Vec<_> = to_file.components().collect();

    // Find common prefix
    let common = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let ups = from_components.len() - common;
    let mut result = String::new();
    for _ in 0..ups {
        result.push_str("../");
    }
    let remaining: Vec<_> = to_components[common..]
        .iter()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    result.push_str(&remaining.join("/"));

    if result.is_empty() {
        // Same directory — to_file should just be the filename
        to_file
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        result
    }
}

fn generate_index(out_dir: &Path, pages: &[(String, String)]) -> Result<()> {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("<title>Index</title>\n");
    html.push_str("<style>");
    html.push_str(default_css());
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");
    html.push_str("<h1>Index</h1>\n");
    html.push_str("<ul class=\"index-list\">\n");

    let mut sorted_pages = pages.to_vec();
    sorted_pages.sort_by(|a, b| a.1.cmp(&b.1));

    for (path, title) in &sorted_pages {
        html.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>\n",
            escape_html(path),
            escape_html(title)
        ));
    }

    html.push_str("</ul>\n");
    html.push_str("</body>\n</html>\n");

    fs::write(out_dir.join("index.html"), &html)?;
    Ok(())
}

/// Export a single `.org` file to HTML in `out_dir`.
/// The output filename mirrors the source filename with `.html` extension.
pub fn export_file(src: &Path, out_dir: &Path) -> Result<()> {
    // Build a minimal id_map scoped to the file's parent directory
    let parent = src.parent().unwrap_or(Path::new("."));
    let id_map = build_id_index(parent).unwrap_or_default();

    let content = fs::read_to_string(src)
        .with_context(|| format!("Failed to read {}", src.display()))?;
    let doc = parse_org_document(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", src.display(), e))?;

    let html = render_html(&doc, &id_map);

    let out_name = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let out_path = out_dir.join(format!("{}.html", out_name));
    fs::create_dir_all(out_dir)?;
    fs::write(&out_path, &html)
        .with_context(|| format!("Failed to write {}", out_path.display()))?;

    Ok(())
}
