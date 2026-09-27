# Wave 3 Plan - AIR Provider

Baseline: Git commit `a4db716` (`MEF Wave 2 qualified baseline`).

Wave 3 tests whether the existing provider-independent `text.generate@1.0.0` contract can be
implemented by AIR without changing `smi-mef-core` or the qualified llama.cpp adapter.

## Gates

1. Characterize AIR public HTTP behavior on the live runtime.
2. Add `smi-mef-air` as an independent provider adapter.
3. Add thin `air-qualify` and `air-generate` CLI surfaces.
4. Preserve raw AIR health/model/runtime/completion evidence while emitting the existing
   `ProviderQualification`, `TextGenerateResponse`, and `TextGenerationReceipt` contracts.
5. Run repository verification with `smi-mef-core` and `smi-mef-llamacpp` unchanged from
   baseline commit `a4db716`.
6. Run live AIR qualification and bounded generation.
7. Compare AIR and llama.cpp normalized execution at the shared contract boundary before
   beginning Wave 4 planning.

## Falsification rule

If AIR requires a legitimate provider-independent semantic that the core contract cannot
express, reproduce that failure first and change core minimally. AIR-specific scheduler,
backend, model-format, or runtime fields do not qualify as core semantics merely because they
are observable.
