# Wave 2 Qualification Failure 001 - rustfmt Drift

## Failure

The first Wave 2 candidate passed bootstrap, static preflight, and architecture checks on WolfCat-Studio, then `cargo fmt --all --check` failed before compile/Clippy/tests.

The formatter reported deterministic layout changes in:

- `crates/smi-mef-cli/src/main.rs`;
- `crates/smi-mef-core/src/provider.rs`;
- `crates/smi-mef-llamacpp/src/lib.rs`.

No live llama.cpp qualification was reached. This is a candidate-byte formatting defect, not provider/runtime evidence.

## Root cause

The Wave 2 candidate was assembled in an environment without Rust/rustfmt. Static checks cannot establish exact rustfmt compliance. The candidate therefore reached WolfCat with semantically intended Rust that did not match Rust 1.98.1 formatter output.

## Correction

The exact formatter hunks emitted by WolfCat were replayed into the candidate. No semantic/provider architecture change was made by those hunks.

The correction pass also hardens qualification reproducibility:

- the WolfCat-generated `Cargo.lock` is now part of the candidate;
- canonical Cargo gates use `--locked`;
- live Wave 2 `cargo run` commands use `--locked`;
- failure collection records a fresh `pre-submit.log` and exit code;
- existing `local/wave2` live evidence is copied into failure evidence without rerunning the provider.

## Regression requirement

The corrected candidate must pass, unchanged:

```bash
./scripts/verify.sh
./scripts/qualify-wave2.sh
```

If either fails, collect evidence before modifying the tree:

```bash
./scripts/collect-failure-evidence.sh
```
