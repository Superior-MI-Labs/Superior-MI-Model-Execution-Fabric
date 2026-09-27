# ADR-0003: llama.cpp is the first real provider, not a new authority

Status: accepted for R0 Wave 2.

## Decision

Implement `provider.llamacpp.http` as a provider adapter for Builder capability
`text.generate@1.0.0`.

The adapter must qualify the configured endpoint before execution and must require a
saved qualification document for generation. The adapter owns translation between the
stable MEF contract and llama.cpp HTTP. It owns neither `SystemGraph` nor provider
selection policy.

The first execution route is `/v1/completions`, not chat completions. This keeps the
initial capability prompt-based and avoids elevating backend-specific chat-template
semantics into the stable R0 contract.

## Consequences

- Wave 2 can prove real model execution without changing Builder R1.
- Provider-specific HTTP JSON is retained as evidence outside Builder IR.
- Multiple advertised models require explicit model selection.
- Wave 3 can implement AIR against the same `TextGenerateRequest` / response contract.
