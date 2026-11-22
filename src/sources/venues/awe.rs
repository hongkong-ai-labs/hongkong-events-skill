//! AsiaWorld-Expo upcoming-events calendar.

use anyhow::{Context, Result};
use chrono::NaiveDate;
use scraper::{Html, Selector};

use crate::api::{Event, hong_kong_today};

const URL: &str = "https://www.asiaworld-expo.com/en-us/whats-on/upcoming-events";

pub async fn load(http: &reqwest::Client) -> Result<Vec<Event>> {
    let response = http
        .get(URL)
        .send()
        .await
        .context("loading AsiaWorld-Expo")?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        anyhow::bail!("{URL} returned HTTP {status}");
    }
    Ok(parse(&body))
}

fn parse(html: &str) -> Vec<Event> {
    let document = Html::parse_document(html);
    let card = Selector::parse("div.event-filter-tab").expect("valid selector");
    let title = Selector::parse("h3").expect("valid selector");
    let location = Selector::parse(".event-filter-tab-location").expect("valid selector");
    let link = Selector::parse("a.event-filter-tab-text").expect("valid selector");
    let today = hong_kong_today();
    let mut events = Vec::new();
    for node in document.select(&card) {
        let Some(link) = node.select(&link).next() else {
            continue;
        };
        let href = link.value().attr("href").unwrap_or_default();
        let name = node
            .select(&title)
            .next()
            .map(|node| clean_text(node.text()))
            .unwrap_or_default();
        if href.is_empty() || name.is_empty() {
            continue;
        }
        let start = node
            .value()
            .attr("data-start")
            .and_then(|value| NaiveDate::parse_from_str(value, "%m/%d/%Y").ok());
        let end = node
            .value()
            .attr("data-end")
            .and_then(|value| NaiveDate::parse_from_str(value, "%m/%d/%Y").ok());
        let mut dates: Vec<NaiveDate> = start.into_iter().chain(end).collect();
        dates.sort_unstable();
        dates.dedup();
        let next_date = dates.iter().copied().find(|date| *date >= today);
        let hall = node
            .select(&location)
            .next()
            .map(|node| clean_text(node.text()))
            .unwrap_or_default();
        let category = node
            .value()
            .attr("data-cat")
            .and_then(|value| value.split('|').next())
            .unwrap_or("event")
            .trim()
            .to_string();
        events.push(Event {
            id: format!("awe:{}", slug(href)),
            source: "awe".into(),
            title_en: name,
            title_zh: String::new(),
            category,
            subcategory: String::new(),
            venue_id: String::new(),
            venue: if hall.is_empty() {
                "AsiaWorld-Expo".to_string()
            } else {
                format!("AsiaWorld-Expo, {hall}")
            },
            dates,
            next_date,
            date_text: published_dates(node),
            duration: String::new(),
            price: String::new(),
            age_limit: String::new(),
            description: String::new(),
            url: Some(absolute(href)),
            presenter: "AsiaWorld-Expo".into(),
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

fn published_dates(node: scraper::ElementRef<'_>) -> String {
    let selector = Selector::parse(".event-filter-tab-date").expect("valid selector");
    node.select(&selector)
        .next()
        .map(|node| clean_text(node.text()))
        .unwrap_or_default()
}

fn absolute(href: &str) -> String {
    if href.starts_with("http") {
        href.to_string()
    } else if href.starts_with('/') {
        format!("https://www.asiaworld-expo.com{href}")
    } else {
        format!("https://www.asiaworld-expo.com/{href}")
    }
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
    fn parses_event_cards() {
        let html = r#"
        <div class="event-filter-tab" data-cat="concert|concert" data-start="12/01/2026" data-end="12/02/2026">
          <a class="event-filter-tab-text" href="/en-us/whats-on/upcoming-events/events/show?event_type=Concert">
            <h3>Test Concert</h3>
            <div class="event-filter-tab-date">01 - 02 Dec 2026</div>
            <div class="event-filter-tab-location">Hall 10</div>
          </a>
        </div>"#;
        let events = parse(html);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title_en, "Test Concert");
        assert_eq!(events[0].venue, "AsiaWorld-Expo, Hall 10");
        assert_eq!(events[0].dates.len(), 2);
    }
}
