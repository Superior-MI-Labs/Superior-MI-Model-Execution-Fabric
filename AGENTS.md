# AGENTS.md

Read in this order:

1. `ARCHITECTURE.md`
2. `docs/status/CURRENT.md`
3. `docs/roadmap/R0_WAVES.md`
4. the relevant contract/ADR
5. exact failing source and evidence before editing a defect

## Constitutional rules

- Superior MI Builder R1 owns canonical System IR and `GraphDelta`. Do not recreate either here.
- `DefinitionRegistry` remains the semantic/module definition authority. Do not create a second capability registry.
- `EnvironmentSnapshot` is read-only evidence for planning, never structural truth.
- Provider adapters execute capabilities. They do not own graph structure.
- Model artifacts and provider runtimes are separate identities.
- Multiple compatible providers must not be resolved by arbitrary iteration order.
- Planning must produce an explicit decision/evidence trail and later propose ordinary Builder mutations.
- Missing compatibility is a structured failure, not generated glue.
- No normal feature work belongs in Wave 5; that wave attacks the R0 claim.

## Change discipline

For bugs: reproduce from exact evidence, identify the violated invariant, fix the canonical owner, add a regression test, run the narrow test, then run `./scripts/verify.sh`.

Do not silence Clippy to move a gate. Do not change toolchain or edition to make a symptom disappear. Do not add parallel pipelines or hidden mutable truth.

## Qualification discipline

Delivered candidates must pass `./scripts/verify.sh` unchanged on the pinned Rust toolchain. Do not run `cargo fmt` before qualification; formatting drift is an artifact defect. Rust-free static checks are an early filter only.
