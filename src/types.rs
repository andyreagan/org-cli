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

impl Link {
    /// Returns true if this is an org-id link (url starts with "id:")
    pub fn is_id_link(&self) -> bool {
        self.url.starts_with("id:")
    }

    /// Returns the ID value if this is an id link, stripping the "id:" prefix
    pub fn id_value(&self) -> Option<&str> {
        if self.is_id_link() {
            Some(&self.url[3..])
        } else {
            None
        }
    }
}

/// Represents a fragment of inline markup within text
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineFragment {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    Verbatim(String),
    Strikethrough(String),
    Underline(String),
    Link(Link),
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

    /// Returns the org-id (:ID: property) of this entry, if any
    pub fn id(&self) -> Option<&str> {
        self.properties.get("ID").map(|s| s.as_str())
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

    /// Extract a #+KEYWORD value from the preamble
    pub fn keyword_value(&self, keyword: &str) -> Option<&str> {
        let prefix = format!("#+{}:", keyword);
        for line in self.preamble.lines() {
            let trimmed = line.trim();
            if trimmed.to_uppercase().starts_with(&prefix.to_uppercase()) {
                let value = trimmed[prefix.len()..].trim();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
        None
    }

    /// Returns the #+TITLE value if present in the preamble
    pub fn title(&self) -> Option<&str> {
        self.keyword_value("TITLE")
    }

    /// Returns the #+AUTHOR value if present in the preamble
    pub fn author(&self) -> Option<&str> {
        self.keyword_value("AUTHOR")
    }

    /// Parse #+OPTIONS: line and return value for a specific option key
    pub fn option_value(&self, key: &str) -> Option<String> {
        // OPTIONS can appear multiple times, all are merged
        for line in self.preamble.lines() {
            let trimmed = line.trim();
            if trimmed.to_uppercase().starts_with("#+OPTIONS:") {
                let opts = trimmed[10..].trim();
                for part in opts.split_whitespace() {
                    if let Some(colon) = part.find(':') {
                        let k = &part[..colon];
                        let v = &part[colon + 1..];
                        if k == key {
                            return Some(v.to_string());
                        }
                    }
                }
            }
        }
        None
    }
}

impl Default for OrgDocument {
    fn default() -> Self {
        Self::new()
    }
}
