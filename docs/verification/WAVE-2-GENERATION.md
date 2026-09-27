# Wave 2 Candidate Generation

Wave 2 was derived from the Wave 1 tree that the user reported passed both
`PRE-SUBMIT: PASS` and `VERIFY: PASS` unchanged on WolfCat-Studio with Rust 1.98.1.

## New scope

- one `provider.llamacpp.http` adapter;
- environment-snapshot-bound provider qualification;
- live `/health`, `/v1/models`, and bounded `/v1/completions` capability probe;
- saved/revalidated qualification before normal generation;
- provider-independent qualification and execution receipts;
- explicit prompt/output/timeout/response-body bounds.

## Producer-side evidence

The candidate-generation environment has no local Rust toolchain. Static preflight,
architecture checks, shell syntax, Python syntax, TOML parsing, source-manifest checks,
and package-mode checks were run here.

An attempt was made to run the exact Rust 1.98.1 gate through an isolated Hugging Face
Job using the official Rust container. The Jobs API rejected execution with HTTP 402
(Payment Required). That is an infrastructure/billing limitation, not Rust evidence.

Therefore this artifact remains **CANDIDATE** until WolfCat-Studio runs the canonical
pinned-toolchain gate and live provider qualification unchanged:

```bash
./scripts/verify.sh
./scripts/qualify-wave2.sh
```

Do not treat producer-side static checks as substitutes for rustfmt, Clippy, tests,
rustdoc, or the live llama.cpp experiment.

## Qualification correction 001

WolfCat-Studio rejected the initial Wave 2 candidate at `cargo fmt --all --check`. The exact Rust 1.98.1 formatter hunks were replayed without semantic changes. The corrected candidate also carries the WolfCat-generated `Cargo.lock` and uses `--locked` for canonical Cargo gates and live Wave 2 execution. See `WAVE-2-FAILURE-001.md`.
