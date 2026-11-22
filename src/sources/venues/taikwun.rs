//! Tai Kwun what's-on calendar.

use anyhow::{Context, Result};
use scraper::{Html, Selector};

use super::parse_range;
use crate::api::{Event, hong_kong_today};

const URL: &str = "https://www.taikwun.hk/en/whats-on";

pub async fn load(http: &reqwest::Client) -> Result<Vec<Event>> {
    let response = http.get(URL).send().await.context("loading Tai Kwun")?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        anyhow::bail!("{URL} returned HTTP {status}");
    }
    Ok(parse(&body))
}

fn parse(html: &str) -> Vec<Event> {
    let document = Html::parse_document(html);
    let card = Selector::parse("div.programme-detail").expect("valid selector");
    let title = Selector::parse("h2 a").expect("valid selector");
    let time = Selector::parse("span.programme-property.time").expect("valid selector");
    let venue = Selector::parse("span.programme-property.venue").expect("valid selector");
    let tags = Selector::parse("label span").expect("valid selector");
    let today = hong_kong_today();
    let mut events = Vec::new();
    for node in document.select(&card) {
        let Some(link) = node.select(&title).next() else {
            continue;
        };
        let name = clean_text(link.text());
        let href = link.value().attr("href").unwrap_or_default();
        if name.is_empty() || href.is_empty() {
            continue;
        }
        let date_text = node
            .select(&time)
            .next()
            .map(|node| clean_text(node.text()))
            .unwrap_or_default();
        let dates = parse_range(&date_text);
        let next_date = dates.iter().copied().find(|date| *date >= today);
        let venue = node
            .select(&venue)
            .next()
            .map(|node| clean_text(node.text()))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Tai Kwun".to_string());
        let category = clean_text(node.select(&tags).flat_map(|tag| tag.text()));
        events.push(Event {
            id: format!("taikwun:{}", slug(href)),
            source: "taikwun".into(),
            title_en: name,
            title_zh: String::new(),
            category,
            subcategory: String::new(),
            venue_id: String::new(),
            venue: format!("Tai Kwun, {venue}"),
            dates,
            next_date,
            date_text,
            duration: String::new(),
            price: String::new(),
            age_limit: String::new(),
            description: String::new(),
            url: Some(absolute(href)),
            presenter: "Tai Kwun".into(),
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

fn absolute(href: &str) -> String {
    if href.starts_with("http") {
        href.to_string()
    } else {
        format!("https://www.taikwun.hk{}", ensure_slash(href))
    }
}

fn ensure_slash(href: &str) -> String {
    if href.starts_with('/') {
        href.to_string()
    } else {
        format!("/{href}")
    }
}

fn slug(href: &str) -> String {
    href.trim_end_matches('/')
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
    use chrono::NaiveDate;

    #[test]
    fn parses_programme_cards() {
        let html = r#"
        <div class="programme-detail">
          <label><span class="tag-performing-arts">Performing Arts</span></label>
          <h2><a href="https://www.taikwun.hk/en/whats-on/detail/show/1887">The Gift of Music</a></h2>
          <span class="programme-property time">1 – 5 Dec, 2026</span>
          <span class="programme-property venue">Parade Ground</span>
        </div>"#;
        let events = parse(html);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title_en, "The Gift of Music");
        assert_eq!(events[0].venue, "Tai Kwun, Parade Ground");
        assert_eq!(events[0].id, "taikwun:1887");
        assert_eq!(events[0].dates.len(), 2);
        assert_eq!(
            events[0].next_date,
            Some(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap())
        );
    }
}
