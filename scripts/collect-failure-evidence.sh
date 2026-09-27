#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$ROOT/failure-evidence/$STAMP"
mkdir -p "$OUT"
{
  echo "date_utc=$(date -u --iso-8601=seconds)"
  rustc --version || true
  cargo --version || true
  git status --short 2>/dev/null || true
} > "$OUT/environment.txt" 2>&1
cp "$ROOT/Cargo.toml" "$ROOT/rust-toolchain.toml" "$OUT/"
if [[ -f "$ROOT/Cargo.lock" ]]; then
  cp "$ROOT/Cargo.lock" "$OUT/"
fi
if [[ -f "$ROOT/SOURCE-SHA256.txt" ]]; then
  cp "$ROOT/SOURCE-SHA256.txt" "$OUT/"
fi
cp -a "$ROOT/crates" "$OUT/"
cp -a "$ROOT/scripts" "$OUT/"
mkdir -p "$OUT/docs"
cp -a "$ROOT/docs/status" "$OUT/docs/"
cp -a "$ROOT/docs/verification" "$OUT/docs/"

# Re-run the non-mutating canonical repository gate to retain the actual failure output.
set +e
"$ROOT/scripts/pre-submit.sh" > "$OUT/pre-submit.log" 2>&1
PRE_SUBMIT_RC=$?
set -e
printf '%s\n' "$PRE_SUBMIT_RC" > "$OUT/pre-submit.exit-code"

# If Wave 2 reached the live-provider stage, preserve those receipts/logs without
# invoking the provider again.
if [[ -d "$ROOT/local/wave2" ]]; then
  mkdir -p "$OUT/local"
  cp -a "$ROOT/local/wave2" "$OUT/local/"
fi
(
  cd "$ROOT/failure-evidence"
  zip -qr "$ROOT/Superior-MI-MEF-Failure-Evidence-$STAMP.zip" "$STAMP"
)
echo "$ROOT/Superior-MI-MEF-Failure-Evidence-$STAMP.zip"
