#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export PYTHONDONTWRITEBYTECODE=1
python3 scripts/pre_submit_static.py
python3 scripts/check_architecture.py
./scripts/bootstrap-check.sh
cargo metadata --locked --no-deps --format-version 1 >/dev/null
cargo fmt --all --check
cargo check --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git diff --check
fi
echo "PRE-SUBMIT: PASS"
