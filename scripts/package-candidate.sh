#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT_NAME="$(basename "$ROOT")"
OUTPUT="${1:-$ROOT/../${PROJECT_NAME}-Candidate.zip}"

cd "$ROOT"

./scripts/pre-submit.sh
./scripts/source-manifest.sh
sha256sum -c SOURCE-SHA256.txt >/dev/null

TMP_PARENT="$(mktemp -d)"
trap 'rm -rf "$TMP_PARENT"' EXIT
STAGE="$TMP_PARENT/$PROJECT_NAME"
mkdir -p "$STAGE"

rsync -a \
  --exclude='.git/' \
  --exclude='target/' \
  --exclude='local/' \
  --exclude='failure-evidence/' \
  --exclude='Superior-MI-MEF-*.zip' \
  "$ROOT/" "$STAGE/"

mkdir -p "$(dirname "$OUTPUT")"
rm -f "$OUTPUT"
(
  cd "$TMP_PARENT"
  zip -X -q -r "$OUTPUT" "$PROJECT_NAME"
)

for script in "$STAGE"/scripts/*.sh; do
  [[ -x "$script" ]] || {
    echo "ERROR: staged script is not executable: ${script#$STAGE/}" >&2
    exit 1
  }
done

unzip -tq "$OUTPUT" >/dev/null
printf 'candidate=%s\n' "$OUTPUT"
sha256sum "$OUTPUT"
