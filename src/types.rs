use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Keyword {
    Todo,
    Done,
    Next,
    Waiting,
    Cancelled,
    InProgress,
}

impl Keyword {
    pub fn as_str(&self) -> &'static str {
        match self {
            Keyword::Todo => "TODO",
            Keyword::Done => "DONE",
            Keyword::Next => "NEXT",
            Keyword::Waiting => "WAITING",
            Keyword::Cancelled => "CANCELLED",
            Keyword::InProgress => "IN-PROGRESS",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "TODO" => Some(Keyword::Todo),
            "DONE" => Some(Keyword::Done),
            "NEXT" => Some(Keyword::Next),
            "WAITING" => Some(Keyword::Waiting),
            "CANCELLED" => Some(Keyword::Cancelled),
            "IN-PROGRESS" => Some(Keyword::InProgress),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    A,
    B,
    C,
}

impl Priority {
    pub fn as_char(&self) -> char {
        match self {
            Priority::A => 'A',
            Priority::B => 'B',
            Priority::C => 'C',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'A' => Some(Priority::A),
            'B' => Some(Priority::B),
            'C' => Some(Priority::C),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub weekday: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Time {
    pub hour: u32,
    pub minute: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timestamp {
    pub active: bool,
    pub date: Date,
    pub time: Option<Time>,
    pub end_time: Option<Time>,
    pub repeater: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrgEntry {
    pub level: usize,
    pub keyword: Option<Keyword>,
    pub priority: Option<Priority>,
    pub title: String,
    pub tags: Vec<String>,
    pub scheduled: Option<Timestamp>,
    pub deadline: Option<Timestamp>,
    pub closed: Option<Timestamp>,
    pub timestamps: Vec<Timestamp>,
    pub properties: HashMap<String, String>,
    pub links: Vec<Link>,
    pub body: String,
    pub line_number: usize,
}

impl OrgEntry {
    pub fn new(level: usize, title: String) -> Self {
        OrgEntry {
            level,
            keyword: None,
            priority: None,
            title,
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            closed: None,
            timestamps: Vec::new(),
            properties: HashMap::new(),
            links: Vec::new(),
            body: String::new(),
            line_number: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrgDocument {
    pub preamble: String,
    pub entries: Vec<OrgEntry>,
}

impl OrgDocument {
    pub fn new() -> Self {
        OrgDocument {
            preamble: String::new(),
            entries: Vec::new(),
        }
    }
}

impl Default for OrgDocument {
    fn default() -> Self {
        Self::new()
    }
}
