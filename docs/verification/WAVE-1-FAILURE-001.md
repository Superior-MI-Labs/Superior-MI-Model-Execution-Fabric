# Wave 1 Qualification Failure 001: Rustfmt Drift

## Status

Corrected candidate. Wave 1 remains CURRENT until the corrected bytes pass the pinned Rust 1.98.1 verification gate on WolfCat-Studio.

## Evidence

The first Wave 1 candidate passed:

- `bootstrap-check.sh`;
- Rust-free pre-submit static checks;
- architecture checks.

The authoritative Rust gate then stopped at:

```text
cargo fmt --all --check
```

The formatter reported three source-formatting deltas in:

```text
crates/smi-mef-observe/src/lib.rs
```

No compile, Clippy, test, runtime, or architecture failure had occurred yet because formatting is intentionally earlier in the gate.

## Root cause

The candidate was generated in an environment without the pinned Rust toolchain. Static review verified architecture and conservative source hazards, but it cannot reproduce `rustfmt`'s exact layout rules.

The defect is therefore a qualification/packaging defect, not evidence of an observer semantic failure.

## Correction

Apply the exact formatter transformations reported by Rust 1.98.1. Do not suppress or reorder the gate, and do not ask WolfCat to run `cargo fmt` before qualification because that would mutate the artifact under test.

## Permanent rule

A candidate may be statically audited without Rust, but it remains a **CANDIDATE**. It is not qualified or closed until its extracted bytes pass the pinned-toolchain `./scripts/verify.sh` gate unchanged.

Rust-free preflight is an early filter only. It must never be described as a replacement for `cargo fmt --check`, compilation, Clippy, tests, or rustdoc.
