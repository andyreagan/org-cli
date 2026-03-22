use crate::parser::{parse_org_document, serialize_org_document};
use crate::types::*;
use anyhow::{Context, Result};
use chrono::{Datelike, Local, NaiveDate, Timelike};
use colored::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ==================== File Operations ====================

pub fn find_org_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    if path.is_file() {
        if path.extension().map_or(false, |ext| ext == "org") {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                files.extend(find_org_files(&entry_path)?);
            } else if entry_path.extension().map_or(false, |ext| ext == "org") {
                files.push(entry_path);
            }
        }
    }
    
    Ok(files)
}

pub fn read_and_parse_file(path: &Path) -> Result<OrgDocument> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    parse_org_document(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", path.display(), e))
}

pub fn write_document(path: &Path, doc: &OrgDocument) -> Result<()> {
    let content = serialize_org_document(doc);
    fs::write(path, content)
        .with_context(|| format!("Failed to write file: {}", path.display()))?;
    Ok(())
}

// ==================== List Command ====================

#[derive(Debug)]
pub struct TodoItem {
    pub file: PathBuf,
    pub line: usize,
    pub keyword: Keyword,
    pub priority: Option<Priority>,
    pub title: String,
    pub tags: Vec<String>,
    pub scheduled: Option<Timestamp>,
    pub deadline: Option<Timestamp>,
}

pub fn list_todos(path: &Path) -> Result<()> {
    let files = find_org_files(path)?;
    let mut todos: HashMap<String, Vec<TodoItem>> = HashMap::new();
    
    for file in files {
        let doc = match read_and_parse_file(&file) {
            Ok(doc) => doc,
            Err(e) => {
                eprintln!("Warning: {}", e);
                continue;
            }
        };
        
        for entry in &doc.entries {
            if let Some(ref keyword) = entry.keyword {
                // Skip DONE and CANCELLED for listing
                match keyword {
                    Keyword::Done | Keyword::Cancelled => continue,
                    _ => {}
                }
                
                let item = TodoItem {
                    file: file.clone(),
                    line: entry.line_number,
                    keyword: keyword.clone(),
                    priority: entry.priority,
                    title: entry.title.clone(),
                    tags: entry.tags.clone(),
                    scheduled: entry.scheduled.clone(),
                    deadline: entry.deadline.clone(),
                };
                
                todos.entry(keyword.as_str().to_string())
                    .or_default()
                    .push(item);
            }
        }
    }
    
    // Print grouped by keyword
    let keyword_order = ["TODO", "NEXT", "IN-PROGRESS", "WAITING"];
    
    for kw in keyword_order {
        if let Some(items) = todos.get(kw) {
            println!("{}", format!("── {} ──", kw).bold().yellow());
            for item in items {
                let file_name = item.file.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?");
                
                let location = format!("{}:{}", file_name, item.line).dimmed();
                
                let priority_str = match item.priority {
                    Some(Priority::A) => "[#A] ".red().to_string(),
                    Some(Priority::B) => "[#B] ".yellow().to_string(),
                    Some(Priority::C) => "[#C] ".blue().to_string(),
                    None => String::new(),
                };
                
                let tags_str = if item.tags.is_empty() {
                    String::new()
                } else {
                    format!(" :{}: ", item.tags.join(":")).cyan().to_string()
                };
                
                let schedule_str = if let Some(ref sched) = item.scheduled {
                    format!(" [{}]", format_date_short(&sched.date)).green().to_string()
                } else {
                    String::new()
                };
                
                let deadline_str = if let Some(ref dl) = item.deadline {
                    format!(" DL:{}", format_date_short(&dl.date)).red().to_string()
                } else {
                    String::new()
                };
                
                println!("  {} {}{}{}{}{}", 
                    location,
                    priority_str,
                    item.title,
                    tags_str,
                    schedule_str,
                    deadline_str
                );
            }
            println!();
        }
    }
    
    Ok(())
}

fn format_date_short(date: &Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

// ==================== Add Command ====================

pub fn add_todo(text: &str, file: &Path, tag: Option<&str>) -> Result<()> {
    let mut doc = if file.exists() {
        read_and_parse_file(file)?
    } else {
        OrgDocument::new()
    };
    
    // Find today's daily entry or create one
    let today = Local::now().date_naive();
    let today_str = today.format("%Y-%m-%d").to_string();
    let weekday = today.format("%A").to_string();
    let today_heading = format!("{} {}", today_str, weekday);
    
    // Find if today's entry exists
    let today_idx = doc.entries.iter().position(|e| {
        e.level == 1 && e.title.starts_with(&today_str)
    });
    
    // Create the new TODO entry
    let mut new_entry = OrgEntry::new(2, text.to_string());
    new_entry.keyword = Some(Keyword::Todo);
    if let Some(t) = tag {
        new_entry.tags = vec![t.to_string()];
    }
    
    match today_idx {
        Some(idx) => {
            // Find where to insert (after today's entry and its children)
            let mut insert_idx = idx + 1;
            while insert_idx < doc.entries.len() && doc.entries[insert_idx].level > 1 {
                insert_idx += 1;
            }
            doc.entries.insert(insert_idx, new_entry);
        }
        None => {
            // Create today's entry first
            let day_entry = OrgEntry::new(1, today_heading);
            doc.entries.push(day_entry);
            doc.entries.push(new_entry);
        }
    }
    
    write_document(file, &doc)?;
    println!("Added TODO: {}", text);
    Ok(())
}

// ==================== Done Command ====================

pub fn mark_done(file: &Path, line: usize) -> Result<()> {
    let mut doc = read_and_parse_file(file)?;
    
    // Find the entry index
    let entry_idx = doc.entries.iter()
        .position(|e| e.line_number == line)
        .ok_or_else(|| anyhow::anyhow!("No entry found at line {}", line))?;
    
    let now = Local::now();
    let entry = &mut doc.entries[entry_idx];
    entry.keyword = Some(Keyword::Done);
    
    // Add CLOSED timestamp
    entry.closed = Some(Timestamp {
        active: false,
        date: Date {
            year: now.year(),
            month: now.month(),
            day: now.day(),
            weekday: Some(now.format("%a").to_string()),
        },
        time: Some(Time {
            hour: now.hour(),
            minute: now.minute(),
        }),
        end_time: None,
        repeater: None,
    });
    
    let title = entry.title.clone();
    write_document(file, &doc)?;
    println!("Marked DONE: {}", title);
    Ok(())
}

// ==================== Cancel Command ====================

pub fn mark_cancelled(file: &Path, line: usize) -> Result<()> {
    let mut doc = read_and_parse_file(file)?;
    
    // Find the entry index
    let entry_idx = doc.entries.iter()
        .position(|e| e.line_number == line)
        .ok_or_else(|| anyhow::anyhow!("No entry found at line {}", line))?;
    
    let now = Local::now();
    let entry = &mut doc.entries[entry_idx];
    entry.keyword = Some(Keyword::Cancelled);
    
    // Add CLOSED timestamp
    entry.closed = Some(Timestamp {
        active: false,
        date: Date {
            year: now.year(),
            month: now.month(),
            day: now.day(),
            weekday: Some(now.format("%a").to_string()),
        },
        time: Some(Time {
            hour: now.hour(),
            minute: now.minute(),
        }),
        end_time: None,
        repeater: None,
    });
    
    let title = entry.title.clone();
    write_document(file, &doc)?;
    println!("Marked CANCELLED: {}", title);
    Ok(())
}

// ==================== Wait Command ====================

pub fn mark_waiting(file: &Path, line: usize, date: Option<&str>) -> Result<()> {
    let mut doc = read_and_parse_file(file)?;
    
    // Find the entry index
    let entry_idx = doc.entries.iter()
        .position(|e| e.line_number == line)
        .ok_or_else(|| anyhow::anyhow!("No entry found at line {}", line))?;
    
    let entry = &mut doc.entries[entry_idx];
    entry.keyword = Some(Keyword::Waiting);
    
    // Add SCHEDULED if date provided
    if let Some(date_str) = date {
        let parsed_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .with_context(|| format!("Invalid date format: {}", date_str))?;
        
        entry.scheduled = Some(Timestamp {
            active: true,
            date: Date {
                year: parsed_date.year(),
                month: parsed_date.month(),
                day: parsed_date.day(),
                weekday: Some(parsed_date.format("%a").to_string()),
            },
            time: None,
            end_time: None,
            repeater: None,
        });
    }
    
    let title = entry.title.clone();
    write_document(file, &doc)?;
    println!("Marked WAITING: {}", title);
    Ok(())
}

// ==================== Reschedule Command ====================

pub fn reschedule(file: &Path, line: usize, date_str: &str) -> Result<()> {
    let mut doc = read_and_parse_file(file)?;
    
    // Find the entry index
    let entry_idx = doc.entries.iter()
        .position(|e| e.line_number == line)
        .ok_or_else(|| anyhow::anyhow!("No entry found at line {}", line))?;
    
    let parsed_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .with_context(|| format!("Invalid date format: {}", date_str))?;
    
    let entry = &mut doc.entries[entry_idx];
    entry.scheduled = Some(Timestamp {
        active: true,
        date: Date {
            year: parsed_date.year(),
            month: parsed_date.month(),
            day: parsed_date.day(),
            weekday: Some(parsed_date.format("%a").to_string()),
        },
        time: None,
        end_time: None,
        repeater: None,
    });
    
    let title = entry.title.clone();
    write_document(file, &doc)?;
    println!("Rescheduled to {}: {}", date_str, title);
    Ok(())
}

// ==================== Show Command ====================

pub fn show_file(file: &Path) -> Result<()> {
    let doc = read_and_parse_file(file)?;
    
    println!("{}", format!("File: {}", file.display()).bold());
    println!();
    
    for entry in &doc.entries {
        // Indentation based on level
        let indent = "  ".repeat(entry.level.saturating_sub(1));
        
        // Format the heading
        let keyword_str = match &entry.keyword {
            Some(Keyword::Todo) => "TODO ".yellow().bold().to_string(),
            Some(Keyword::Done) => "DONE ".green().bold().to_string(),
            Some(Keyword::Next) => "NEXT ".cyan().bold().to_string(),
            Some(Keyword::Waiting) => "WAITING ".magenta().bold().to_string(),
            Some(Keyword::Cancelled) => "CANCELLED ".red().bold().to_string(),
            Some(Keyword::InProgress) => "IN-PROGRESS ".blue().bold().to_string(),
            None => String::new(),
        };
        
        let priority_str = match entry.priority {
            Some(Priority::A) => "[#A] ".red().to_string(),
            Some(Priority::B) => "[#B] ".yellow().to_string(),
            Some(Priority::C) => "[#C] ".blue().to_string(),
            None => String::new(),
        };
        
        let stars = "*".repeat(entry.level).purple().to_string();
        
        let tags_str = if entry.tags.is_empty() {
            String::new()
        } else {
            format!(" :{}:", entry.tags.join(":")).cyan().to_string()
        };
        
        println!("{}{} {}{}{}{}", indent, stars, keyword_str, priority_str, entry.title, tags_str);
        
        // Show planning info
        if let Some(ref sched) = entry.scheduled {
            println!("{}  {} {}", indent, "SCHEDULED:".dimmed(), format_date_short(&sched.date).green());
        }
        if let Some(ref dl) = entry.deadline {
            println!("{}  {} {}", indent, "DEADLINE:".dimmed(), format_date_short(&dl.date).red());
        }
        if let Some(ref closed) = entry.closed {
            println!("{}  {} {}", indent, "CLOSED:".dimmed(), format_date_short(&closed.date));
        }
    }
    
    Ok(())
}
