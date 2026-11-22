//! M+ exhibition calendar.

use anyhow::{Context, Result};
use chrono::NaiveDate;
use scraper::{Html, Selector};

use super::parse_date;
use crate::api::{Event, hong_kong_today};

const URL: &str = "https://www.mplus.org.hk/en/exhibitions/";

pub async fn load(http: &reqwest::Client) -> Result<Vec<Event>> {
    let response = http.get(URL).send().await.context("loading M+")?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        anyhow::bail!("{URL} returned HTTP {status}");
    }
    Ok(parse(&body))
}

fn parse(html: &str) -> Vec<Event> {
    let document = Html::parse_document(html);
    let card = Selector::parse("a.CommonExhibitionsItem").expect("valid selector");
    let title = Selector::parse("h3").expect("valid selector");
    let dates = Selector::parse(".CommonTitleColors-titleText").expect("valid selector");
    let location = Selector::parse(".CommonExhibitionsItem-bodyLocation").expect("valid selector");
    let today = hong_kong_today();
    let mut events = Vec::new();
    for node in document.select(&card) {
        let href = node.value().attr("href").unwrap_or_default();
        let name = node
            .select(&title)
            .next()
            .map(|node| clean_text(node.text()))
            .unwrap_or_default();
        if href.is_empty() || name.is_empty() {
            continue;
        }
        let published: Vec<String> = node
            .select(&dates)
            .map(|node| clean_text(node.text()))
            .filter(|value| !value.is_empty())
            .collect();
        let mut parsed: Vec<NaiveDate> = published
            .iter()
            .filter_map(|text| parse_date(text))
            .collect();
        parsed.sort_unstable();
        parsed.dedup();
        let next_date = parsed.iter().copied().find(|date| *date >= today);
        let venue = node
            .select(&location)
            .next()
            .map(|node| clean_text(node.text()))
            .unwrap_or_default();
        let date_text = published.join(" – ");
        events.push(Event {
            id: format!("mplus:{}", slug(href)),
            source: "mplus".into(),
            title_en: name,
            title_zh: String::new(),
            category: "exhibition".into(),
            subcategory: String::new(),
            venue_id: String::new(),
            venue: if venue.is_empty() {
                "M+".to_string()
            } else {
                format!("M+, {venue}")
            },
            dates: parsed,
            next_date,
            date_text,
            duration: String::new(),
            price: String::new(),
            age_limit: String::new(),
            description: String::new(),
            url: Some(absolute(href)),
            presenter: "M+".into(),
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
    } else if href.starts_with('/') {
        format!("https://www.mplus.org.hk{href}")
    } else {
        format!("https://www.mplus.org.hk/{href}")
    }
}

fn slug(href: &str) -> String {
    href.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("exhibition")
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
    fn parses_exhibition_cards() {
        let html = r#"
        <a href="/en/exhibitions/test-show/" class="CommonExhibitionsItem">
          <div class="CommonTitleColors CommonLabeledImage-labels date">
            <span class="CommonTitleColors-title"><span class="CommonTitleColors-titleText">17 Oct 2026</span></span>
            <span class="CommonTitleColors-title"><span class="CommonTitleColors-titleText">4 Apr 2027</span></span>
          </div>
          <div class="CommonExhibitionsItem-body">
            <h3 class="CommonExhibitionsItem-bodyTitle">Test Show</h3>
            <div class="CommonExhibitionsItem-bodyLocation">West Gallery, L2</div>
          </div>
        </a>"#;
        let events = parse(html);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title_en, "Test Show");
        assert_eq!(events[0].venue, "M+, West Gallery, L2");
        assert_eq!(events[0].dates.len(), 2);
    }
}
