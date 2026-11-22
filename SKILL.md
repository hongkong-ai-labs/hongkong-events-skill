---
name: hongkong-events-skill
description: Hong Kong events — LCSD cultural programmes plus POPTICKET, Tai Kwun, AsiaWorld-Expo, M+ and Live Nation calendars. Use for questions about what's on in Hong Kong, concerts, theatre, exhibitions, event dates, venues, ticket prices, or upcoming performances.
license: MIT
---

# Hong Kong events

`hkevents` is a Rust CLI and MCP server for Hong Kong event listings. It reads
the Leisure and Cultural Services Department's open data plus POPTICKET and
public venue and promoter calendars. It needs no API key and no account. It is
an independent, unofficial project.

Use it when the user asks about:

- what's on in Hong Kong this week, this month or on a date
- a concert, play, dance, opera, film, exhibition or workshop
- events at a venue such as the Cultural Centre, City Hall, a town hall, Tai
  Kwun, AsiaWorld-Expo or M+
- concerts and tours promoted by Live Nation HK
- free cultural programmes
- ticket price, age limit, presenter, enquiry number or the organiser's link

The sources cover **current and forthcoming events** from LCSD, POPTICKET,
Tai Kwun, AsiaWorld-Expo, M+ and Live Nation HK. A licensed Cityline or
Ticketflap JSON export can be added through `HKEVENTS_PARTNER_FEED`. This is
still not a complete list of every event in Hong Kong: unlicensed Cityline and
Ticketflap inventory, private gallery shows, district festivals and venue-hire
events may be absent.

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
$BIN list --source popticket --limit 10
$BIN list --json
$BIN sources                           # sources compiled into this binary

$BIN show 188273                       # full details
$BIN venues --query "Town Hall"        # venue ids and coordinates
$BIN mcp
$BIN update --check
```

Dates are Hong Kong dates. `--days 0` means no upper date bound; `--limit 0`
means no row limit. `--query` searches the English and Chinese titles, the
description and the presenter. `--source` takes comma-separated source slugs;
omit it to search everything compiled in.

### What a listing contains

| Field | Meaning |
| --- | --- |
| `source` | Source slug: `lcsd`, `popticket`, `partner`, `taikwun`, `awe`, `mplus` or `livenation` |
| `next_date` | First performance on or after today |
| `dates` | Every published performance or exhibition date |
| `date_text` | Original date text |
| `venue` | Venue name, including the hall where published |
| `price` | Published price text, or `Free` / `Free Activities` |
| `category`, `subcategory` | Source category or raw LCSD codes |
| `duration` | Running time or duration text |
| `age_limit` | Age guidance |
| `presenter` | Presenter or organiser |
| `enquiry` | Enquiry phone number |
| `url` | Presenter or ticketing link |
| `description` | Full description when the source publishes one |

For LCSD, `category` is the raw code (`inc4`, etc.) because LCSD does not
publish a stable label table. Do not guess a human category from the code; use
the title, description and venue instead. External sources often leave price,
duration and description empty rather than inventing values.

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
| `list_hk_events` | `{ from?, days?, venue?, source?, query?, free?, limit? }` |
| `get_hk_event` | `{ id }` |
| `list_event_venues` | `{ query? }` |
| `list_event_sources` | `{}` |

## Answering patterns

- "What's on this weekend?" → `list --days 7 --limit 20`; mention the date,
  venue, source and price, not just the title.
- "Any gigs?" → `list --source popticket,livenation --query live`.
- "What's on at AsiaWorld-Expo?" → `list --source awe --venue AsiaWorld`.
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
- The list covers the compiled-in sources, not all Hong Kong events. Cityline,
  Ticketflap, Timable and social platforms are not scraped.
- Not an official channel of LCSD, POPTICKET or any venue. Check the ticketing
  page or venue for confirmation.
- Venue calendars publish HTML and can change without notice. A failed source
  is reported as a warning while the others still return results.

## Sources

- LCSD: `https://www.lcsd.gov.hk/datagovhk/event/events.xml`,
  `eventDates.xml`, `venues.xml`, published under DATA.GOV.HK.
- POPTICKET: `https://www.popticket.hk/en/events` and its public listing API.
- Tai Kwun: `https://www.taikwun.hk/en/whats-on`.
- AsiaWorld-Expo: `https://www.asiaworld-expo.com/en-us/whats-on/upcoming-events`.
- M+: `https://www.mplus.org.hk/en/exhibitions/`.
- Live Nation HK: `https://www.livenation.hk/`.
- Licensed partner export: set `HKEVENTS_PARTNER_FEED` to a JSON path or URL.

Event data remains subject to each publisher's terms; link to the original
page. Responses are not cached between processes.
