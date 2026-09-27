# Model Execution Fabric R0 Architecture

## Purpose

MEF connects stable Builder capability intent to concrete local execution stacks while preserving Builder as structural authority.

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
  Provider Adapter     runtime-specific translation/execution
  Qualification Record measured compatibility evidence
  Deployment Planner   derived selection proposal
        │
        ▼
External runtimes
  llama.cpp / AIR / later stacks
```

MEF must never become a second graph or registry.

## Wave 1 data flow

```text
read-only host observation
  ├── OS / architecture
  ├── logical CPU count
  ├── Linux memory total when available
  ├── NVIDIA metadata via read-only nvidia-smi query when available
  ├── executable presence in PATH
  ├── configured endpoint candidates
  └── configured model-root existence
              │
              ▼
       EnvironmentSnapshot
              │
              ├── canonical sorted representation
              └── SHA-256 content identity
```

No network request and no model invocation occurs in Wave 1.

## Stable capability identity

MEF reuses Builder's `CapabilityRef`. The initial R0 capability is:

```text
text.generate@1.0.0
```

MEF does not define a separate capability namespace.

## Provider boundary

A provider is a concrete implementation of a capability for a runtime/model/environment combination. The model is not the provider. llama.cpp is not the model. AIR is not System IR.

Future provider identity therefore includes at least:

```text
adapter implementation
runtime endpoint/process
model artifact identity
observed environment
```

Exact qualification semantics arrive in Waves 2–5.

## Dependency direction

```text
smi-ir (Builder R1)
   ↑
smi-mef-core
   ↑
smi-mef-observe
   ↑
smi-mef-cli
```

No MEF crate is imported by Builder R1.


## Wave 2 provider flow

```text
EnvironmentSnapshot SHA-256
        │
configured llama.cpp endpoint
        │
        ├── GET /health
        └── GET /v1/models
                 │
                 ▼
       ProviderQualification
                 │
                 ▼
       POST /v1/completions
                 │
                 ▼
       TextGenerationReceipt
```

`ProviderQualification` is evidence, not a provider registry. The `llama.cpp` adapter
may translate requests and retain provider-specific JSON, but it cannot create or mutate
Builder `SystemGraph`, `DefinitionRegistry`, or `GraphDelta`.

The qualification is bound to a specific `EnvironmentSnapshot` identity. A later planner
may use that evidence, but qualification itself does not choose among providers.

## Wave 3 second-provider flow

```text
EnvironmentSnapshot SHA-256
        │
configured AIR endpoint
        │
        ├── GET /health
        ├── GET /v1/models
        ├── GET /model
        ├── GET /runtime
        └── POST /v1/completions probe
                 │
                 ▼
       ProviderQualification
                 │
                 ▼
       POST /v1/completions
                 │
                 ▼
       TextGenerationReceipt
```

AIR-specific model/runtime evidence remains inside `smi-mef-air`. The shared core contract
continues to describe capability qualification and execution evidence without importing AIR
scheduler, backend, or model-format semantics.

## R0 dependency direction after Wave 3

```text
smi-ir (Builder R1)
   ↑
smi-mef-core
   ├──→ smi-mef-observe
   ├──→ smi-mef-llamacpp
   └──→ smi-mef-air
                ↑
          smi-mef-cli
```
