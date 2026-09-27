# R0 Waves

R0 has exactly six planned waves, numbered 0 through 5.

## Wave 0 - Architecture and Authority Freeze

Freeze ownership, dependency direction, initial capability identity, evidence-vs-authority distinction, and release endpoint.

## Wave 1 - Environment Snapshot and Runtime Inventory

Produce deterministic read-only machine evidence. No model execution.

Exit proof: the same observations in different insertion order serialize identically and produce the same SHA-256 identity; absent optional tools remain explicit rather than fabricated.

## Wave 2 - llama.cpp Provider

Implement the first live `text.generate@1.0.0` provider using llama.cpp's HTTP server. Require explicit health/model probing and bounded requests. Preserve provider-specific responses as evidence without leaking their schema into Builder IR.

Exit proof: a configured local llama.cpp server can be discovered, qualified for the bounded R0 contract, invoked, cancelled/timed out, and rejected cleanly when unhealthy or incompatible.

## Wave 3 - AIR Provider

Implement the same stable capability through AIR's public HTTP surface. No AIR-specific semantic fork of `text.generate`.

Exit proof: both providers satisfy the same MEF request/response contract while retaining independent runtime receipts.

## Wave 4 - Deployment Planner and Builder Integration

Use environment + qualification evidence to produce a deterministic provider choice under explicit policy, then express realization through ordinary Builder definitions/bindings/`GraphDelta`.

Exit proof: provider order does not affect a deterministic policy result; ambiguous/no-policy cases do not guess; Builder remains structural authority.

## Wave 5 - Destructive R0 Qualification

Stop normal feature development. Attack snapshots, provider failures, malformed responses, cancellation, stale evidence, artifact mismatch, ambiguous planning, replay, and clean-room agent continuation.

Final R0 proof:

> The same high-level `text.generate@1.0.0` intent is executed through two real provider stacks by changing only explicit deployment realization. Builder's structural contract does not change, provider selection is evidence/policy driven, and missing compatibility fails structurally rather than fabricating integration code.
