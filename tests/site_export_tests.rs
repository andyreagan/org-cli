use org_cli::html::*;
use std::fs;
use tempfile::tempdir;

// ==================== ID Index Building ====================

#[test]
fn test_build_id_index_single_file() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("notes.org");
    fs::write(
        &file,
        "* Entry One\n:PROPERTIES:\n:ID: id-one\n:END:\n* Entry Two\n:PROPERTIES:\n:ID: id-two\n:END:\n",
    )
    .unwrap();

    let index = build_id_index(dir.path()).unwrap();
    assert_eq!(index.len(), 2);
    assert!(index.contains_key("id-one"));
    assert!(index.contains_key("id-two"));
}

#[test]
fn test_build_id_index_multiple_files() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("file1.org"),
        "* Entry\n:PROPERTIES:\n:ID: from-file1\n:END:\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("file2.org"),
        "* Entry\n:PROPERTIES:\n:ID: from-file2\n:END:\n",
    )
    .unwrap();

    let index = build_id_index(dir.path()).unwrap();
    assert_eq!(index.len(), 2);
    assert!(index.contains_key("from-file1"));
    assert!(index.contains_key("from-file2"));
}

#[test]
fn test_build_id_index_maps_to_html_path() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("notes.org"),
        "* Entry\n:PROPERTIES:\n:ID: my-id\n:END:\n",
    )
    .unwrap();

    let index = build_id_index(dir.path()).unwrap();
    let target = index.get("my-id").unwrap();
    // Should map to the HTML file path with anchor
    assert!(
        target.contains("notes.html"),
        "Expected notes.html in target, got: {}",
        target
    );
    assert!(
        target.contains("#my-id"),
        "Expected #my-id anchor in target, got: {}",
        target
    );
}

#[test]
fn test_build_id_index_nested_directory() {
    let dir = tempdir().unwrap();
    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();
    fs::write(
        subdir.join("deep.org"),
        "* Entry\n:PROPERTIES:\n:ID: deep-id\n:END:\n",
    )
    .unwrap();

    let index = build_id_index(dir.path()).unwrap();
    assert!(index.contains_key("deep-id"));
    let target = index.get("deep-id").unwrap();
    assert!(
        target.contains("deep.html"),
        "Expected deep.html in target, got: {}",
        target
    );
}

#[test]
fn test_build_id_index_entries_without_id_ignored() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("test.org"),
        "* No ID entry\n* With ID\n:PROPERTIES:\n:ID: has-id\n:END:\n",
    )
    .unwrap();

    let index = build_id_index(dir.path()).unwrap();
    assert_eq!(index.len(), 1);
    assert!(index.contains_key("has-id"));
}

// ==================== Site Export ====================

#[test]
fn test_export_site_creates_html_files() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    fs::write(dir.path().join("notes.org"), "* Heading One\n").unwrap();
    fs::write(dir.path().join("tasks.org"), "* TODO Task One\n").unwrap();

    export_site(dir.path(), out_dir.path()).unwrap();

    assert!(
        out_dir.path().join("notes.html").exists(),
        "notes.html should be created"
    );
    assert!(
        out_dir.path().join("tasks.html").exists(),
        "tasks.html should be created"
    );
}

#[test]
fn test_export_site_creates_css() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    fs::write(dir.path().join("test.org"), "* Heading\n").unwrap();

    export_site(dir.path(), out_dir.path()).unwrap();

    // Should create a CSS file or embed styles
    let html = fs::read_to_string(out_dir.path().join("test.html")).unwrap();
    assert!(
        html.contains("<style") || out_dir.path().join("style.css").exists(),
        "Should include CSS"
    );
}

#[test]
fn test_export_site_preserves_directory_structure() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    let subdir = dir.path().join("projects");
    fs::create_dir(&subdir).unwrap();
    fs::write(subdir.join("project1.org"), "* Project 1\n").unwrap();

    export_site(dir.path(), out_dir.path()).unwrap();

    assert!(
        out_dir.path().join("projects/project1.html").exists(),
        "Should preserve directory structure"
    );
}

#[test]
fn test_export_site_resolves_id_links() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    fs::write(
        dir.path().join("source.org"),
        "* Source\nSee [[id:target-id][Target Entry]].\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("target.org"),
        "* Target Entry\n:PROPERTIES:\n:ID: target-id\n:END:\nContent here.\n",
    )
    .unwrap();

    export_site(dir.path(), out_dir.path()).unwrap();

    let source_html = fs::read_to_string(out_dir.path().join("source.html")).unwrap();
    assert!(
        source_html.contains("href=\"target.html#target-id\""),
        "ID link should resolve to target.html#target-id, got: {}",
        source_html
    );
}

#[test]
fn test_export_site_resolves_cross_directory_id_links() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    let subdir = dir.path().join("sub");
    fs::create_dir(&subdir).unwrap();

    fs::write(
        dir.path().join("root.org"),
        "* Root\nSee [[id:sub-id][Sub Entry]].\n",
    )
    .unwrap();
    fs::write(
        subdir.join("nested.org"),
        "* Sub Entry\n:PROPERTIES:\n:ID: sub-id\n:END:\n",
    )
    .unwrap();

    export_site(dir.path(), out_dir.path()).unwrap();

    let root_html = fs::read_to_string(out_dir.path().join("root.html")).unwrap();
    assert!(
        root_html.contains("sub/nested.html#sub-id"),
        "Cross-directory ID link should use relative path, got: {}",
        root_html
    );
}

#[test]
fn test_export_site_unresolved_id_links_dont_crash() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    fs::write(
        dir.path().join("broken.org"),
        "* Source\nSee [[id:nonexistent][Missing]].\n",
    )
    .unwrap();

    // Should not panic
    let result = export_site(dir.path(), out_dir.path());
    assert!(result.is_ok(), "Export should succeed even with broken ID links");

    let html = fs::read_to_string(out_dir.path().join("broken.html")).unwrap();
    assert!(
        html.contains("Missing"),
        "Broken link text should still appear"
    );
}

#[test]
fn test_export_site_external_links_preserved() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    fs::write(
        dir.path().join("links.org"),
        "* External\nVisit [[https://example.com][Example]].\n",
    )
    .unwrap();

    export_site(dir.path(), out_dir.path()).unwrap();

    let html = fs::read_to_string(out_dir.path().join("links.html")).unwrap();
    assert!(html.contains("href=\"https://example.com\""));
}

#[test]
fn test_export_site_title_from_preamble() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    fs::write(
        dir.path().join("titled.org"),
        "#+TITLE: My Great Document\n* Heading\n",
    )
    .unwrap();

    export_site(dir.path(), out_dir.path()).unwrap();

    let html = fs::read_to_string(out_dir.path().join("titled.html")).unwrap();
    assert!(html.contains("<title>My Great Document</title>"));
}

#[test]
fn test_export_site_generates_index() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    fs::write(
        dir.path().join("page1.org"),
        "#+TITLE: Page One\n* Content\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("page2.org"),
        "#+TITLE: Page Two\n* Content\n",
    )
    .unwrap();

    export_site(dir.path(), out_dir.path()).unwrap();

    // Should generate an index.html with links to all pages
    assert!(
        out_dir.path().join("index.html").exists(),
        "Should generate index.html"
    );
    let index_html = fs::read_to_string(out_dir.path().join("index.html")).unwrap();
    assert!(
        index_html.contains("page1.html"),
        "Index should link to page1.html"
    );
    assert!(
        index_html.contains("page2.html"),
        "Index should link to page2.html"
    );
}

#[test]
fn test_export_site_empty_directory() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    let result = export_site(dir.path(), out_dir.path());
    assert!(
        result.is_ok(),
        "Export should succeed on empty directory"
    );
}

#[test]
fn test_export_site_ignores_non_org_files() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    fs::write(dir.path().join("notes.org"), "* Heading\n").unwrap();
    fs::write(dir.path().join("readme.md"), "# Not org\n").unwrap();
    fs::write(dir.path().join("data.txt"), "plain text\n").unwrap();

    export_site(dir.path(), out_dir.path()).unwrap();

    assert!(out_dir.path().join("notes.html").exists());
    assert!(!out_dir.path().join("readme.html").exists());
    assert!(!out_dir.path().join("data.html").exists());
}

// ==================== CLI Export Subcommand ====================

#[test]
fn test_export_cli_subcommand() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    fs::write(dir.path().join("test.org"), "* Heading\nBody text.\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_org-cli"))
        .args([
            "export",
            dir.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Export command should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        out_dir.path().join("test.html").exists(),
        "Should create test.html"
    );
}

#[test]
fn test_export_cli_with_id_resolution() {
    let dir = tempdir().unwrap();
    let out_dir = tempdir().unwrap();

    fs::write(
        dir.path().join("a.org"),
        "* Source\nLink to [[id:b-id][entry B]].\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("b.org"),
        "* Entry B\n:PROPERTIES:\n:ID: b-id\n:END:\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_org-cli"))
        .args([
            "export",
            dir.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    let a_html = fs::read_to_string(out_dir.path().join("a.html")).unwrap();
    assert!(
        a_html.contains("b.html#b-id"),
        "Should resolve id link, got: {}",
        a_html
    );
}
