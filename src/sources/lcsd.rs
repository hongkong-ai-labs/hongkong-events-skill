//! Official LCSD cultural-programme XML feeds.
//!
//! Three files make up the dataset:
//! - `events.xml` — programme details,
//! - `eventDates.xml` — exact performance dates per programme id,
//! - `venues.xml` — venue id to name lookup.

use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::api::{Event, Venue, clean, hong_kong_today, non_empty};

const BASE: &str = "https://www.lcsd.gov.hk/datagovhk/event";

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

pub async fn load(http: &reqwest::Client) -> Result<(Vec<Event>, Vec<Venue>)> {
    let (events_xml, dates_xml, venues_xml) = tokio::try_join!(
        text(http, "events.xml"),
        text(http, "eventDates.xml"),
        text(http, "venues.xml")
    )?;
    let events_doc: EventsDoc =
        quick_xml::de::from_str(&events_xml).context("parsing LCSD events.xml")?;
    let dates_doc: DatesDoc =
        quick_xml::de::from_str(&dates_xml).context("parsing LCSD eventDates.xml")?;
    let venues_doc: VenuesDoc =
        quick_xml::de::from_str(&venues_xml).context("parsing LCSD venues.xml")?;

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
            let next_date = event_dates.iter().copied().find(|date| *date >= today);
            let venue = venue_names
                .get(raw.venueid.as_str())
                .map(|name| (*name).to_string())
                .unwrap_or_else(|| raw.venueid.clone());
            Event {
                id: raw.id,
                source: "lcsd".into(),
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
    Ok((events, venues))
}

async fn text(http: &reqwest::Client, path: &str) -> Result<String> {
    let url = format!("{BASE}/{path}");
    let response = http.get(&url).send().await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        anyhow::bail!("{url} returned HTTP {status}");
    }
    Ok(body)
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
}
