//! LCSD Cultural Programmes open-data client.
//!
//! Three XML files make up the dataset:
//! - `events.xml` — programme details,
//! - `eventDates.xml` — exact performance dates per programme id,
//! - `venues.xml` — venue id to name lookup.
//!
//! LCSD publishes only current and forthcoming programmes, so the list is
//! naturally an upcoming-events feed.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

const BASE: &str = "https://www.lcsd.gov.hk/datagovhk/event";

/// One cultural programme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub title_en: String,
    pub title_zh: String,
    /// Raw LCSD first-level category code, for example `inc4`.
    pub category: String,
    /// Raw LCSD second-level category code.
    pub subcategory: String,
    pub venue_id: String,
    pub venue: String,
    /// Exact performance dates, ascending.
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
        let first = self.dates.first().map(|date| date.to_string()).unwrap_or_default();
        let last = self.dates.last().map(|date| date.to_string()).unwrap_or_default();
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
    pub free_only: bool,
    pub limit: usize,
}

#[derive(Debug, Deserialize)]
struct EventsDoc {
    #[serde(rename = "event", default)]
    events: Vec<EventRaw>,
}

#[derive(Debug, Default, Deserialize)]
struct EventRaw {
    #[serde(rename = "@id", default)]
    id: String,
    #[serde(default)]
    titlec: String,
    #[serde(default)]
    titlee: String,
    #[serde(default)]
    cat1: String,
    #[serde(default)]
    cat2: String,
    #[serde(default, rename = "predateE")]
    predate_e: String,
    #[serde(default, rename = "progtimee")]
    progtime_e: String,
    #[serde(default)]
    venueid: String,
    #[serde(default, rename = "agelimite")]
    age_limit_e: String,
    #[serde(default, rename = "pricee")]
    price_e: String,
    #[serde(default)]
    desce: String,
    #[serde(default)]
    urle: String,
    #[serde(default, rename = "presenterorge")]
    presenter_e: String,
    #[serde(default)]
    enquiry: String,
}

#[derive(Debug, Deserialize)]
struct VenuesDoc {
    #[serde(rename = "venue", default)]
    venues: Vec<VenueRaw>,
}

#[derive(Debug, Default, Deserialize)]
struct VenueRaw {
    #[serde(rename = "@id", default)]
    id: String,
    #[serde(default)]
    venuee: String,
    #[serde(default)]
    venuec: String,
    #[serde(default)]
    latitude: String,
    #[serde(default)]
    longitude: String,
}

#[derive(Debug, Deserialize)]
struct DatesDoc {
    #[serde(rename = "event", default)]
    events: Vec<DatesRaw>,
}

#[derive(Debug, Default, Deserialize)]
struct DatesRaw {
    #[serde(rename = "@id", default)]
    id: String,
    #[serde(rename = "indate", default)]
    dates: Vec<String>,
}

/// Loaded events, venues and date index.
pub struct Index {
    pub events: Vec<Event>,
    pub venues: Vec<Venue>,
}

/// Cached LCSD client.
pub struct Client {
    http: reqwest::Client,
    cache: Mutex<HashMap<String, (Instant, String)>>,
}

impl Client {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "hongkong-events-skill/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/hongkong-ai-labs/hongkong-events-skill)"
            ))
            .timeout(Duration::from_secs(40))
            .build()?;
        Ok(Self {
            http,
            cache: Mutex::new(HashMap::new()),
        })
    }

    async fn text(&self, path: &str) -> Result<String> {
        if let Some((at, body)) = self
            .cache
            .lock()
            .expect("events cache poisoned")
            .get(path)
            .cloned()
            && at.elapsed() < Duration::from_secs(600)
        {
            return Ok(body);
        }
        let url = format!("{BASE}/{path}");
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("requesting {url}"))?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            bail!("{url} returned HTTP {status}");
        }
        self.cache
            .lock()
            .expect("events cache poisoned")
            .insert(path.to_string(), (Instant::now(), body.clone()));
        Ok(body)
    }

    /// Fetch and join all three XML files.
    pub async fn load(&self) -> Result<Index> {
        let (events_xml, dates_xml, venues_xml) = tokio::try_join!(
            self.text("events.xml"),
            self.text("eventDates.xml"),
            self.text("venues.xml")
        )?;
        let events_doc: EventsDoc = quick_xml::de::from_str(&events_xml)
            .context("parsing LCSD events.xml")?;
        let dates_doc: DatesDoc = quick_xml::de::from_str(&dates_xml)
            .context("parsing LCSD eventDates.xml")?;
        let venues_doc: VenuesDoc = quick_xml::de::from_str(&venues_xml)
            .context("parsing LCSD venues.xml")?;

        let mut dates: HashMap<String, Vec<NaiveDate>> = HashMap::new();
        for entry in dates_doc.events {
            let parsed: Vec<NaiveDate> = entry
                .dates
                .iter()
                .filter_map(|raw| NaiveDate::parse_from_str(raw.trim(), "%Y%m%d").ok())
                .collect();
            dates.insert(entry.id, parsed);
        }
        let venues: Vec<Venue> = venues_doc
            .venues
            .into_iter()
            .map(|venue| Venue {
                id: venue.id,
                name_en: venue.venuee.trim().to_string(),
                name_zh: venue.venuec.trim().to_string(),
                latitude: venue.latitude.trim().parse().ok(),
                longitude: venue.longitude.trim().parse().ok(),
            })
            .collect();
        let venue_names: HashMap<&str, &str> = venues
            .iter()
            .map(|venue| (venue.id.as_str(), venue.name_en.as_str()))
            .collect();
        let today = hong_kong_today();
        let mut events: Vec<Event> = events_doc
            .events
            .into_iter()
            .map(|raw| {
                let mut event_dates = dates.remove(&raw.id).unwrap_or_default();
                if event_dates.is_empty() {
                    event_dates = dates_from_text(&raw.predate_e);
                }
                event_dates.sort_unstable();
                event_dates.dedup();
                let next_date = event_dates
                    .iter()
                    .copied()
                    .find(|date| *date >= today);
                let venue = venue_names
                    .get(raw.venueid.as_str())
                    .map(|name| (*name).to_string())
                    .unwrap_or_else(|| raw.venueid.clone());
                Event {
                    id: raw.id,
                    title_en: clean(&raw.titlee),
                    title_zh: clean(&raw.titlec),
                    category: raw.cat1,
                    subcategory: raw.cat2,
                    venue_id: raw.venueid,
                    venue,
                    dates: event_dates,
                    next_date,
                    date_text: clean(&raw.predate_e),
                    duration: clean(&raw.progtime_e),
                    price: clean(&raw.price_e),
                    age_limit: clean(&raw.age_limit_e),
                    description: clean(&raw.desce),
                    url: non_empty(&raw.urle),
                    presenter: clean(&raw.presenter_e),
                    enquiry: clean(&raw.enquiry),
                }
            })
            .filter(|event| !event.title_en.is_empty() || !event.title_zh.is_empty())
            .collect();
        events.sort_by(|left, right| {
            left.next_date
                .cmp(&right.next_date)
                .then_with(|| left.title_en.cmp(&right.title_en))
        });
        Ok(Index { events, venues })
    }
}

fn clean(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn non_empty(value: &str) -> Option<String> {
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

fn dates_from_text(text: &str) -> Vec<NaiveDate> {
    // `predateE` uses `dd/mm/yyyy` and may contain several lines.
    let mut dates = Vec::new();
    for token in text.split(|character: char| !(character.is_ascii_digit() || character == '/')) {
        let token = token.trim_matches('/');
        if token.matches('/').count() == 2
            && let Ok(date) = NaiveDate::parse_from_str(token, "%d/%m/%Y")
        {
            dates.push(date);
        }
    }
    dates
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
            if filter.free_only
                && !event.price.to_lowercase().contains("free")
                && !event.price.trim().is_empty()
            {
                return false;
            }
            true
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

    #[test]
    fn parses_dates_and_titles() {
        let xml = r#"<events><event id="1"><titlec>節目</titlec><titlee> Show </titlee>
            <cat1>inc4</cat1><cat2>inc4sc1</cat2><predateE>19/12/2026 (Sat) 19:30</predateE>
            <venueid>100</venueid><pricee>$300</pricee></event></events>"#;
        let doc: EventsDoc = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(doc.events[0].id, "1");
        assert_eq!(clean(&doc.events[0].titlee), "Show");
        let dates = dates_from_text(&doc.events[0].predate_e);
        assert_eq!(dates, vec![NaiveDate::from_ymd_opt(2026, 12, 19).unwrap()]);
    }

    #[test]
    fn parses_venues_and_date_index() {
        let venues: VenuesDoc = quick_xml::de::from_str(
            r#"<venues><venue id="100"><venuec>大埔</venuec><venuee>Tai Po</venuee>
               <latitude>22.45</latitude><longitude>114.16</longitude></venue></venues>"#,
        )
        .unwrap();
        assert_eq!(venues.venues[0].venuee, "Tai Po");
        let dates: DatesDoc = quick_xml::de::from_str(
            r#"<event_dates><event id="1"><indate>20261219</indate>
               <indate>20261220</indate></event></event_dates>"#,
        )
        .unwrap();
        assert_eq!(dates.events[0].dates.len(), 2);
    }

    #[test]
    fn filters_by_query_and_free() {
        let events = vec![Event {
            id: "1".into(),
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
        }];
        let filtered = filter_events(
            &events,
            &Filter {
                query: Some("jazz".into()),
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
    }
}
