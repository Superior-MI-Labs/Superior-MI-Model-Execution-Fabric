#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

ENDPOINT="${MEF_LLAMA_ENDPOINT:-http://127.0.0.1:1920}"
AIR_ENDPOINT="${MEF_AIR_ENDPOINT:-http://127.0.0.1:8181}"
MODEL_ROOT="${MEF_MODEL_ROOT:-$HOME/Models}"
MODEL="${MEF_LLAMA_MODEL:-}"
TIMEOUT_SECONDS="${MEF_LLAMA_TIMEOUT_SECONDS:-30}"
OUT="$ROOT/local/wave2"

rm -rf "$OUT"
mkdir -p "$OUT"
exec > >(tee "$OUT/qualify-wave2.log") 2>&1

./scripts/verify.sh

cargo run --locked --quiet -p smi-mef-cli -- snapshot \
  --llama-endpoint "$ENDPOINT" \
  --air-endpoint "$AIR_ENDPOINT" \
  --model-root "$MODEL_ROOT" \
  --output "$OUT/environment.json"

QUALIFY=(
  cargo run --locked --quiet -p smi-mef-cli -- llama-qualify
  --endpoint "$ENDPOINT"
  --environment "$OUT/environment.json"
  --output "$OUT/llama-qualification.json"
  --timeout-seconds "$TIMEOUT_SECONDS"
)
if [[ -n "$MODEL" ]]; then
  QUALIFY+=(--model "$MODEL")
fi
"${QUALIFY[@]}"

cargo run --locked --quiet -p smi-mef-cli -- llama-generate \
  --qualification "$OUT/llama-qualification.json" \
  --prompt "Respond briefly with the text MEF_WAVE2_OK." \
  --max-output-tokens 32 \
  --output "$OUT/llama-execution.json" \
  --timeout-seconds "$TIMEOUT_SECONDS"

python3 - "$OUT" <<'PY'
import json
import pathlib
import sys

out = pathlib.Path(sys.argv[1])
environment = json.loads((out / "environment.json").read_text())
qualification = json.loads((out / "llama-qualification.json").read_text())
execution = json.loads((out / "llama-execution.json").read_text())

assert environment["schema"] == 1
q = qualification["qualification"]
assert q["schema"] == 1
assert q["provider"] == "provider.llamacpp.http"
assert q["selected_model"]
assert q["advertised_models"]
assert q["selected_model"] in q["advertised_models"]
assert q["environment_sha256"]
assert q["provider_evidence_sha256"]
assert qualification["health_response"]["status"] == "ok"
assert len(qualification["completion_probe_response"]["choices"]) == 1

assert execution["schema"] == 1
assert execution["receipt"]["schema"] == 1
assert execution["receipt"]["provider"] == "provider.llamacpp.http"
assert execution["receipt"]["model"] == q["selected_model"]
assert execution["response"]["text"].strip()
assert execution["receipt"]["request_sha256"]
assert execution["receipt"]["provider_response_sha256"]
PY

sha256sum \
  "$OUT/environment.json" \
  "$OUT/llama-qualification.json" \
  "$OUT/llama-execution.json" \
  > "$OUT/SHA256SUMS.txt"

echo "WAVE2 LIVE QUALIFICATION: PASS"
