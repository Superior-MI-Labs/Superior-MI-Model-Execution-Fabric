#!/usr/bin/env bash
set -euo pipefail
for cmd in python3 cargo rustc; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "MISSING $cmd" >&2
    exit 1
  fi
  echo "READY  $cmd: $($cmd --version 2>&1 | head -n1)"
done
expected="1.98.1"
actual="$(rustc --version | awk '{print $2}')"
if [[ "$actual" != "$expected" ]]; then
  echo "ERROR expected rustc $expected, found $actual" >&2
  exit 1
fi
echo "READY  pinned Rust toolchain: $expected"
