//! POPTICKET public event listing.
//!
//! The site publishes a public listing API used by its own event page. A page
//! load supplies the CSRF token; the cookie jar keeps the session for the
//! paginated requests. `robots.txt` permits these pages and only excludes
//! query-string and cart/account paths.

use anyhow::{Context, Result, bail};
use chrono::{NaiveDate, NaiveDateTime};
use scraper::Selector;
use serde::Deserialize;

use crate::api::{Event, hong_kong_today};

const LISTING: &str = "https://www.popticket.hk/en/events";
const API: &str = "https://www.popticket.hk/web_api/events_list/all/events";

#[derive(Debug, Deserialize)]
struct Page {
    #[serde(default)]
    events: Vec<Row>,
    #[serde(rename = "totalPages", default)]
    total_pages: usize,
}

#[derive(Debug, Deserialize)]
struct Row {
    slug: String,
    title: String,
    #[serde(default)]
    venue: String,
    #[serde(default)]
    date: String,
    #[serde(rename = "isPast", default)]
    is_past: bool,
    #[serde(rename = "isCancelled", default)]
    is_cancelled: bool,
}

pub async fn load(http: &reqwest::Client, days: i64) -> Result<Vec<Event>> {
    let page = http
        .get(LISTING)
        .send()
        .await
        .context("loading POPTICKET listing page")?;
    let status = page.status();
    let html = page.text().await?;
    if !status.is_success() {
        bail!("{LISTING} returned HTTP {status}");
    }
    let token = csrf_token(&html).context("POPTICKET page has no CSRF token")?;

    let first: Page = post(http, &token, 1).await?;
    let pages = page_count(days, first.total_pages);
    let mut rows = first.events;
    for page in 2..=pages {
        rows.extend(post(http, &token, page).await?.events);
    }

    let today = hong_kong_today();
    let mut events: Vec<Event> = rows
        .into_iter()
        .filter(|row| !row.is_cancelled && !row.is_past)
        .filter_map(|row| {
            let dates = parse_dates(&row.date);
            let next_date = dates.iter().copied().find(|date| *date >= today);
            (!row.title.trim().is_empty()).then(|| Event {
                id: format!("popticket:{}", row.slug),
                source: "popticket".into(),
                title_en: row.title.trim().to_string(),
                title_zh: String::new(),
                category: "event".into(),
                subcategory: String::new(),
                venue_id: String::new(),
                venue: row.venue.trim().to_string(),
                dates,
                next_date,
                date_text: row.date.trim().to_string(),
                duration: String::new(),
                price: String::new(),
                age_limit: String::new(),
                description: String::new(),
                url: Some(format!("https://www.popticket.hk/en/event/{}", row.slug)),
                presenter: String::new(),
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

async fn post(http: &reqwest::Client, token: &str, page: usize) -> Result<Page> {
    let response = http
        .post(API)
        .header("X-CSRF-TOKEN", token)
        .header("Accept", "application/json")
        .json(&serde_json::json!({ "page": page }))
        .send()
        .await
        .with_context(|| format!("requesting POPTICKET page {page}"))?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        bail!("POPTICKET page {page} returned HTTP {status}");
    }
    serde_json::from_str(&body).with_context(|| format!("parsing POPTICKET page {page}"))
}

fn page_count(days: i64, total: usize) -> usize {
    if days <= 0 {
        return total.max(1);
    }
    // The listing is roughly date-sorted; about a week of events per page.
    (((days + 6) / 7) + 3).clamp(1, total.max(1) as i64) as usize
}

fn csrf_token(html: &str) -> Option<String> {
    let document = scraper::Html::parse_document(html);
    let selector = Selector::parse("meta[name=csrf-token]").ok()?;
    document
        .select(&selector)
        .next()?
        .value()
        .attr("content")
        .map(ToOwned::to_owned)
}

fn parse_dates(text: &str) -> Vec<NaiveDate> {
    let mut dates: Vec<NaiveDate> = text
        .split(" - ")
        .filter_map(|part| NaiveDate::parse_from_str(part.trim(), "%d-%m-%Y").ok())
        .collect();
    if dates.is_empty()
        && let Ok(datetime) = NaiveDateTime::parse_from_str(text.trim(), "%d-%m-%Y %H:%M")
    {
        dates.push(datetime.date());
    }
    dates.sort_unstable();
    dates.dedup();
    dates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_listing_dates() {
        assert_eq!(
            parse_dates("23-11-2026 - 05-12-2026"),
            vec![
                NaiveDate::from_ymd_opt(2026, 11, 23).unwrap(),
                NaiveDate::from_ymd_opt(2026, 12, 5).unwrap()
            ]
        );
        assert_eq!(
            parse_dates("25-11-2026"),
            vec![NaiveDate::from_ymd_opt(2026, 11, 25).unwrap()]
        );
    }

    #[test]
    fn reads_csrf_and_rows() {
        let html = r#"<meta name="csrf-token" content="abc"><script id="brief" type="application/json">{}</script>"#;
        assert_eq!(csrf_token(html).as_deref(), Some("abc"));
        let page: Page = serde_json::from_str(
            r#"{"events":[{"slug":"show","title":"Show","venue":"Hall","date":"01-12-2026","isPast":false,"isCancelled":false}],"totalPages":3}"#,
        )
        .unwrap();
        assert_eq!(page.events[0].slug, "show");
        assert_eq!(page.total_pages, 3);
    }
}
