# Superior MI Model Execution Fabric

<p align="center">
  <strong>Evidence-bound execution for provider-independent model capabilities.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/R0-QUALIFIED-2ea44f?style=for-the-badge" alt="R0 Qualified">
  <img src="https://img.shields.io/badge/Rust-1.98.1-orange?style=for-the-badge" alt="Rust 1.98.1">
  <img src="https://img.shields.io/badge/Builder-R1-4c6ef5?style=for-the-badge" alt="Builder R1">
</p>

<p align="center">
  <code>text.generate@1.0.0</code> · <code>llama.cpp</code> · <code>AIR</code>
</p>

---

## What MEF is

**Superior MI Model Execution Fabric (MEF)** connects stable capability intent to concrete local execution stacks while preserving **Superior MI Builder** as structural authority.

R0 proves **Qualified Provider Substitution**: the same canonical text.generate@1.0.0 request can execute through independently qualified llama.cpp and AIR providers while only the explicit physical realization changes.

MEF is deliberately **not** another System IR, model registry, scheduler, inference engine, or hidden fallback layer.

## R0 result

> **MEF R0 QUALIFIED PROVIDER SUBSTITUTION: PASS**

| Property | Frozen R0 result |
| --- | --- |
| Status | **CLOSED · QUALIFIED · FROZEN** |
| Capability | text.generate@1.0.0 |
| Provider 1 | provider.llamacpp.http |
| Provider 2 | provider.air.http |
| Qualified commit | 33f63246244f91acfba5659bba80447e6e181108 |
| Tag | mef-r0-qualified |
| Evidence SHA-256 | f524669a5a82036efcd954c41aa2de3ec2a2ec2e5f6e6f532f307e38ea4cd700 |
| Qualified source SHA-256 | caccc50d45d0019e448a9b80cfc573aba6f7ad62e345943ae029e7ac10f8d4cf |

## Architecture

~~~mermaid
flowchart TD
    Intent["Capability intent<br/>text.generate@1.0.0"]
    Env["EnvironmentSnapshot<br/>read-only evidence"]
    Qual["ProviderQualification<br/>measured compatibility"]
    Policy["Explicit DeploymentPolicy"]
    Plan["Deterministic DeploymentPlan"]
    Delta["Builder GraphDelta proposal"]
    Llama["provider.llamacpp.http"]
    Air["provider.air.http"]
    Evidence["Execution evidence"]

    Intent --> Env
    Env --> Qual
    Qual --> Policy
    Policy --> Plan
    Plan --> Delta
    Plan --> Llama
    Plan --> Air
    Llama --> Evidence
    Air --> Evidence
~~~

### Authority boundaries

~~~mermaid
flowchart LR
    Builder["Superior MI Builder R1<br/>definitions · SystemGraph · GraphDelta"]
    MEF["MEF R0<br/>observe · qualify · plan · evidence"]
    Llama["llama.cpp"]
    AIR["AIR"]
    IF["Inference Fabric<br/>higher-level reuse / continuity"]

    Builder -->|"structural authority"| MEF
    MEF -->|"explicit qualified execution"| Llama
    MEF -->|"explicit qualified execution"| AIR
    IF -.->|"separate higher-level layer"| MEF
~~~

Builder owns structural truth. MEF observes and qualifies physical realizations, produces deterministic plans under explicit policy, and proposes ordinary Builder mutations. Provider runtimes execute. None of those roles silently absorb the others.

## Completed R0 program

| Wave | Proof | Status |
| ---: | --- | --- |
| 0 | Architecture and authority freeze | ✅ Complete |
| 1 | Deterministic environment observation | ✅ Complete |
| 2 | Live llama.cpp qualification | ✅ Qualified |
| 3 | Live AIR qualification + differential provider proof | ✅ Qualified |
| 4 | Deterministic evidence-bound deployment planning | ✅ Qualified |
| 5 | Destructive provider-substitution qualification | ✅ Qualified |

The original wave plan is preserved in [docs/roadmap/R0_WAVES.md](docs/roadmap/R0_WAVES.md) as a completed historical roadmap.

## What the destructive qualification rejects

R0 deliberately fails closed on:

- missing provider policy;
- stale environment evidence;
- tampered qualification evidence;
- cross-provider qualification substitution;
- unreachable configured providers;
- provider/plan mismatches;
- ambiguous realization where policy does not resolve it.

Deterministic replay also requires byte-identical planning output when qualification input ordering changes.

## Repository map

~~~text
crates/
├── smi-mef-core        provider-independent contracts and receipts
├── smi-mef-observe     deterministic environment observation
├── smi-mef-llamacpp    llama.cpp adapter
├── smi-mef-air         AIR adapter
├── smi-mef-plan        evidence-bound deterministic planner
└── smi-mef-cli         qualification / execution surface

docs/
├── contracts/          versioned MEF contracts
├── decisions/          architecture decisions
├── roadmap/            completed R0 development sequence
├── status/             authoritative current state
└── verification/       qualification lineage and plans
~~~

## Verification

Repository verification:

~~~bash
./scripts/verify.sh
~~~

Full live R0 qualification requires compatible local llama.cpp and AIR endpoints:

~~~bash
./scripts/qualify-r0.sh
~~~

Evidence packaging:

~~~bash
./scripts/collect-r0-evidence.sh
~~~

The immutable scientific freeze is the **mef-r0-qualified** tag. Public documentation may continue to improve on main without moving that tag.

## Design rules

- one authority per domain;
- evidence is not structural truth;
- explicit provider policy, never arbitrary iteration order;
- no silent fallback;
- deterministic plans;
- ordinary Builder GraphDelta for structural proposals;
- provider-specific details stay behind provider adapters;
- failures remain structured and observable;
- qualification claims are falsifiable and evidence-backed.

## Relationship to Superior MI

MEF is one layer in a broader modular architecture.

- **Superior MI Builder** describes and verifies system structure.
- **MEF** qualifies and binds abstract capabilities to physical execution.
- **AIR** and other runtimes execute those bound capabilities.
- **Inference Fabric** remains a separate higher-level continuity/reuse layer.

The long-term direction is modular above compilation and specialized below it: system intent remains explicit while execution mechanisms can be independently qualified and substituted.

## Project status

**R0 is complete and frozen.**

Future development should consume this boundary rather than casually reopen it. If new evidence falsifies an R0 invariant, repair the owning layer and re-qualify deliberately.

## License

Apache License 2.0. See [LICENSE](LICENSE).

---

<p align="center">
  <strong>Superior MI Technologies</strong><br/>
  Building modular, inspectable, evidence-driven intelligent systems.
</p>
