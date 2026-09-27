#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT_DIR="$ROOT/local/wave2"
if [[ ! -f "$OUT_DIR/llama-execution.json" ]]; then
  echo "ERROR: Wave 2 live evidence is missing. Run ./scripts/qualify-wave2.sh first." >&2
  exit 1
fi

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
EVIDENCE="$STAGE/Superior-MI-MEF-Wave2-Evidence-$STAMP"
mkdir -p "$EVIDENCE" "$EVIDENCE/docs"

cp -a "$OUT_DIR" "$EVIDENCE/live"
cp Cargo.toml rust-toolchain.toml SOURCE-SHA256.txt "$EVIDENCE/"
[[ -f Cargo.lock ]] && cp Cargo.lock "$EVIDENCE/"
cp -a docs/status docs/contracts docs/decisions docs/verification "$EVIDENCE/docs"
cp -a scripts "$EVIDENCE/"

{
  echo "date_utc=$(date -u --iso-8601=seconds)"
  rustc --version
  cargo --version
} > "$EVIDENCE/toolchain.txt"

ARCHIVE="$ROOT/Superior-MI-MEF-Wave2-Evidence-$STAMP.zip"
rm -f "$ARCHIVE"
(
  cd "$STAGE"
  zip -X -q -r "$ARCHIVE" "$(basename "$EVIDENCE")"
)
sha256sum "$ARCHIVE"
echo "$ARCHIVE"
