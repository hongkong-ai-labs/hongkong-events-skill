//! Event listing tables.

use crate::api::{Event, Venue};

fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if index < widths.len() {
                widths[index] = widths[index].max(cell.chars().count());
            }
        }
    }
    let mut out = String::new();
    out.push_str(
        headers
            .iter()
            .enumerate()
            .map(|(index, header)| format!("{header:<width$}", width = widths[index]))
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end(),
    );
    out.push('\n');
    out.push_str(
        &widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>()
            .join("  "),
    );
    out.push('\n');
    for row in rows {
        out.push_str(
            row.iter()
                .enumerate()
                .map(|(index, cell)| format!("{cell:<width$}", width = widths[index]))
                .collect::<Vec<_>>()
                .join("  ")
                .trim_end(),
        );
        out.push('\n');
    }
    out
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let mut out: String = value.chars().take(limit.saturating_sub(1)).collect();
    out.push('…');
    out
}

pub fn list(events: &[Event]) -> String {
    if events.is_empty() {
        return "No matching cultural programmes.".to_string();
    }
    let rows: Vec<Vec<String>> = events
        .iter()
        .map(|event| {
            vec![
                event
                    .next_date
                    .map(|date| date.to_string())
                    .unwrap_or_else(|| event.date_summary()),
                truncate(&event.title_en, 56),
                truncate(&event.venue, 28),
                event.source.clone(),
                event.price.clone(),
            ]
        })
        .collect();
    table(
        &["NEXT DATE", "PROGRAMME", "VENUE", "SOURCE", "PRICE"],
        &rows,
    )
    .trim_end()
    .to_string()
}

pub fn show(event: &Event) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n", event.title_en));
    if !event.title_zh.is_empty() && event.title_zh != event.title_en {
        out.push_str(&format!("{}\n", event.title_zh));
    }
    out.push('\n');
    out.push_str(&format!("Dates:     {}\n", event.date_summary()));
    if !event.date_text.is_empty() && event.date_text != event.date_summary() {
        out.push_str(&format!("Published: {}\n", event.date_text));
    }
    out.push_str(&format!("Venue:     {}\n", event.venue));
    out.push_str(&format!("Source:    {}\n", event.source));
    out.push_str(&format!(
        "Price:     {}\n",
        if event.price.is_empty() {
            "-"
        } else {
            &event.price
        }
    ));
    if !event.duration.is_empty() {
        out.push_str(&format!("Duration:  {}\n", event.duration));
    }
    if !event.age_limit.is_empty() {
        out.push_str(&format!("Age:       {}\n", event.age_limit));
    }
    if !event.presenter.is_empty() {
        out.push_str(&format!("Presenter: {}\n", event.presenter));
    }
    if !event.enquiry.is_empty() {
        out.push_str(&format!("Enquiry:   {}\n", event.enquiry));
    }
    if let Some(url) = &event.url {
        out.push_str(&format!("Link:      {url}\n"));
    }
    if !event.description.is_empty() {
        out.push_str(&format!("\n{}\n", event.description));
    }
    out.push_str(&format!("\nid: {}\n", event.id));
    out.trim_end().to_string()
}

pub fn venues(venues: &[Venue]) -> String {
    let rows: Vec<Vec<String>> = venues
        .iter()
        .map(|venue| {
            vec![
                venue.id.clone(),
                venue.name_en.clone(),
                venue.name_zh.clone(),
                match (venue.latitude, venue.longitude) {
                    (Some(lat), Some(lng)) => format!("{lat:.4}, {lng:.4}"),
                    _ => "-".to_string(),
                },
            ]
        })
        .collect();
    table(&["ID", "VENUE", "中文名", "COORDINATES"], &rows)
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn event() -> Event {
        Event {
            id: "1".to_string(),
            source: "lcsd".to_string(),
            title_en: "Jazz Night".to_string(),
            title_zh: String::new(),
            category: "inc4".to_string(),
            subcategory: "inc4sc1".to_string(),
            venue_id: "100".to_string(),
            venue: "Tai Po Civic Centre".to_string(),
            dates: vec![NaiveDate::from_ymd_opt(2026, 12, 19).unwrap()],
            next_date: Some(NaiveDate::from_ymd_opt(2026, 12, 19).unwrap()),
            date_text: String::new(),
            duration: String::new(),
            price: String::new(),
            age_limit: String::new(),
            description: String::new(),
            url: None,
            presenter: String::new(),
            enquiry: String::new(),
        }
    }

    #[test]
    fn list_reports_when_nothing_matches() {
        assert_eq!(list(&[]), "No matching cultural programmes.");
    }

    #[test]
    fn list_prefers_next_date_and_truncates_long_titles() {
        let event = Event {
            title_en: "An Extremely Long Programme Title That Will Not Fit In A Table Column"
                .to_string(),
            ..event()
        };
        let out = list(&[event]);
        assert!(out.contains("2026-12-19"));
        assert!(out.contains('…'));
        assert!(out.contains("Tai Po Civic Centre"));
    }

    #[test]
    fn list_falls_back_to_the_date_summary_when_undated() {
        let event = Event {
            next_date: None,
            dates: Vec::new(),
            date_text: "19/12/2026 (Sat)".to_string(),
            ..event()
        };
        let out = list(&[event]);
        assert!(out.contains("19/12/2026 (Sat)"));
    }

    #[test]
    fn show_marks_missing_price_and_skips_optional_fields() {
        let out = show(&event());
        assert!(out.contains("Jazz Night"));
        assert!(out.contains("Dates:     2026-12-19"));
        assert!(out.contains("Price:     -"));
        assert!(!out.contains("Duration:"));
        assert!(!out.contains("Presenter:"));
        assert!(!out.contains("Link:"));
        assert!(out.contains("id: 1"));
    }

    #[test]
    fn show_includes_optional_details() {
        let event = Event {
            title_zh: "爵士夜".to_string(),
            duration: "1 hour".to_string(),
            age_limit: "6".to_string(),
            presenter: "LCSD".to_string(),
            enquiry: "2268 7321".to_string(),
            url: Some("https://example.test/event".to_string()),
            description: "Live jazz from local players.".to_string(),
            price: "$200".to_string(),
            ..event()
        };
        let out = show(&event);
        assert!(out.contains("爵士夜"));
        assert!(out.contains("Duration:  1 hour"));
        assert!(out.contains("Age:       6"));
        assert!(out.contains("Presenter: LCSD"));
        assert!(out.contains("Enquiry:   2268 7321"));
        assert!(out.contains("https://example.test/event"));
        assert!(out.contains("Live jazz from local players."));
    }

    #[test]
    fn venues_format_coordinates_or_a_dash() {
        let venue_rows = vec![
            Venue {
                id: "100".to_string(),
                name_en: "Tai Po Civic Centre".to_string(),
                name_zh: "大埔文娛中心".to_string(),
                latitude: Some(22.45),
                longitude: Some(114.16),
            },
            Venue {
                id: "101".to_string(),
                name_en: "Unknown Venue".to_string(),
                name_zh: String::new(),
                latitude: None,
                longitude: None,
            },
        ];
        let out = venues(&venue_rows);
        assert!(out.contains("22.4500, 114.1600"));
        assert!(out.contains("Unknown Venue"));
        assert!(out.contains("-"));
    }
}
