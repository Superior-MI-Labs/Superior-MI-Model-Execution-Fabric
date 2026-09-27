# Wave 1 Qualification Failure 002: Hidden Static-Identifier Panic

## Status

Corrected candidate. Wave 1 remains CURRENT until the corrected bytes pass the pinned Rust 1.98.1 verification gate on WolfCat-Studio.

## Evidence

The second Wave 1 qualification attempt passed:

- bootstrap/toolchain check;
- Rust-free static preflight;
- architecture check;
- `cargo fmt --all --check`;
- `cargo check --workspace --all-targets`.

The authoritative gate then stopped in Clippy with `-D warnings`:

```text
clippy::missing-panics-doc
crates/smi-mef-core/src/capability.rs
text_generate_v1()
```

The public helper called `CapabilityId::new("text.generate").expect(...)`. Builder R1 intentionally exposes semantic-name construction as fallible, so the helper could not truthfully claim to be infallible without hiding that contract.

## Root cause

MEF treated a compile-time-known canonical string as though it bypassed Builder's runtime validation contract. That introduced a latent panic and made the public API weaker than the dependency it wraps.

A second instance of the same pattern existed in the private runtime-adapter observer for `provider.llamacpp.http` and `provider.air.http`. It had not yet triggered a Clippy diagnostic because the function was private, but it violated the same design principle.

## Correction

- `text_generate_v1()` now returns `Result<CapabilityRef, NameError>`.
- `observe_environment()` now returns `Result<EnvironmentSnapshot, NameError>`.
- built-in runtime adapter construction propagates `QualifiedName::new(...)` failures instead of calling `expect`.
- the CLI maps that failure into an explicit user-facing error.
- static preflight now rejects non-test `.expect(...)` and `.unwrap(...)` in MEF Rust source.

No capability identity, snapshot schema, provider semantics, or Builder authority changed.

## Permanent rule

MEF production code must not hide fallible Builder identity construction behind `expect`/`unwrap`. Static literals are still subject to Builder's canonical validation contract. If validation is fallible at the dependency boundary, MEF propagates or models that failure explicitly.
