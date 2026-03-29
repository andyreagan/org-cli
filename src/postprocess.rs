/// Stage 3 — HTML output post-processing.
///
/// All functions operate on files in the output directory only.
/// They never touch source `.org` files.
///
/// Covers issues:
///   #6  — remove HTML elements with class hidden/HIDDEN/private/PRIVATE
///   #7  — personal information scrubbing via scrub.toml rules
///   #8  — image pipeline (resize / greyscale / grain via ImageMagick)
///   #9  — strip absolute local path prefix from href/src attributes

use crate::config::{ImagesConfig, ScrubCategory, ScrubConfig, ScrubRules};
use anyhow::Result;
use std::path::{Path, PathBuf};

// ==================== #9 — path prefix stripping ====================

/// Strip `prefix` from all `href="..."` and `src="..."` attribute values
/// in `html`. Only attribute values are affected, not visible text.
pub fn strip_path_prefix(html: &str, prefix: &str) -> (String, bool) {
    if prefix.is_empty() {
        return (html.to_string(), false);
    }
    let mut changed = false;
    // We rewrite attribute values only: href="PREFIX..." and src="PREFIX..."
    let mut out = String::with_capacity(html.len());
    let mut remaining = html;

    for attr in &["href=\"", "src=\""] {
        let _ = attr; // iterated below
    }

    // Simple state-machine: scan for href=" or src=" then rewrite value
    while !remaining.is_empty() {
        // Find earliest of href=" or src="
        let href_pos = remaining.find("href=\"").map(|p| (p, 6usize));
        let src_pos = remaining.find("src=\"").map(|p| (p, 5usize));

        let found = match (href_pos, src_pos) {
            (Some(h), Some(s)) => Some(if h.0 <= s.0 { h } else { s }),
            (Some(h), None) => Some(h),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        };

        let Some((pos, attr_len)) = found else {
            out.push_str(remaining);
            break;
        };

        out.push_str(&remaining[..pos + attr_len]);
        remaining = &remaining[pos + attr_len..];

        // Now we're just after the opening quote — find the closing quote
        let Some(close) = remaining.find('"') else {
            out.push_str(remaining);
            break;
        };
        let value = &remaining[..close];

        // Strip prefix if present
        let new_value = if value.contains(prefix) {
            changed = true;
            value.replace(prefix, "")
        } else {
            value.to_string()
        };

        out.push_str(&new_value);
        remaining = &remaining[close..]; // leave the closing quote for next iteration
    }

    (out, changed)
}

// ==================== #6 — hidden/private class redaction ====================

const REDACT_CLASSES: &[&str] = &["hidden", "HIDDEN", "private", "PRIVATE"];

/// Remove all HTML elements (and their subtrees) whose `class` attribute
/// contains one of the redact classes.
///
/// Uses a simple bracket-counting approach — sufficient for well-formed
/// org-exported HTML which has no unexpected nesting.
pub fn redact_private_elements(html: &str) -> (String, bool) {
    let mut changed = false;
    let mut out = String::with_capacity(html.len());
    let mut remaining = html;

    while !remaining.is_empty() {
        // Look for an opening tag that has a class we want to remove
        if let Some((tag_start, tag_end, _tag_name)) = find_redactable_open_tag(remaining) {
            out.push_str(&remaining[..tag_start]);

            // Scan forward to find the matching closing tag
            let tag_content = &remaining[tag_start..];
            if let Some(skip_end) = find_element_end(tag_content) {
                remaining = &remaining[tag_start + skip_end..];
                changed = true;
            } else {
                // Can't find end — skip past the opening tag to avoid infinite loop
                out.push_str(&remaining[tag_start..tag_end]);
                remaining = &remaining[tag_end..];
            }
        } else {
            out.push_str(remaining);
            break;
        }
    }

    (out, changed)
}

/// Find the first opening tag in `s` whose class attribute contains a
/// redact class.  Returns `(start, end_of_open_tag, tag_name)`.
fn find_redactable_open_tag(s: &str) -> Option<(usize, usize, String)> {
    let mut search = s;
    let mut offset = 0usize;

    while let Some(lt) = search.find('<') {
        let after_lt = &search[lt + 1..];

        // Skip closing tags and comments
        if after_lt.starts_with('/') || after_lt.starts_with('!') {
            offset += lt + 1;
            search = &search[lt + 1..];
            continue;
        }

        // Find the end of this tag
        let Some(gt) = after_lt.find('>') else {
            break;
        };
        let tag_body = &after_lt[..gt]; // e.g. `div class="HIDDEN" id="x"`

        // Extract tag name
        let tag_name = tag_body
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches('/')
            .to_string();

        // Check class attribute
        if has_redact_class(tag_body) {
            let abs_start = offset + lt;
            let abs_end = abs_start + 1 + gt + 1;
            return Some((abs_start, abs_end, tag_name));
        }

        offset += lt + 1;
        search = &search[lt + 1..];
    }
    None
}

fn has_redact_class(tag_body: &str) -> bool {
    // Find class="..." value
    if let Some(ci) = tag_body.to_lowercase().find("class=") {
        let rest = &tag_body[ci + 6..];
        let value = if rest.starts_with('"') {
            let inner = &rest[1..];
            inner.split('"').next().unwrap_or("")
        } else if rest.starts_with('\'') {
            let inner = &rest[1..];
            inner.split('\'').next().unwrap_or("")
        } else {
            rest.split_whitespace().next().unwrap_or("")
        };
        for cls in value.split_whitespace() {
            if REDACT_CLASSES.contains(&cls) {
                return true;
            }
        }
    }
    false
}

/// Given a string starting at an opening tag `<tag ...>`, find the byte
/// offset just past the matching closing `</tag>`.
fn find_element_end(s: &str) -> Option<usize> {
    // Extract tag name from opening tag
    let after_lt = s.strip_prefix('<')?;
    let tag_name = after_lt
        .split(|c: char| c.is_whitespace() || c == '>')
        .next()?
        .to_lowercase();
    let tag_name = tag_name.trim_end_matches('/');

    let open_pat = format!("<{}", tag_name);
    let close_pat = format!("</{}", tag_name);

    let mut depth = 0usize;
    let mut i = 0usize;
    let bytes = s.as_bytes();

    while i < bytes.len() {
        if s[i..].to_lowercase().starts_with(&open_pat) {
            depth += 1;
            i += open_pat.len();
        } else if s[i..].to_lowercase().starts_with(&close_pat) {
            depth -= 1;
            if depth == 0 {
                // Skip past the closing ">"
                if let Some(gt) = s[i..].find('>') {
                    return Some(i + gt + 1);
                }
                return None;
            }
            i += close_pat.len();
        } else {
            i += 1;
        }
    }
    None
}

// ==================== #7 — personal information scrubbing ====================

/// Expand a single `ScrubRule` into all the string pairs that should be
/// substituted (handles case variants for towns, phone format variants, etc.).
pub fn expand_rule(rule: &crate::config::ScrubRule) -> Vec<(String, String)> {
    let r = &rule.real;
    let f = &rule.fake;
    match rule.category {
        ScrubCategory::Town | ScrubCategory::Address | ScrubCategory::Carrier => {
            // Four case variants
            vec![
                (r.clone(), f.clone()),
                (r.to_uppercase(), f.to_uppercase()),
                (r.to_lowercase(), f.to_lowercase()),
                (title_case(r), title_case(f)),
            ]
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
        }
        ScrubCategory::Phone => {
            // Raw digits + two formatted variants
            let digits_r = r.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
            let digits_f = f.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
            if digits_r.len() == 10 && digits_f.len() == 10 {
                vec![
                    (digits_r.clone(), digits_f.clone()),
                    (
                        format!("({}) {}-{}", &digits_r[..3], &digits_r[3..6], &digits_r[6..]),
                        format!("({}) {}-{}", &digits_f[..3], &digits_f[3..6], &digits_f[6..]),
                    ),
                    (
                        format!("{}-{}-{}", &digits_r[..3], &digits_r[3..6], &digits_r[6..]),
                        format!("{}-{}-{}", &digits_f[..3], &digits_f[3..6], &digits_f[6..]),
                    ),
                ]
            } else {
                vec![(r.clone(), f.clone())]
            }
        }
        _ => vec![(r.clone(), f.clone())],
    }
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Apply all scrub rules to `html`, returning the result and a changed flag.
pub fn scrub_html(html: &str, rules: &ScrubRules) -> (String, bool) {
    let mut text = html.to_string();
    let mut changed = false;

    for rule in &rules.rules {
        for (real, fake) in expand_rule(rule) {
            if text.contains(&real) {
                text = text.replace(&real, &fake);
                changed = true;
            }
        }
    }
    (text, changed)
}

// ==================== #8 — image pipeline ====================

/// Process all images under `dir` using ImageMagick.
/// Silently skips if `magick` is not on PATH.
pub fn process_images(dir: &Path, config: &ImagesConfig) -> Result<()> {
    if !config.enabled {
        return Ok(());
    }

    // Check if magick is available
    if std::process::Command::new("magick")
        .arg("-version")
        .output()
        .is_err()
    {
        eprintln!("Warning: ImageMagick (`magick`) not found — skipping image processing");
        return Ok(());
    }

    let image_paths = collect_images(dir)?;
    for img in &image_paths {
        if let Err(e) = process_single_image(img, config) {
            eprintln!("Warning: image processing failed for {}: {}", img.display(), e);
        }
    }
    Ok(())
}

fn collect_images(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    for entry in walkdir(dir)? {
        let ext = entry
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());
        match ext.as_deref() {
            Some("jpg") | Some("jpeg") | Some("png") | Some("heic") => out.push(entry),
            _ => {}
        }
    }
    Ok(out)
}

fn walkdir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            out.extend(walkdir(&p)?);
        } else {
            out.push(p);
        }
    }
    Ok(out)
}

fn process_single_image(path: &Path, config: &ImagesConfig) -> Result<()> {
    let resize = format!("{}x{}>", config.max_width, config.max_height);
    let quality = config.quality.to_string();

    let mut args: Vec<String> = vec![
        path.to_string_lossy().into_owned(),
        "-strip".into(),
        "-resize".into(),
        resize,
        "-quality".into(),
        quality,
        "-auto-orient".into(),
    ];

    if config.grain {
        args.extend([
            "+noise".into(),
            "Gaussian".into(),
            "-attenuate".into(),
            "70%".into(),
        ]);
    }
    if config.greyscale {
        args.extend(["-colorspace".into(), "Gray".into()]);
    }

    // Write to a temp file then rename, so the original is never partially written
    let tmp = path.with_extension("_processing_tmp");
    args.push(tmp.to_string_lossy().into_owned());

    let status = std::process::Command::new("magick")
        .args(&args)
        .status()?;

    if !status.success() {
        anyhow::bail!("magick exited with status {}", status);
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

// ==================== Postprocess directory ====================

/// Run all post-processing steps on every HTML file in `output_dir`.
pub fn postprocess_dir(
    output_dir: &Path,
    strip_prefix: &str,
    scrub_config: &ScrubConfig,
    scrub_rules: &ScrubRules,
    images_config: &ImagesConfig,
) -> Result<()> {
    // Collect HTML files
    let html_files: Vec<PathBuf> = walkdir(output_dir)?
        .into_iter()
        .filter(|p| p.extension().map_or(false, |e| e == "html"))
        .collect();

    for path in &html_files {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let content = std::fs::read_to_string(path)?;
        let mut current = content;
        let mut dirty = false;

        // #9 — path prefix
        if !strip_prefix.is_empty() {
            let (next, ch) = strip_path_prefix(&current, strip_prefix);
            if ch { dirty = true; current = next; }
        }

        // #6 — class redaction
        {
            let (next, ch) = redact_private_elements(&current);
            if ch { dirty = true; current = next; }
        }

        // #7 — scrubbing
        if scrub_config.enabled && !scrub_config.skip_files.contains(&filename) {
            let (next, ch) = scrub_html(&current, scrub_rules);
            if ch { dirty = true; current = next; }
        }

        if dirty {
            std::fs::write(path, &current)?;
        }
    }

    // #8 — images
    process_images(output_dir, images_config)?;

    Ok(())
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ScrubCategory, ScrubRule};

    // --- path prefix stripping ---

    #[test]
    fn test_strip_prefix_in_href() {
        let html = r#"<a href="Library/CloudStorage/SynologyDrive/org/foo.html">link</a>"#;
        let (result, changed) = strip_path_prefix(html, "Library/CloudStorage/SynologyDrive/org/");
        assert!(changed);
        assert!(result.contains(r#"href="foo.html""#));
    }

    #[test]
    fn test_strip_prefix_in_src() {
        let html = r#"<img src="Library/CloudStorage/SynologyDrive/org/img.png">"#;
        let (result, changed) = strip_path_prefix(html, "Library/CloudStorage/SynologyDrive/org/");
        assert!(changed);
        assert!(result.contains(r#"src="img.png""#));
    }

    #[test]
    fn test_strip_prefix_not_in_text() {
        // The prefix in visible text should NOT be stripped
        let html = r#"<p>see Library/CloudStorage/SynologyDrive/org/foo</p>"#;
        let (result, _changed) = strip_path_prefix(html, "Library/CloudStorage/SynologyDrive/org/");
        // No href or src, so text is unchanged
        assert_eq!(result, html);
    }

    #[test]
    fn test_strip_prefix_empty_prefix_noop() {
        let html = "<a href=\"foo.html\">x</a>";
        let (result, changed) = strip_path_prefix(html, "");
        assert!(!changed);
        assert_eq!(result, html);
    }

    // --- class redaction ---

    #[test]
    fn test_redact_hidden_div() {
        let html = r#"<p>before</p><div class="HIDDEN"><p>secret</p></div><p>after</p>"#;
        let (result, changed) = redact_private_elements(html);
        assert!(changed);
        assert!(result.contains("before"));
        assert!(result.contains("after"));
        assert!(!result.contains("secret"));
    }

    #[test]
    fn test_redact_private_class() {
        let html = r#"<div class="private"><p>hidden content</p></div><p>visible</p>"#;
        let (result, changed) = redact_private_elements(html);
        assert!(changed);
        assert!(!result.contains("hidden content"));
        assert!(result.contains("visible"));
    }

    #[test]
    fn test_redact_no_match_unchanged() {
        let html = r#"<div class="normal"><p>public</p></div>"#;
        let (result, changed) = redact_private_elements(html);
        assert!(!changed);
        assert_eq!(result, html);
    }

    #[test]
    fn test_redact_mixed_classes() {
        let html = r#"<div class="outline HIDDEN"><p>secret</p></div><p>ok</p>"#;
        let (result, changed) = redact_private_elements(html);
        assert!(changed);
        assert!(!result.contains("secret"));
        assert!(result.contains("ok"));
    }

    // --- scrubbing ---

    fn make_rules(rules: Vec<ScrubRule>) -> ScrubRules {
        ScrubRules { rules }
    }

    #[test]
    fn test_scrub_address() {
        let rules = make_rules(vec![ScrubRule {
            category: ScrubCategory::Address,
            real: "97 Buell St".into(),
            fake: "103 Campbell Rd".into(),
        }]);
        let html = "<p>I live at 97 Buell St, Burlington.</p>";
        let (result, changed) = scrub_html(html, &rules);
        assert!(changed);
        assert!(result.contains("103 Campbell Rd"));
        assert!(!result.contains("97 Buell St"));
    }

    #[test]
    fn test_scrub_phone_expands_formats() {
        let rules = make_rules(vec![ScrubRule {
            category: ScrubCategory::Phone,
            real: "8023553455".into(),
            fake: "2484345509".into(),
        }]);
        // All three formats should be replaced
        let html = "<p>call 8023553455 or (802) 355-3455 or 802-355-3455</p>";
        let (result, _) = scrub_html(html, &rules);
        assert!(!result.contains("8023553455"));
        assert!(!result.contains("(802) 355-3455"));
        assert!(!result.contains("802-355-3455"));
        assert!(result.contains("2484345509"));
    }

    #[test]
    fn test_scrub_town_case_variants() {
        let rules = make_rules(vec![ScrubRule {
            category: ScrubCategory::Town,
            real: "Burlington".into(),
            fake: "Essex".into(),
        }]);
        let html = "<p>BURLINGTON, burlington, Burlington</p>";
        let (result, changed) = scrub_html(html, &rules);
        assert!(changed);
        assert!(!result.contains("Burlington"));
        assert!(!result.contains("BURLINGTON"));
        assert!(!result.contains("burlington"));
    }

    #[test]
    fn test_expand_rule_phone_10_digits() {
        let rule = ScrubRule {
            category: ScrubCategory::Phone,
            real: "8023553455".into(),
            fake: "2484345509".into(),
        };
        let pairs = expand_rule(&rule);
        assert_eq!(pairs.len(), 3);
        assert!(pairs.iter().any(|(r, _)| r == "(802) 355-3455"));
        assert!(pairs.iter().any(|(r, _)| r == "802-355-3455"));
    }
}
