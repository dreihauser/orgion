//! Minimal repeater expansion for the agenda view. Org's repeater
//! grammar (`+1w`, `++2d`, `.+1m`, plus warning-period cookies) has more
//! nuance than is worth fully reproducing for v0.1 — this only answers
//! "does this recurring timestamp land on `target`", which is what the
//! agenda needs, and treats `+`/`++`/`.+` identically (ignoring the
//! habit-mode distinction between them). See docs/mvp.md.

use chrono::{Datelike, NaiveDate};

fn parse_repeater(repeater: &str) -> Option<(i64, char)> {
    let s = repeater
        .strip_prefix("++")
        .or_else(|| repeater.strip_prefix(".+"))
        .unwrap_or_else(|| repeater.strip_prefix('+').unwrap_or(repeater));
    if s.len() < 2 {
        return None;
    }
    let unit = s.chars().next_back()?;
    let n: i64 = s[..s.len() - 1].parse().ok()?;
    if n <= 0 || !matches!(unit, 'd' | 'w' | 'm' | 'y') {
        return None;
    }
    Some((n, unit))
}

/// Does a timestamp anchored at `base` (with optional `repeater`) have an
/// occurrence on `target`?
pub fn occurs_on(base: NaiveDate, repeater: Option<&str>, target: NaiveDate) -> bool {
    if target == base {
        return true;
    }
    if target < base {
        return false;
    }
    let Some(repeater) = repeater else { return false };
    let Some((n, unit)) = parse_repeater(repeater) else {
        return false;
    };
    match unit {
        'd' => (target - base).num_days() % n == 0,
        'w' => (target - base).num_days() % (n * 7) == 0,
        'm' => {
            if target.day() != base.day() {
                return false;
            }
            let months = (target.year() - base.year()) as i64 * 12
                + (target.month() as i64 - base.month() as i64);
            months >= 0 && months % n == 0
        }
        'y' => {
            if target.day() != base.day() || target.month() != base.month() {
                return false;
            }
            let years = (target.year() - base.year()) as i64;
            years >= 0 && years % n == 0
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn base_date_always_occurs() {
        assert!(occurs_on(d(2026, 10, 20), None, d(2026, 10, 20)));
        assert!(!occurs_on(d(2026, 10, 20), None, d(2026, 10, 21)));
    }

    #[test]
    fn weekly_repeater_expands() {
        let base = d(2026, 9, 28);
        assert!(occurs_on(base, Some("+1w"), d(2026, 10, 5)));
        assert!(occurs_on(base, Some("+1w"), d(2026, 10, 12)));
        assert!(!occurs_on(base, Some("+1w"), d(2026, 10, 6)));
    }

    #[test]
    fn daily_repeater_expands() {
        let base = d(2026, 9, 28);
        assert!(occurs_on(base, Some("+3d"), d(2026, 10, 1)));
        assert!(!occurs_on(base, Some("+3d"), d(2026, 9, 30)));
    }

    #[test]
    fn monthly_repeater_expands() {
        let base = d(2026, 1, 15);
        assert!(occurs_on(base, Some("+1m"), d(2026, 3, 15)));
        assert!(!occurs_on(base, Some("+1m"), d(2026, 3, 16)));
    }

    #[test]
    fn no_repeater_means_single_occurrence() {
        assert!(!occurs_on(d(2026, 9, 28), None, d(2026, 10, 5)));
    }
}
