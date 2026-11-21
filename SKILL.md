---
name: hongkong-events-skill
description: Hong Kong LCSD cultural programmes — concerts, theatre, dance, exhibitions, film, workshops and free entertainment in LCSD venues. Use for questions about what's on in Hong Kong, event dates, venues, ticket prices, or upcoming cultural performances.
license: MIT
---

# Hong Kong cultural events

`hkevents` is a Rust CLI and MCP server for the Leisure and Cultural Services
Department's Cultural Programmes open data. It needs no API key and no account.
It is an independent, unofficial project.

Use it when the user asks about:

- what's on in Hong Kong this week, this month or on a date
- a concert, play, dance, opera, film, exhibition or workshop
- events at a venue such as the Cultural Centre, City Hall, a town hall or a
  public library
- free cultural programmes
- ticket price, age limit, presenter, enquiry number or the organiser's link

The feed covers **current and forthcoming LCSD cultural programmes**. It is
not a complete list of every event in Hong Kong: commercial concerts, private
gallery shows, district festivals and venue-hire events may be absent.

## Before first use

```console
$ bash scripts/ensure-binary.sh
/Users/you/.local/bin/hkevents
```

Use the printed path:

```bash
BIN="$(bash scripts/ensure-binary.sh)"
```

It checks `$HKEVENTS_BIN`, `hkevents` on `PATH`, `scripts/install.sh`, then
`cargo build --release --locked` in the repo root.

### Install the binary directly

`npx skills add` installs the skill files. If the binary is missing, install it
with the release installer or cargo:

```bash
curl -fsSL https://raw.githubusercontent.com/hongkong-ai-labs/hongkong-events-skill/main/scripts/install.sh | sh
# or
cargo install --git https://github.com/hongkong-ai-labs/hongkong-events-skill --locked
```

Register it with any MCP client:

```bash
npx add-mcp "$(command -v hkevents) mcp" --name hongkong-events -a claude-code -a codex -a cursor -y
claude mcp add hongkong-events -- hkevents mcp
codex mcp add hongkong-events -- hkevents mcp
```

`add-mcp` detects the client configuration files in the current project; add
`-g` for a global install. `$(command -v hkevents)` keeps the entry correct
when the binary is on `PATH`; substitute an absolute path if your client does
not inherit the shell `PATH`.

## CLI

```bash
$BIN list                              # next 30 days, first 30
$BIN list --days 7 --limit 10          # this week
$BIN list --from 2026-12-01 --days 31
$BIN list --venue "Cultural Centre"
$BIN list --query jazz
$BIN list --free
$BIN list --json

$BIN show 188273                       # full details
$BIN venues --query "Town Hall"        # venue ids and coordinates
$BIN mcp
$BIN update --check
```

Dates are Hong Kong dates. `--days 0` means no upper date bound; `--limit 0`
means no row limit. `--query` searches the English and Chinese titles, the
description and the presenter.

### What a listing contains

| Field | Meaning |
| --- | --- |
| `next_date` | First performance on or after today |
| `dates` | Every performance date in the current run |
| `date_text` | Original LCSD date text, including times |
| `venue` | Venue name resolved from the venue id |
| `price` | Published price text, or `Free` / `Free Activities` |
| `category`, `subcategory` | Raw LCSD category codes |
| `duration` | Running time or duration text |
| `age_limit` | Age guidance |
| `presenter` | Presenter or organiser |
| `enquiry` | Enquiry phone number |
| `url` | Presenter or ticketing link |
| `description` | Full English description |

`category` is the raw code (`inc4`, etc.) because LCSD does not publish a
stable label table in the feed. Do not guess a human category from the code;
use the title, description and venue instead.

## MCP

```json
{
  "mcpServers": {
    "hongkong-events": {
      "command": "/absolute/path/to/hkevents",
      "args": ["mcp"]
    }
  }
}
```

Tools:

| Tool | Arguments |
| --- | --- |
| `list_hk_events` | `{ from?, days?, venue?, query?, free?, limit? }` |
| `get_hk_event` | `{ id }` |
| `list_event_venues` | `{ query? }` |

## Answering patterns

- "What's on this weekend?" → `list --days 7 --limit 20`; mention the date,
  venue and price, not just the title.
- "Anything free?" → `list --free`.
- "Is there a concert at the Cultural Centre?" → `list --venue "Cultural
  Centre" --query concert`.
- "How much are tickets?" → quote `price` exactly; it can be a range or a list
  of tiers, and does not include booking fees.
- "Tell me more about event N" → `show N`.
- "Where is that venue?" → `venues --query "<name>"` for the coordinates.

## Cautions

- Dates and line-ups change. The feed is a snapshot; a cancelled or sold-out
  performance may still appear.
- A programme with `next_date: null` has no performance today or later in the
  current published run. It can still be in the feed as a past or undated item.
- Ticket links point to the presenter or URBTIX. This tool does not check
  availability, sell tickets or hold seats.
- `price` is published text, not a live inventory.
- The list is LCSD programmes, not all Hong Kong events.
- Not an official LCSD channel. Check the venue or URBTIX for confirmation.

## Source

`https://www.lcsd.gov.hk/datagovhk/event/events.xml`,
`eventDates.xml` and `venues.xml`, published under DATA.GOV.HK. Responses are
cached for 10 minutes in-process.
