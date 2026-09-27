# Current Status

Program: Superior MI Model Execution Fabric R0

- Wave 0: CLOSED - architecture/authority freeze
- Wave 1: CLOSED - environment snapshot + runtime inventory; pinned-toolchain VERIFY/PRE-SUBMIT passed on WolfCat-Studio
- Wave 2: CURRENT - live `llama.cpp` `text.generate@1.0.0` provider qualification
- Waves 3–5: NOT STARTED
- R0 status: UNQUALIFIED

## Wave 1 qualification lineage

Wave 1 required three candidate corrections before closure:

- `WAVE-1-FAILURE-001`: initial rustfmt drift;
- `WAVE-1-FAILURE-002`: public capability helper hid Builder name validation behind a panic;
- `WAVE-1-FAILURE-003`: rustfmt drift introduced by the ownership repair.

The final Wave 1 candidate then passed both `PRE-SUBMIT: PASS` and `VERIFY: PASS` unchanged on Rust 1.98.1.

## Wave 2 scope

Wave 2 adds exactly one real provider adapter:

```text
provider.llamacpp.http
        ↓
text.generate@1.0.0
```

A capability claim requires:

1. a valid Wave 1 `EnvironmentSnapshot` containing the configured endpoint candidate;
2. `GET /health` with `status == "ok"`;
3. `GET /v1/models` with at least one model;
4. deterministic model selection, or explicit selection when more than one model is advertised;
5. retained qualification evidence bound to the environment snapshot SHA-256.

Generation requires that serialized qualification document and uses `/v1/completions` with bounded output, `temperature = 0`, and non-streaming execution.

## Explicit boundary

Wave 2 does not choose among different providers, mutate Builder `SystemGraph`, create `GraphDelta`, introduce chat-template semantics into `text.generate`, or touch AIR execution.

## Exit gate

Run on WolfCat-Studio with the local `llama.cpp` server already available:

```bash
./scripts/qualify-wave2.sh
```

The wave closes only after the full repository gate and real live provider test end with:

```text
VERIFY: PASS
WAVE2 LIVE QUALIFICATION: PASS
```

Then run:

```bash
./scripts/collect-wave2-evidence.sh
```

Wave 3 is the AIR provider for the same capability contract.
