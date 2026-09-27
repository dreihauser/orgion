//! Parsing of the `SCHEDULED:`/`DEADLINE:`/`CLOSED:` planning line.

use crate::timestamp::{parse_timestamp, render_timestamp};
use org_model::Planning;

const KEYWORDS: [&str; 3] = ["SCHEDULED:", "DEADLINE:", "CLOSED:"];

/// A line is a planning line only if, once trimmed, it consists entirely
/// of one or more `KEYWORD: <timestamp>` pairs (org does not allow other
/// text to share the line).
pub fn parse_planning_line(line: &str) -> Option<Planning> {
    let mut rest = line.trim();
    if rest.is_empty() {
        return None;
    }
    let mut planning = Planning::default();
    let mut found_any = false;

    'outer: while !rest.is_empty() {
        for kw in KEYWORDS {
            if let Some(after_kw) = rest.strip_prefix(kw) {
                let after_kw = after_kw.trim_start();
                let (ts, consumed) = parse_timestamp(after_kw)?;
                match kw {
                    "SCHEDULED:" => planning.scheduled = Some(ts),
                    "DEADLINE:" => planning.deadline = Some(ts),
                    "CLOSED:" => planning.closed = Some(ts),
                    _ => unreachable!(),
                }
                found_any = true;
                rest = after_kw[consumed..].trim_start();
                continue 'outer;
            }
        }
        // Leftover text that isn't a recognized keyword: not a planning line.
        return None;
    }

    if found_any {
        Some(planning)
    } else {
        None
    }
}

/// Render a `Planning` back to a single planning line (no leading
/// indentation, no trailing newline), or `None` if there's nothing to
/// render.
pub fn render_planning_line(p: &Planning) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(ts) = &p.deadline {
        parts.push(format!("DEADLINE: {}", render_timestamp(ts)));
    }
    if let Some(ts) = &p.scheduled {
        parts.push(format!("SCHEDULED: {}", render_timestamp(ts)));
    }
    if let Some(ts) = &p.closed {
        parts.push(format!("CLOSED: {}", render_timestamp(ts)));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_deadline() {
        let p = parse_planning_line("DEADLINE: <2026-10-01 Thu>").unwrap();
        assert!(p.deadline.is_some());
        assert!(p.scheduled.is_none());
    }

    #[test]
    fn parses_combined_line() {
        let p = parse_planning_line(
            "DEADLINE: <2026-10-01 Thu> SCHEDULED: <2026-09-28 Mon>",
        )
        .unwrap();
        assert!(p.deadline.is_some());
        assert!(p.scheduled.is_some());
    }

    #[test]
    fn rejects_body_text() {
        assert!(parse_planning_line("This is just body text.").is_none());
        assert!(parse_planning_line("").is_none());
    }

    #[test]
    fn render_round_trips_order() {
        let p = parse_planning_line(
            "DEADLINE: <2026-10-01 Thu> SCHEDULED: <2026-09-28 Mon>",
        )
        .unwrap();
        let rendered = render_planning_line(&p).unwrap();
        assert_eq!(rendered, "DEADLINE: <2026-10-01 Thu> SCHEDULED: <2026-09-28 Mon>");
    }
}
