/// Tests for Emacs lock-file (.#*.org) exclusion across all commands.
/// These should pass after fixing find_org_files / export / build_id_index
/// to use collect_org_files (or equivalent filtering) instead.

use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn org_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_org-cli"))
}

// ==================== list command ====================

#[test]
fn test_list_skips_lock_files() {
    let dir = tempdir().unwrap();
    // Create a real org file
    fs::write(dir.path().join("real.org"), "* TODO Real task\n").unwrap();
    // Create an Emacs lock symlink (just a regular file here since we can't
    // guarantee symlink creation; the real invariant is the .# prefix)
    fs::write(dir.path().join(".#real.org"), "dangling lock").unwrap();

    let output = org_cli()
        .args(["list", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    // Must not crash (exit 0) and must still show the real task
    assert!(
        output.status.success(),
        "list crashed on .# lock file: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Real task"),
        "real task missing from output"
    );
}

// ==================== export command ====================

#[test]
fn test_export_skips_lock_files() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    // Real org file
    fs::write(
        dir.path().join("about.org"),
        "* About\nSome body.\n",
    )
    .unwrap();

    // Emacs lock file (dangling symlink represented as a regular file
    // with .# prefix — same filtering rule applies)
    fs::write(dir.path().join(".#about.org"), "lock").unwrap();

    let output = org_cli()
        .args([
            "export",
            dir.path().to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "export crashed on .# lock file:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // The real file should have been exported
    assert!(
        out.join("about.html").exists(),
        "about.html was not generated"
    );

    // The lock file must NOT produce output
    assert!(
        !out.join(".#about.html").exists(),
        ".#about.html should not be generated"
    );
}

// ==================== title fallback: first heading ====================

/// When a file has no #+TITLE: keyword, render_html should use the first
/// top-level heading as the page title, not "Untitled".
#[test]
fn test_export_title_falls_back_to_first_heading() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    // No #+TITLE:, but has a first-level heading
    fs::write(
        dir.path().join("about.org"),
        "#+CREATED: [2024-10-24 Thu]\n* About Me\nBody text.\n",
    )
    .unwrap();

    let output = org_cli()
        .args([
            "export",
            dir.path().to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "export failed");

    let html = fs::read_to_string(out.join("about.html")).unwrap();
    // Title tag must not be "Untitled"; it must be the first heading
    assert!(
        !html.contains("<title>Untitled</title>"),
        "title should not fall back to 'Untitled' when a heading is present"
    );
    assert!(
        html.contains("<title>About Me</title>"),
        "title should be the first heading 'About Me', got:\n{}",
        &html[..html.find("</head>").unwrap_or(200)]
    );
}

/// When a file has #+TITLE:, that must take priority over any heading.
#[test]
fn test_export_title_prefers_plus_title_keyword() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    fs::write(
        dir.path().join("page.org"),
        "#+TITLE: My Custom Title\n* First Heading\nBody.\n",
    )
    .unwrap();

    let output = org_cli()
        .args([
            "export",
            dir.path().to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "export failed");

    let html = fs::read_to_string(out.join("page.html")).unwrap();
    assert!(
        html.contains("<title>My Custom Title</title>"),
        "#+TITLE should take priority; got:\n{}",
        &html[..html.find("</head>").unwrap_or(300)]
    );
    assert!(
        !html.contains("<title>First Heading</title>"),
        "heading should not override #+TITLE"
    );
}
