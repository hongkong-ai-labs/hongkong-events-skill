#!/usr/bin/env bash
# Print the absolute path of a working hkevents binary.
set -euo pipefail

if [ -n "${HKEVENTS_BIN:-}" ] && [ -x "$HKEVENTS_BIN" ]; then
  printf '%s\n' "$HKEVENTS_BIN"
  exit 0
fi

if command -v hkevents >/dev/null 2>&1; then
  command -v hkevents
  exit 0
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -f "$ROOT/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
  (cd "$ROOT" && cargo build --release --locked >/dev/null)
  BIN="$ROOT/target/release/hkevents"
  if [ -x "$BIN" ]; then
    printf '%s\n' "$BIN"
    exit 0
  fi
fi

echo "hkevents: no binary found and no Cargo build available" >&2
echo "Set HKEVENTS_BIN, put hkevents on PATH, or build with: cargo build --release --locked" >&2
exit 1
