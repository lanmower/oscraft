# 03-eliminate: gm-tooling-concurrency-witness (session gm-oscraft-w1-07)

U1 gm-tooling-u1-observe-path (pool-observe reaches supply_exhausted when only held rows remain)
- expected (from row contract): state supply_exhausted when no open row is launchable and only held rows remain.
- code path (located, not the expected value): observe.rs:217-226, handle_observe calls supply_exhausted_route at line 221 and sets state "supply_exhausted" at line 222.
- live run, dispatch 1792264389182-500-19d0fe9b42179d73, body {live:1, held:[row]}: refused floor_gate_denied (live 1 < floor 10, open_rows 17, launchable 17, shortfall 9). Observed: the floor gate, not supply_exhausted. Consistent with the contract: launchable rows exist, so exhaustion is not expected.
- status: PARTIAL. The supply_exhausted branch itself is not reached live, because the supply state of 17 launchable rows cannot be changed from this session without launching the whole pool. Open question carried: a live exhaustion probe needs launchable=0.

U2 gm-tooling-u2-ci-read-verb (single CI read dispatch)
- live/code check: codesearch literal "ci-log" over /workspace/gm found skills/gm-publish/SKILL.md:13 (step 4 composes one gm_codemode dispatch: git_show, ci-status, ci-log, log_sha256) and skills/gm/docs/verbs.md:373-382 (ci-status and ci-log verbs exist).
- observed: the single CI read exists as a composed body in the skill, not as a verb.
- status: KNOWN. The verb pair exists; no single-verb wrapper exists.

U3 gm-tooling-u3-ci-job (gm CI job covering supply_exhausted and the single CI read)
- check: codesearch literal "supply_exhausted" over /workspace/gm (exhaustive, 1670 files) returns no .github/ file. The .github/workflows glob returned 8 files with no match for "name: ci".
- observed: no CI job covers supply_exhausted.
- status: KNOWN, false. The edit must add the CI job in /workspace/gm, which is an edit, not a blocker.

open unknowns after this stage: U1 partial (branch not reached live). Per the rule that a probe answering an unknown is not handed to a blocker unprobed, U1 stays open with its probe recorded above; the exhaustion probe is the next step.
