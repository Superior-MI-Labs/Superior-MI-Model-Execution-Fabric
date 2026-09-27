# Deployment Plan V1

`DeploymentPlan` is MEF R0's deterministic evidence-bound realization proposal.

It does not mutate Builder state. It selects exactly one qualified provider under an explicit policy,
binds the selection to the current `EnvironmentSnapshot`, and carries an ordinary Builder `GraphDelta`
proposal for the structural owner to accept or reject.

## Inputs

- one current `EnvironmentSnapshot`;
- one stable capability identity;
- independently validated `ProviderQualification` evidence;
- one explicit `DeploymentPolicy` naming the desired provider adapter;
- one Builder-owned `RealizationTarget` containing the exact graph revision, consumer, requirement slot,
  selected provider instance, and optional current provider instance.

## Selection rules

1. No policy means no plan.
2. Only qualifications for the explicit provider are considered.
3. The qualification capability must exactly equal the requested capability.
4. The qualification must be bound to the current environment snapshot identity.
5. Exact duplicate qualification evidence is harmless and deduplicated by qualification identity.
6. Multiple distinct current qualifications for the selected provider are ambiguous and rejected.
7. Input iteration order cannot affect the result.

R0 intentionally does not implement implicit ranking, fastest-provider selection, or arbitrary fallback.
Those are later policy questions, not planner defaults.

## Builder realization

The plan contains either:

- no `GraphDelta` when the requested provider instance is already bound; or
- one `BindCapability` operation when no provider is currently bound; or
- `UnbindCapability` followed by `BindCapability` when substituting providers.

MEF never applies the mutation itself. The delta remains revision-bound and is applied, validated, and
committed only by Builder's existing `GraphDelta` authority.

## Execution

`execute-plan` does not select a provider. It follows an already explicit plan, requires the exact
qualification identity named by that plan, revalidates the provider-specific qualification envelope,
and then delegates to the existing provider adapter.

A plan cannot execute with another provider's qualification evidence.

## R0 qualification claim

The final proof requires the same canonical `TextGenerateRequest` to produce the same request SHA-256
through both `provider.llamacpp.http` and `provider.air.http`, while the deployment plan and Builder
binding realization explicitly change provider identity.
