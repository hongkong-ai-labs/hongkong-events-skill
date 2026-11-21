//! MCP over stdio for LCSD cultural programmes.

use std::io::{BufRead, Write};

use anyhow::{Result, bail};
use chrono::NaiveDate;
use serde_json::{Value, json};

use crate::api::{self, Client, Filter};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
const SUPPORTED_VERSIONS: [&str; 2] = ["2025-06-18", "2024-11-05"];

pub async fn serve(client: Client) -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    eprintln!("hkevents MCP server listening on stdio");
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(value) => dispatch(&client, &value).await,
            Err(error) => Some(error_response(
                None,
                -32700,
                &format!("parse error: {error}"),
            )),
        };
        if let Some(response) = response {
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }
    }
    Ok(())
}

async fn dispatch(client: &Client, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = match request.get("method").and_then(Value::as_str) {
        Some(method) => method,
        None => {
            return Some(error_response(
                id,
                -32600,
                "invalid request: missing method",
            ));
        }
    };
    if method.starts_with("notifications/") {
        return None;
    }
    match method {
        "initialize" => Some(success(id, initialize(request.get("params")))),
        "ping" => Some(success(id, json!({}))),
        "tools/list" => Some(success(id, tools())),
        "tools/call" => Some(call(client, id, request.get("params")).await),
        other => Some(error_response(
            id,
            -32601,
            &format!("method not found: {other}"),
        )),
    }
}

fn initialize(params: Option<&Value>) -> Value {
    let requested = params
        .and_then(|value| value.get("protocolVersion"))
        .and_then(Value::as_str);
    let version = requested
        .filter(|value| SUPPORTED_VERSIONS.contains(value))
        .unwrap_or(PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "hongkong-events-skill",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn tools() -> Value {
    json!({ "tools": [
        {
            "name": "list_hk_events",
            "description": "Upcoming LCSD cultural programmes in Hong Kong, filtered by date, venue, free-text query and free admission.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "First date, YYYY-MM-DD. Default today."},
                    "days": {"type": "integer", "minimum": 0, "description": "Days from `from`; 0 means all. Default 30."},
                    "venue": {"type": "string", "description": "Venue name substring."},
                    "query": {"type": "string", "description": "Free-text search in title, description and presenter."},
                    "free": {"type": "boolean", "description": "Only programmes marked Free."},
                    "limit": {"type": "integer", "minimum": 0, "description": "Maximum rows; 0 means all. Default 30."}
                }
            }
        },
        {
            "name": "get_hk_event",
            "description": "Full details for one LCSD cultural programme id.",
            "inputSchema": {
                "type": "object",
                "properties": {"id": {"type": "string", "description": "Programme id from list_hk_events."}},
                "required": ["id"]
            }
        },
        {
            "name": "list_event_venues",
            "description": "LCSD cultural venues, with a name or district filter.",
            "inputSchema": {
                "type": "object",
                "properties": {"query": {"type": "string", "description": "Venue name or district substring."}}
            }
        }
    ]})
}

async fn call(client: &Client, id: Option<Value>, params: Option<&Value>) -> Value {
    let Some(params) = params else {
        return error_response(id, -32602, "missing params");
    };
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let outcome: Result<Value> = async {
        match name {
            "list_hk_events" => {
                let from = match args.get("from").and_then(Value::as_str) {
                    Some(value) => NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
                        .map_err(|_| anyhow::anyhow!("from must be YYYY-MM-DD"))?,
                    None => api::hong_kong_today(),
                };
                let days = args.get("days").and_then(Value::as_i64).unwrap_or(30);
                if days < 0 {
                    bail!("days must be zero or positive");
                }
                let to = if days == 0 {
                    None
                } else {
                    Some(from + chrono::Days::new(days as u64))
                };
                let limit = args
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or(30);
                let index = client.load().await?;
                let events = api::filter_events(
                    &index.events,
                    &Filter {
                        from: Some(from),
                        to,
                        venue: args
                            .get("venue")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        query: args
                            .get("query")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        free_only: args.get("free").and_then(Value::as_bool).unwrap_or(false),
                        limit,
                    },
                );
                Ok(serde_json::to_value(events)?)
            }
            "get_hk_event" => {
                let id = args
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("id is required"))?;
                let index = client.load().await?;
                let event = index
                    .events
                    .iter()
                    .find(|event| event.id == id)
                    .ok_or_else(|| anyhow::anyhow!("no programme with id {id}"))?;
                Ok(serde_json::to_value(event)?)
            }
            "list_event_venues" => {
                let index = client.load().await?;
                let mut venues = index.venues;
                if let Some(query) = args.get("query").and_then(Value::as_str) {
                    let needle = query.to_lowercase();
                    venues.retain(|venue| {
                        venue.name_en.to_lowercase().contains(&needle)
                            || venue.name_zh.contains(query)
                    });
                }
                Ok(serde_json::to_value(venues)?)
            }
            other => bail!("unknown tool: {other}"),
        }
    }
    .await;
    match outcome {
        Ok(value) => success(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&value).unwrap_or_default()
                }],
                "structuredContent": value,
                "isError": false
            }),
        ),
        Err(error) => error_response(id, -32603, &format!("{error:#}")),
    }
}

fn success(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}
