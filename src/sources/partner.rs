//! Licensed partner feed for Cityline or Ticketflap exports.
//!
//! There is no public Cityline or Ticketflap discovery API. When a partner
//! supplies a licensed JSON export, set `HKEVENTS_PARTNER_FEED` to its URL or
//! local path. The adapter accepts either a bare array or `{ "events": [...] }`
//! and maps the documented fields into the shared event model.

use anyhow::{Context, Result, bail};
use chrono::{DateTime, NaiveDate};
use serde::Deserialize;

use crate::api::{Event, hong_kong_today};

/// One event in a partner export.
#[derive(Debug, Deserialize)]
struct Row {
    #[serde(default)]
    id: String,
    title: String,
    #[serde(default)]
    title_zh: String,
    /// `YYYY-MM-DD`, `YYYYMMDD` or an RFC 3339 timestamp.
    #[serde(alias = "date")]
    start: String,
    #[serde(default, alias = "end_date")]
    end: Option<String>,
    #[serde(default)]
    venue: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    price: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    presenter: String,
    #[serde(default, alias = "link")]
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Feed {
    Array(Vec<Row>),
    Wrapped { events: Vec<Row> },
}

pub async fn load(http: &reqwest::Client) -> Result<Vec<Event>> {
    let source = match std::env::var("HKEVENTS_PARTNER_FEED") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return Ok(Vec::new()),
    };
    let body = if source.starts_with("http://") || source.starts_with("https://") {
        let response = http
            .get(&source)
            .send()
            .await
            .with_context(|| format!("requesting partner feed {source}"))?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            bail!("partner feed {source} returned HTTP {status}");
        }
        body
    } else {
        std::fs::read_to_string(&source)
            .with_context(|| format!("reading partner feed {source}"))?
    };
    let feed: Feed = serde_json::from_str(body.trim_start_matches('\u{feff}'))
        .context("decoding partner feed")?;
    let rows = match feed {
        Feed::Array(rows) => rows,
        Feed::Wrapped { events } => events,
    };
    let today = hong_kong_today();
    let mut events: Vec<Event> = rows
        .into_iter()
        .filter_map(|row| {
            let start = parse_date(&row.start)?;
            let end = row.end.as_deref().and_then(parse_date);
            let dates: Vec<NaiveDate> = [Some(start), end]
                .into_iter()
                .flatten()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let next_date = dates.iter().copied().find(|date| *date >= today);
            let id = if row.id.trim().is_empty() {
                format!(
                    "partner:{}",
                    row.title
                        .to_lowercase()
                        .chars()
                        .map(|character| if character.is_ascii_alphanumeric() {
                            character
                        } else {
                            '-'
                        })
                        .collect::<String>()
                        .trim_matches('-')
                )
            } else {
                format!("partner:{}", row.id.trim())
            };
            Some(Event {
                id,
                source: "partner".into(),
                title_en: row.title.trim().to_string(),
                title_zh: row.title_zh.trim().to_string(),
                category: row.category,
                subcategory: String::new(),
                venue_id: String::new(),
                venue: row.venue,
                dates,
                next_date,
                date_text: row.start.clone(),
                duration: String::new(),
                price: row.price,
                age_limit: String::new(),
                description: row.description,
                url: Some(row.url).filter(|url| !url.trim().is_empty()),
                presenter: row.presenter,
                enquiry: String::new(),
            })
        })
        .collect();
    events.sort_by(|left, right| {
        left.next_date
            .cmp(&right.next_date)
            .then_with(|| left.title_en.cmp(&right.title_en))
    });
    Ok(events)
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    let value = value.trim();
    for format in ["%Y-%m-%d", "%Y%m%d"] {
        if let Ok(date) = NaiveDate::parse_from_str(value, format) {
            return Some(date);
        }
    }
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|stamp| stamp.date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dates_and_feed_shapes() {
        assert_eq!(
            parse_date("2026-12-01"),
            Some(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap())
        );
        assert_eq!(
            parse_date("20261201"),
            Some(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap())
        );
        let feed: Feed = serde_json::from_str(
            r#"{"events":[{"title":"Show","start":"2026-12-01","venue":"Hall"}]}"#,
        )
        .unwrap();
        match feed {
            Feed::Wrapped { events } => assert_eq!(events[0].title, "Show"),
            Feed::Array(_) => panic!("wrong shape"),
        }
    }
}
