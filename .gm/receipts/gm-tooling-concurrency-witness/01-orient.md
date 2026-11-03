# 01-orient: gm-tooling-concurrency-witness (session gm-oscraft-w1-07)

acceptance (row text, not derived):
- pool-observe returns supply_exhausted when only held rows remain
- a row resolves with one CI read dispatch
- both covered by a gm CI job

codeinsight (callers/impact), oscraft root: supply_exhausted, pool-observe, ci-status have 0 call edges and no definition in /workspace/oscraft. Stale against this root.

codeinsight find, /workspace/gm root:
- supply_exhausted -> /workspace/gm/rs-plugkit/crates/plugkit-core/src/orchestrator/pool_slots/observe.rs:259-264 (fn supply_exhausted_route(held: &[String], unlaunchable: u64) -> String)

codesearch dual (oscraft): 0 literal matches for supply_exhausted / pool-observe; ci-status hits are oscraft gate/ci files (rust/gate/src/checks.rs, js/client/play.js), not the gm verb.

codesearch literal (root /workspace/gm, exhaustive over 1670 files): "pool-observe supply_exhausted" matches only /workspace/gm/.gm/prd-log.jsonl:279 (the row's own log entry). Corpus note: 11 nested worktrees, one listing unfinished, so zero is not conclusive for those.

facts relied on:
- the edit target is /workspace/gm (invariant 2), not /workspace/oscraft; the row names no file.
- the row's pool-observe and supply_exhausted symbols exist in /workspace/gm under observe.rs.

open: which file holds the pool-observe verb dispatch and the CI job definition in /workspace/gm (for gm-unknowns).
