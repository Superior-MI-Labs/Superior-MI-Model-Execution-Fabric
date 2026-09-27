# R0 Waves - Completed Historical Roadmap

> **R0 is complete. This document is a historical development record, not an active plan.**

R0 used exactly six waves, numbered 0 through 5.

| Wave | Goal | Final status |
| ---: | --- | --- |
| 0 | Architecture and authority freeze | ✅ CLOSED |
| 1 | Environment snapshot and runtime inventory | ✅ CLOSED |
| 2 | llama.cpp provider | ✅ QUALIFIED |
| 3 | AIR provider and differential provider proof | ✅ QUALIFIED |
| 4 | Deployment planner and Builder integration | ✅ QUALIFIED |
| 5 | Destructive R0 qualification | ✅ QUALIFIED |

## Wave 0 - Architecture and Authority Freeze

Frozen ownership, dependency direction, initial capability identity, evidence-vs-authority distinction, and the R0 release endpoint.

## Wave 1 - Environment Snapshot and Runtime Inventory

Produced deterministic read-only machine evidence without model execution.

Exit proof: equivalent observations serialize canonically and produce stable SHA-256 identity; absent optional tools remain explicit rather than fabricated.

## Wave 2 - llama.cpp Provider

Implemented and live-qualified the first text.generate@1.0.0 provider using llama.cpp's HTTP server.

Qualification requires explicit health/model probing, deterministic or explicit model selection, bounded generation requests, and retained evidence bound to the environment snapshot.

## Wave 3 - AIR Provider

Implemented an independent AIR adapter for the same provider-independent capability contract.

Exit proof included live AIR qualification and a differential provider test showing that the same canonical request identity could execute through llama.cpp and AIR while provider identity changed.

## Wave 4 - Deployment Planner and Builder Integration

Added deterministic evidence-bound planning under explicit provider policy.

The planner emits ordinary Builder GraphDelta proposals and does not become a structural authority. Candidate order cannot determine the result, and ambiguous/no-policy cases do not guess.

## Wave 5 - Destructive R0 Qualification

Attacked the endpoint claim rather than adding normal features.

Qualification exercised stale environment evidence, tampered qualification evidence, provider mismatch, missing policy, unreachable providers, deterministic replay, and provider substitution.

## Final R0 proof

> The same high-level text.generate@1.0.0 intent executes through two independently qualified real provider stacks by changing only explicit deployment realization. Builder's structural contract remains authoritative, provider selection is evidence- and policy-bound, and missing compatibility fails structurally rather than fabricating integration code.

Final verdict:

~~~text
MEF R0 QUALIFIED PROVIDER SUBSTITUTION: PASS
~~~

Frozen identity:

~~~text
commit 33f63246244f91acfba5659bba80447e6e181108
tag    mef-r0-qualified
~~~

Any future MEF roadmap should begin as a new post-R0 plan rather than silently extending this completed wave sequence.
