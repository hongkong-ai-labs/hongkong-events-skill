# hkevents

Hong Kong LCSD cultural programmes as a Rust CLI and MCP server.

Reads the official, keyless LCSD Cultural Programmes dataset: programme
details, exact performance dates and venue names.

## Install

**Agent skill** — installs `SKILL.md` and the binary bootstrap script:

```bash
npx skills add hongkong-ai-labs/hongkong-events-skill
npx skills update        # refresh the installed skill later
```

**Prebuilt binary** — macOS (Intel or Apple silicon) and Linux (x86_64 or
aarch64, static musl), checksum-verified, into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/hongkong-ai-labs/hongkong-events-skill/main/scripts/install.sh | sh
```

**From source** — Rust 1.89 or newer:

```bash
git clone https://github.com/hongkong-ai-labs/hongkong-events-skill
cd hongkong-events-skill
cargo install --path . --locked
# or directly from GitHub:
cargo install --git https://github.com/hongkong-ai-labs/hongkong-events-skill --locked
# or: cargo build --release --locked   # target/release/hkevents
```

Verify:

```console
$ hkevents --version
```

`hkevents update` detects whether the binary came from cargo, the release
installer, a source checkout or Homebrew, and updates with that same
method. `hkevents update --check` only reports.

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
hkevents update --check
```

## MCP

```json
{
  "mcpServers": {
    "hongkong-events": { "command": "hkevents", "args": ["mcp"] }
  }
}
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


Tools: `list_hk_events`, `get_hk_event`, `list_event_venues`.

## Data

- `https://www.lcsd.gov.hk/datagovhk/event/events.xml`
- `https://www.lcsd.gov.hk/datagovhk/event/eventDates.xml`
- `https://www.lcsd.gov.hk/datagovhk/event/venues.xml`

Coverage is current and forthcoming LCSD cultural programmes, not every event
in Hong Kong. Dates, prices and line-ups can change; check URBTIX or the venue.

## Run modes and deployment

| Mode | Start with | Use for |
| --- | --- | --- |
| CLI | `hkevents <command>` | Interactive use and shell scripts |
| JSON | `<command>` with `--json` | `jq`, automation, other programs |
| MCP stdio | `hkevents mcp` | Desktop and CLI MCP clients |
| Container | `docker run --rm -i hkevents mcp` | No local Rust toolchain |

### MCP and authentication

This server is **stdio-only**: the client launches the process and speaks MCP
over stdin/stdout. There is no listening port, so there is no network
authentication layer and no API-key setting. Optional API keys in this project
family apply only to servers that expose an HTTP transport. Keep the process
local and let the MCP client manage it.

Typical client entry:

```json
{
  "mcpServers": {
    "hongkong-events-skill": {
      "command": "hkevents",
      "args": ["mcp"]
    }
  }
}
```

If the client does not inherit `PATH`, use the absolute installed path.

### Docker and Apple container

```bash
docker build -t hkevents .
docker run --rm hkevents --help
docker run --rm -i hkevents mcp          # stdio MCP
docker compose run --rm stdio             # same, through Compose
```

Apple's `container` runtime uses the same OCI image:

```bash
container build -t hkevents .
container run --rm hkevents --help
container run --rm -i hkevents mcp
```

The runtime image contains the release binary and CA certificates, runs as a
non-root user, and defaults to `--help` so a bare `docker run` is harmless.

### Cloudflare and WASM

There is no Worker or WASM deployment. The binary talks directly to upstream
HTTP APIs over TLS and uses a Tokio runtime; packaging it as a stdio server is
the supported integration. A Worker would need a separate HTTP/JavaScript
adapter and is out of scope for this repo.

### Releases

Pushing a `v*` tag runs the release workflow, which checks the tag against
`Cargo.toml` and `CHANGELOG.md`, builds four targets, smoke-tests native
binaries and publishes:

- `hkevents-<tag>-<target>.tar.gz` for macOS and static Linux
- `SHA256SUMS` covering every tarball

`scripts/install.sh` downloads the latest release, verifies its checksum and
falls back to `cargo build --release --locked` when no release is available.
`just tag X.Y.Z` creates the annotated tag locally; `just release X.Y.Z` tags
and pushes to trigger the workflow.

## Development

```bash
just                 # fmt check, clippy with -D warnings, tests
just skill-check     # validate SKILL.md and shell syntax
just container       # build the image and print CLI help
cargo test --all --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

`rust-toolchain.toml` pins the stable toolchain with `rustfmt` and
`clippy`, so `rustup` sets contributors up automatically. `just doc`
builds the API documentation. See `CONTRIBUTING.md` for the full check
list and the signed-commit requirement.


CI runs the same checks. Dependabot proposes weekly Cargo and GitHub Actions
updates, and the audit workflow checks the lockfile against the RustSec
advisory database.

## Licence

MIT. Event data remains subject to DATA.GOV.HK and LCSD terms.
