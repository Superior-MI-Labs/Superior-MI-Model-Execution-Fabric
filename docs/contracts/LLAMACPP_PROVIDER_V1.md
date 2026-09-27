# llama.cpp HTTP Provider Contract - Wave 2

Provider identity:

```text
provider.llamacpp.http
```

Capability claimed after qualification:

```text
text.generate@1.0.0
```

## Qualification sequence

The adapter must not claim compatibility merely because an endpoint was configured.
It must first load a valid Wave 1 `EnvironmentSnapshot`, verify that the endpoint was
configured as a `provider.llamacpp.http` endpoint candidate, and bind the resulting
qualification to that snapshot SHA-256. It must then successfully observe, in order:

1. `GET /health` with HTTP 2xx and JSON `status == "ok"`;
2. `GET /v1/models` with at least one non-empty model `id`;
3. an unambiguous model selection;
4. a bounded `POST /v1/completions` probe whose response satisfies the normalized response shape.

If exactly one model is advertised, omission of an explicit model is deterministic.
If multiple models are advertised, the caller must select one explicitly. Iteration
order is never a provider-selection policy.

The retained health, model-list, completion-probe JSON, and environment snapshot identity are evidence. The generic
`ProviderQualification` stores only stable MEF/Builder identities and a SHA-256
of canonical retained probe evidence.

## Execution sequence

Generation requires a previously serialized and revalidated qualification document.
The adapter uses `POST /v1/completions` with:

- the exact qualified model id;
- provider-independent prompt text;
- the bounded R0 `max_output_tokens` value;
- `temperature = 0`;
- `stream = false`;
- a 30-second default request timeout;
- a 4 MiB maximum HTTP response body.

The normalized response retains:

- generated text;
- finish reason when reported;
- completion token count when reported.

The provider-specific response JSON remains in the execution evidence document and is
represented in the generic execution receipt by SHA-256. It does not enter Builder
System IR.

## Endpoint boundary

Wave 2 accepts plain HTTP only on syntactic loopback hosts:

- `127.0.0.1`
- `localhost`
- `::1`

No credentials, query string, fragment, or base-path component are accepted. Remote
provider/auth/TLS policy is deliberately deferred.

## Failure semantics

The adapter fails structurally on:

- unreachable or timed-out endpoint;
- non-success HTTP status;
- malformed health/models/completion JSON;
- health status other than `ok`;
- zero advertised models;
- ambiguous model set without explicit selection;
- requested model not advertised;
- qualification/provider/capability mismatch;
- retained qualification evidence tampering.

No failure path generates glue code or silently switches models/providers.
