//! Venue and promoter calendars.
//!
//! Each provider is a separate module so it can be compiled in or out with a
//! Cargo feature. These calendars publish HTML, not a shared API, so every
//! adapter has its own small parser and should fail independently.

#[cfg(feature = "source-awe")]
pub mod awe;
#[cfg(feature = "source-livenation")]
pub mod livenation;
#[cfg(feature = "source-mplus")]
pub mod mplus;
#[cfg(feature = "source-taikwun")]
pub mod taikwun;

use chrono::{Datelike, NaiveDate};

/// Parse a published English date such as `1 Dec 2026`, `01 December 2026`
/// or a bare day number when the surrounding range supplies month and year.
pub(crate) fn parse_date(text: &str) -> Option<NaiveDate> {
    let text = text
        .trim()
        .trim_start_matches("Opening")
        .trim()
        .trim_end_matches(',');
    if text.is_empty() || text.eq_ignore_ascii_case("ongoing") {
        return None;
    }
    for format in ["%d %b %Y", "%e %b %Y", "%d %B %Y", "%e %B %Y", "%Y-%m-%d"] {
        if let Ok(date) = NaiveDate::parse_from_str(text, format) {
            return Some(date);
        }
    }
    None
}

/// Parse the date forms used by venue calendars, including `1 – 5 Dec, 2026`
/// and `1 Dec 2026 – 3 Jan 2027`.
pub(crate) fn parse_range(text: &str) -> Vec<NaiveDate> {
    let cleaned = text.replace(['–', '—'], " - ").replace(',', " ");
    let parts: Vec<&str> = cleaned
        .split(" - ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 2 {
        return parse_date(parts.first().copied().unwrap_or(text))
            .into_iter()
            .collect();
    }
    let end = parse_date(parts[1]);
    let start = parse_date(parts[0]).or_else(|| {
        let day: u32 = parts[0].parse().ok()?;
        let end = end?;
        NaiveDate::from_ymd_opt(end.year(), end.month(), day)
    });
    let mut dates: Vec<NaiveDate> = start.into_iter().chain(end).collect();
    dates.sort_unstable();
    dates.dedup();
    dates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shared_date_forms() {
        assert_eq!(
            parse_range("1 Dec 2026 – 3 Jan 2027"),
            vec![
                NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
                NaiveDate::from_ymd_opt(2027, 1, 3).unwrap()
            ]
        );
        assert_eq!(
            parse_range("1 – 5 Dec, 2026"),
            vec![
                NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 12, 5).unwrap()
            ]
        );
        assert_eq!(
            parse_date("27 Jun 2025"),
            Some(NaiveDate::from_ymd_opt(2025, 6, 27).unwrap())
        );
    }
}
