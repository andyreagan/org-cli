//! The `build` command pipeline:
//!   Stage 1 — normalise source files
//!   Stage 2a — blog index + tag pages + nav injection
//!   Stage 2b — org → HTML export (incremental)
//!   Stage 2c — title injection for non-org-cli HTML files
//!   Stage 3  — post-process (path stripping, scrubbing, images)

use crate::blog;
use crate::config::{Config, ScrubRules};
use crate::normalise::collect_org_files;
use crate::postprocess;
use crate::render::RenderOptions;
use crate::site::{build_id_index, export_file_with_map, generate_site_index};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ==================== Entry point ====================

pub fn run_build(
    dir: &Path,
    config_path: Option<&Path>,
    force: bool,
) -> Result<()> {
    let cfg_dir = config_path.and_then(|p| p.parent()).unwrap_or(dir);
    let config = Config::load(cfg_dir)?;
    let output_dir = config.resolved_output(dir);

    println!("=== Build: {} → {} ===", dir.display(), output_dir.display());

    // Stage 1 — normalise
    println!("--- Stage 1: normalise ---");
    let modified = crate::normalise::normalise_dir(dir, false)?;
    println!("  normalised {} file(s)", modified.len());

    // Stage 2a — blog index + tags + nav
    if config.blog.enabled {
        println!("--- Stage 2a: blog index + tags + nav ---");
        let posts = blog::build_blog(dir, &config.blog)?;
        println!("  {} posts indexed", posts.len());
    }

    // Stage 2b — org → HTML
    println!("--- Stage 2b: org → HTML ---");
    std::fs::create_dir_all(&output_dir)?;

    let source_files = collect_org_files(dir)?;
    let id_map = build_id_index(dir)?;

    let preamble  = load_optional_file(dir, &config.site.preamble,   "preamble")?;
    let head      = load_optional_file(dir, &config.site.head,       "head")?;
    let head_extra = load_optional_file(dir, &config.site.head_extra, "head_extra")?;
    let render_opts = RenderOptions {
        preamble:   preamble.as_deref(),
        head:       head.as_deref(),
        head_extra: head_extra.as_deref(),
    };

    let cache_path = output_dir.join(".org-cli-cache.json");
    let mut cache = load_cache(&cache_path);

    let mut exported = 0usize;
    let mut skipped  = 0usize;

    for src in &source_files {
        let fname = src.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let raw = std::fs::read_to_string(src)?;

        // Privacy gate: skip whole-file private pages
        if raw.to_lowercase().contains("#+private: true") {
            let out_path = output_dir.join(fname.replace(".org", ".html"));
            let placeholder = dir.join(&config.site.private_placeholder);
            if placeholder.exists() {
                std::fs::copy(&placeholder, &out_path)?;
            } else {
                std::fs::write(&out_path, "<html><body><p>Private page.</p></body></html>")?;
            }
            skipped += 1;
            continue;
        }

        // Incremental check
        let mtime = src.metadata().ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let out_path = output_dir.join(fname.replace(".org", ".html"));
        if !force && out_path.exists() {
            if let Some(&cached) = cache.get(&fname) {
                if cached == mtime { skipped += 1; continue; }
            }
        }

        // Strip #+BEGIN_PRIVATE … #+END_PRIVATE blocks before export
        let clean = strip_private_blocks(&raw);
        let tmp_path = src.with_extension("_build_tmp.org");
        std::fs::write(&tmp_path, &clean)?;

        match export_file_with_map(&tmp_path, &output_dir, &id_map, &render_opts) {
            Ok(_) => {
                let tmp_out = output_dir.join(
                    tmp_path.file_name().unwrap().to_str().unwrap().replace(".org", ".html")
                );
                if tmp_out.exists() {
                    std::fs::rename(&tmp_out, &out_path)?;
                }
                cache.insert(fname, mtime);
                exported += 1;
            }
            Err(e) => eprintln!("Warning: export failed for {}: {}", src.display(), e),
        }
        let _ = std::fs::remove_file(&tmp_path);
    }

    save_cache(&cache_path, &cache)?;
    println!("  exported {}, skipped {} (cached)", exported, skipped);

    // Generate index.html
    generate_site_index(&output_dir, &source_files, &config.site.title)?;

    // Copy static dirs
    for sd in &config.site.static_dirs {
        let src_dir = dir.join(sd);
        if src_dir.is_dir() {
            copy_dir_recursive(&src_dir, &output_dir.join(sd))?;
        }
    }

    // Copy root_files (can override the generated index.html)
    for rf in &config.site.root_files {
        let src_path = dir.join(rf);
        if src_path.exists() {
            let dest = output_dir.join(src_path.file_name().unwrap_or_default());
            std::fs::copy(&src_path, &dest)?;
        } else {
            eprintln!("Warning: root_file not found: {}", src_path.display());
        }
    }

    // Stage 2c — title injection (for any HTML files that lack <title>)
    println!("--- Stage 2c: <title> injection ---");
    inject_titles(&output_dir, &source_files, &config.site.title)?;

    // Stage 3 — post-process
    println!("--- Stage 3: post-process ---");
    let scrub_rules = if config.scrub.enabled {
        ScrubRules::load(&dir.join(&config.scrub.rules_file))?
    } else {
        ScrubRules::default()
    };
    postprocess::postprocess_dir(
        &output_dir,
        &config.site.strip_path_prefix,
        &config.scrub,
        &scrub_rules,
        &config.images,
    )?;

    println!("=== Build complete → {} ===", output_dir.display());
    Ok(())
}

// ==================== Private helpers ====================

/// Strip `#+BEGIN_PRIVATE` … `#+END_PRIVATE` blocks from org source.
fn strip_private_blocks(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut in_private = false;
    for line in content.lines() {
        let t = line.trim().to_uppercase();
        if t.starts_with("#+BEGIN_PRIVATE") { in_private = true; continue; }
        if t.starts_with("#+END_PRIVATE")   { in_private = false; continue; }
        if !in_private { out.push_str(line); out.push('\n'); }
    }
    out
}

/// Inject a `<title>` into every HTML file in `output_dir` that lacks one.
fn inject_titles(
    output_dir: &Path,
    source_files: &[PathBuf],
    site_title: &str,
) -> Result<()> {
    // Build a map from html filename → title
    let mut title_map: HashMap<String, String> = HashMap::new();
    for src in source_files {
        let fname = src.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if let Ok(content) = std::fs::read_to_string(src) {
            title_map.insert(fname.replace(".org", ".html"), extract_title(&content, site_title));
        }
    }

    for entry in std::fs::read_dir(output_dir)? {
        let path = entry?.path();
        if path.extension().map_or(false, |e| e == "html") {
            let html = std::fs::read_to_string(&path)?;
            if html.contains("<title>") { continue; }
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let title = title_map.get(&fname).map(String::as_str).unwrap_or(site_title);
            let injected = html.replacen("<head>", &format!("<head><title>{}</title>", title), 1);
            if injected != html { std::fs::write(&path, injected)?; }
        }
    }
    Ok(())
}

fn extract_title(content: &str, fallback: &str) -> String {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("#+TITLE:").or_else(|| line.strip_prefix("#+title:")) {
            let t = rest.trim().to_string();
            if !t.is_empty() { return t; }
        }
    }
    for line in content.lines() {
        if line.starts_with("* ") {
            let heading = line[2..].trim();
            if let Some(tag_start) = heading.rfind("   :") {
                return heading[..tag_start].trim().to_string();
            }
            if !heading.is_empty() { return heading.to_string(); }
        }
    }
    fallback.to_string()
}

// ==================== Incremental cache ====================

fn load_cache(path: &Path) -> HashMap<String, u64> {
    if !path.exists() { return HashMap::new(); }
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_cache(path: &Path, cache: &HashMap<String, u64>) -> Result<()> {
    let json = serde_json::to_string_pretty(cache)
        .context("Failed to serialise build cache")?;
    std::fs::write(path, json)?;
    Ok(())
}

// ==================== File loading / directory copy ====================

/// Load an optional HTML snippet file relative to `dir`.
pub fn load_optional_file(dir: &Path, rel: &str, label: &str) -> Result<Option<String>> {
    if rel.is_empty() { return Ok(None); }
    let path = dir.join(rel);
    if path.exists() {
        Ok(Some(std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}: {}", label, path.display()))?))
    } else {
        eprintln!("Warning: {} file not found: {}", label, path.display());
        Ok(None)
    }
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
