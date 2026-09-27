# Model Execution Fabric R0 Architecture

## Purpose

MEF connects stable Builder capability intent to concrete local execution stacks while preserving
Builder as structural authority.

## Authority map

```text
Superior MI Builder R1
  DefinitionRegistry   semantic/module definitions
  SystemGraph          canonical structure
  GraphDelta           only structural mutation path
        │
        ▼
Model Execution Fabric
  EnvironmentSnapshot  immutable observation evidence
  Provider Adapters    runtime-specific qualification/execution
  Deployment Planner   deterministic evidence/policy proposal
        │
        ▼
External runtimes
  llama.cpp / AIR / later stacks
```

MEF must never become a second graph, registry, scheduler, model server, or capability authority.

## Stable capability identity

MEF reuses Builder's `CapabilityRef`. R0 qualifies:

```text
text.generate@1.0.0
```

The semantic capability does not encode llama.cpp, AIR, CUDA, GGUF, endpoint, or model-path details.

## Observation authority

`EnvironmentSnapshot` is deterministic read-only evidence. It may describe observed hardware,
runtime candidates, and model roots, but it is not System IR and cannot mutate Builder state.

## Provider boundary

Provider adapters independently qualify a concrete runtime/model/environment combination for a stable
capability and retain provider-specific evidence.

```text
smi-mef-core
  ProviderQualification
  TextGenerateRequest
  TextGenerateResponse
  TextGenerationReceipt
        ▲
   ┌────┴────┐
   │         │
smi-mef-llamacpp   smi-mef-air
```

Provider-local model identity remains provider-local in R0. Cross-provider model artifact equivalence
is not fabricated from names or paths.

## Deployment planning

`smi-mef-plan` consumes current environment evidence plus independently validated provider
qualifications under an explicit `DeploymentPolicy`.

```text
EnvironmentSnapshot
      +
ProviderQualification[]
      +
explicit provider policy
      +
Builder-owned realization target
      │
      ▼
DeploymentPlan
      ├── selected qualification identity
      ├── provider / endpoint / model evidence
      └── ordinary Builder GraphDelta proposal
```

The planner is stateless. It does not apply `GraphDelta`, mutate `SystemGraph`, or register semantic
definitions. Missing, stale, capability-incompatible, or ambiguous evidence is a structured failure.
Candidate iteration order cannot decide the plan.

## Planned execution

`execute-plan` is not a second planner. It follows an already explicit plan, revalidates the exact
provider-specific qualification selected by that plan, and delegates to the existing provider adapter.

```text
DeploymentPlan + exact qualification + TextGenerateRequest
        │
        ├── provider.llamacpp.http -> existing llama.cpp adapter
        └── provider.air.http      -> existing AIR adapter
```

No fallback occurs inside execution. A mismatched qualification is rejected.

## Builder realization

The planner proposes only ordinary Builder binding mutations:

```text
unbound -> BindCapability
bound A -> UnbindCapability(A) + BindCapability(B)
bound B -> no structural mutation required
```

Every mutation proposal is bound to an exact `GraphRevision`. Builder remains responsible for
application, validation, and commit. The R0 test suite applies a planner-generated substitution delta
through Builder's own `GraphDelta::apply` path to prove this boundary is real.

## R0 dependency direction

```text
smi-ir (Builder R1)
   ↑
smi-mef-core
   ├── smi-mef-observe
   ├── smi-mef-llamacpp
   ├── smi-mef-air
   └── smi-mef-plan
            ↑
       smi-mef-cli
```

Builder imports no MEF crate.

## R0 endpoint

The release claim is **Qualified Provider Substitution**:

> The same high-level `text.generate@1.0.0` intent executes through two independently qualified real
> provider stacks while only explicit deployment realization changes. Selection is evidence- and
> policy-bound, Builder remains structural authority, and missing compatibility fails rather than
> generating hidden integration glue.
