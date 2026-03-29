/// Stage 2 (pre-export) — blog index, tag pages, and navigation injection.
///
/// Covers issues:
///   #1  — blog index page (blog.org)
///   #2  — per-tag index pages (tag_<name>.org)
///   #3  — prev/next/random navigation links injected into each post

use crate::config::BlogConfig;
use crate::normalise::collect_org_files;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ==================== Post metadata ====================

#[derive(Debug, Clone)]
pub struct PostMeta {
    pub path: PathBuf,
    pub filename: String, // e.g. "2026-03-08-post.org"
    pub date: String,     // "YYYY-MM-DD"
    pub title: String,
    pub tags: Vec<String>,
    pub word_count: usize,
}

/// Return true if `filename` looks like a blog post (`YYYY-MM-DD-*.org`).
pub fn is_blog_post(filename: &str) -> bool {
    filename.len() >= 11
        && filename[..4].chars().all(|c| c.is_ascii_digit())
        && filename.chars().nth(4) == Some('-')
        && filename[5..7].chars().all(|c| c.is_ascii_digit())
        && filename.chars().nth(7) == Some('-')
        && filename[8..10].chars().all(|c| c.is_ascii_digit())
        && filename.chars().nth(10) == Some('-')
        && filename.ends_with(".org")
}

/// Extract metadata from a single `.org` file.
pub fn extract_post_meta(path: &Path) -> Result<PostMeta> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let date = filename[..10].to_string();

    let content = std::fs::read_to_string(path)?;

    // Title: first `* ` heading, tags stripped
    let (title, tags) = extract_title_and_tags(&content);

    // Word count: strip #+directives and heading lines, count whitespace-split tokens
    let word_count = count_words(&content);

    Ok(PostMeta {
        path: path.to_path_buf(),
        filename,
        date,
        title,
        tags,
        word_count,
    })
}

fn extract_title_and_tags(content: &str) -> (String, Vec<String>) {
    for line in content.lines() {
        if line.starts_with("* ") {
            let rest = &line[2..];
            let rest_trimmed = rest.trim_end();

            // Org tag block: a trailing token that looks like `:tag1:tag2:`
            // It is separated from the title by whitespace, starts and ends with ':'.
            // Strategy: find the last ':' that has a matching opening ':' before it,
            // with only tag-name characters (no spaces) between them.
            if let Some(tag_block_start) = find_tag_block_start(rest_trimmed) {
                let title = rest_trimmed[..tag_block_start].trim().to_string();
                let tag_str = &rest_trimmed[tag_block_start + 1..rest_trimmed.len() - 1];
                let tags: Vec<String> = tag_str
                    .split(':')
                    .filter(|t| !t.is_empty())
                    .map(String::from)
                    .collect();
                return (title, tags);
            }

            return (rest_trimmed.to_string(), vec![]);
        }
    }
    ("Untitled".into(), vec![])
}

/// Find the start of the tag block in a heading body string.
/// The tag block must:
/// - end with ':'
/// - start with ':' preceded by whitespace (or start of string)
/// - contain only non-whitespace, non-empty tokens between colons
///
/// Returns the byte index of the opening ':'.
fn find_tag_block_start(s: &str) -> Option<usize> {
    if !s.ends_with(':') {
        return None;
    }
    // Work backwards through ':word:word:...:' from the end
    let bytes = s.as_bytes();
    let mut i = bytes.len() - 1; // points at trailing ':'

    // Scan backwards: each iteration handles one ':tag' from the right
    loop {
        // i points at a ':'
        // scan left through the tag name
        let colon_pos = i;
        if i == 0 {
            // tag block covers the whole string — valid only if title would be empty
            return Some(0);
        }
        i -= 1;
        let tag_start = i;
        while i > 0 && bytes[i - 1] != b':' && !bytes[i - 1].is_ascii_whitespace() {
            i -= 1;
        }
        // If no characters consumed, we have an empty tag name — invalid
        if i > tag_start {
            return None; // went backwards (shouldn't happen)
        }
        let tag_len = tag_start - i + 1;
        if tag_len == 0 {
            return None;
        }
        // Now bytes[i-1] is ':' or whitespace or i==0
        if i == 0 {
            // The whole suffix is a tag block starting at 0... unlikely but valid
            return Some(0);
        }
        if bytes[i - 1].is_ascii_whitespace() {
            // The ':' at colon_pos that starts this run is at... 
            // we need to find the ':' that opens the block.
            // The opening ':' is at i (which is the first tag char), but we need
            // the ':' before the first tag char.
            // Actually i points to the first char of the first tag we found.
            // The opening ':' is at i-1 if bytes[i-1] is ':', or there's no opening ':'.
            // Wait — let me re-think. 
            // At this point bytes[i-1] is whitespace. 
            // The ':' that opens the tag block is the one we encountered as `colon_pos`
            // for the *first* (leftmost) tag, which we haven't scanned yet.
            // Instead, let's just find the leftmost ':' in the trailing no-whitespace run.
            let end_of_title = i; // index after whitespace
            // Find where the non-whitespace tag-block run starts (going right from end_of_title)
            // We already know colon_pos is valid. We just need the leftmost ':' in the run.
            // Scan right from end_of_title to find the first ':'
            let mut j = end_of_title;
            while j < bytes.len() && bytes[j] != b':' {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b':' {
                // Verify the entire suffix from j is a valid tag block
                let candidate = &s[j..];
                if is_valid_tag_block(candidate) {
                    return Some(j);
                }
            }
            return None;
        }
        if bytes[i - 1] == b':' {
            // More tags to the left — continue loop
            i -= 1; // i now points at the ':' before this tag
            continue;
        }
        // Non-colon, non-whitespace before: part of the title, not a tag block
        return None;
    }
}

/// A valid tag block matches `:word:word:...:` with no spaces.
fn is_valid_tag_block(s: &str) -> bool {
    if !s.starts_with(':') || !s.ends_with(':') {
        return false;
    }
    let inner = &s[1..s.len() - 1];
    if inner.is_empty() {
        return false;
    }
    inner.split(':').all(|t| !t.is_empty() && !t.contains(' '))
}

fn count_words(content: &str) -> usize {
    content
        .lines()
        .filter(|l| !l.starts_with("#+") && !l.starts_with('*'))
        .flat_map(|l| l.split_whitespace())
        .count()
}

// ==================== Navigation links (#3) ====================

/// Build the navigation line for post at `index` in the sorted `posts` slice.
/// Uses a simple LCG seeded by `seed + index` for the random pick so the
/// choice is deterministic across runs.
pub fn make_nav_line(posts: &[PostMeta], index: usize, seed: u64) -> String {
    let mut parts: Vec<String> = Vec::new();

    if index > 0 {
        parts.push(format!(
            "[[file:{}][previous]]",
            posts[index - 1].filename
        ));
    }
    if index + 1 < posts.len() {
        parts.push(format!(
            "[[file:{}][next]]",
            posts[index + 1].filename
        ));
    }

    // Deterministic random: LCG with seed
    let random_index = pick_random(posts.len(), index, seed);
    parts.push(format!(
        "[[file:{}][random]]",
        posts[random_index].filename
    ));

    parts.join(" | ")
}

/// Simple LCG to pick a random index ≠ `exclude`.
fn pick_random(len: usize, exclude: usize, seed: u64) -> usize {
    if len <= 1 {
        return 0;
    }
    let mut state = seed.wrapping_add(exclude as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    state ^= state >> 33;
    state = state.wrapping_mul(0xff51afd7ed558ccd);
    state ^= state >> 33;
    let candidate = (state as usize) % (len - 1);
    if candidate >= exclude { candidate + 1 } else { candidate }
}

/// Inject or update the navigation line in `content`.
/// Looks for a line containing `[random]`; if found, replaces it.
/// Returns `(new_content, changed, nav_found)`.
pub fn inject_nav(content: &str, nav_line: &str) -> (String, bool, bool) {
    let mut out = String::with_capacity(content.len());
    let mut changed = false;
    let mut nav_found = false;

    for line in content.lines() {
        if line.contains("[random]") {
            nav_found = true;
            if line != nav_line {
                changed = true;
                out.push_str(nav_line);
            } else {
                out.push_str(line);
            }
        } else if line == nav_line {
            // Already correct nav line (prev/next/random all present)
            nav_found = true;
            out.push_str(line);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }

    if !content.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }

    (out, changed, nav_found)
}

// ==================== Blog index + tag pages (#1, #2) ====================

/// Generate `blog.org` content from a sorted (newest-first) list of posts.
pub fn generate_blog_index(posts: &[PostMeta], tag_counts: &HashMap<String, usize>) -> String {
    let mut out = String::new();

    out.push_str("\n** Posts\n\n");
    for post in posts {
        let tags_str = post
            .tags
            .iter()
            .map(|t| format!("[[file:tag_{}.org][{}]]", t, t))
            .collect::<Vec<_>>()
            .join(" ");
        let tags_suffix = if tags_str.is_empty() {
            String::new()
        } else {
            format!(" {}", tags_str)
        };
        out.push_str(&format!(
            "- [[file:{}][{}]] ({} words, {}){}\n",
            post.filename, post.title, post.word_count, post.date, tags_suffix
        ));
    }

    out.push_str("\n** Tags\n\n");

    // Sort by count desc, then alphabetically
    let mut tag_vec: Vec<(&String, &usize)> = tag_counts.iter().collect();
    tag_vec.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

    for (tag, count) in &tag_vec {
        if **count > 1 {
            out.push_str(&format!(
                "- [[file:tag_{}.org][{}]] ({})\n",
                tag, tag, count
            ));
        }
    }

    out
}

/// Generate the content of `tag_<name>.org`.
pub fn generate_tag_page(tag: &str, posts: &[PostMeta]) -> String {
    let mut out = format!("* Posts tagged with '{}'\n\n", tag);
    for post in posts {
        if post.tags.contains(&tag.to_string()) {
            out.push_str(&format!(
                "- [[file:{}][{}]] ({} words, {})\n",
                post.filename, post.title, post.word_count, post.date
            ));
        }
    }
    out
}

// ==================== Main entry point ====================

/// Scan `dir` for blog posts, update navigation in each post, write
/// `blog.org` and `tag_*.org` files into `dir`.
///
/// Returns the list of posts (newest-first) for use by the exporter.
pub fn build_blog(dir: &Path, config: &BlogConfig) -> Result<Vec<PostMeta>> {
    let all_files = collect_org_files(dir)?;

    // Collect posts sorted chronologically (oldest first for nav)
    let mut posts: Vec<PostMeta> = all_files
        .iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(is_blog_post)
                .unwrap_or(false)
        })
        .filter_map(|p| match extract_post_meta(p) {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!("Warning: could not read {}: {}", p.display(), e);
                None
            }
        })
        .collect();

    posts.sort_by(|a, b| a.date.cmp(&b.date)); // oldest first for nav

    // --- nav injection (#3) ---
    for (i, post) in posts.iter().enumerate() {
        let nav_line = make_nav_line(&posts, i, config.nav_random_seed);
        let content = std::fs::read_to_string(&post.path)?;
        let (new_content, changed, nav_found) = inject_nav(&content, &nav_line);
        if !nav_found {
            eprintln!(
                "Warning: {}: no navigation link placeholder found (add a line containing [random])",
                post.filename
            );
        }
        if changed {
            std::fs::write(&post.path, new_content)?;
        }
    }

    // --- build tag counts ---
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for post in &posts {
        for tag in &post.tags {
            *tag_counts.entry(tag.clone()).or_insert(0) += 1;
        }
    }

    // --- tag pages (#2) ---
    // Collect all tags across posts
    let all_tags: Vec<String> = {
        let mut t: Vec<String> = tag_counts.keys().cloned().collect();
        t.sort();
        t
    };
    for tag in &all_tags {
        let tag_content = generate_tag_page(tag, &posts);
        let tag_path = dir.join(format!("tag_{}.org", tag));
        std::fs::write(&tag_path, &tag_content)?;
    }

    // --- blog index (#1) ---
    // Reverse to newest-first for the index display
    let mut display_posts = posts.clone();
    display_posts.reverse();
    let index_content = generate_blog_index(&display_posts, &tag_counts);
    let index_path = dir.join(&config.index_file);
    std::fs::write(&index_path, &index_content)?;

    eprintln!(
        "Blog: {} posts, {} tags → {}",
        posts.len(),
        all_tags.len(),
        config.index_file
    );

    Ok(display_posts)
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_blog_post() {
        assert!(is_blog_post("2026-03-08-my-post.org"));
        assert!(is_blog_post("2014-09-04-first-post.org"));
        assert!(!is_blog_post("about.org"));
        assert!(!is_blog_post("blog.org"));
        assert!(!is_blog_post("2026-03-08.org")); // no slug after date
        assert!(!is_blog_post(".#2026-03-08-locked.org"));
    }

    #[test]
    fn test_extract_title_and_tags_no_tags() {
        let content = "* My Post Title\n\nBody text.\n";
        let (title, tags) = extract_title_and_tags(content);
        assert_eq!(title, "My Post Title");
        assert!(tags.is_empty());
    }

    #[test]
    fn test_extract_title_and_tags_with_tags() {
        let content = "* Running in the Rain       :running:fitness:\n\nBody.\n";
        let (title, tags) = extract_title_and_tags(content);
        assert_eq!(title, "Running in the Rain");
        assert_eq!(tags, vec!["running", "fitness"]);
    }

    #[test]
    fn test_count_words_excludes_directives_and_headings() {
        let content = "#+TITLE: My Post\n* Heading\nHello world foo bar.\n";
        assert_eq!(count_words(content), 4); // only "Hello world foo bar."
    }

    #[test]
    fn test_make_nav_line_first_post() {
        let posts = vec![
            PostMeta { path: "a.org".into(), filename: "2020-01-01-a.org".into(), date: "2020-01-01".into(), title: "A".into(), tags: vec![], word_count: 10 },
            PostMeta { path: "b.org".into(), filename: "2020-06-01-b.org".into(), date: "2020-06-01".into(), title: "B".into(), tags: vec![], word_count: 10 },
            PostMeta { path: "c.org".into(), filename: "2021-01-01-c.org".into(), date: "2021-01-01".into(), title: "C".into(), tags: vec![], word_count: 10 },
        ];
        let nav = make_nav_line(&posts, 0, 42);
        assert!(!nav.contains("previous")); // no previous for first post
        assert!(nav.contains("next"));
        assert!(nav.contains("random"));
    }

    #[test]
    fn test_make_nav_line_last_post() {
        let posts = vec![
            PostMeta { path: "a.org".into(), filename: "2020-01-01-a.org".into(), date: "2020-01-01".into(), title: "A".into(), tags: vec![], word_count: 10 },
            PostMeta { path: "b.org".into(), filename: "2020-06-01-b.org".into(), date: "2020-06-01".into(), title: "B".into(), tags: vec![], word_count: 10 },
        ];
        let nav = make_nav_line(&posts, 1, 42);
        assert!(nav.contains("previous"));
        assert!(!nav.contains("next")); // no next for last post
        assert!(nav.contains("random"));
    }

    #[test]
    fn test_inject_nav_replaces_random_placeholder() {
        let content = "* Post\n\n[random]\n\nBody.\n";
        let nav = "[[file:prev.org][previous]] | [[file:next.org][next]] | [[file:rand.org][random]]";
        let (result, changed, found) = inject_nav(content, nav);
        assert!(found);
        assert!(changed);
        assert!(result.contains(nav));
        assert!(!result.contains("[random]\n"));
    }

    #[test]
    fn test_inject_nav_idempotent() {
        let nav = "[[file:prev.org][previous]] | [[file:next.org][next]] | [[file:rand.org][random]]";
        let content = format!("* Post\n\n{}\n\nBody.\n", nav);
        let (_, changed, found) = inject_nav(&content, nav);
        assert!(found);
        assert!(!changed);
    }

    #[test]
    fn test_inject_nav_warns_when_missing() {
        let content = "* Post\n\nNo placeholder here.\n";
        let nav = "[[file:a.org][previous]] | [[file:b.org][random]]";
        let (_, _, found) = inject_nav(content, nav);
        assert!(!found);
    }

    #[test]
    fn test_generate_blog_index_format() {
        let posts = vec![
            PostMeta { path: "b.org".into(), filename: "2021-01-01-b.org".into(), date: "2021-01-01".into(), title: "B Post".into(), tags: vec!["running".into()], word_count: 50 },
            PostMeta { path: "a.org".into(), filename: "2020-01-01-a.org".into(), date: "2020-01-01".into(), title: "A Post".into(), tags: vec![], word_count: 30 },
        ];
        let mut counts = HashMap::new();
        counts.insert("running".into(), 1usize);
        let index = generate_blog_index(&posts, &counts);
        assert!(index.contains("** Posts"));
        assert!(index.contains("** Tags"));
        assert!(index.contains("[[file:2021-01-01-b.org][B Post]]"));
        assert!(index.contains("50 words"));
        assert!(index.contains("[[file:tag_running.org][running]]"));
    }

    #[test]
    fn test_generate_tag_page() {
        let posts = vec![
            PostMeta { path: "a.org".into(), filename: "2020-01-01-a.org".into(), date: "2020-01-01".into(), title: "A".into(), tags: vec!["running".into()], word_count: 10 },
            PostMeta { path: "b.org".into(), filename: "2020-06-01-b.org".into(), date: "2020-06-01".into(), title: "B".into(), tags: vec!["cycling".into()], word_count: 20 },
        ];
        let page = generate_tag_page("running", &posts);
        assert!(page.contains("* Posts tagged with 'running'"));
        assert!(page.contains("[[file:2020-01-01-a.org][A]]"));
        assert!(!page.contains("[[file:2020-06-01-b.org]")); // B not tagged running
    }
}
