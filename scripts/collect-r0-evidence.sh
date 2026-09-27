#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT_DIR="$ROOT/local/r0"
if [[ ! -f "$OUT_DIR/execution-air.json" || ! -f "$OUT_DIR/execution-llama.json" ]]; then
  echo "ERROR: R0 live qualification evidence is missing. Run ./scripts/qualify-r0.sh first." >&2
  exit 1
fi

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
EVIDENCE="$STAGE/Superior-MI-MEF-R0-Evidence-$STAMP"
mkdir -p "$EVIDENCE" "$EVIDENCE/docs"

cp -a "$OUT_DIR" "$EVIDENCE/live"
cp Cargo.toml Cargo.lock rust-toolchain.toml SOURCE-SHA256.txt "$EVIDENCE/"
cp -a docs/status docs/contracts docs/decisions docs/verification "$EVIDENCE/docs"
cp -a scripts "$EVIDENCE/"

{
  echo "date_utc=$(date -u --iso-8601=seconds)"
  echo "git_head=$(git rev-parse HEAD 2>/dev/null || echo unavailable)"
  echo "wave3_baseline=$(cat docs/verification/WAVE-3-BASELINE.txt 2>/dev/null || echo unavailable)"
  rustc --version
  cargo --version
} > "$EVIDENCE/toolchain.txt"

ARCHIVE="$ROOT/Superior-MI-MEF-R0-Evidence-$STAMP.zip"
rm -f "$ARCHIVE"
(
  cd "$STAGE"
  zip -X -q -r "$ARCHIVE" "$(basename "$EVIDENCE")"
)
sha256sum "$ARCHIVE"
echo "$ARCHIVE"
