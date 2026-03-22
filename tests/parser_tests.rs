use org_cli::parser::*;
use org_cli::types::*;

// ==================== Heading Tests ====================

#[test]
fn test_parse_simple_heading() {
    let input = "* Simple heading\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries.len(), 1);
    assert_eq!(doc.entries[0].level, 1);
    assert_eq!(doc.entries[0].title, "Simple heading");
}

#[test]
fn test_parse_heading_depth_2() {
    let input = "** Second level heading\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries.len(), 1);
    assert_eq!(doc.entries[0].level, 2);
    assert_eq!(doc.entries[0].title, "Second level heading");
}

#[test]
fn test_parse_heading_depth_5() {
    let input = "***** Fifth level\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].level, 5);
    assert_eq!(doc.entries[0].title, "Fifth level");
}

#[test]
fn test_parse_multiple_headings() {
    let input = "* First\n** Second\n*** Third\n* Another first\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries.len(), 4);
    assert_eq!(doc.entries[0].level, 1);
    assert_eq!(doc.entries[1].level, 2);
    assert_eq!(doc.entries[2].level, 3);
    assert_eq!(doc.entries[3].level, 1);
}

// ==================== TODO Keyword Tests ====================

#[test]
fn test_parse_todo_keyword() {
    let input = "* TODO Task to do\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].keyword, Some(Keyword::Todo));
    assert_eq!(doc.entries[0].title, "Task to do");
}

#[test]
fn test_parse_done_keyword() {
    let input = "* DONE Completed task\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].keyword, Some(Keyword::Done));
    assert_eq!(doc.entries[0].title, "Completed task");
}

#[test]
fn test_parse_next_keyword() {
    let input = "* NEXT Up next\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].keyword, Some(Keyword::Next));
}

#[test]
fn test_parse_waiting_keyword() {
    let input = "* WAITING On hold\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].keyword, Some(Keyword::Waiting));
}

#[test]
fn test_parse_cancelled_keyword() {
    let input = "* CANCELLED Not doing this\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].keyword, Some(Keyword::Cancelled));
}

#[test]
fn test_parse_in_progress_keyword() {
    let input = "* IN-PROGRESS Working on it\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].keyword, Some(Keyword::InProgress));
}

#[test]
fn test_parse_heading_without_keyword() {
    let input = "* Regular heading\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].keyword, None);
}

// ==================== Tag Tests ====================

#[test]
fn test_parse_single_tag() {
    let input = "* Heading :tag1:\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].tags, vec!["tag1"]);
    assert_eq!(doc.entries[0].title, "Heading");
}

#[test]
fn test_parse_multiple_tags() {
    let input = "* Heading :tag1:tag2:tag3:\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].tags, vec!["tag1", "tag2", "tag3"]);
}

#[test]
fn test_parse_todo_with_tags() {
    let input = "* TODO Task :work:urgent:\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].keyword, Some(Keyword::Todo));
    assert_eq!(doc.entries[0].title, "Task");
    assert_eq!(doc.entries[0].tags, vec!["work", "urgent"]);
}

#[test]
fn test_parse_tags_with_special_chars() {
    let input = "* Heading :@home:work_stuff:tag#1:\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].tags, vec!["@home", "work_stuff", "tag#1"]);
}

// ==================== Timestamp Tests ====================

#[test]
fn test_parse_active_timestamp_date_only() {
    let input = "* Meeting\n<2026-03-21 Sat>\n";
    let doc = parse_org_document(input).unwrap();
    assert!(!doc.entries[0].timestamps.is_empty());
    let ts = &doc.entries[0].timestamps[0];
    assert!(ts.active);
    assert_eq!(ts.date.year, 2026);
    assert_eq!(ts.date.month, 3);
    assert_eq!(ts.date.day, 21);
    assert_eq!(ts.date.weekday.as_deref(), Some("Sat"));
}

#[test]
fn test_parse_active_timestamp_with_time() {
    let input = "* Meeting\n<2026-03-21 Sat 14:30>\n";
    let doc = parse_org_document(input).unwrap();
    let ts = &doc.entries[0].timestamps[0];
    assert!(ts.active);
    assert_eq!(ts.time, Some(Time { hour: 14, minute: 30 }));
}

#[test]
fn test_parse_inactive_timestamp() {
    let input = "* Note\n[2026-03-21 Sat]\n";
    let doc = parse_org_document(input).unwrap();
    let ts = &doc.entries[0].timestamps[0];
    assert!(!ts.active);
    assert_eq!(ts.date.year, 2026);
}

#[test]
fn test_parse_scheduled() {
    let input = "* TODO Task\nSCHEDULED: <2026-03-21 Sat>\n";
    let doc = parse_org_document(input).unwrap();
    assert!(doc.entries[0].scheduled.is_some());
    let sched = doc.entries[0].scheduled.as_ref().unwrap();
    assert_eq!(sched.date.year, 2026);
    assert_eq!(sched.date.month, 3);
    assert_eq!(sched.date.day, 21);
}

#[test]
fn test_parse_deadline() {
    let input = "* TODO Task\nDEADLINE: <2026-03-25 Wed>\n";
    let doc = parse_org_document(input).unwrap();
    assert!(doc.entries[0].deadline.is_some());
    let dl = doc.entries[0].deadline.as_ref().unwrap();
    assert_eq!(dl.date.year, 2026);
    assert_eq!(dl.date.month, 3);
    assert_eq!(dl.date.day, 25);
}

#[test]
fn test_parse_closed() {
    let input = "* DONE Completed\nCLOSED: [2026-03-20 Fri 15:30]\n";
    let doc = parse_org_document(input).unwrap();
    assert!(doc.entries[0].closed.is_some());
    let closed = doc.entries[0].closed.as_ref().unwrap();
    assert_eq!(closed.date.year, 2026);
    assert_eq!(closed.time, Some(Time { hour: 15, minute: 30 }));
}

#[test]
fn test_parse_scheduled_and_deadline() {
    let input = "* TODO Task\nSCHEDULED: <2026-03-21 Sat> DEADLINE: <2026-03-25 Wed>\n";
    let doc = parse_org_document(input).unwrap();
    assert!(doc.entries[0].scheduled.is_some());
    assert!(doc.entries[0].deadline.is_some());
}

// ==================== Properties Drawer Tests ====================

#[test]
fn test_parse_properties_drawer() {
    let input = "* Task\n:PROPERTIES:\n:ID: abc123\n:END:\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].properties.get("ID"), Some(&"abc123".to_string()));
}

#[test]
fn test_parse_multiple_properties() {
    let input = "* Task\n:PROPERTIES:\n:Title: My Title\n:Author: John\n:Year: 2026\n:END:\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].properties.get("Title"), Some(&"My Title".to_string()));
    assert_eq!(doc.entries[0].properties.get("Author"), Some(&"John".to_string()));
    assert_eq!(doc.entries[0].properties.get("Year"), Some(&"2026".to_string()));
}

#[test]
fn test_properties_after_planning() {
    let input = "* TODO Task\nSCHEDULED: <2026-03-21 Sat>\n:PROPERTIES:\n:ID: 123\n:END:\n";
    let doc = parse_org_document(input).unwrap();
    assert!(doc.entries[0].scheduled.is_some());
    assert_eq!(doc.entries[0].properties.get("ID"), Some(&"123".to_string()));
}

// ==================== Priority Tests ====================

#[test]
fn test_parse_priority_a() {
    let input = "* TODO [#A] High priority task\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].priority, Some(Priority::A));
    assert_eq!(doc.entries[0].title, "High priority task");
}

#[test]
fn test_parse_priority_b() {
    let input = "* TODO [#B] Medium priority task\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].priority, Some(Priority::B));
}

#[test]
fn test_parse_priority_c() {
    let input = "* TODO [#C] Low priority task\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].priority, Some(Priority::C));
}

#[test]
fn test_parse_priority_with_tags() {
    let input = "* TODO [#A] Task :important:\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].priority, Some(Priority::A));
    assert_eq!(doc.entries[0].tags, vec!["important"]);
    assert_eq!(doc.entries[0].title, "Task");
}

// ==================== Link Tests ====================

#[test]
fn test_parse_link_with_description() {
    let input = "* See [[https://example.com][Example Site]]\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].links.len(), 1);
    assert_eq!(doc.entries[0].links[0].url, "https://example.com");
    assert_eq!(doc.entries[0].links[0].description, Some("Example Site".to_string()));
}

#[test]
fn test_parse_link_without_description() {
    let input = "* Check [[https://orgmode.org]]\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].links.len(), 1);
    assert_eq!(doc.entries[0].links[0].url, "https://orgmode.org");
    assert_eq!(doc.entries[0].links[0].description, None);
}

#[test]
fn test_parse_file_link() {
    let input = "* Open [[file:~/docs/notes.org]]\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].links[0].url, "file:~/docs/notes.org");
}

#[test]
fn test_parse_multiple_links() {
    let input = "* Links [[https://a.com][A]] and [[https://b.com][B]]\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries[0].links.len(), 2);
}

// ==================== Body Text Tests ====================

#[test]
fn test_parse_body_text() {
    let input = "* Heading\nSome body text here.\nMore text on another line.\n";
    let doc = parse_org_document(input).unwrap();
    assert!(doc.entries[0].body.contains("Some body text here."));
    assert!(doc.entries[0].body.contains("More text on another line."));
}

#[test]
fn test_body_text_separate_from_next_heading() {
    let input = "* First\nBody of first.\n* Second\nBody of second.\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries.len(), 2);
    assert!(doc.entries[0].body.contains("Body of first."));
    assert!(doc.entries[1].body.contains("Body of second."));
}

#[test]
fn test_body_text_not_include_properties() {
    let input = "* Task\n:PROPERTIES:\n:ID: 123\n:END:\nActual body text.\n";
    let doc = parse_org_document(input).unwrap();
    assert!(!doc.entries[0].body.contains("PROPERTIES"));
    assert!(doc.entries[0].body.contains("Actual body text."));
}

// ==================== Round-trip Tests ====================

#[test]
fn test_roundtrip_simple_heading() {
    let input = "* Simple heading\n";
    let doc = parse_org_document(input).unwrap();
    let output = serialize_org_document(&doc);
    assert_eq!(input, output);
}

#[test]
fn test_roundtrip_todo_with_tags() {
    let input = "* TODO Task :work:urgent:\n";
    let doc = parse_org_document(input).unwrap();
    let output = serialize_org_document(&doc);
    assert_eq!(input, output);
}

#[test]
fn test_roundtrip_with_priority() {
    let input = "* TODO [#A] Important task\n";
    let doc = parse_org_document(input).unwrap();
    let output = serialize_org_document(&doc);
    assert_eq!(input, output);
}

#[test]
fn test_roundtrip_with_scheduling() {
    let input = "* TODO Task\nSCHEDULED: <2026-03-21 Sat>\n";
    let doc = parse_org_document(input).unwrap();
    let output = serialize_org_document(&doc);
    assert_eq!(input, output);
}

#[test]
fn test_roundtrip_with_deadline() {
    let input = "* TODO Task\nDEADLINE: <2026-03-25 Wed>\n";
    let doc = parse_org_document(input).unwrap();
    let output = serialize_org_document(&doc);
    assert_eq!(input, output);
}

#[test]
fn test_roundtrip_with_closed() {
    let input = "* DONE Task\nCLOSED: [2026-03-20 Fri 15:30]\n";
    let doc = parse_org_document(input).unwrap();
    let output = serialize_org_document(&doc);
    assert_eq!(input, output);
}

#[test]
fn test_roundtrip_with_properties() {
    let input = "* Task\n:PROPERTIES:\n:ID: abc123\n:END:\n";
    let doc = parse_org_document(input).unwrap();
    let output = serialize_org_document(&doc);
    assert_eq!(input, output);
}

#[test]
fn test_roundtrip_with_body() {
    let input = "* Heading\nSome body text.\n";
    let doc = parse_org_document(input).unwrap();
    let output = serialize_org_document(&doc);
    assert_eq!(input, output);
}

#[test]
fn test_roundtrip_complex_entry() {
    let input = "* TODO [#A] Complex task :work:urgent:\nSCHEDULED: <2026-03-21 Sat> DEADLINE: <2026-03-25 Wed>\n:PROPERTIES:\n:ID: task-001\n:END:\nThis is the body text.\n";
    let doc = parse_org_document(input).unwrap();
    let output = serialize_org_document(&doc);
    assert_eq!(input, output);
}

#[test]
fn test_roundtrip_multiple_headings() {
    let input = "* First heading\n** Sub heading\n*** Deep heading\n* Second heading\n";
    let doc = parse_org_document(input).unwrap();
    let output = serialize_org_document(&doc);
    assert_eq!(input, output);
}

#[test]
fn test_roundtrip_preserves_preamble() {
    let input = "#+TITLE: My Document\n\n* First heading\n";
    let doc = parse_org_document(input).unwrap();
    let output = serialize_org_document(&doc);
    assert_eq!(input, output);
}

// ==================== Edge Cases ====================

#[test]
fn test_empty_document() {
    let input = "";
    let doc = parse_org_document(input).unwrap();
    assert!(doc.entries.is_empty());
}

#[test]
fn test_document_with_only_preamble() {
    let input = "#+TITLE: Just a title\n#+AUTHOR: Someone\n";
    let doc = parse_org_document(input).unwrap();
    assert!(doc.entries.is_empty());
    assert!(doc.preamble.contains("#+TITLE:"));
}

#[test]
fn test_heading_with_no_content() {
    let input = "* Empty\n* Another\n";
    let doc = parse_org_document(input).unwrap();
    assert_eq!(doc.entries.len(), 2);
    assert!(doc.entries[0].body.is_empty());
}

#[test]
fn test_timestamp_with_time_range() {
    let input = "* Meeting\n<2026-03-21 Sat 10:00-12:00>\n";
    let doc = parse_org_document(input).unwrap();
    let ts = &doc.entries[0].timestamps[0];
    assert!(ts.active);
    assert!(ts.time.is_some());
}
