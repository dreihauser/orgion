//! Parsing and rendering of org timestamps: `<2026-10-20 Tue>`,
//! `[2026-10-20 Tue 09:00-11:00 +1w]`, etc.

use chrono::{NaiveDate, NaiveTime};
use org_model::Timestamp;

/// Try to parse a timestamp starting at the beginning of `s` (which must
/// start with `<` or `[`). Returns the parsed timestamp and the number of
/// bytes consumed from `s` (i.e. `&s[..consumed]` is the whole
/// `<...>`/`[...]` token), or `None` if `s` doesn't start with a
/// syntactically valid timestamp.
pub fn parse_timestamp(s: &str) -> Option<(Timestamp, usize)> {
    let bytes = s.as_bytes();
    let (open, close) = match bytes.first()? {
        b'<' => (b'<', b'>'),
        b'[' => (b'[', b']'),
        _ => return None,
    };
    let _ = open;
    let end = s.find(close as char)?;
    let inner = &s[1..end];
    let consumed = end + 1;
    let active = bytes[0] == b'<';

    let mut tokens = inner.split_whitespace();
    let date_tok = tokens.next()?;
    let date = NaiveDate::parse_from_str(date_tok, "%Y-%m-%d").ok()?;

    let mut time = None;
    let mut end_time = None;
    let mut repeater = None;

    for tok in tokens {
        if let Some((start, end)) = parse_time_range(tok) {
            time = Some(start);
            end_time = end;
        } else if is_repeater_token(tok) {
            repeater = Some(tok.to_string());
        }
        // Warning-period cookies (e.g. `-2d` on its own) and day names we
        // failed to skip are silently ignored: they don't currently have
        // a place in the domain model, and ignoring unknown tokens keeps
        // the parser forward-compatible with timestamp forms we don't
        // special-case yet.
    }

    Some((
        Timestamp {
            date,
            time,
            end_time,
            active,
            repeater,
        },
        consumed,
    ))
}

fn parse_time_range(tok: &str) -> Option<(NaiveTime, Option<NaiveTime>)> {
    if let Some((start, end)) = tok.split_once('-') {
        if let (Some(start), Some(end)) = (parse_hhmm(start), parse_hhmm(end)) {
            return Some((start, Some(end)));
        }
    }
    parse_hhmm(tok).map(|t| (t, None))
}

fn parse_hhmm(tok: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(tok, "%H:%M").ok()
}

fn is_repeater_token(tok: &str) -> bool {
    let mut chars = tok.chars().peekable();
    match chars.peek() {
        Some('+') | Some('.') => {}
        _ => return false,
    }
    // Consume leading repeater marker: `+`, `++`, or `.+`.
    let rest: String = {
        let s = tok;
        let s = s.strip_prefix("++").or_else(|| s.strip_prefix(".+")).unwrap_or(
            s.strip_prefix('+').unwrap_or(s),
        );
        s.to_string()
    };
    let mut it = rest.chars();
    let has_digit = it.next().map(|c| c.is_ascii_digit()).unwrap_or(false);
    has_digit && rest.chars().next_back().map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
}

/// Render a timestamp back to its `<...>`/`[...]` textual form.
pub fn render_timestamp(ts: &Timestamp) -> String {
    let (open, close) = if ts.active { ('<', '>') } else { ('[', ']') };
    let day = ts.date.format("%a");
    let mut s = format!("{open}{} {day}", ts.date.format("%Y-%m-%d"));
    if let Some(time) = ts.time {
        if let Some(end) = ts.end_time {
            s.push_str(&format!(" {}-{}", time.format("%H:%M"), end.format("%H:%M")));
        } else {
            s.push_str(&format!(" {}", time.format("%H:%M")));
        }
    }
    if let Some(rep) = &ts.repeater {
        s.push(' ');
        s.push_str(rep);
    }
    s.push(close);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_date() {
        let (ts, consumed) = parse_timestamp("<2026-10-20 Tue>").unwrap();
        assert_eq!(ts.date, NaiveDate::from_ymd_opt(2026, 10, 20).unwrap());
        assert!(ts.active);
        assert_eq!(ts.time, None);
        assert_eq!(consumed, "<2026-10-20 Tue>".len());
    }

    #[test]
    fn parses_inactive_with_time_and_repeater() {
        let (ts, _) = parse_timestamp("[2026-10-20 Tue 09:00 +1w]").unwrap();
        assert!(!ts.active);
        assert_eq!(ts.time.unwrap().to_string(), "09:00:00");
        assert_eq!(ts.repeater.as_deref(), Some("+1w"));
    }

    #[test]
    fn parses_time_range() {
        let (ts, _) = parse_timestamp("<2026-10-20 Tue 09:00-11:00>").unwrap();
        assert_eq!(ts.time.unwrap().to_string(), "09:00:00");
        assert_eq!(ts.end_time.unwrap().to_string(), "11:00:00");
    }

    #[test]
    fn round_trips() {
        let src = "<2026-10-20 Tue 09:00-11:00 +1w>";
        let (ts, _) = parse_timestamp(src).unwrap();
        assert_eq!(render_timestamp(&ts), src);
    }

    #[test]
    fn rejects_non_timestamp() {
        assert!(parse_timestamp("hello").is_none());
        assert!(parse_timestamp("<not a date>").is_none());
    }
}
