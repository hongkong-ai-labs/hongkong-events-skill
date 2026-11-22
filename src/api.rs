//! Shared event model and compile-time source orchestration.

use std::time::Duration;

use anyhow::Result;
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// One event from any source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    /// Source slug, for example `lcsd`, `popticket` or `taikwun`.
    #[serde(default)]
    pub source: String,
    pub title_en: String,
    pub title_zh: String,
    pub category: String,
    pub subcategory: String,
    pub venue_id: String,
    pub venue: String,
    /// Exact performance dates, or the published start and end dates.
    pub dates: Vec<NaiveDate>,
    /// First date on or after today, when there is one.
    pub next_date: Option<NaiveDate>,
    /// Human-readable date text as published.
    pub date_text: String,
    /// Running time or duration text as published.
    pub duration: String,
    pub price: String,
    pub age_limit: String,
    pub description: String,
    pub url: Option<String>,
    pub presenter: String,
    pub enquiry: String,
}

impl Event {
    pub fn date_summary(&self) -> String {
        if self.dates.is_empty() {
            return self.date_text.clone();
        }
        let first = self
            .dates
            .first()
            .map(|date| date.to_string())
            .unwrap_or_default();
        let last = self
            .dates
            .last()
            .map(|date| date.to_string())
            .unwrap_or_default();
        if first == last {
            first
        } else {
            format!("{first} to {last} ({} dates)", self.dates.len())
        }
    }
}

/// A venue in the LCSD lookup file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venue {
    pub id: String,
    pub name_en: String,
    pub name_zh: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Filter for an event listing.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub venue: Option<String>,
    pub query: Option<String>,
    pub sources: Vec<String>,
    pub free_only: bool,
    pub limit: usize,
}

/// Loaded events, venues and non-fatal source failures.
pub struct Index {
    pub events: Vec<Event>,
    pub venues: Vec<Venue>,
    pub warnings: Vec<String>,
}

/// HTTP client shared by every enabled source.
pub struct Client {
    // Read by sources that are compiled in; an empty source set does not use it.
    #[allow(dead_code)]
    http: reqwest::Client,
}

impl Client {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "hongkong-events-skill/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/hongkong-ai-labs/hongkong-events-skill)"
            ))
            .cookie_store(true)
            .timeout(Duration::from_secs(40))
            .build()?;
        Ok(Self { http })
    }

    /// Load every compiled-in source. `days` is the requested listing window
    /// and lets paginated sources avoid fetching their full archive; zero
    /// means all available dates.
    pub async fn load(&self, days: i64) -> Result<Index> {
        let _ = days;
        let mut events: Vec<Event> = Vec::new();
        // Mutated only when the matching source feature is compiled in.
        #[allow(unused_mut)]
        let mut venues: Vec<Venue> = Vec::new();
        #[allow(unused_mut)]
        let mut warnings: Vec<String> = Vec::new();

        #[cfg(feature = "source-lcsd")]
        match crate::sources::lcsd::load(&self.http).await {
            Ok((mut source_events, source_venues)) => {
                events.append(&mut source_events);
                venues = source_venues;
            }
            Err(error) => warnings.push(format!("lcsd: {error:#}")),
        }

        #[cfg(feature = "source-popticket")]
        match crate::sources::popticket::load(&self.http, days).await {
            Ok(mut source_events) => events.append(&mut source_events),
            Err(error) => warnings.push(format!("popticket: {error:#}")),
        }

        #[cfg(feature = "source-taikwun")]
        match crate::sources::venues::taikwun::load(&self.http).await {
            Ok(mut source_events) => events.append(&mut source_events),
            Err(error) => warnings.push(format!("taikwun: {error:#}")),
        }

        #[cfg(feature = "source-awe")]
        match crate::sources::venues::awe::load(&self.http).await {
            Ok(mut source_events) => events.append(&mut source_events),
            Err(error) => warnings.push(format!("awe: {error:#}")),
        }

        #[cfg(feature = "source-mplus")]
        match crate::sources::venues::mplus::load(&self.http).await {
            Ok(mut source_events) => events.append(&mut source_events),
            Err(error) => warnings.push(format!("mplus: {error:#}")),
        }

        #[cfg(feature = "source-livenation")]
        match crate::sources::venues::livenation::load(&self.http).await {
            Ok(mut source_events) => events.append(&mut source_events),
            Err(error) => warnings.push(format!("livenation: {error:#}")),
        }

        if events.is_empty() && !warnings.is_empty() {
            anyhow::bail!(
                "every enabled event source failed:\n{}",
                warnings.join("\n")
            );
        }

        events.sort_by(|left, right| {
            left.next_date
                .cmp(&right.next_date)
                .then_with(|| left.title_en.cmp(&right.title_en))
        });
        Ok(Index {
            events,
            venues,
            warnings,
        })
    }

    pub fn sources() -> Vec<crate::sources::SourceInfo> {
        crate::sources::available()
    }
}

#[cfg(feature = "source-lcsd")]
pub(crate) fn clean(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(feature = "source-lcsd")]
pub(crate) fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Current date in Hong Kong (UTC+8).
pub fn hong_kong_today() -> NaiveDate {
    let offset = chrono::FixedOffset::east_opt(8 * 3600).expect("valid offset");
    Utc::now().with_timezone(&offset).date_naive()
}

/// Apply a filter, preserving sort order and truncating to `limit`.
pub fn filter_events(events: &[Event], filter: &Filter) -> Vec<Event> {
    let query = filter.query.as_ref().map(|value| value.to_lowercase());
    let venue = filter.venue.as_ref().map(|value| value.to_lowercase());
    let mut out: Vec<Event> = events
        .iter()
        .filter(|event| {
            if let Some(from) = filter.from
                && !event.dates.is_empty()
                && event.dates.iter().all(|date| *date < from)
            {
                return false;
            }
            if let Some(to) = filter.to
                && !event.dates.is_empty()
                && event.dates.iter().all(|date| *date > to)
            {
                return false;
            }
            if filter.from.is_some() && event.dates.is_empty() {
                return false;
            }
            if let Some(venue) = &venue
                && !event.venue.to_lowercase().contains(venue)
            {
                return false;
            }
            if !filter.sources.is_empty() && !filter.sources.contains(&event.source) {
                return false;
            }
            if let Some(query) = &query {
                let haystack = format!(
                    "{} {} {} {}",
                    event.title_en, event.title_zh, event.description, event.presenter
                )
                .to_lowercase();
                if !haystack.contains(query) {
                    return false;
                }
            }
            !(filter.free_only
                && !event.price.to_lowercase().contains("free")
                && !event.price.trim().is_empty())
        })
        .cloned()
        .collect();
    if filter.limit > 0 {
        out.truncate(filter.limit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Event {
        Event {
            id: "1".into(),
            source: "lcsd".into(),
            title_en: "Jazz Night".into(),
            title_zh: "爵士夜".into(),
            category: "inc4".into(),
            subcategory: "inc4sc1".into(),
            venue_id: "100".into(),
            venue: "Tai Po".into(),
            dates: vec![NaiveDate::from_ymd_opt(2026, 12, 19).unwrap()],
            next_date: Some(NaiveDate::from_ymd_opt(2026, 12, 19).unwrap()),
            date_text: "19/12/2026".into(),
            duration: "1 hour".into(),
            price: "Free".into(),
            age_limit: String::new(),
            description: "Live jazz".into(),
            url: None,
            presenter: "LCSD".into(),
            enquiry: String::new(),
        }
    }

    #[test]
    fn filters_by_query_free_and_source() {
        let events = vec![sample()];
        let filtered = filter_events(
            &events,
            &Filter {
                query: Some("jazz".into()),
                sources: vec!["lcsd".into()],
                free_only: true,
                limit: 10,
                ..Filter::default()
            },
        );
        assert_eq!(filtered.len(), 1);
        let none = filter_events(
            &events,
            &Filter {
                query: Some("opera".into()),
                ..Filter::default()
            },
        );
        assert!(none.is_empty());
        let wrong_source = filter_events(
            &events,
            &Filter {
                sources: vec!["popticket".into()],
                ..Filter::default()
            },
        );
        assert!(wrong_source.is_empty());
    }
}
