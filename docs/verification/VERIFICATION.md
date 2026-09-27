# Verification

Canonical development gate:

```bash
./scripts/verify.sh
```

The gate checks architecture, formatting, all-target compilation, Clippy with warnings denied, tests, and rustdoc warnings.

Wave 1 is not closed until this exact candidate passes on the pinned Rust 1.98.1 toolchain.

## Artifact qualification rule

Do not run `cargo fmt` or another source-mutating repair command before qualifying a delivered candidate. `cargo fmt --all --check` must validate the candidate bytes as extracted.

An environment without the pinned Rust toolchain may run the Rust-free static and architecture checks, but those results establish only candidate-level confidence. They do not qualify an artifact.

If the authoritative gate fails, capture exact evidence:

```bash
./scripts/collect-failure-evidence.sh
```

Repair the owning defect from that evidence and repeat the full gate.

## Candidate packaging

The canonical candidate packager is:

```bash
./scripts/package-candidate.sh [output.zip]
```

It runs the full pinned-toolchain `pre-submit.sh` gate before producing an archive. If Rust 1.98.1 or any authoritative gate is unavailable, it must not claim to have produced a qualified candidate.

## Failure evidence

Immediately after any failed gate, before editing the tree:

```bash
./scripts/collect-failure-evidence.sh
```

The collector retains the exact source, toolchain metadata, `Cargo.lock`, a fresh `pre-submit.log` plus exit code, and any already-produced `local/wave2` provider evidence.
