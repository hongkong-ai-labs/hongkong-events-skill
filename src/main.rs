//! `hkevents` — LCSD cultural programmes CLI and MCP server.

mod api;
mod mcp;
mod render;

use anyhow::{Result, bail};
use chrono::NaiveDate;
use clap::{Parser, Subcommand};

use api::{Client, Filter};

#[derive(Parser)]
#[command(
    name = "hkevents",
    version,
    about = "Hong Kong LCSD cultural programmes — CLI and MCP server",
    arg_required_else_help = true
)]
struct Cli {
    /// Print machine-readable JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Upcoming cultural programmes
    #[command(alias = "upcoming")]
    List {
        /// First date to include, YYYY-MM-DD (default today)
        #[arg(long)]
        from: Option<String>,
        /// Number of days to include (default 30; 0 means all)
        #[arg(long, short, default_value_t = 30)]
        days: i64,
        /// Venue name substring
        #[arg(long, short)]
        venue: Option<String>,
        /// Free-text search in title, description and presenter
        #[arg(long, short)]
        query: Option<String>,
        /// Only programmes whose price says Free
        #[arg(long)]
        free: bool,
        /// Maximum rows (default 30; 0 means all)
        #[arg(long, short, default_value_t = 30)]
        limit: usize,
    },
    /// Full details for one programme id
    Show {
        /// Programme id from `list`
        id: String,
    },
    /// List LCSD venues
    Venues {
        /// Venue name or district substring
        #[arg(long, short)]
        query: Option<String>,
    },
    /// Run the MCP server over stdio
    Mcp,
}

fn parse_date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("date must be YYYY-MM-DD, got {value:?}"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new()?;

    if let Command::Mcp = cli.command {
        return mcp::serve(client).await;
    }

    match cli.command {
        Command::List {
            from,
            days,
            venue,
            query,
            free,
            limit,
        } => {
            let from = match from {
                Some(value) => parse_date(&value)?,
                None => api::hong_kong_today(),
            };
            if days < 0 {
                bail!("--days must be zero or positive");
            }
            let to = if days == 0 {
                None
            } else {
                Some(from + chrono::Days::new(days as u64))
            };
            let index = client.load().await?;
            let events = api::filter_events(
                &index.events,
                &Filter {
                    from: Some(from),
                    to,
                    venue,
                    query,
                    free_only: free,
                    limit,
                },
            );
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&events)?);
            } else {
                println!("{}", render::list(&events));
            }
        }
        Command::Show { id } => {
            let index = client.load().await?;
            let event = index
                .events
                .iter()
                .find(|event| event.id == id)
                .ok_or_else(|| anyhow::anyhow!("no programme with id {id}"))?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(event)?);
            } else {
                println!("{}", render::show(event));
            }
        }
        Command::Venues { query } => {
            let index = client.load().await?;
            let mut venues = index.venues;
            if let Some(query) = &query {
                let needle = query.to_lowercase();
                venues.retain(|venue| {
                    venue.name_en.to_lowercase().contains(&needle) || venue.name_zh.contains(query)
                });
            }
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&venues)?);
            } else {
                println!("{}", render::venues(&venues));
            }
        }
        Command::Mcp => unreachable!(),
    }
    Ok(())
}
