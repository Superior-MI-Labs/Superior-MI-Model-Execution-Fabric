# ADR-0004: Explicit Evidence-Bound Planning

## Decision

MEF R0 provider selection requires an explicit provider policy. The planner does not rank or guess
between qualified providers.

A deployment plan is valid only when its qualification evidence matches the requested capability and
the exact current `EnvironmentSnapshot` identity. Multiple distinct matching qualifications are an
ambiguity error.

The planner emits a Builder `GraphDelta` proposal but never applies it. Planned execution follows the
selected provider only after the plan and exact qualification identity agree.

## Why

The R0 research question is provider substitution without changing semantic intent. Automatic ranking
would introduce a second research problem and could hide arbitrary iteration-order behavior.

Requiring explicit policy makes selection falsifiable, deterministic, and auditable while preserving
Builder as structural authority.

## Consequences

- provider-list order cannot decide execution;
- stale evidence cannot silently authorize a provider;
- ambiguous evidence fails structurally;
- MEF can propose a realization without owning `SystemGraph`;
- future policy engines can evolve without changing the capability contract or provider adapters.
