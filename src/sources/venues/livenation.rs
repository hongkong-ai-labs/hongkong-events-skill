//! Live Nation Hong Kong promoter calendar.

use std::collections::HashSet;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use scraper::{Html, Selector};

use crate::api::{Event, hong_kong_today};

const URL: &str = "https://www.livenation.hk/";

pub async fn load(http: &reqwest::Client) -> Result<Vec<Event>> {
    let response = http
        .get(URL)
        .send()
        .await
        .context("loading Live Nation HK")?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        anyhow::bail!("{URL} returned HTTP {status}");
    }
    Ok(parse(&body))
}

fn parse(html: &str) -> Vec<Event> {
    let document = Html::parse_document(html);
    let card = Selector::parse("a[href^='/event/']").expect("valid selector");
    let title = Selector::parse("p").expect("valid selector");
    let detail = Selector::parse("small").expect("valid selector");
    let today = hong_kong_today();
    let mut seen = HashSet::new();
    let mut events = Vec::new();
    for node in document.select(&card) {
        let href = node.value().attr("href").unwrap_or_default();
        if href == "/event/allevents" || !seen.insert(href.to_string()) {
            continue;
        }
        let name = node
            .select(&title)
            .next()
            .map(|node| clean_text(node.text()))
            .unwrap_or_default();
        let detail = node
            .select(&detail)
            .next()
            .map(|node| clean_text(node.text()))
            .unwrap_or_default();
        if name.is_empty() || detail.is_empty() {
            continue;
        }
        let (date_text, venue) = detail.split_once('|').unwrap_or((&detail, ""));
        let date_text = date_text.trim().to_string();
        let date =
            NaiveDate::parse_from_str(date_text.get(..10).unwrap_or(&date_text), "%Y.%m.%d").ok();
        let dates: Vec<NaiveDate> = date.into_iter().collect();
        let next_date = dates.iter().copied().find(|date| *date >= today);
        events.push(Event {
            id: format!("livenation:{}", slug(href)),
            source: "livenation".into(),
            title_en: name,
            title_zh: String::new(),
            category: "concert".into(),
            subcategory: String::new(),
            venue_id: String::new(),
            venue: venue.trim().to_string(),
            dates,
            next_date,
            date_text,
            duration: String::new(),
            price: String::new(),
            age_limit: String::new(),
            description: String::new(),
            url: Some(format!("https://www.livenation.hk{href}")),
            presenter: "Live Nation HK".into(),
            enquiry: String::new(),
        });
    }
    events.retain(|event| event.next_date.is_some() || !event.dates.is_empty());
    events.sort_by(|left, right| {
        left.next_date
            .cmp(&right.next_date)
            .then_with(|| left.title_en.cmp(&right.title_en))
    });
    events
}

fn slug(href: &str) -> String {
    href.split('?')
        .next()
        .unwrap_or(href)
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("event")
        .to_string()
}

fn clean_text<'a>(text: impl Iterator<Item = &'a str>) -> String {
    text.collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_promoter_cards() {
        let html = r#"
        <a href="/event/test-show-hong-kong-tickets-edp1">
          <p>Test Show</p>
          <small>2026.12.05 | AsiaWorld-Expo</small>
        </a>
        <a href="/event/allevents">All events</a>"#;
        let events = parse(html);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title_en, "Test Show");
        assert_eq!(events[0].venue, "AsiaWorld-Expo");
        assert_eq!(
            events[0].next_date,
            Some(NaiveDate::from_ymd_opt(2026, 12, 5).unwrap())
        );
    }
}
