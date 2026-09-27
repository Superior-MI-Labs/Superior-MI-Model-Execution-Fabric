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
- Provider adapters execute capabilities. They do not own graph structure or provider-selection policy.
- Model artifacts and provider runtimes are separate identities.
- Multiple compatible providers must not be resolved by arbitrary iteration order.
- R0 planning requires explicit provider policy and current qualification evidence.
- `DeploymentPlan` is a proposal. It never commits Builder state.
- `execute-plan` follows a plan; it must never perform fallback selection.
- Missing compatibility is a structured failure, not generated glue.
- No normal feature work belongs in Wave 5; that wave attacks the R0 claim.

## Frozen R0 surfaces

After Wave 3 qualification, do not modify these during the Wave 4/5 endpoint push unless new evidence
falsifies them:

- `smi-mef-core`
- `smi-mef-llamacpp`
- `smi-mef-air`
- `text.generate@1.0.0`

## Change discipline

For bugs: reproduce from exact evidence, identify the violated invariant, fix the canonical owner, add a
regression test, run the narrow test, then run `./scripts/verify.sh`.

Do not silence Clippy to move a gate. Do not change toolchain or edition to make a symptom disappear.
Do not add parallel pipelines, hidden mutable truth, implicit fallback, or another registry.

## Qualification discipline

Delivered candidates must pass `./scripts/verify.sh` on the pinned Rust toolchain. Final R0 qualification
is `./scripts/qualify-r0.sh`; final evidence packaging is `./scripts/collect-r0-evidence.sh`.
