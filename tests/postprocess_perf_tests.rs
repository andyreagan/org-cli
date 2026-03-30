/// Performance regression tests for postprocess operations.
/// strip_path_prefix must run in O(n) time, not O(n²).

use org_cli::postprocess::strip_path_prefix;

/// Generate an HTML string with `n` href attributes containing the prefix,
/// padded to make the total file large.
fn make_large_html(n_hrefs: usize, prefix: &str) -> String {
    let mut s = String::from("<html><body>\n");
    for i in 0..n_hrefs {
        s.push_str(&format!(
            "<a href=\"{}some/path/{}.html\">link {}</a>\n",
            prefix, i, i
        ));
    }
    s.push_str("</body></html>\n");
    s
}

#[test]
fn test_strip_prefix_correctness() {
    let prefix = "Library/CloudStorage/SynologyDrive-OnDemandSync/org/";
    let html = format!(
        "<a href=\"{}foo.html\">x</a> <img src=\"{}bar.png\">",
        prefix, prefix
    );
    let (result, changed) = strip_path_prefix(&html, prefix);
    assert!(changed);
    assert!(result.contains("href=\"foo.html\""));
    assert!(result.contains("src=\"bar.png\""));
    assert!(!result.contains(prefix));
}

#[test]
fn test_strip_prefix_large_file_completes_quickly() {
    // 5000 hrefs × ~100 chars each ≈ 500KB — similar to blogroll.html.
    // Must complete in well under 1 second on any modern machine.
    let prefix = "Library/CloudStorage/SynologyDrive-OnDemandSync/org/";
    let html = make_large_html(5000, prefix);
    assert!(html.len() > 400_000, "test html should be large");

    let start = std::time::Instant::now();
    let (result, changed) = strip_path_prefix(&html, prefix);
    let elapsed = start.elapsed();

    assert!(changed);
    assert!(!result.contains(prefix));
    assert!(
        elapsed.as_millis() < 500,
        "strip_path_prefix took {}ms on a 500KB file — O(n²) regression",
        elapsed.as_millis()
    );
}

#[test]
fn test_strip_prefix_no_matches_is_fast() {
    let prefix = "Library/CloudStorage/SynologyDrive-OnDemandSync/org/";
    // Large file with many hrefs but none containing the prefix
    let html = make_large_html(0, "https://example.com/");
    let mut big = html.clone();
    for i in 0..5000 {
        big.push_str(&format!("<a href=\"https://example.com/{}.html\">x</a>\n", i));
    }

    let start = std::time::Instant::now();
    let (_, changed) = strip_path_prefix(&big, prefix);
    let elapsed = start.elapsed();

    assert!(!changed);
    assert!(
        elapsed.as_millis() < 500,
        "strip_path_prefix with no matches took {}ms — should be O(n)",
        elapsed.as_millis()
    );
}
