# R0 Endpoint Qualification Plan

The R0 endpoint is **Qualified Provider Substitution** for `text.generate@1.0.0`.

`./scripts/qualify-r0.sh` performs the final proof:

1. repository verification and source-manifest check;
2. frozen Wave 3 provider/core surface check;
3. fresh shared environment observation;
4. fresh llama.cpp and AIR qualifications against the same environment identity;
5. deterministic explicit plans for llama.cpp and AIR;
6. replay with reversed qualification ordering;
7. Builder `GraphDelta` serialization for both realization choices;
8. the same canonical generation request executed through both plans;
9. destructive rejection of missing policy;
10. destructive rejection of stale environment evidence;
11. destructive rejection of tampered provider evidence;
12. destructive rejection of cross-provider qualification use;
13. destructive rejection of an unreachable configured provider;
14. final assertion that capability and request identity remain unchanged while provider realization changes.

The `smi-mef-plan` test suite separately constructs a real Builder `SystemGraph` and
`DefinitionRegistry`, applies the planner-produced substitution `GraphDelta` through Builder's own
authority, and verifies the resulting capability binding.

No planner test or live qualification is allowed to mutate Builder source or introduce a second graph,
registry, scheduler, inference pipeline, or state owner.
