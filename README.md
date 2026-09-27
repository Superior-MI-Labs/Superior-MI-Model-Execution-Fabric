# Superior MI Model Execution Fabric

**Provider-independent local model execution built on Superior MI Builder R1.**

Status: **R0 Wave 3 candidate**
Builder dependency: immutable `builder-r1` commit `82a5ed814a42dc9ca4e8c1540227f18d4f97e11e`
Rust toolchain: `1.98.1`

Model Execution Fabric (MEF) is the post-Builder layer that observes a machine, identifies compatible model execution providers, gathers qualification evidence, and eventually proposes an explicit provider realization through ordinary Builder `GraphDelta`.

It is deliberately **not** another System IR, model registry, scheduler, or inference engine.

Project boundaries are intentional: **AIR** is an inference runtime, **Inference Fabric** explores reusable/continuous inference infrastructure, while **Model Execution Fabric** owns provider observation, compatibility evidence, and deployment realization across runtimes.

## R0 endpoint

R0 is complete only when the same Builder-level `text.generate@1.0.0` intent can be realized through two real local provider stacks, initially llama.cpp HTTP and AIR, without changing the high-level capability contract.

```text
high-level intent
      │
      ▼
text.generate@1.0.0
      │
      ▼
Deployment Planner
      │
      ├── llama.cpp HTTP provider
      └── AIR HTTP provider
      │
      ▼
ordinary Builder GraphDelta / explicit binding
      │
      ▼
measured execution evidence
```

Provider discovery and planning are derived evidence. `SystemGraph` remains structural truth and `GraphDelta` remains the structural write path.

## Planned waves

| Wave | Goal | Status |
| --- | --- | --- |
| 0 | Architecture and authority freeze | COMPLETE |
| 1 | Deterministic environment snapshot and runtime inventory | CLOSED |
| 2 | Live llama.cpp `text.generate` provider | CLOSED / QUALIFIED |
| 3 | Live AIR `text.generate` provider | **CURRENT** |
| 4 | Deterministic deployment planner and Builder integration | planned |
| 5 | Destructive R0 qualification and provider-swap proof | planned |

There are **6 planned waves total, 0 through 5**. Do not add Wave 6 merely because qualification finds a defect. Repair the owning layer and repeat qualification.

## Wave 1 behavior

Wave 1 is intentionally observation-only. It may inspect host metadata, memory, NVIDIA telemetry, executable presence, configured endpoint candidates, and model-root existence. It does **not** contact an inference endpoint or invoke a model.

Generate a snapshot:

```bash
cargo run -p smi-mef-cli -- snapshot \
  --llama-endpoint http://127.0.0.1:1920 \
  --air-endpoint http://127.0.0.1:8181 \
  --model-root "$HOME/Models" \
  --output local/environment.json
```

The command writes canonical JSON and prints its SHA-256 identity. Endpoint entries are configured candidates only. Provider adapters convert configured endpoint candidates into measured qualification evidence.

## Verification

```bash
./scripts/bootstrap-check.sh
./scripts/verify.sh
```

On failure:

```bash
./scripts/collect-failure-evidence.sh
```

Fresh agents should read `AGENTS.md`, `ARCHITECTURE.md`, and `docs/status/CURRENT.md` before editing.


## Wave 2 live llama.cpp proof

Wave 2 qualifies the already-running local `llama.cpp` server before allowing it to
claim `text.generate@1.0.0`. Qualification is bound to the Wave 1 environment snapshot
and generation requires that saved qualification document.

On WolfCat-Studio, the default live gate is:

```bash
./scripts/verify.sh
./scripts/qualify-wave2.sh
```

Defaults are `http://127.0.0.1:1920` for `llama.cpp`,
`http://127.0.0.1:8181` for AIR observation, and `$HOME/Models` for the model root.
If `/v1/models` advertises more than one model, set `MEF_LLAMA_MODEL` explicitly.

Successful live evidence is written under `local/wave2/` and can be packaged with:

```bash
./scripts/collect-wave2-evidence.sh
```


## Wave 3 live AIR proof

Wave 3 adds an independent AIR adapter for the same `text.generate@1.0.0` contract.
AIR-specific model/runtime responses remain provider evidence and do not widen MEF core.

With AIR already running on its configured endpoint:

```bash
./scripts/verify.sh
./scripts/qualify-wave3.sh
```

Successful live evidence is written under `local/wave3/`.
