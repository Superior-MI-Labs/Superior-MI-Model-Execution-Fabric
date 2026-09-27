#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

LLAMA_ENDPOINT="${MEF_LLAMA_ENDPOINT:-http://127.0.0.1:1920}"
AIR_ENDPOINT="${MEF_AIR_ENDPOINT:-http://127.0.0.1:8181}"
MODEL_ROOT="${MEF_MODEL_ROOT:-$HOME/Models}"
LLAMA_MODEL="${MEF_LLAMA_MODEL:-}"
AIR_MODEL="${MEF_AIR_MODEL:-}"
TIMEOUT_SECONDS="${MEF_R0_TIMEOUT_SECONDS:-30}"
OUT="$ROOT/local/r0"
BASELINE_FILE="$ROOT/docs/verification/WAVE-3-BASELINE.txt"

if [[ ! -f "$BASELINE_FILE" ]]; then
  echo "ERROR: missing Wave 3 baseline identity: $BASELINE_FILE" >&2
  exit 1
fi
WAVE3_BASELINE="$(tr -d '[:space:]' < "$BASELINE_FILE")"
if ! git rev-parse --verify "$WAVE3_BASELINE^{commit}" >/dev/null 2>&1; then
  echo "ERROR: recorded Wave 3 baseline is not a commit: $WAVE3_BASELINE" >&2
  exit 1
fi
for frozen in crates/smi-mef-core crates/smi-mef-llamacpp crates/smi-mef-air; do
  if ! git diff --quiet "$WAVE3_BASELINE" -- "$frozen"; then
    echo "ERROR: R0 endpoint work changed frozen provider/core surface '$frozen'." >&2
    exit 1
  fi
done

sha256sum -c SOURCE-SHA256.txt >/dev/null
./scripts/verify.sh

rm -rf "$OUT"
mkdir -p "$OUT"
exec > >(tee "$OUT/qualify-r0.log") 2>&1

cargo run --locked --quiet -p smi-mef-cli -- snapshot \
  --llama-endpoint "$LLAMA_ENDPOINT" \
  --air-endpoint "$AIR_ENDPOINT" \
  --model-root "$MODEL_ROOT" \
  --output "$OUT/environment.json"

LLAMA_QUALIFY=(
  cargo run --locked --quiet -p smi-mef-cli -- llama-qualify
  --endpoint "$LLAMA_ENDPOINT"
  --environment "$OUT/environment.json"
  --output "$OUT/llama-qualification.json"
  --timeout-seconds "$TIMEOUT_SECONDS"
)
if [[ -n "$LLAMA_MODEL" ]]; then
  LLAMA_QUALIFY+=(--model "$LLAMA_MODEL")
fi
"${LLAMA_QUALIFY[@]}"

AIR_QUALIFY=(
  cargo run --locked --quiet -p smi-mef-cli -- air-qualify
  --endpoint "$AIR_ENDPOINT"
  --environment "$OUT/environment.json"
  --output "$OUT/air-qualification.json"
  --timeout-seconds "$TIMEOUT_SECONDS"
)
if [[ -n "$AIR_MODEL" ]]; then
  AIR_QUALIFY+=(--model "$AIR_MODEL")
fi
"${AIR_QUALIFY[@]}"

# The live harness supplies an explicit Builder revision token. The planner never owns or invents
# graph state. Real GraphDelta application against Builder authority is independently exercised by
# smi-mef-plan's integration test during ./scripts/verify.sh.
BASE_REVISION="$(printf '%s' 'mef-r0-live-qualification-target' | sha256sum | awk '{print $1}')"

cargo run --locked --quiet -p smi-mef-cli -- plan \
  --environment "$OUT/environment.json" \
  --qualification "$OUT/air-qualification.json" \
  --qualification "$OUT/llama-qualification.json" \
  --provider provider.llamacpp.http \
  --base-revision "$BASE_REVISION" \
  --consumer app.generator \
  --slot generator \
  --provider-instance runtime.llama \
  --current-provider-instance runtime.air \
  --output "$OUT/plan-llama.json" \
  --delta-output "$OUT/delta-llama.json"

cargo run --locked --quiet -p smi-mef-cli -- plan \
  --environment "$OUT/environment.json" \
  --qualification "$OUT/llama-qualification.json" \
  --qualification "$OUT/air-qualification.json" \
  --provider provider.air.http \
  --base-revision "$BASE_REVISION" \
  --consumer app.generator \
  --slot generator \
  --provider-instance runtime.air \
  --current-provider-instance runtime.llama \
  --output "$OUT/plan-air.json" \
  --delta-output "$OUT/delta-air.json"

# Replay with reversed candidate order. Output must be byte-identical.
cargo run --locked --quiet -p smi-mef-cli -- plan \
  --environment "$OUT/environment.json" \
  --qualification "$OUT/air-qualification.json" \
  --qualification "$OUT/llama-qualification.json" \
  --provider provider.air.http \
  --base-revision "$BASE_REVISION" \
  --consumer app.generator \
  --slot generator \
  --provider-instance runtime.air \
  --current-provider-instance runtime.llama \
  --output "$OUT/plan-air-replay.json" \
  --delta-output "$OUT/delta-air-replay.json"
cmp "$OUT/plan-air.json" "$OUT/plan-air-replay.json"
cmp "$OUT/delta-air.json" "$OUT/delta-air-replay.json"

PROMPT='MEF_R0_PROVIDER_SUBSTITUTION_PROBE'
TOKENS=8
cargo run --locked --quiet -p smi-mef-cli -- execute-plan \
  --plan "$OUT/plan-llama.json" \
  --qualification "$OUT/llama-qualification.json" \
  --prompt "$PROMPT" \
  --max-output-tokens "$TOKENS" \
  --output "$OUT/execution-llama.json" \
  --timeout-seconds "$TIMEOUT_SECONDS"

cargo run --locked --quiet -p smi-mef-cli -- execute-plan \
  --plan "$OUT/plan-air.json" \
  --qualification "$OUT/air-qualification.json" \
  --prompt "$PROMPT" \
  --max-output-tokens "$TOKENS" \
  --output "$OUT/execution-air.json" \
  --timeout-seconds "$TIMEOUT_SECONDS"

# Destructive test: absence of explicit policy must not guess.
if cargo run --locked --quiet -p smi-mef-cli -- plan \
  --environment "$OUT/environment.json" \
  --qualification "$OUT/llama-qualification.json" \
  --qualification "$OUT/air-qualification.json" \
  --base-revision "$BASE_REVISION" \
  --consumer app.generator \
  --slot generator \
  --provider-instance runtime.air \
  --output "$OUT/should-not-exist-no-policy.json"; then
  echo "ERROR: planner guessed without explicit provider policy" >&2
  exit 1
else
  echo "PASS destructive: missing policy rejected"
fi

# Destructive test: stale qualification evidence must not be rebound to a changed environment.
python3 - "$OUT/environment.json" "$OUT/stale-environment.json" <<'PY'
import json
import pathlib
import sys
source = pathlib.Path(sys.argv[1])
target = pathlib.Path(sys.argv[2])
value = json.loads(source.read_text())
value.setdefault("warnings", []).append("r0.stale.environment.probe")
target.write_text(json.dumps(value, separators=(",", ":")))
PY
if cargo run --locked --quiet -p smi-mef-cli -- plan \
  --environment "$OUT/stale-environment.json" \
  --qualification "$OUT/llama-qualification.json" \
  --qualification "$OUT/air-qualification.json" \
  --provider provider.air.http \
  --base-revision "$BASE_REVISION" \
  --consumer app.generator \
  --slot generator \
  --provider-instance runtime.air \
  --output "$OUT/should-not-exist-stale.json"; then
  echo "ERROR: planner accepted stale qualification evidence" >&2
  exit 1
else
  echo "PASS destructive: stale evidence rejected"
fi

# Destructive test: retained provider evidence tampering must be caught before planning.
python3 - "$OUT/air-qualification.json" "$OUT/tampered-air-qualification.json" <<'PY'
import json
import pathlib
import sys
source = pathlib.Path(sys.argv[1])
target = pathlib.Path(sys.argv[2])
value = json.loads(source.read_text())
value["runtime_response"]["backend"] = "tampered"
target.write_text(json.dumps(value, separators=(",", ":")))
PY
if cargo run --locked --quiet -p smi-mef-cli -- plan \
  --environment "$OUT/environment.json" \
  --qualification "$OUT/tampered-air-qualification.json" \
  --provider provider.air.http \
  --base-revision "$BASE_REVISION" \
  --consumer app.generator \
  --slot generator \
  --provider-instance runtime.air \
  --output "$OUT/should-not-exist-tampered.json"; then
  echo "ERROR: planner accepted tampered provider evidence" >&2
  exit 1
else
  echo "PASS destructive: tampered evidence rejected"
fi

# Destructive test: a plan cannot execute against another provider's qualification.
if cargo run --locked --quiet -p smi-mef-cli -- execute-plan \
  --plan "$OUT/plan-air.json" \
  --qualification "$OUT/llama-qualification.json" \
  --prompt "$PROMPT" \
  --max-output-tokens "$TOKENS" \
  --output "$OUT/should-not-exist-cross-provider.json" \
  --timeout-seconds "$TIMEOUT_SECONDS"; then
  echo "ERROR: plan executed with mismatched provider qualification" >&2
  exit 1
else
  echo "PASS destructive: cross-provider qualification rejected"
fi

# Destructive test: configured but unreachable provider fails as evidence, not invented glue.
cargo run --locked --quiet -p smi-mef-cli -- snapshot \
  --llama-endpoint http://127.0.0.1:1 \
  --air-endpoint "$AIR_ENDPOINT" \
  --model-root "$MODEL_ROOT" \
  --output "$OUT/unreachable-environment.json"
if cargo run --locked --quiet -p smi-mef-cli -- llama-qualify \
  --endpoint http://127.0.0.1:1 \
  --environment "$OUT/unreachable-environment.json" \
  --output "$OUT/should-not-exist-unreachable.json" \
  --timeout-seconds 2; then
  echo "ERROR: unreachable provider unexpectedly qualified" >&2
  exit 1
else
  echo "PASS destructive: unreachable provider rejected"
fi

python3 - "$OUT" <<'PY'
import json
import pathlib
import sys

out = pathlib.Path(sys.argv[1])
llama_plan = json.loads((out / "plan-llama.json").read_text())
air_plan = json.loads((out / "plan-air.json").read_text())
llama_exec = json.loads((out / "execution-llama.json").read_text())
air_exec = json.loads((out / "execution-air.json").read_text())

assert llama_plan["schema"] == 1
assert air_plan["schema"] == 1
assert llama_plan["capability"] == air_plan["capability"]
assert llama_plan["capability"] == {
    "id": "text.generate",
    "version": {"major": 1, "minor": 0, "patch": 0},
}
assert llama_plan["environment_sha256"] == air_plan["environment_sha256"]
assert llama_plan["provider"] == "provider.llamacpp.http"
assert air_plan["provider"] == "provider.air.http"
assert llama_plan["qualification_sha256"] != air_plan["qualification_sha256"]
assert llama_plan["target"]["consumer"] == air_plan["target"]["consumer"] == "app.generator"
assert llama_plan["target"]["slot"] == air_plan["target"]["slot"] == "generator"
assert llama_plan["target"]["provider_instance"] == "runtime.llama"
assert air_plan["target"]["provider_instance"] == "runtime.air"
assert llama_plan["graph_delta"] is not None
assert air_plan["graph_delta"] is not None

lr = llama_exec["receipt"]
ar = air_exec["receipt"]
assert lr["capability"] == ar["capability"] == llama_plan["capability"]
assert lr["request_sha256"] == ar["request_sha256"]
assert lr["provider"] == "provider.llamacpp.http"
assert ar["provider"] == "provider.air.http"
assert lr["qualification_sha256"] == llama_plan["qualification_sha256"]
assert ar["qualification_sha256"] == air_plan["qualification_sha256"]
assert llama_exec["response"]["text"]
assert air_exec["response"]["text"]

print("MEF R0 QUALIFIED PROVIDER SUBSTITUTION: PASS")
print("capability=", lr["capability"])
print("request_sha256=", lr["request_sha256"])
print("llama_plan_provider=", llama_plan["provider"])
print("air_plan_provider=", air_plan["provider"])
PY

(
  cd "$OUT"
  sha256sum \
    environment.json \
    llama-qualification.json \
    air-qualification.json \
    plan-llama.json \
    plan-air.json \
    delta-llama.json \
    delta-air.json \
    execution-llama.json \
    execution-air.json
) > "$OUT/SHA256SUMS.txt"
