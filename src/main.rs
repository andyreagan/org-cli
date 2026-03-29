use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod html;
mod parser;
mod types;
mod config;
mod normalise;
mod blog;
mod postprocess;

use commands::*;
use config::{Config, ScrubRules};

#[derive(Parser)]
#[clap(name = "org-cli")]
#[clap(about = "CLI for interacting with org-mode files")]
#[clap(version)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all TODO/NEXT/WAITING items across files, grouped by keyword
    List {
        #[clap(default_value = ".")]
        path: PathBuf,
    },

    /// Add a new TODO item
    Add {
        text: String,
        #[clap(long, short)]
        file: PathBuf,
        #[clap(long, short)]
        tag: Option<String>,
    },

    /// Mark an item as DONE
    Done {
        file: PathBuf,
        line: usize,
    },

    /// Mark an item as CANCELLED
    Cancel {
        file: PathBuf,
        line: usize,
    },

    /// Mark an item as WAITING
    Wait {
        file: PathBuf,
        line: usize,
        #[clap(long, short)]
        date: Option<String>,
    },

    /// Reschedule an item to a new date
    Reschedule {
        file: PathBuf,
        line: usize,
        #[clap(long, short)]
        date: String,
    },

    /// Pretty-print headings and TODOs in a file
    Show {
        file: PathBuf,
    },

    /// Export org files as HTML site with resolved org-id links
    Export {
        path: PathBuf,
        #[clap(long, short)]
        output: PathBuf,
    },

    /// Normalise .org source files in-place (flatten nested id: links,
    /// consolidate :BACKLINKS: drawers). Safe to run repeatedly.
    Normalise {
        /// Directory of .org files to normalise (default: current directory)
        #[clap(default_value = ".")]
        dir: PathBuf,

        /// Print what would change without writing any files
        #[clap(long)]
        dry_run: bool,
    },

    /// Full site build: normalise → blog index → export HTML → post-process
    Build {
        /// Project root containing org-cli.toml (default: current directory)
        #[clap(default_value = ".")]
        dir: PathBuf,

        /// Path to config file (default: <dir>/org-cli.toml)
        #[clap(long)]
        config: Option<PathBuf>,

        /// Force a full rebuild, ignoring the incremental cache
        #[clap(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { path } => list_todos(&path)?,
        Commands::Add { text, file, tag } => add_todo(&text, &file, tag.as_deref())?,
        Commands::Done { file, line } => mark_done(&file, line)?,
        Commands::Cancel { file, line } => mark_cancelled(&file, line)?,
        Commands::Wait { file, line, date } => mark_waiting(&file, line, date.as_deref())?,
        Commands::Reschedule { file, line, date } => reschedule(&file, line, &date)?,
        Commands::Show { file } => show_file(&file)?,

        Commands::Export { path, output } => {
            html::export_site(&path, &output)?;
            println!("Exported to {}", output.display());
        }

        Commands::Normalise { dir, dry_run } => {
            let modified = normalise::normalise_dir(&dir, dry_run)?;
            if modified.is_empty() {
                println!("Nothing to normalise.");
            } else {
                for p in &modified {
                    println!("{} {}", if dry_run { "would modify" } else { "modified" }, p.display());
                }
                println!("{} file(s) {}.", modified.len(), if dry_run { "would be modified" } else { "modified" });
            }
        }

        Commands::Build { dir, config: config_path, force } => {
            run_build(&dir, config_path.as_deref(), force)?;
        }
    }

    Ok(())
}

fn run_build(dir: &std::path::Path, config_path: Option<&std::path::Path>, force: bool) -> Result<()> {
    // Load config
    let cfg_dir = config_path
        .and_then(|p| p.parent())
        .unwrap_or(dir);
    let config = Config::load(cfg_dir)?;
    let output_dir = config.resolved_output(dir);

    println!("=== Build: {} → {} ===", dir.display(), output_dir.display());

    // Stage 1 — normalise
    println!("--- Stage 1: normalise ---");
    let modified = normalise::normalise_dir(dir, false)?;
    println!("  normalised {} file(s)", modified.len());

    // Stage 2a — blog index + tag pages + nav injection
    if config.blog.enabled {
        println!("--- Stage 2a: blog index + tags + nav ---");
        let posts = blog::build_blog(dir, &config.blog)?;
        println!("  {} posts indexed", posts.len());
    }

    // Stage 2b — HTML export
    println!("--- Stage 2b: org → HTML ---");
    std::fs::create_dir_all(&output_dir)?;

    // Collect source files (skipping lock files via collect_org_files)
    let source_files = normalise::collect_org_files(dir)?;
    let cache_path = output_dir.join(".org-cli-cache.json");
    let mut cache = load_cache(&cache_path);

    let mut exported = 0usize;
    let mut skipped = 0usize;

    for src in &source_files {
        let fname = src.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();

        // Privacy gate (#4): skip whole-file private pages
        let raw = std::fs::read_to_string(src)?;
        if raw.to_lowercase().contains("#+private: true") {
            // Copy placeholder HTML instead of exporting
            let out_name = fname.replace(".org", ".html");
            let out_path = output_dir.join(&out_name);
            let placeholder = dir.join(&config.site.private_placeholder);
            if placeholder.exists() {
                std::fs::copy(&placeholder, &out_path)?;
            } else {
                std::fs::write(&out_path, "<html><body><p>Private page.</p></body></html>")?;
            }
            skipped += 1;
            continue;
        }

        // Incremental check (#14)
        let mtime = src.metadata().ok().and_then(|m| {
            m.modified().ok().and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
            })
        }).unwrap_or(0);
        let out_path = output_dir.join(fname.replace(".org", ".html"));
        if !force && out_path.exists() {
            if let Some(cached_mtime) = cache.get(&fname) {
                if *cached_mtime == mtime {
                    skipped += 1;
                    continue;
                }
            }
        }

        // Strip #+BEGIN_PRIVATE / #+END_PRIVATE blocks (#5) before export
        let clean_content = strip_private_blocks(&raw);

        // Write a temporary cleaned file for the exporter to pick up,
        // then export it, then remove the temp file.
        let tmp_path = src.with_extension("_build_tmp.org");
        std::fs::write(&tmp_path, &clean_content)?;

        match html::export_file(&tmp_path, &output_dir) {
            Ok(_) => {
                // Rename output from tmp name to real name
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

    // Copy static dirs
    for sd in &config.site.static_dirs {
        let src_dir = dir.join(sd);
        if src_dir.is_dir() {
            let dst_dir = output_dir.join(sd);
            copy_dir_recursive(&src_dir, &dst_dir)?;
        }
    }

    // Inject <title> tags (#10) — post-export pass over HTML files
    println!("--- Stage 2c: <title> injection ---");
    inject_titles(&output_dir, &source_files, &config.site.title, dir)?;

    // Stage 3 — post-process
    println!("--- Stage 3: post-process ---");
    let scrub_rules = if config.scrub.enabled {
        let rules_path = dir.join(&config.scrub.rules_file);
        ScrubRules::load(&rules_path)?
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

// ==================== Helpers ====================

/// Strip `#+BEGIN_PRIVATE` … `#+END_PRIVATE` blocks from org source (#5).
fn strip_private_blocks(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut in_private = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.to_uppercase().starts_with("#+BEGIN_PRIVATE") {
            in_private = true;
            continue;
        }
        if trimmed.to_uppercase().starts_with("#+END_PRIVATE") {
            in_private = false;
            continue;
        }
        if !in_private {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Inject a `<title>` into every HTML file that lacks one (#10).
fn inject_titles(
    output_dir: &std::path::Path,
    source_files: &[PathBuf],
    site_title: &str,
    _source_dir: &std::path::Path,
) -> Result<()> {
    // Build a map from html filename → title (from #+TITLE: or first heading)
    let mut title_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for src in source_files {
        let fname = src.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let html_name = fname.replace(".org", ".html");
        if let Ok(content) = std::fs::read_to_string(src) {
            let title = extract_org_title(&content, site_title);
            title_map.insert(html_name, title);
        }
    }

    for entry in std::fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "html") {
            let html = std::fs::read_to_string(&path)?;
            if html.contains("<title>") {
                continue;
            }
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let title = title_map.get(&fname).map(String::as_str).unwrap_or(site_title);
            let injected = html.replacen("<head>", &format!("<head><title>{}</title>", title), 1);
            if injected != html {
                std::fs::write(&path, injected)?;
            }
        }
    }
    Ok(())
}

fn extract_org_title(content: &str, fallback: &str) -> String {
    // 1. #+TITLE:
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("#+TITLE:").or_else(|| line.strip_prefix("#+title:")) {
            let t = rest.trim().to_string();
            if !t.is_empty() { return t; }
        }
    }
    // 2. First heading
    for line in content.lines() {
        if line.starts_with("* ") {
            let heading = line[2..].trim();
            // Strip trailing tags
            if let Some(tag_start) = heading.rfind("   :") {
                return heading[..tag_start].trim().to_string();
            }
            if !heading.is_empty() { return heading.to_string(); }
        }
    }
    // 3. Site default
    fallback.to_string()
}

// ==================== Incremental cache (#14) ====================

fn load_cache(path: &std::path::Path) -> std::collections::HashMap<String, u64> {
    if !path.exists() {
        return std::collections::HashMap::new();
    }
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    // Very simple JSON: {"file.org": 1234567890, ...}
    let mut map = std::collections::HashMap::new();
    for line in raw.lines() {
        let line = line.trim().trim_matches(['{', '}', ','].as_ref());
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim().trim_matches('"').to_string();
            let v: u64 = v.trim().parse().unwrap_or(0);
            if !k.is_empty() {
                map.insert(k, v);
            }
        }
    }
    map
}

fn save_cache(path: &std::path::Path, cache: &std::collections::HashMap<String, u64>) -> Result<()> {
    let mut entries: Vec<String> = cache
        .iter()
        .map(|(k, v)| format!("  \"{}\": {}", k, v))
        .collect();
    entries.sort();
    let json = format!("{{\n{}\n}}", entries.join(",\n"));
    std::fs::write(path, json)?;
    Ok(())
}

// ==================== Directory copy ====================

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
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
