#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BASELINE="a4db716"
AIR_ENDPOINT="${MEF_AIR_ENDPOINT:-http://127.0.0.1:8181}"
LLAMA_ENDPOINT="${MEF_LLAMA_ENDPOINT:-http://127.0.0.1:1920}"
MODEL_ROOT="${MEF_MODEL_ROOT:-$HOME/Models}"
MODEL="${MEF_AIR_MODEL:-}"
TIMEOUT_SECONDS="${MEF_AIR_TIMEOUT_SECONDS:-30}"
OUT="$ROOT/local/wave3"

if git rev-parse --verify "$BASELINE^{commit}" >/dev/null 2>&1; then
  if ! git diff --quiet "$BASELINE" -- crates/smi-mef-core; then
    echo "ERROR: Wave 3 changed frozen smi-mef-core relative to $BASELINE." >&2
    exit 1
  fi
  if ! git diff --quiet "$BASELINE" -- crates/smi-mef-llamacpp; then
    echo "ERROR: Wave 3 changed frozen smi-mef-llamacpp relative to $BASELINE." >&2
    exit 1
  fi
fi

rm -rf "$OUT"
mkdir -p "$OUT"
exec > >(tee "$OUT/qualify-wave3.log") 2>&1

./scripts/verify.sh

cargo run --locked --quiet -p smi-mef-cli -- snapshot \
  --llama-endpoint "$LLAMA_ENDPOINT" \
  --air-endpoint "$AIR_ENDPOINT" \
  --model-root "$MODEL_ROOT" \
  --output "$OUT/environment.json"

QUALIFY=(
  cargo run --locked --quiet -p smi-mef-cli -- air-qualify
  --endpoint "$AIR_ENDPOINT"
  --environment "$OUT/environment.json"
  --output "$OUT/air-qualification.json"
  --timeout-seconds "$TIMEOUT_SECONDS"
)
if [[ -n "$MODEL" ]]; then
  QUALIFY+=(--model "$MODEL")
fi
"${QUALIFY[@]}"

cargo run --locked --quiet -p smi-mef-cli -- air-generate \
  --qualification "$OUT/air-qualification.json" \
  --prompt "Respond briefly with the text MEF_WAVE3_OK." \
  --max-output-tokens 32 \
  --output "$OUT/air-execution.json" \
  --timeout-seconds "$TIMEOUT_SECONDS"

python3 - "$OUT" <<'PY'
import json
import pathlib
import sys

out = pathlib.Path(sys.argv[1])
environment = json.loads((out / "environment.json").read_text())
qualification = json.loads((out / "air-qualification.json").read_text())
execution = json.loads((out / "air-execution.json").read_text())

assert environment["schema"] == 1
q = qualification["qualification"]
assert q["schema"] == 1
assert q["provider"] == "provider.air.http"
assert q["selected_model"]
assert q["advertised_models"]
assert q["selected_model"] in q["advertised_models"]
assert q["environment_sha256"]
assert q["provider_evidence_sha256"]
assert qualification["health_response"]["status"] == "ok"
assert qualification["model_response"]["id"] == q["selected_model"]
assert qualification["runtime_response"]["backend"]
assert len(qualification["completion_probe_response"]["choices"]) == 1

assert execution["schema"] == 1
assert execution["receipt"]["schema"] == 1
assert execution["receipt"]["provider"] == "provider.air.http"
assert execution["receipt"]["model"] == q["selected_model"]
assert execution["response"]["text"]
assert execution["receipt"]["request_sha256"]
assert execution["receipt"]["provider_response_sha256"]
PY

sha256sum \
  "$OUT/environment.json" \
  "$OUT/air-qualification.json" \
  "$OUT/air-execution.json" \
  > "$OUT/SHA256SUMS.txt"

echo "WAVE3 AIR LIVE QUALIFICATION: PASS"
