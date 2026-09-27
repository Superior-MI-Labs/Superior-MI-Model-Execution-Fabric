# text.generate@1.0.0 R0 Contract

The canonical capability identity is Builder's:

```text
text.generate@1.0.0
```

The first R0 execution contract is intentionally narrow:

Request:

- non-empty UTF-8 prompt text no larger than 262144 bytes;
- `max_output_tokens` in the inclusive R0 range `1..=4096`.

Response:

- generated UTF-8 text;
- provider-independent finish category when known;
- optional provider-reported output token count;
- provider/runtime receipt retained outside Builder structural truth.

Sampling is intentionally constrained during the first cross-provider proof. Provider-specific knobs do not enter the universal Builder capability vocabulary merely because one backend supports them.

## Construction contract

The Rust accessor for this capability returns `Result<CapabilityRef, NameError>` because Builder R1 is the authority for canonical semantic-name validation. MEF does not bypass or mask that validation with a panic.
