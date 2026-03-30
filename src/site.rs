//! Site-level HTML operations:
//!   - Building the global org-id → HTML-path index
//!   - Exporting all .org files in a directory tree to HTML
//!   - Generating an `index.html` listing all pages
//!   - Low-level `export_file` / `export_file_with_map`

use crate::normalise::collect_org_files_recursive;
use crate::parser::parse_org_document;
use crate::render::{default_css, escape_html, render_html, render_html_opts, resolve_page_title, RenderOptions};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ==================== ID index ====================

/// Build a mapping from org-id values to HTML file paths with fragment anchors.
///
/// Scans all `.org` files under `root` recursively (skipping Emacs lock files).
/// Returns entries like `{ "uuid-123" => "subdir/file.html#uuid-123" }`.
pub fn build_id_index(root: &Path) -> Result<HashMap<String, String>> {
    let files = collect_org_files_recursive(root)?;
    let mut index = HashMap::new();

    for file_path in &files {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;
        let doc = parse_org_document(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", file_path.display(), e))?;

        let rel_path = file_path
            .strip_prefix(root)
            .unwrap_or(file_path)
            .with_extension("html");
        let rel_str = rel_path.to_string_lossy().to_string();

        for entry in &doc.entries {
            if let Some(id) = entry.id() {
                index.insert(id.to_string(), format!("{}#{}", rel_str, id));
            }
        }
    }
    Ok(index)
}

// ==================== Full site export (used by `export` command) ====================

/// Export every `.org` file under `src_dir` to HTML files in `out_dir`,
/// resolving org-id links across the whole site.  Also generates `index.html`.
pub fn export_site(src_dir: &Path, out_dir: &Path) -> Result<()> {
    let id_map = build_id_index(src_dir)?;
    let files = collect_org_files_recursive(src_dir)?;
    let mut pages: Vec<(String, String)> = Vec::new();

    for file_path in &files {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;
        let doc = parse_org_document(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", file_path.display(), e))?;

        let rel_org = file_path.strip_prefix(src_dir).unwrap_or(file_path);
        let rel_html = rel_org.with_extension("html");
        let out_file = out_dir.join(&rel_html);

        let file_dir = rel_html.parent().unwrap_or(Path::new(""));
        let relative_id_map = make_relative_id_map(&id_map, file_dir);

        let html = render_html(&doc, &relative_id_map, None);
        if let Some(parent) = out_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&out_file, &html)
            .with_context(|| format!("Failed to write {}", out_file.display()))?;

        pages.push((rel_html.to_string_lossy().to_string(), resolve_page_title(&doc).to_string()));
    }

    generate_index(out_dir, &pages)
}

// ==================== Index generation ====================

/// Generate `index.html` from the list of already-exported source files.
/// Called by the `build` command after the export loop.
pub fn generate_site_index(out_dir: &Path, source_files: &[PathBuf], site_title: &str) -> Result<()> {
    let mut pages: Vec<(String, String)> = Vec::new();
    for src in source_files {
        let fname = src.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let html_name = fname.replace(".org", ".html");
        if !out_dir.join(&html_name).exists() {
            continue;
        }
        let title = fs::read_to_string(src)
            .ok()
            .and_then(|c| parse_org_document(&c).ok())
            .map(|doc| resolve_page_title(&doc).to_string())
            .unwrap_or_else(|| site_title.to_string());
        pages.push((html_name, title));
    }
    generate_index(out_dir, &pages)
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
    html.push_str("</head>\n<body>\n<h1>Index</h1>\n<ul class=\"index-list\">\n");

    let mut sorted = pages.to_vec();
    sorted.sort_by(|a, b| a.1.cmp(&b.1));
    for (path, title) in &sorted {
        html.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>\n",
            escape_html(path), escape_html(title)
        ));
    }
    html.push_str("</ul>\n</body>\n</html>\n");
    fs::write(out_dir.join("index.html"), &html)?;
    Ok(())
}

// ==================== Single-file export ====================

/// Export a single `.org` file to HTML in `out_dir`.
/// Builds a minimal ID map scoped to the file's parent directory.
#[allow(dead_code)]
pub fn export_file(src: &Path, out_dir: &Path) -> Result<()> {
    let parent = src.parent().unwrap_or(Path::new("."));
    let id_map = build_id_index(parent).unwrap_or_default();
    export_file_with_map(src, out_dir, &id_map, &RenderOptions::none())
}

/// Export a single `.org` file to HTML using a pre-built ID map and render options.
pub fn export_file_with_map(
    src: &Path,
    out_dir: &Path,
    id_map: &HashMap<String, String>,
    render_opts: &RenderOptions,
) -> Result<()> {
    let content = fs::read_to_string(src)
        .with_context(|| format!("Failed to read {}", src.display()))?;
    let doc = parse_org_document(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", src.display(), e))?;

    let html = render_html_opts(&doc, id_map, render_opts);

    let out_name = src.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let out_path = out_dir.join(format!("{}.html", out_name));
    fs::create_dir_all(out_dir)?;
    fs::write(&out_path, &html)
        .with_context(|| format!("Failed to write {}", out_path.display()))?;
    Ok(())
}

// ==================== Relative path helpers ====================

fn make_relative_id_map(
    id_map: &HashMap<String, String>,
    file_dir: &Path,
) -> HashMap<String, String> {
    let mut relative = HashMap::new();
    for (id, target) in id_map {
        let target_path = PathBuf::from(target.split('#').next().unwrap_or(""));
        let anchor = target.split('#').nth(1).unwrap_or("");
        let rel = make_relative_path(file_dir, &target_path);
        let rel_str = if anchor.is_empty() { rel } else { format!("{}#{}", rel, anchor) };
        relative.insert(id.clone(), rel_str);
    }
    relative
}

fn make_relative_path(from_dir: &Path, to_file: &Path) -> String {
    let from_components: Vec<_> = from_dir.components().collect();
    let to_components: Vec<_> = to_file.components().collect();
    let common = from_components.iter().zip(to_components.iter())
        .take_while(|(a, b)| a == b).count();
    let ups = from_components.len() - common;
    let mut result = "../".repeat(ups);
    let remaining: Vec<_> = to_components[common..]
        .iter().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    result.push_str(&remaining.join("/"));
    if result.is_empty() {
        to_file.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()
    } else {
        result
    }
}
