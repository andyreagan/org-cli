use crate::types::*;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while, take_while1},
    character::complete::{char, digit1, line_ending, space0, space1},
    combinator::{map, map_res, opt, recognize},
    multi::separated_list1,
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};
use std::collections::HashMap;

// ==================== Timestamp Parsing ====================

fn parse_weekday(input: &str) -> IResult<&str, &str> {
    alt((
        tag("Mon"),
        tag("Tue"),
        tag("Wed"),
        tag("Thu"),
        tag("Fri"),
        tag("Sat"),
        tag("Sun"),
    ))(input)
}

fn parse_date(input: &str) -> IResult<&str, Date> {
    let (input, year) = map_res(digit1, |s: &str| s.parse::<i32>())(input)?;
    let (input, _) = char('-')(input)?;
    let (input, month) = map_res(digit1, |s: &str| s.parse::<u32>())(input)?;
    let (input, _) = char('-')(input)?;
    let (input, day) = map_res(digit1, |s: &str| s.parse::<u32>())(input)?;
    let (input, weekday) = opt(preceded(space1, parse_weekday))(input)?;

    Ok((
        input,
        Date {
            year,
            month,
            day,
            weekday: weekday.map(|s| s.to_string()),
        },
    ))
}

fn parse_time(input: &str) -> IResult<&str, Time> {
    let (input, hour) = map_res(digit1, |s: &str| s.parse::<u32>())(input)?;
    let (input, _) = char(':')(input)?;
    let (input, minute) = map_res(digit1, |s: &str| s.parse::<u32>())(input)?;
    Ok((input, Time { hour, minute }))
}

fn parse_time_range(input: &str) -> IResult<&str, (Time, Option<Time>)> {
    let (input, start) = parse_time(input)?;
    let (input, end) = opt(preceded(char('-'), parse_time))(input)?;
    Ok((input, (start, end)))
}

fn parse_repeater(input: &str) -> IResult<&str, &str> {
    recognize(tuple((
        alt((tag("++"), tag(".+"), tag("+"))),
        digit1,
        alt((tag("d"), tag("w"), tag("m"), tag("y"))),
    )))(input)
}

fn parse_active_timestamp(input: &str) -> IResult<&str, Timestamp> {
    let (input, _) = char('<')(input)?;
    let (input, date) = parse_date(input)?;
    let (input, time_info) = opt(preceded(space1, parse_time_range))(input)?;
    let (input, repeater) = opt(preceded(space1, parse_repeater))(input)?;
    let (input, _) = opt(preceded(space0, take_while(|c: char| c != '>')))(input)?;
    let (input, _) = char('>')(input)?;

    let (time, end_time) = match time_info {
        Some((t, et)) => (Some(t), et),
        None => (None, None),
    };

    Ok((
        input,
        Timestamp {
            active: true,
            date,
            time,
            end_time,
            repeater: repeater.map(|s| s.to_string()),
        },
    ))
}

fn parse_inactive_timestamp(input: &str) -> IResult<&str, Timestamp> {
    let (input, _) = char('[')(input)?;
    let (input, date) = parse_date(input)?;
    let (input, time_info) = opt(preceded(space1, parse_time_range))(input)?;
    let (input, _) = opt(preceded(space0, take_while(|c: char| c != ']')))(input)?;
    let (input, _) = char(']')(input)?;

    let (time, end_time) = match time_info {
        Some((t, et)) => (Some(t), et),
        None => (None, None),
    };

    Ok((
        input,
        Timestamp {
            active: false,
            date,
            time,
            end_time,
            repeater: None,
        },
    ))
}

#[allow(dead_code)]
fn parse_timestamp(input: &str) -> IResult<&str, Timestamp> {
    alt((parse_active_timestamp, parse_inactive_timestamp))(input)
}

fn parse_scheduled(input: &str) -> IResult<&str, Timestamp> {
    let (input, _) = tag("SCHEDULED:")(input)?;
    let (input, _) = space0(input)?;
    parse_active_timestamp(input)
}

fn parse_deadline(input: &str) -> IResult<&str, Timestamp> {
    let (input, _) = tag("DEADLINE:")(input)?;
    let (input, _) = space0(input)?;
    parse_active_timestamp(input)
}

fn parse_closed(input: &str) -> IResult<&str, Timestamp> {
    let (input, _) = tag("CLOSED:")(input)?;
    let (input, _) = space0(input)?;
    parse_inactive_timestamp(input)
}

// ==================== Link Parsing ====================

fn parse_link(input: &str) -> IResult<&str, Link> {
    let (input, _) = tag("[[")(input)?;
    let (input, url) = take_until("]")(input)?;
    let (input, _) = char(']')(input)?;
    let (input, description) = opt(delimited(char('['), take_until("]"), char(']')))(input)?;
    let (input, _) = char(']')(input)?;

    Ok((
        input,
        Link {
            url: url.to_string(),
            description: description.map(|s: &str| s.to_string()),
        },
    ))
}

fn extract_links(text: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let mut remaining = text;
    
    while let Some(start) = remaining.find("[[") {
        let after_start = &remaining[start..];
        if let Ok((rest, link)) = parse_link(after_start) {
            links.push(link);
            remaining = rest;
        } else {
            if remaining.len() > start + 2 {
                remaining = &remaining[start + 2..];
            } else {
                break;
            }
        }
    }
    
    links
}

// ==================== Heading Parsing ====================

fn parse_stars(input: &str) -> IResult<&str, usize> {
    map(take_while1(|c| c == '*'), |s: &str| s.len())(input)
}

fn parse_keyword(input: &str) -> IResult<&str, Keyword> {
    let (input, keyword_str) = alt((
        tag("TODO"),
        tag("DONE"),
        tag("NEXT"),
        tag("WAITING"),
        tag("CANCELLED"),
        tag("IN-PROGRESS"),
    ))(input)?;
    
    Ok((input, Keyword::from_str(keyword_str).unwrap()))
}

fn parse_priority(input: &str) -> IResult<&str, Priority> {
    let (input, _) = tag("[#")(input)?;
    let (input, prio_char) = alt((char('A'), char('B'), char('C')))(input)?;
    let (input, _) = char(']')(input)?;
    
    Ok((input, Priority::from_char(prio_char).unwrap()))
}

fn parse_tags(input: &str) -> IResult<&str, Vec<String>> {
    let (input, _) = char(':')(input)?;
    let (input, tags) = separated_list1(
        char(':'),
        take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '@' || c == '#' || c == '%'),
    )(input)?;
    let (input, _) = char(':')(input)?;
    
    Ok((input, tags.into_iter().map(|s: &str| s.to_string()).collect()))
}

fn parse_heading_line(input: &str) -> IResult<&str, OrgEntry> {
    let (input, level) = parse_stars(input)?;
    let (input, _) = space1(input)?;
    
    // Try to parse keyword
    let (input, keyword) = opt(terminated(parse_keyword, space1))(input)?;
    
    // Try to parse priority
    let (input, priority) = opt(terminated(parse_priority, space1))(input)?;
    
    // Get the rest of the line (title + possibly tags)
    let (input, rest) = take_while(|c| c != '\n')(input)?;
    let (input, _) = opt(line_ending)(input)?;
    
    // Parse tags from end of rest
    let rest = rest.trim_end();
    let (title, tags) = if let Some(tag_start) = rest.rfind(" :") {
        let potential_tags = &rest[tag_start + 1..];
        if potential_tags.ends_with(':') {
            if let Ok((_, tags)) = parse_tags(potential_tags) {
                (rest[..tag_start].trim(), tags)
            } else {
                (rest, Vec::new())
            }
        } else {
            (rest, Vec::new())
        }
    } else if rest.starts_with(':') && rest.ends_with(':') {
        // Title is empty, just tags
        if let Ok((_, tags)) = parse_tags(rest) {
            ("", tags)
        } else {
            (rest, Vec::new())
        }
    } else {
        (rest, Vec::new())
    };
    
    let mut entry = OrgEntry::new(level, title.to_string());
    entry.keyword = keyword;
    entry.priority = priority;
    entry.tags = tags;
    entry.links = extract_links(title);
    
    Ok((input, entry))
}

// ==================== Properties Drawer Parsing ====================

fn parse_property_line(input: &str) -> IResult<&str, (String, String)> {
    let (input, _) = space0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, key) = take_while1(|c: char| c != ':' && c != '\n')(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = take_while(|c| c != '\n')(input)?;
    let (input, _) = opt(line_ending)(input)?;
    
    Ok((input, (key.to_string(), value.trim().to_string())))
}

fn parse_properties_drawer(input: &str) -> IResult<&str, HashMap<String, String>> {
    let (input, _) = space0(input)?;
    let (input, _) = tag(":PROPERTIES:")(input)?;
    let (input, _) = opt(line_ending)(input)?;
    
    let mut properties = HashMap::new();
    let mut remaining = input;
    
    loop {
        // Check for :END:
        let trimmed = remaining.trim_start();
        if trimmed.starts_with(":END:") {
            let (rest, _) = tag(":END:")(trimmed)?;
            let (rest, _) = opt(line_ending)(rest)?;
            return Ok((rest, properties));
        }
        
        // Parse a property line
        match parse_property_line(remaining) {
            Ok((rest, (key, value))) => {
                if key != "END" {
                    properties.insert(key, value);
                }
                remaining = rest;
            }
            Err(_) => {
                // Skip this line
                if let Some(idx) = remaining.find('\n') {
                    remaining = &remaining[idx + 1..];
                } else {
                    break;
                }
            }
        }
    }
    
    Ok((remaining, properties))
}

// ==================== Planning Line Parsing ====================

fn parse_planning_line(input: &str) -> IResult<&str, (Option<Timestamp>, Option<Timestamp>, Option<Timestamp>)> {
    let (input, _) = space0(input)?;
    
    let mut closed = None;
    let mut scheduled = None;
    let mut deadline = None;
    let mut remaining = input;
    
    // Try to parse CLOSED, SCHEDULED, DEADLINE in any order
    loop {
        let trimmed = remaining.trim_start();
        
        if let Ok((rest, ts)) = parse_closed(trimmed) {
            closed = Some(ts);
            remaining = rest;
        } else if let Ok((rest, ts)) = parse_scheduled(trimmed) {
            scheduled = Some(ts);
            remaining = rest;
        } else if let Ok((rest, ts)) = parse_deadline(trimmed) {
            deadline = Some(ts);
            remaining = rest;
        } else {
            break;
        }
        
        // Consume any spaces
        remaining = remaining.trim_start();
    }
    
    let (remaining, _) = opt(line_ending)(remaining)?;
    
    if closed.is_some() || scheduled.is_some() || deadline.is_some() {
        Ok((remaining, (closed, scheduled, deadline)))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
    }
}

// ==================== Document Parsing ====================

pub fn parse_org_document(input: &str) -> Result<OrgDocument, String> {
    let mut doc = OrgDocument::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut line_idx = 0;
    
    // Collect preamble (everything before first heading)
    let mut preamble_lines = Vec::new();
    while line_idx < lines.len() {
        if lines[line_idx].starts_with('*') {
            break;
        }
        preamble_lines.push(lines[line_idx]);
        line_idx += 1;
    }
    if !preamble_lines.is_empty() {
        doc.preamble = preamble_lines.join("\n");
        if !doc.preamble.is_empty() && line_idx > 0 {
            doc.preamble.push('\n');
        }
    }
    
    // Parse entries
    while line_idx < lines.len() {
        let line = lines[line_idx];
        
        if line.starts_with('*') {
            // Parse heading
            let line_with_newline = format!("{}\n", line);
            match parse_heading_line(&line_with_newline) {
                Ok((_, mut entry)) => {
                    entry.line_number = line_idx + 1;
                    line_idx += 1;
                    
                    // Parse planning line if present
                    if line_idx < lines.len() {
                        let next_line = lines[line_idx];
                        let next_with_newline = format!("{}\n", next_line);
                        if let Ok((_, (closed, scheduled, deadline))) = parse_planning_line(&next_with_newline) {
                            entry.closed = closed;
                            entry.scheduled = scheduled;
                            entry.deadline = deadline;
                            line_idx += 1;
                        }
                    }
                    
                    // Parse properties drawer if present
                    if line_idx < lines.len() {
                        let next_line = lines[line_idx].trim();
                        if next_line == ":PROPERTIES:" {
                            // Collect all lines until :END:
                            let mut drawer_content = String::new();
                            while line_idx < lines.len() {
                                drawer_content.push_str(lines[line_idx]);
                                drawer_content.push('\n');
                                if lines[line_idx].trim() == ":END:" {
                                    line_idx += 1;
                                    break;
                                }
                                line_idx += 1;
                            }
                            if let Ok((_, props)) = parse_properties_drawer(&drawer_content) {
                                entry.properties = props;
                            }
                        }
                    }
                    
                    // Collect body text until next heading
                    let mut body_lines = Vec::new();
                    while line_idx < lines.len() && !lines[line_idx].starts_with('*') {
                        body_lines.push(lines[line_idx]);
                        // Extract timestamps from body
                        let body_line = lines[line_idx];
                        
                        // Extract active timestamps using char-safe iteration
                        let mut search_start = 0;
                        while search_start < body_line.len() {
                            if let Some(rel_start) = body_line[search_start..].find('<') {
                                let abs_start = search_start + rel_start;
                                if let Ok((_, ts)) = parse_active_timestamp(&body_line[abs_start..]) {
                                    entry.timestamps.push(ts);
                                }
                                // Move past this '<' character safely (it's ASCII, so 1 byte)
                                search_start = abs_start + 1;
                            } else {
                                break;
                            }
                        }
                        
                        // Extract inactive timestamps using char-safe iteration
                        search_start = 0;
                        while search_start < body_line.len() {
                            if let Some(rel_start) = body_line[search_start..].find('[') {
                                let abs_start = search_start + rel_start;
                                // Don't parse links as timestamps - check for preceding '['
                                // Use char-safe method to look at previous character
                                let prev_char = if abs_start > 0 {
                                    // Get the character just before abs_start
                                    body_line[..abs_start].chars().last()
                                } else {
                                    None
                                };
                                if prev_char == Some('[') {
                                    // This is part of '[[', skip it
                                    search_start = abs_start + 1;
                                    continue;
                                }
                                if let Ok((_, ts)) = parse_inactive_timestamp(&body_line[abs_start..]) {
                                    entry.timestamps.push(ts);
                                }
                                // Move past this '[' character safely (it's ASCII, so 1 byte)
                                search_start = abs_start + 1;
                            } else {
                                break;
                            }
                        }
                        
                        // Extract links from body
                        let links = extract_links(body_line);
                        entry.links.extend(links);
                        
                        line_idx += 1;
                    }
                    if !body_lines.is_empty() {
                        entry.body = body_lines.join("\n");
                        entry.body.push('\n');
                    }
                    
                    doc.entries.push(entry);
                }
                Err(_) => {
                    line_idx += 1;
                }
            }
        } else {
            line_idx += 1;
        }
    }
    
    Ok(doc)
}

// ==================== Serialization ====================

fn format_timestamp(ts: &Timestamp) -> String {
    let (open, close) = if ts.active { ('<', '>') } else { ('[', ']') };
    
    let mut result = format!(
        "{}{:04}-{:02}-{:02}",
        open, ts.date.year, ts.date.month, ts.date.day
    );
    
    if let Some(ref weekday) = ts.date.weekday {
        result.push(' ');
        result.push_str(weekday);
    }
    
    if let Some(ref time) = ts.time {
        result.push_str(&format!(" {:02}:{:02}", time.hour, time.minute));
        if let Some(ref end_time) = ts.end_time {
            result.push_str(&format!("-{:02}:{:02}", end_time.hour, end_time.minute));
        }
    }
    
    if let Some(ref repeater) = ts.repeater {
        result.push(' ');
        result.push_str(repeater);
    }
    
    result.push(close);
    result
}

pub fn serialize_org_document(doc: &OrgDocument) -> String {
    let mut output = String::new();
    
    // Write preamble
    output.push_str(&doc.preamble);
    
    // Write entries
    for entry in &doc.entries {
        // Heading line
        for _ in 0..entry.level {
            output.push('*');
        }
        output.push(' ');
        
        if let Some(ref keyword) = entry.keyword {
            output.push_str(keyword.as_str());
            output.push(' ');
        }
        
        if let Some(ref priority) = entry.priority {
            output.push_str(&format!("[#{}] ", priority.as_char()));
        }
        
        output.push_str(&entry.title);
        
        if !entry.tags.is_empty() {
            output.push(' ');
            output.push(':');
            output.push_str(&entry.tags.join(":"));
            output.push(':');
        }
        
        output.push('\n');
        
        // Planning line
        let has_planning = entry.closed.is_some() || entry.scheduled.is_some() || entry.deadline.is_some();
        if has_planning {
            let mut planning_parts = Vec::new();
            
            if let Some(ref closed) = entry.closed {
                planning_parts.push(format!("CLOSED: {}", format_timestamp(closed)));
            }
            if let Some(ref scheduled) = entry.scheduled {
                planning_parts.push(format!("SCHEDULED: {}", format_timestamp(scheduled)));
            }
            if let Some(ref deadline) = entry.deadline {
                planning_parts.push(format!("DEADLINE: {}", format_timestamp(deadline)));
            }
            
            output.push_str(&planning_parts.join(" "));
            output.push('\n');
        }
        
        // Properties drawer
        if !entry.properties.is_empty() {
            output.push_str(":PROPERTIES:\n");
            for (key, value) in &entry.properties {
                output.push_str(&format!(":{}: {}\n", key, value));
            }
            output.push_str(":END:\n");
        }
        
        // Body
        output.push_str(&entry.body);
    }
    
    output
}

// ==================== Inline Markup Parsing ====================

/// Parse inline markup in a text string, returning a list of fragments.
/// Handles *bold*, /italic/, ~code~, =verbatim=, +strikethrough+, _underline_,
/// and [[links]].
pub fn parse_inline_markup(input: &str) -> Vec<InlineFragment> {
    let mut fragments = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut pos = 0;
    let mut current_text = String::new();

    while pos < len {
        // Check for links first: [[...]]
        if pos + 1 < len && chars[pos] == '[' && chars[pos + 1] == '[' {
            // Try to parse a link
            let remaining: String = chars[pos..].iter().collect();
            if let Ok((_, link)) = parse_link(&remaining) {
                if !current_text.is_empty() {
                    fragments.push(InlineFragment::Text(current_text.clone()));
                    current_text.clear();
                }
                // Advance pos past the link
                let link_str = format_link_source(&link);
                let link_char_len = link_str.chars().count();
                pos += link_char_len;
                fragments.push(InlineFragment::Link(link));
                continue;
            }
        }

        // Check for markup markers: * / ~ = + _
        let marker = chars[pos];
        if is_markup_char(marker) {
            // Check pre-condition: must be at start of string or preceded by whitespace/punctuation
            let pre_ok = pos == 0 || {
                let prev = chars[pos - 1];
                prev.is_whitespace() || is_pre_marker_char(prev)
            };

            if pre_ok {
                // Look for matching closing marker
                if let Some(end) = find_closing_marker(&chars, pos, marker) {
                    // Check post-condition: closing marker must be at end or followed by whitespace/punctuation
                    let post_ok = end + 1 >= len || {
                        let next = chars[end + 1];
                        next.is_whitespace() || is_post_marker_char(next)
                    };

                    if post_ok {
                        // We have a valid markup span
                        if !current_text.is_empty() {
                            fragments.push(InlineFragment::Text(current_text.clone()));
                            current_text.clear();
                        }
                        let content: String = chars[pos + 1..end].iter().collect();
                        let fragment = match marker {
                            '*' => InlineFragment::Bold(content),
                            '/' => InlineFragment::Italic(content),
                            '~' => InlineFragment::Code(content),
                            '=' => InlineFragment::Verbatim(content),
                            '+' => InlineFragment::Strikethrough(content),
                            '_' => InlineFragment::Underline(content),
                            _ => unreachable!(),
                        };
                        fragments.push(fragment);
                        pos = end + 1;
                        continue;
                    }
                }
            }
        }

        current_text.push(chars[pos]);
        pos += 1;
    }

    if !current_text.is_empty() {
        fragments.push(InlineFragment::Text(current_text));
    }

    fragments
}

fn is_markup_char(c: char) -> bool {
    matches!(c, '*' | '/' | '~' | '=' | '+' | '_')
}

fn is_pre_marker_char(c: char) -> bool {
    // Characters that can precede an opening markup marker
    matches!(c, '(' | '{' | '\'' | '"' | '-' | '<' | '[')
}

fn is_post_marker_char(c: char) -> bool {
    // Characters that can follow a closing markup marker
    matches!(c, ')' | '}' | '\'' | '"' | '-' | '.' | ',' | ';' | ':' | '!' | '?' | '>' | ']')
}

fn find_closing_marker(chars: &[char], open_pos: usize, marker: char) -> Option<usize> {
    // Search for a closing marker after at least one character
    for i in (open_pos + 2)..chars.len() {
        if chars[i] == marker {
            // The character before the closing marker must not be whitespace
            if !chars[i - 1].is_whitespace() {
                return Some(i);
            }
        }
    }
    None
}

fn format_link_source(link: &Link) -> String {
    let mut s = String::from("[[");
    s.push_str(&link.url);
    s.push(']');
    if let Some(ref desc) = link.description {
        s.push('[');
        s.push_str(desc);
        s.push(']');
    }
    s.push(']');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        let result = parse_date("2026-03-21 Sat");
        assert!(result.is_ok());
        let (_, date) = result.unwrap();
        assert_eq!(date.year, 2026);
        assert_eq!(date.month, 3);
        assert_eq!(date.day, 21);
        assert_eq!(date.weekday.as_deref(), Some("Sat"));
    }

    #[test]
    fn test_parse_time() {
        let result = parse_time("14:30");
        assert!(result.is_ok());
        let (_, time) = result.unwrap();
        assert_eq!(time.hour, 14);
        assert_eq!(time.minute, 30);
    }

    #[test]
    fn test_parse_active_timestamp() {
        let result = parse_active_timestamp("<2026-03-21 Sat 14:30>");
        assert!(result.is_ok());
        let (_, ts) = result.unwrap();
        assert!(ts.active);
        assert_eq!(ts.date.year, 2026);
        assert_eq!(ts.time.unwrap().hour, 14);
    }

    #[test]
    fn test_parse_inactive_timestamp() {
        let result = parse_inactive_timestamp("[2026-03-21 Sat]");
        assert!(result.is_ok());
        let (_, ts) = result.unwrap();
        assert!(!ts.active);
    }

    #[test]
    fn test_parse_link() {
        let result = parse_link("[[https://example.com][Example]]");
        assert!(result.is_ok());
        let (_, link) = result.unwrap();
        assert_eq!(link.url, "https://example.com");
        assert_eq!(link.description.as_deref(), Some("Example"));
    }
}
