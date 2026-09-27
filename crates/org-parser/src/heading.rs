//! Parsing and rendering of a single heading line:
//! `*** TODO [#A] Write HRI Paper   :paper:hri:`

use org_model::TodoKeywords;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedHeading {
    pub level: u32,
    pub todo_state: Option<String>,
    pub priority: Option<char>,
    pub title: String,
    pub tags: Vec<String>,
}

/// Returns `None` if `line` is not a valid heading line (must start with
/// one or more `*` immediately followed by a space).
pub fn parse_heading_line(line: &str, todo_keywords: &TodoKeywords) -> Option<ParsedHeading> {
    let stars = line.bytes().take_while(|&b| b == b'*').count();
    if stars == 0 {
        return None;
    }
    if line.as_bytes().get(stars) != Some(&b' ') {
        return None;
    }
    let mut rest = &line[stars + 1..];
    rest = rest.trim_start();

    let (rest_no_tags, tags) = split_trailing_tags(rest);
    let mut rest = rest_no_tags.trim_end();

    let mut todo_state = None;
    if let Some((first, after)) = split_first_word(rest) {
        if todo_keywords.is_known(first) {
            todo_state = Some(first.to_string());
            rest = after.trim_start();
        }
    }

    let mut priority = None;
    if rest.len() >= 4 && rest.starts_with("[#") && rest.as_bytes()[3] == b']' {
        let c = rest.as_bytes()[2] as char;
        if c.is_ascii_alphanumeric() {
            priority = Some(c);
            rest = rest[4..].trim_start();
        }
    }

    Some(ParsedHeading {
        level: stars as u32,
        todo_state,
        priority,
        title: rest.trim().to_string(),
        tags,
    })
}

fn split_first_word(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    let idx = s.find(char::is_whitespace)?;
    Some((&s[..idx], &s[idx..]))
}

/// Splits a trailing `:tag1:tag2:` block off the end of a heading's
/// title/todo/priority region, if present. Tags must be preceded by
/// whitespace (or be the entire remaining string) and contain only
/// `[A-Za-z0-9_@#%]` characters between colons.
fn split_trailing_tags(s: &str) -> (&str, Vec<String>) {
    let trimmed = s.trim_end();
    if !trimmed.ends_with(':') {
        return (s, Vec::new());
    }
    // Walk backwards from the end looking for the start of a valid tag
    // block, stopping at the first whitespace boundary that yields one,
    // or at the start of the string.
    let byte_positions: Vec<usize> = trimmed
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(trimmed.len()))
        .collect();
    for &start in byte_positions.iter() {
        if start > 0 {
            let prev_char = trimmed[..start].chars().next_back().unwrap();
            if !prev_char.is_whitespace() {
                continue;
            }
        }
        let candidate = &trimmed[start..];
        if let Some(tags) = try_parse_tags_block(candidate) {
            let before = trimmed[..start].trim_end();
            return (before, tags);
        }
    }
    (s, Vec::new())
}

fn try_parse_tags_block(s: &str) -> Option<Vec<String>> {
    if s.len() < 2 || !s.starts_with(':') || !s.ends_with(':') {
        return None;
    }
    let inner = &s[1..s.len() - 1];
    if inner.is_empty() {
        return None;
    }
    let parts: Vec<&str> = inner.split(':').collect();
    if parts.iter().any(|p| p.is_empty() || !p.chars().all(is_tag_char)) {
        return None;
    }
    Some(parts.into_iter().map(String::from).collect())
}

fn is_tag_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '@' | '#' | '%')
}

/// Render a heading's components back into a single heading line (no
/// trailing newline). This is used both for whole-document rendering and
/// for targeted single-line patches (see `patch.rs`).
pub fn render_heading_line(h: &ParsedHeading) -> String {
    let mut s = "*".repeat(h.level as usize);
    if let Some(todo) = &h.todo_state {
        s.push(' ');
        s.push_str(todo);
    }
    if let Some(p) = h.priority {
        s.push_str(" [#");
        s.push(p);
        s.push(']');
    }
    if !h.title.is_empty() {
        s.push(' ');
        s.push_str(&h.title);
    }
    if !h.tags.is_empty() {
        let tags_str = format!(":{}:", h.tags.join(":"));
        // Match standard org style: at least one space before the tag
        // block.
        s.push(' ');
        s.push_str(&tags_str);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use org_model::TodoKeyword;
    use org_model::TodoType;

    fn keywords() -> TodoKeywords {
        TodoKeywords(vec![
            TodoKeyword { keyword: "TODO".into(), todo_type: TodoType::Todo },
            TodoKeyword { keyword: "NEXT".into(), todo_type: TodoType::Todo },
            TodoKeyword { keyword: "DONE".into(), todo_type: TodoType::Done },
        ])
    }

    #[test]
    fn parses_plain_heading() {
        let h = parse_heading_line("* Create repository", &keywords()).unwrap();
        assert_eq!(h.level, 1);
        assert_eq!(h.todo_state, None);
        assert_eq!(h.title, "Create repository");
        assert!(h.tags.is_empty());
    }

    #[test]
    fn parses_todo_priority_tags() {
        let h = parse_heading_line(
            "*** TODO [#A] Write HRI Paper   :paper:hri:",
            &keywords(),
        )
        .unwrap();
        assert_eq!(h.level, 3);
        assert_eq!(h.todo_state.as_deref(), Some("TODO"));
        assert_eq!(h.priority, Some('A'));
        assert_eq!(h.title, "Write HRI Paper");
        assert_eq!(h.tags, vec!["paper", "hri"]);
    }

    #[test]
    fn does_not_confuse_colon_in_title_for_tags() {
        let h = parse_heading_line("* Meeting: budget review", &keywords()).unwrap();
        assert_eq!(h.title, "Meeting: budget review");
        assert!(h.tags.is_empty());
    }

    #[test]
    fn rejects_non_heading_lines() {
        assert!(parse_heading_line("not a heading", &keywords()).is_none());
        assert!(parse_heading_line("*no space after star", &keywords()).is_none());
    }

    #[test]
    fn render_round_trips() {
        let line = "*** NEXT [#B] Implement parser :dev:core:";
        let h = parse_heading_line(line, &keywords()).unwrap();
        assert_eq!(render_heading_line(&h), line);
    }
}
