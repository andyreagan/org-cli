/// Stage 1 — source normalisation.
///
/// All functions in this module operate on `.org` source files in-place.
/// They are idempotent: running them multiple times produces the same result.
///
/// Covers issues:
///   #11 — flatten nested id: links, strip zero-width spaces
///   #12 — consolidate :BACKLINKS: drawer entries
///   #13 — .#*.org Emacs lock-file exclusion (via `collect_org_files`)

use anyhow::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ==================== #13 — safe directory scanning ====================

/// Collect all `.org` files under `dir` (non-recursive), skipping Emacs
/// lock symlinks (`.#*.org`).
pub fn collect_org_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().map_or(false, |e| e == "org") {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if !name.starts_with(".#") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

// ==================== #11 — nested id: link flattening ====================

/// Parse the org link starting at byte `start` in `s` (must be `[[`).
/// Returns `(end, id, desc)` where `end` is the exclusive end index,
/// `id` is the link target (e.g. `id:UUID`), and `desc` is the raw
/// description (which may itself contain nested links).
fn parse_link_at(s: &str, start: usize) -> Option<(usize, String, String)> {
    let bytes = s.as_bytes();
    if start + 1 >= bytes.len() || bytes[start] != b'[' || bytes[start + 1] != b'[' {
        return None;
    }
    // Walk forward, counting brackets, to find the "][" separator at depth 2
    // then the closing "]]" at depth 0.
    let mut i = start + 2; // right after "[["
    let mut depth: i32 = 2;
    let mut sep: Option<usize> = None; // position of ']' in "]["

    while i < bytes.len() {
        match bytes[i] {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                // At depth==1, the next char should be '[' for the separator
                if depth == 1 && sep.is_none() {
                    if i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                        sep = Some(i);
                    }
                }
                if depth == 0 {
                    // This is the outer closing ']'
                    // The link is s[start..=i], i.e. end = i+1
                    let sep_pos = sep?;
                    let id = s[start + 2..sep_pos].to_string();
                    let desc = s[sep_pos + 2..i - 1].to_string(); // between "][" and "]]"
                    return Some((i + 1, id, desc));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Flatten one level of nesting in `s`:
///   `[[id:X][[[id:X][Text]]]]`  →  `[[id:X][Text]]`
fn flatten_once(s: &str) -> String {
    let mut result = String::new();
    let mut pos = 0usize;
    let bytes = s.as_bytes();

    while pos < bytes.len() {
        if pos + 1 < bytes.len() && bytes[pos] == b'[' && bytes[pos + 1] == b'[' {
            if let Some((end, id, desc)) = parse_link_at(s, pos) {
                if id.starts_with("id:") {
                    let flat_desc = flatten_description(&desc);
                    result.push_str(&format!("[[{}][{}]]", id, flat_desc));
                    pos = end;
                    continue;
                }
            }
        }
        // SAFETY: org files are UTF-8; we advance one byte at a time only on
        // ASCII characters or the start of multi-byte sequences.
        let ch = s[pos..].chars().next().unwrap_or('\0');
        result.push(ch);
        pos += ch.len_utf8();
    }
    result
}

/// If `desc` is itself a `[[...][...]]` link, recurse to get plain text.
fn flatten_description(desc: &str) -> String {
    let trimmed = desc.trim();
    if trimmed.starts_with("[[") {
        if let Some((_end, _id, inner)) = parse_link_at(trimmed, 0) {
            return flatten_description(&inner);
        }
    }
    desc.to_string()
}

/// Normalise all `id:` links in a string:
/// 1. Strip zero-width spaces (`\u{200B}`)
/// 2. Repeatedly flatten nested links until stable
pub fn flatten_id_links(s: &str) -> String {
    // Step 1: strip ZWSPs
    let s = s.replace('\u{200B}', "");
    // Step 2: flatten until stable (max 4 passes is plenty)
    let mut current = s;
    for _ in 0..4 {
        let next = flatten_once(&current);
        if next == current {
            break;
        }
        current = next;
    }
    current
}

/// Apply `flatten_id_links` to every line in a file that contains `id:`.
/// Returns `(new_content, changed)`.
pub fn normalise_links_in_text(content: &str) -> (String, bool) {
    let mut changed = false;
    let mut out = String::with_capacity(content.len());
    for line in content.lines() {
        if line.contains("id:") {
            let fixed = flatten_id_links(line);
            if fixed != line {
                changed = true;
            }
            out.push_str(&fixed);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    // Preserve absence of trailing newline if original had none
    if !content.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    (out, changed)
}

// ==================== #12 — backlinks drawer consolidation ====================

/// Consolidate `:BACKLINKS:` drawers in a string.
/// Returns `(new_content, changed)`.
pub fn consolidate_backlinks(content: &str) -> (String, bool) {
    let mut out = String::with_capacity(content.len());
    let mut changed = false;
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.trim() == ":BACKLINKS:" {
            // Collect everything until :END:
            let mut raw_lines: Vec<&str> = Vec::new();
            for inner in lines.by_ref() {
                if inner.trim() == ":END:" {
                    break;
                }
                raw_lines.push(inner);
            }

            // Parse: split into already-formatted line(s) and raw timestamped entries
            // Use BTreeMap keyed by id so we sort alphabetically by title later
            let mut links: BTreeMap<String, String> = BTreeMap::new(); // id -> title

            for raw in &raw_lines {
                let trimmed = raw.trim();
                if trimmed.starts_with("/Backlinks:") {
                    // Already formatted: extract all [[id:...][...]] pairs
                    for (id, title) in extract_id_links(trimmed) {
                        links.insert(id, title);
                    }
                } else {
                    // Timestamped entry: [2024-... Tue ...] <- [[id:...][...]]
                    for (id, title) in extract_id_links(trimmed) {
                        links.insert(id, title);
                    }
                }
            }

            // Build the consolidated line
            let mut sorted: Vec<(&String, &String)> = links.iter().collect();
            sorted.sort_by_key(|(_, title)| title.to_lowercase());
            let backlinks_str = sorted
                .iter()
                .map(|(id, title)| format!("[[id:{}][{}]]", id, title))
                .collect::<Vec<_>>()
                .join(" | ");

            // Determine if we actually need to change anything
            let expected_inner = if backlinks_str.is_empty() {
                Vec::new()
            } else {
                vec![format!("/Backlinks: {}/", backlinks_str)]
            };
            let current_inner: Vec<String> = raw_lines.iter().map(|l| l.to_string()).collect();
            if current_inner != expected_inner {
                changed = true;
            }

            out.push_str(line); // :BACKLINKS:
            out.push('\n');
            if !backlinks_str.is_empty() {
                out.push_str(&format!("/Backlinks: {}/", backlinks_str));
                out.push('\n');
            }
            out.push_str(":END:");
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    if !content.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }

    (out, changed)
}

/// Extract all `(id, title)` pairs from `[[id:UUID][Title]]` occurrences in a string.
fn extract_id_links(s: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut remaining = s;
    while let Some(start) = remaining.find("[[id:") {
        let after = &remaining[start + 2..]; // skip "[["
        if let Some(sep) = after.find("][") {
            let id = after[3..sep].to_string(); // skip "id:"
            let rest = &after[sep + 2..];
            if let Some(end) = rest.find("]]") {
                let title = rest[..end].to_string();
                results.push((id, title));
                remaining = &remaining[start + 2 + sep + 2 + end + 2..];
                continue;
            }
        }
        break;
    }
    results
}

// ==================== Top-level normalise pass ====================

/// Run all normalisation steps on a single file.
/// Returns `true` if the file was modified.
pub fn normalise_file(path: &Path, dry_run: bool) -> Result<bool> {
    let original = std::fs::read_to_string(path)?;

    let (after_links, links_changed) = normalise_links_in_text(&original);
    let (after_backlinks, bl_changed) = consolidate_backlinks(&after_links);

    let changed = links_changed || bl_changed;
    if changed && !dry_run {
        std::fs::write(path, &after_backlinks)?;
    }
    Ok(changed)
}

/// Run normalisation across all `.org` files in `dir`.
pub fn normalise_dir(dir: &Path, dry_run: bool) -> Result<Vec<PathBuf>> {
    let files = collect_org_files(dir)?;
    let mut modified = Vec::new();
    for path in files {
        match normalise_file(&path, dry_run) {
            Ok(true) => {
                modified.push(path);
            }
            Ok(false) => {}
            Err(e) => eprintln!("Warning: skipping {}: {}", path.display(), e),
        }
    }
    Ok(modified)
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    // --- link flattening ---

    #[test]
    fn test_flatten_noop_plain_link() {
        let s = "[[id:ABC][Cars]]";
        assert_eq!(flatten_id_links(s), s);
    }

    #[test]
    fn test_flatten_single_nesting() {
        let input = "[[id:ABC][[[id:ABC][Cars]]]]";
        assert_eq!(flatten_id_links(input), "[[id:ABC][Cars]]");
    }

    #[test]
    fn test_flatten_double_nesting() {
        let input = "[[id:ABC][[[id:ABC][[[id:ABC][Cars]]]]]]";
        assert_eq!(flatten_id_links(input), "[[id:ABC][Cars]]");
    }

    #[test]
    fn test_flatten_strips_zwsp() {
        // Zero-width space between brackets
        let input = "[[id:ABC][\u{200B}[[id:ABC][Cars]]\u{200B}]]";
        // After stripping ZWSPs it becomes [[id:ABC][[[id:ABC][Cars]]]]
        let result = flatten_id_links(input);
        assert_eq!(result, "[[id:ABC][Cars]]");
    }

    #[test]
    fn test_flatten_multiple_links_in_line() {
        let input = "- [[id:AAA][[[id:AAA][Bikes]]]] and [[id:BBB][Cars]]";
        let result = flatten_id_links(input);
        assert_eq!(result, "- [[id:AAA][Bikes]] and [[id:BBB][Cars]]");
    }

    #[test]
    fn test_flatten_no_id_links_untouched() {
        let s = "[[file:foo.org][Foo]] and plain text";
        assert_eq!(flatten_id_links(s), s);
    }

    #[test]
    fn test_normalise_links_in_text_only_touches_id_lines() {
        let content = "* Heading\n[[file:foo.org][Foo]]\n[[id:X][[[id:X][Bar]]]]\n";
        let (result, changed) = normalise_links_in_text(content);
        assert!(changed);
        assert!(result.contains("[[id:X][Bar]]"));
        assert!(result.contains("[[file:foo.org][Foo]]"));
    }

    // --- backlinks consolidation ---

    #[test]
    fn test_consolidate_backlinks_basic() {
        let input = ":BACKLINKS:\n\
[2024-08-13 Tue 12:12] <- [[id:AAA][Whoop]]\n\
[2024-08-13 Tue 12:14] <- [[id:BBB][Fitness Tech]]\n\
:END:\n";
        let (result, changed) = consolidate_backlinks(input);
        assert!(changed);
        assert!(result.contains("/Backlinks:"));
        assert!(result.contains("[[id:AAA][Whoop]]"));
        assert!(result.contains("[[id:BBB][Fitness Tech]]"));
        assert!(result.contains(":END:"));
    }

    #[test]
    fn test_consolidate_backlinks_idempotent() {
        let input = ":BACKLINKS:\n\
/Backlinks: [[id:AAA][Fitness Tech]] | [[id:BBB][Whoop]]/\n\
:END:\n";
        let (result, changed) = consolidate_backlinks(input);
        assert!(!changed);
        assert_eq!(result, input);
    }

    #[test]
    fn test_consolidate_backlinks_merge_new_entry() {
        let input = ":BACKLINKS:\n\
/Backlinks: [[id:AAA][Whoop]]/\n\
[2024-09-01 Sun 10:00] <- [[id:BBB][Fitness Tech]]\n\
:END:\n";
        let (result, changed) = consolidate_backlinks(input);
        assert!(changed);
        // Sorted alphabetically: Fitness Tech before Whoop
        assert!(result.contains("[[id:BBB][Fitness Tech]] | [[id:AAA][Whoop]]"));
    }

    #[test]
    fn test_consolidate_backlinks_deduplicates() {
        let input = ":BACKLINKS:\n\
/Backlinks: [[id:AAA][Whoop]]/\n\
[2024-09-01 Sun 10:00] <- [[id:AAA][Whoop]]\n\
:END:\n";
        let (result, _) = consolidate_backlinks(input);
        // Should only appear once
        assert_eq!(result.matches("[[id:AAA][Whoop]]").count(), 1);
    }

    #[test]
    fn test_consolidate_no_backlinks_drawer_unchanged() {
        let input = "* Heading\n\nSome body text.\n";
        let (result, changed) = consolidate_backlinks(input);
        assert!(!changed);
        assert_eq!(result, input);
    }

    // --- collect_org_files ---

    #[test]
    fn test_collect_org_files_skips_lock_files() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::write(p.join("real.org"), "* heading").unwrap();
        std::fs::write(p.join(".#locked.org"), "dangling symlink content").unwrap();
        std::fs::write(p.join("notes.txt"), "ignored").unwrap();
        let files = collect_org_files(p).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().unwrap() == "real.org");
    }
}
