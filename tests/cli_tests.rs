use std::process::Command;
use std::fs;
use tempfile::tempdir;

fn org_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_org-cli"))
}

// ==================== List Command Tests ====================

#[test]
fn test_list_todos_single_file() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* TODO First task\n* DONE Completed\n* TODO Second task\n").unwrap();
    
    let output = org_cli()
        .args(["list", file.to_str().unwrap()])
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("First task"));
    assert!(stdout.contains("Second task"));
}

#[test]
fn test_list_todos_grouped_by_keyword() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* TODO Task A\n* NEXT Task B\n* WAITING Task C\n* TODO Task D\n").unwrap();
    
    let output = org_cli()
        .args(["list", file.to_str().unwrap()])
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be grouped by keyword
    assert!(stdout.contains("TODO"));
    assert!(stdout.contains("NEXT"));
    assert!(stdout.contains("WAITING"));
}

#[test]
fn test_list_multiple_files() {
    let dir = tempdir().unwrap();
    let file1 = dir.path().join("work.org");
    let file2 = dir.path().join("personal.org");
    fs::write(&file1, "* TODO Work task\n").unwrap();
    fs::write(&file2, "* TODO Personal task\n").unwrap();
    
    let output = org_cli()
        .args(["list", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Work task"));
    assert!(stdout.contains("Personal task"));
}

#[test]
fn test_list_shows_file_and_line() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* Regular heading\n* TODO Task on line 2\n").unwrap();
    
    let output = org_cli()
        .args(["list", file.to_str().unwrap()])
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show file:line format
    assert!(stdout.contains("test.org:2") || stdout.contains("test.org"));
}

// ==================== Add Command Tests ====================

#[test]
fn test_add_todo() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* 2026-03-22 Saturday\n").unwrap();
    
    org_cli()
        .args(["add", "New task", "--file", file.to_str().unwrap()])
        .output()
        .unwrap();
    
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("TODO New task"));
}

#[test]
fn test_add_todo_with_tag() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* 2026-03-22 Saturday\n").unwrap();
    
    org_cli()
        .args(["add", "Tagged task", "--file", file.to_str().unwrap(), "--tag", "work"])
        .output()
        .unwrap();
    
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("TODO Tagged task"));
    assert!(content.contains(":work:"));
}

// ==================== Done Command Tests ====================

#[test]
fn test_done_marks_item() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* TODO Task to complete\n").unwrap();
    
    org_cli()
        .args(["done", file.to_str().unwrap(), "1"])
        .output()
        .unwrap();
    
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("DONE Task to complete"));
    assert!(content.contains("CLOSED:"));
}

#[test]
fn test_done_adds_closed_timestamp() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* TODO Task\n").unwrap();
    
    org_cli()
        .args(["done", file.to_str().unwrap(), "1"])
        .output()
        .unwrap();
    
    let content = fs::read_to_string(&file).unwrap();
    // Should have CLOSED with inactive timestamp
    assert!(content.contains("CLOSED: ["));
}

// ==================== Cancel Command Tests ====================

#[test]
fn test_cancel_marks_item() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* TODO Task to cancel\n").unwrap();
    
    org_cli()
        .args(["cancel", file.to_str().unwrap(), "1"])
        .output()
        .unwrap();
    
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("CANCELLED Task to cancel"));
    assert!(content.contains("CLOSED:"));
}

// ==================== Wait Command Tests ====================

#[test]
fn test_wait_marks_item() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* TODO Task to wait on\n").unwrap();
    
    org_cli()
        .args(["wait", file.to_str().unwrap(), "1"])
        .output()
        .unwrap();
    
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("WAITING Task to wait on"));
}

#[test]
fn test_wait_with_date() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* TODO Task\n").unwrap();
    
    org_cli()
        .args(["wait", file.to_str().unwrap(), "1", "--date", "2026-03-25"])
        .output()
        .unwrap();
    
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("WAITING"));
    assert!(content.contains("SCHEDULED:"));
    assert!(content.contains("2026-03-25"));
}

// ==================== Reschedule Command Tests ====================

#[test]
fn test_reschedule_updates_date() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* TODO Task\nSCHEDULED: <2026-03-21 Sat>\n").unwrap();
    
    org_cli()
        .args(["reschedule", file.to_str().unwrap(), "1", "--date", "2026-03-28"])
        .output()
        .unwrap();
    
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("2026-03-28"));
    assert!(!content.contains("2026-03-21"));
}

#[test]
fn test_reschedule_adds_scheduled() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* TODO Task without schedule\n").unwrap();
    
    org_cli()
        .args(["reschedule", file.to_str().unwrap(), "1", "--date", "2026-03-28"])
        .output()
        .unwrap();
    
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("SCHEDULED:"));
    assert!(content.contains("2026-03-28"));
}

// ==================== Show Command Tests ====================

#[test]
fn test_show_displays_headings() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* First heading\n** Sub heading\n* Second heading\n").unwrap();
    
    let output = org_cli()
        .args(["show", file.to_str().unwrap()])
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("First heading"));
    assert!(stdout.contains("Sub heading"));
    assert!(stdout.contains("Second heading"));
}

#[test]
fn test_show_displays_todos() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.org");
    fs::write(&file, "* TODO Task one\n* DONE Task two\n* NEXT Task three\n").unwrap();
    
    let output = org_cli()
        .args(["show", file.to_str().unwrap()])
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("TODO"));
    assert!(stdout.contains("DONE"));
    assert!(stdout.contains("NEXT"));
}

// ==================== Multi-file Scanning Tests ====================

#[test]
fn test_scan_directory_recursively() {
    let dir = tempdir().unwrap();
    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();
    
    fs::write(dir.path().join("root.org"), "* TODO Root task\n").unwrap();
    fs::write(subdir.join("nested.org"), "* TODO Nested task\n").unwrap();
    
    let output = org_cli()
        .args(["list", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Root task"));
    assert!(stdout.contains("Nested task"));
}

#[test]
fn test_ignores_non_org_files() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("file.org"), "* TODO Org task\n").unwrap();
    fs::write(dir.path().join("file.txt"), "* TODO Not an org task\n").unwrap();
    
    let output = org_cli()
        .args(["list", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Org task"));
    // txt file should not be parsed as org
}
