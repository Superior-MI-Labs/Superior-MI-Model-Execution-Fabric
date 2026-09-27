# Model Execution Fabric R0 Architecture

> **Status: CLOSED · QUALIFIED · FROZEN**

MEF connects stable Builder capability intent to concrete local execution stacks while preserving Superior MI Builder as structural authority.

## R0 authority model

~~~mermaid
flowchart TD
    Builder["Superior MI Builder R1<br/>DefinitionRegistry · SystemGraph · GraphDelta"]
    MEF["Model Execution Fabric R0"]
    Observe["EnvironmentSnapshot"]
    Qualify["ProviderQualification"]
    Plan["DeploymentPolicy + DeploymentPlan"]
    Llama["llama.cpp"]
    AIR["AIR"]
    Evidence["Execution receipts / evidence"]

    Builder -->|"capability intent + structural target"| MEF
    Observe --> MEF
    Qualify --> MEF
    MEF --> Plan
    Plan -->|"explicit realization"| Llama
    Plan -->|"explicit realization"| AIR
    Llama --> Evidence
    AIR --> Evidence
    MEF -.->|"GraphDelta proposal only"| Builder
~~~

### Authority rules

- Builder owns semantic definitions, canonical SystemGraph, validation, and the structural mutation path.
- EnvironmentSnapshot is immutable observation evidence, never structural truth.
- Provider adapters translate, qualify, and execute provider-local protocols.
- ProviderQualification is evidence, not a provider registry.
- DeploymentPolicy makes provider preference explicit.
- DeploymentPlan is a deterministic proposal, not committed state.
- execute-plan follows an already explicit plan and performs no fallback selection.
- MEF never becomes a second graph, registry, scheduler, model server, or capability authority.

## Stable capability

R0 qualifies one provider-independent capability:

~~~text
text.generate@1.0.0
~~~

The capability identity does not encode runtime, endpoint, model path, CUDA backend, GGUF format, or provider-specific model names.

## Provider boundary

~~~mermaid
flowchart LR
    Core["smi-mef-core<br/>shared request / response / qualification / receipt"]
    L["smi-mef-llamacpp"]
    A["smi-mef-air"]
    LP["llama.cpp HTTP"]
    AP["AIR HTTP"]

    Core --> L --> LP
    Core --> A --> AP
~~~

Provider-local model identity remains provider-local in R0. Cross-provider artifact equivalence is not inferred from paths or display names.

## Observation and qualification

~~~text
read-only machine observation
        ↓
EnvironmentSnapshot
        ↓
provider-specific probing
        ↓
ProviderQualification
        ↓
retained evidence + digest
~~~

Qualification is bound to the exact environment snapshot used to establish it. Stale or tampered evidence is rejected.

## Deterministic planning

smi-mef-plan consumes:

~~~text
EnvironmentSnapshot
      +
ProviderQualification[]
      +
explicit DeploymentPolicy
      +
Builder-owned realization target
      ↓
DeploymentPlan
~~~

A plan records the chosen provider, endpoint, model identity, qualification identity, environment identity, and Builder realization proposal.

Planning is deterministic with respect to canonical inputs. Candidate iteration order cannot choose a provider. Missing or ambiguous policy fails structurally.

## Builder realization

MEF proposes only ordinary Builder mutations.

~~~text
unbound
  -> BindCapability

bound to A, target B
  -> UnbindCapability(A)
  -> BindCapability(B)

already bound to target
  -> no structural mutation
~~~

Every proposed mutation is bound to an exact GraphRevision. Builder remains responsible for validation and application through GraphDelta::apply.

## Planned execution

~~~mermaid
flowchart TD
    Request["TextGenerateRequest"]
    Plan["DeploymentPlan"]
    Qualification["exact selected qualification"]
    Dispatch["execute-plan"]
    L["existing llama.cpp adapter"]
    A["existing AIR adapter"]
    Receipt["TextGenerationReceipt"]

    Request --> Dispatch
    Plan --> Dispatch
    Qualification --> Dispatch
    Dispatch --> L --> Receipt
    Dispatch --> A --> Receipt
~~~

A qualification that does not exactly match the plan is rejected. No implicit fallback occurs.

## Dependency direction

~~~text
smi-ir (Builder R1)
   ↑
smi-mef-core
   ├── smi-mef-observe
   ├── smi-mef-llamacpp
   ├── smi-mef-air
   └── smi-mef-plan
            ↑
       smi-mef-cli
~~~

Builder imports no MEF crate.

## Qualified provider substitution

R0 established:

> The same canonical text.generate@1.0.0 request identity executes through independently qualified llama.cpp and AIR provider stacks while only explicit deployment realization changes.

The destructive qualification also demonstrated fail-closed behavior for missing policy, stale environment evidence, tampered qualification evidence, cross-provider qualification substitution, and unreachable configured providers.

## Frozen boundary

The immutable qualified source is tagged:

~~~text
mef-r0-qualified
commit 33f63246244f91acfba5659bba80447e6e181108
~~~

Public documentation may evolve on main. The qualified tag must not move.
