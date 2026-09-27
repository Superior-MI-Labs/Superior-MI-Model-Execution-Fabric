# Wave 2 Qualification Plan

Wave 2 is the first live model-execution wave. It is not complete merely because HTTP code compiles.

## Static/repository gate

`./scripts/verify.sh` must pass unchanged on Rust 1.98.1, including rustfmt, all-target check, Clippy with warnings denied, tests, and rustdoc.

## Live gate

`./scripts/qualify-wave2.sh` must:

- regenerate a Wave 1 environment snapshot;
- bind provider qualification to that exact snapshot identity;
- qualify `/health` and `/v1/models` on the configured local endpoint;
- execute a bounded completion probe before the provider may claim `text.generate@1.0.0`;
- refuse implicit choice if multiple models are advertised;
- serialize qualification evidence;
- reload and revalidate the qualification before generation;
- execute one bounded `/v1/completions` request;
- normalize a non-empty text result;
- preserve provider-specific response JSON and generic execution receipt;
- hash the resulting evidence files.

## Negative behavior required by source tests/contracts

- remote/non-loopback endpoint rejected;
- HTTPS rejected in R0 Wave 2;
- path/query/fragment credentials rejected;
- malformed health/models/completion responses rejected;
- empty model list rejected;
- multi-model ambiguity rejected without explicit selection;
- selected model must be advertised;
- generated execution cannot bypass qualification;
- provider/capability/evidence tampering rejected on qualification reload;
- request output bound enforced by the shared `text.generate@1.0.0` contract.
