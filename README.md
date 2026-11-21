# hkevents

Hong Kong LCSD cultural programmes as a Rust CLI and MCP server.

Reads the official, keyless LCSD Cultural Programmes dataset: programme
details, exact performance dates and venue names.

## Install

```bash
npx skills add hongkong-ai-labs/hongkong-events-skill
cargo install --path . --locked
sh scripts/install.sh
```

## CLI

```bash
hkevents list
hkevents list --days 7 --limit 10
hkevents list --from 2026-12-01 --days 31
hkevents list --venue "Cultural Centre" --query concert
hkevents list --free
hkevents list --json
hkevents show 188273
hkevents venues --query "Town Hall"
hkevents mcp
```

## MCP

```json
{
  "mcpServers": {
    "hongkong-events": { "command": "hkevents", "args": ["mcp"] }
  }
}
```

Tools: `list_hk_events`, `get_hk_event`, `list_event_venues`.

## Data

- `https://www.lcsd.gov.hk/datagovhk/event/events.xml`
- `https://www.lcsd.gov.hk/datagovhk/event/eventDates.xml`
- `https://www.lcsd.gov.hk/datagovhk/event/venues.xml`

Coverage is current and forthcoming LCSD cultural programmes, not every event
in Hong Kong. Dates, prices and line-ups can change; check URBTIX or the venue.

## Licence

MIT. Event data remains subject to DATA.GOV.HK and LCSD terms.
