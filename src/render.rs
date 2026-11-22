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
