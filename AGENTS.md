# AGENTS.md

## R0 state

**MEF R0 is CLOSED / QUALIFIED / FROZEN.**

Qualified tag: **mef-r0-qualified**

Qualified commit: **33f63246244f91acfba5659bba80447e6e181108**

Do not treat docs/roadmap/R0_WAVES.md as an active plan. It is the historical record of the completed R0 sequence.

Read in this order:

1. ARCHITECTURE.md
2. docs/status/CURRENT.md
3. relevant contract/ADR
4. docs/roadmap/R0_WAVES.md only for historical R0 lineage
5. exact failing source and evidence before editing a defect

## Constitutional rules

- Superior MI Builder R1 owns canonical System IR and GraphDelta. Do not recreate either here.
- DefinitionRegistry remains the semantic/module definition authority. Do not create a second capability registry.
- EnvironmentSnapshot is read-only evidence for planning, never structural truth.
- Provider adapters execute capabilities. They do not own graph structure or provider-selection policy.
- Model artifacts and provider runtimes are separate identities.
- Multiple compatible providers must not be resolved by arbitrary iteration order.
- Planning requires explicit provider policy and current qualification evidence.
- DeploymentPlan is a proposal. It never commits Builder state.
- execute-plan follows a plan; it must never perform fallback selection.
- Missing compatibility is a structured failure, not generated glue.

## Frozen R0 surfaces

Do not casually modify the R0 contract or qualified provider path:

- smi-mef-core
- smi-mef-llamacpp
- smi-mef-air
- smi-mef-plan
- text.generate@1.0.0
- Builder authority boundaries
- explicit evidence-bound planning semantics

Future development should consume the R0 boundary. If new evidence falsifies an R0 invariant, reproduce the failure first, repair the owning layer, add regression coverage, and re-qualify deliberately. Do not silently redefine the old claim.

Never move or retarget the mef-r0-qualified tag.

## Change discipline

For bugs:

1. reproduce from exact evidence;
2. identify the violated invariant;
3. fix the canonical owner;
4. add regression and edge-case tests;
5. run the narrow test;
6. run ./scripts/verify.sh;
7. run live qualification when the changed surface affects an R0 claim.

Do not silence Clippy to move a gate. Do not change toolchain or edition to make a symptom disappear. Do not add parallel pipelines, hidden mutable truth, implicit fallback, or another registry.

## Qualification discipline

Repository gate:

~~~bash
./scripts/verify.sh
~~~

R0 live qualification:

~~~bash
./scripts/qualify-r0.sh
~~~

Evidence packaging:

~~~bash
./scripts/collect-r0-evidence.sh
~~~

Historical evidence under docs/verification/ should not be rewritten merely to make old in-progress wording look current.
