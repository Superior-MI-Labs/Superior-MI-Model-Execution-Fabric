# Current Status

Program: Superior MI Model Execution Fabric R0

- Wave 0: CLOSED - architecture/authority freeze
- Wave 1: CLOSED - deterministic environment snapshot + runtime inventory
- Wave 2: CLOSED / QUALIFIED - live llama.cpp `text.generate@1.0.0` provider
- Wave 2 Git baseline: `a4db716` (`MEF Wave 2 qualified baseline`)
- Wave 3: CURRENT - AIR provider for the same `text.generate@1.0.0` contract
- Waves 4–5: NOT STARTED
- R0 status: UNQUALIFIED

## Wave 3 hypothesis

AIR should satisfy the existing provider-independent capability contract without changing
`smi-mef-core` or the qualified llama.cpp adapter.

```text
text.generate@1.0.0
        │
   ┌────┴─────┐
   │          │
llama.cpp    AIR
qualified    candidate
```

AIR-specific `/model` and `/runtime` data are retained as provider evidence. They do not
become Builder structural truth or generic MEF core fields.

## Current gate

Wave 3 protocol characterization passed against the local CUDA AIR runtime. The current
implementation gate is `smi-mef-air` plus the thin `air-qualify` / `air-generate` CLI surface.

Run repository verification first:

```bash
./scripts/verify.sh
```

With AIR already running on the configured endpoint, run:

```bash
./scripts/qualify-wave3.sh
```
