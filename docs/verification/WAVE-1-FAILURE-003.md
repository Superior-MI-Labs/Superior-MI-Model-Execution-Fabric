# Wave 1 Failure 003 - Candidate Rustfmt Drift After Ownership Repair

## Evidence

WolfCat-Studio ran the authoritative pinned Rust 1.98.1 gate on the second corrected Wave 1 candidate.

The candidate passed:

- bootstrap/toolchain validation;
- Rust-free static preflight;
- architecture validation.

`cargo fmt --all --check` then rejected two layout differences:

1. the `text_generate_v1()` assertion in `smi-mef-core` required multiline formatting;
2. the `EnvironmentSnapshot::new(...)` return expression in `smi-mef-observe` required multiline formatting.

No compile, Clippy, test, or runtime stage was reached in this qualification attempt.

## Root cause

The preceding ownership repair was authored and packaged in an environment without the pinned Rust toolchain. Static checks cannot reproduce rustfmt's exact layout decisions. The semantic repair was correct, but the delivered bytes had not been validated by authoritative `cargo fmt --all --check`.

## Repair

The two formatter transformations reported by WolfCat were applied exactly. No runtime, API, or architecture semantics changed.

A new `scripts/package-candidate.sh` entrypoint now makes artifact production explicitly dependent on `./scripts/pre-submit.sh`. A normal release candidate should therefore be packaged only on a machine with the pinned Rust toolchain after the entire authoritative gate passes.

## Rule

Do not infer rustfmt cleanliness from source appearance or Rust-free checks. A candidate packaged outside a pinned-Rust environment remains only a manually prepared candidate until WolfCat verifies it unchanged.
