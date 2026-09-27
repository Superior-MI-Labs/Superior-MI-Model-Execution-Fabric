# AIR HTTP Provider Contract - Wave 3

Provider identity:

```text
provider.air.http
```

Capability claimed after qualification:

```text
text.generate@1.0.0
```

## Qualification sequence

The adapter must not claim compatibility merely because an AIR endpoint was configured.
It must load a valid `EnvironmentSnapshot`, verify that the endpoint was configured as a
`provider.air.http` endpoint candidate, and bind qualification to that snapshot SHA-256.
It must then successfully observe, in order:

1. `GET /health` with HTTP 2xx and JSON `status == "ok"`;
2. `GET /v1/models` with at least one non-empty model `id`;
3. deterministic model selection, or explicit selection when more than one model is advertised;
4. `GET /model` whose non-empty `id` exactly matches the selected advertised model;
5. `GET /runtime` with a non-empty `backend` string;
6. a bounded `POST /v1/completions` probe whose response satisfies the shared normalized response shape.

The selected model identifier is provider-local identity. Wave 3 does not claim that an AIR
model id and a llama.cpp GGUF path are a universal artifact identity. Cross-provider artifact
identity is a separate concern and must not be inferred from string equality.

The retained health, model-list, model, runtime, completion-probe JSON, and environment
snapshot identity are evidence. AIR-specific runtime fields remain provider evidence and do
not enter `smi-mef-core` or Builder System IR.

## Execution sequence

Generation requires a previously serialized and revalidated AIR qualification document.
The adapter uses `POST /v1/completions` with:

- the exact qualified provider-local model id;
- provider-independent prompt text;
- the bounded R0 `max_output_tokens` value mapped to `max_tokens`;
- `temperature = 0`;
- `stream = false`;
- a 30-second default request timeout;
- a 4 MiB maximum HTTP response body.

The normalized response retains generated text, finish reason when reported, and completion
token count when reported. The raw AIR response remains provider-specific execution evidence
and is represented in the generic execution receipt by SHA-256.

## Endpoint boundary

Wave 3 accepts plain HTTP only on syntactic loopback hosts:

- `127.0.0.1`
- `localhost`
- `::1`

No credentials, query string, fragment, or base-path component are accepted.

## Authority boundary

The AIR adapter may observe AIR runtime state but does not own or mutate AIR scheduler state,
Builder `SystemGraph`, `DefinitionRegistry`, or `GraphDelta`. It does not choose between AIR
and other providers. Provider selection remains a later planning concern.

## Failure semantics

The adapter fails structurally on unreachable/timed-out endpoints, non-success HTTP status,
malformed evidence, unhealthy AIR state, zero advertised models, ambiguous selection,
requested model absence, disagreement between `/v1/models` and `/model`, missing runtime
backend identity, qualification/provider/capability mismatch, or retained-evidence tampering.

No failure path silently switches providers, models, or AIR backends.
