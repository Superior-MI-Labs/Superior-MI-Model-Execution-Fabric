#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
find . -type f \
  ! -path './target/*' \
  ! -path './.git/*' \
  ! -path './local/*' \
  ! -path './failure-evidence/*' \
  ! -name 'Superior-MI-MEF-*.zip' \
  ! -name 'SOURCE-SHA256.txt' \
  -print0 | sort -z | xargs -0 sha256sum > SOURCE-SHA256.txt
echo "wrote SOURCE-SHA256.txt"
