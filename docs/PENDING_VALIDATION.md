# Pending validation

Phase A changes (owner rule, 2026-11-08, recorded in AGENTS.md) are written and committed locally without builds, tests, gates, witnesses or browser runs. Each entry names the file, the item and the validation Phase B must run before the push. Phase B clears an entry by recording its result in DESIGN.md and deleting the entry here.

## A1. Strike-on-move flag, one-hand path tests

- Files: rust/crates/mod-host/tests/one_hand_strike_on_move.rs (new, three tests).
- Item: the lock's `one_hand` ui call sets `strike_on_move` for the owner's hero only, and the default is off. The client side is js/client/play.js `syncOneHand` (lock engaged sends `one_hand` on); the Lua side is scenarios/moba/controls.lua `sync_strike_on_move`.
- Validation: `cargo test -p mod-host --test one_hand_strike_on_move` (all three pass).
- Status: written, not run.

## A2. Sim strike-on-move (already committed earlier)

- Files: rust/crates/sim/src/combat.rs (attack gate on walking and strike_on_move), arrays.rs, persist.rs (STATE_VERSION 68), hash.rs, rust/crates/mod-host/src/piccolo_host.rs (native set_strike_on_move), rust/crates/sim/tests/strikes.rs.
- Item: a walking unit strikes its target in reach only when strike_on_move is set; a plain move order does not strike.
- Validation: `cargo test -p sim --test strikes` (both move-order tests), then `cargo test -p sim`.
- Status: the two strikes tests passed before the Phase rule; re-run in Phase B.

## A3. Strike-target hook (uncommitted before this phase)

- Files: rust/crates/web/src/lib.rs (`Game::unit_strike_of`, bounds-checked wrapper over `commanded_strike_index`), js/client/play.js (`window.__ciStrikeOf`).
- Item: the witness can read a unit's committed strike target by unit id. `unit_strike_target` is alive-filtered and indexes by position, so it cannot answer for a given unit id.
- Validation: the wasm must be rebuilt (`WASM=1 SKIP_PACKS=1 bash scripts/dev/stage.sh`, then `node scripts/dev/serve.mjs`); the witness in A4 must return a non-null target for the hero (`targetsSeen` non-empty in the green legs).
- Status: written, not built.

## A4. Live witness: strike while walking

- Files: scripts/witness/strike-while-walking.mjs (new). It exports `LEGS`, `CONFIG`, `strikeWhileWalkingLeg` (page-side), `verdict` (pure) and `runWitness(page)`.
- Item: three legs in one match (one-hand on, off, on). The red run is the one-hand-off leg, which matches attack() before the fix. A leg counts only when the hero is in reach while moving, so the red run proves the flag and not the geometry.
- Owner filter: `__ciHostile(me, owner)` takes player ids (rust/crates/web/src/lib.rs `hostile(a, b)` is player-indexed). The earlier witness passed unit ids, so its filter marked allied units hostile. The module uses player ids.
- Validation: one bounded playwriter run (docs/BROWSER_WITNESSES.md), staged from HEAD with SKIP_PACKS=1. Import the module from the gm playwriter code and call `runWitness(state.page)`; if dynamic import is unavailable there, paste the module's functions into the code. Record the three legs and the verdict in DESIGN.md. `CONFIG.stage` (78.5, 82) and `CONFIG.minStartHp` (200) are map and hero values to check against the live match.
- Per-leg reset: each leg waits for the hero, walks it to `CONFIG.stage` and waits for `minStartHp` (up to `resetMs`), then primes the effect buffer so effects from earlier legs are not counted. A leg that cannot start is recorded as `ready: false`.
- Counting: a strike sample is a sample where the hero moved (or stood still) and either a hero-sourced effect appeared or the committed strike target lost hp since the last sample. The hp rule is needed because melee hits came with source -1 in the logs. Allies can also damage the target, so the in-reach count bounds it.
- The attempted run that preceded this module is recorded as B2 below. It is not valid and must be rerun through the module.
- Status: written, not run.

## A7. Verdict check for the witness

- Files: scripts/witness/strike-while-walking-check.mjs (new). It runs the pure `verdict` on synthetic legs: the green, red, green pass; a red leg with strikes fails; a green leg with none fails; a red leg never in reach fails; a leg not ready fails; an aborted leg fails.
- Not in the gate (check-all keeps 73 entries), so Phase B runs it by hand.
- Validation: `node scripts/witness/strike-while-walking-check.mjs` prints `strike-while-walking verdict checks ok (6 cases)`.
- Status: written, not run.

## A5. Browser witness spec, four witnesses

- Files: docs/BROWSER_WITNESSES.md (four witnesses; strike-while-walking added with its red-run rule), scripts/check-browser-witnesses.mjs (checks the fourth witness name).
- Validation: `node scripts/check-browser-witnesses.mjs` prints `browser witness spec ok (4 witnesses, 4 bounded-run rules)`.
- Status: written, not run.

## A6. Two-phase rule

- Files: AGENTS.md (Invariants bullet, owner rule 2026-11-08).
- Validation: none beyond the gate in B1.
- Status: written.

## B1. Gate and gm run

- Command: `node scripts/check-all.mjs`, run through the gm run, baseline `RESULT gates=73 ok=73 failed=0 drift=0 skipped=6 strict=false`.
- Covers: A1 through A5 (check-browser-witnesses is a gate entry; the Rust tests run through cargo, not the gate).
- Status: not run.

## B2. Attempted three-leg run before the module (Phase B item, not valid)

- Ran in Phase A by mistake, against the rule. Recorded here as a Phase B item; it is not evidence.
- Command: a gm playwriter run (MCP task kufn44cj7) on the dev server at http://localhost:8831/js/client/play.html?ci=1&host=1, staged with `WASM=1 SKIP_PACKS=1 bash scripts/dev/stage.sh` from a15ca5da plus the uncommitted hook of A3. The run lobby-started a bot-filled match (All Pick, Fill with bots, Start match), then looped lock on, off, on. Each leg was a page-side function with `{ ms: 8000, stage: { x: 78.5, z: 82 }, minHp: 560 }`, waiting up to 60 s to reach 560 hp at the stage point before the leg.
- Result per leg: `readyAtFullHp: false` in all three legs; hp 258, 303, 348 at the end; samples 47, 43, 43; `strikesWhileMoving`, `strikesWhileStopped`, `inReachMoving` and `targetsSeen` all 0; kinds {}.
- Finding: 560 hp was not reached inside the 60 s window, so no leg started at full health and the legs prove nothing. Hero regeneration is about 1 hp per second, so a full-hp gate costs minutes; A4's `minStartHp` 200 with a longer `recoverMs` is the replacement, and each leg records `startHp` so Phase B can check it.
- Other attempts in this session, also not valid and not recorded here before: an orbit around structure 13 (target absent from the roster by the time of the run), chases of enemy hero 133 and creep 163 with move and attack orders, and 20 s neutral chases with lock on, off, on. All recorded 0 strikes. The owner filter they used marked allied units hostile, so their target choices are not evidence.
- Status: not run in Phase B. Phase B reruns the module (A4) and records its verdict in DESIGN.md.
- Cleanup done in Phase A: dev server PID 2961824 stopped; witness browser PID 2705643 stopped. Browser PIDs 3507855 and 2587965 were not touched.

## Pillar slopes (no pending change)

- Committed in a15ca5da. `node scenarios/moba/tools/check-pillar-coverage.mjs` reported `ok (56 of 56 impassable slope cells decorated, one prop each)` on 2026-11-08, and map.osd has 56 pillar-rock prop lines. No A-phase change touches the pillars.
- Validation in B1 only, through the gate.

## C1. Twin-stick pure-logic check (code only)

- Files: js/client/twin-stick-check.mjs (new). It checks `twin_stick.js`: deadzone edge and scaling, the sign of the left stick's y and of the right stick's y, the first connected pad, button down edges (a held button does not repeat), and `aimPoint` at yaw 0 and 90 degrees.
- Item: rank 2 below. No check covered `twin_stick.js` before this.
- Not in the gate (check-all keeps 73 entries). Phase B runs it by hand; adding it to the gate is a separate decision because it changes the baseline.
- Validation: `node js/client/twin-stick-check.mjs` prints `twin stick checks ok (...)`.
- Status: written, not run.

## D. Remaining work, twin-stick walking and one-hand lock mode (ranked, 2026-11-08)

1. Live witness that a walking hero strikes while the lock is on (one-hand on), and does not in the red leg. Status: code done (A4, 62c5038b); run is Phase B (B2 explains the invalid earlier attempt).
2. Pure-logic check for `twin_stick.js`. Status: coded now (C1); run is Phase B.
3. Lua tests for `cast_at` and one-handed targeting (`one_hand_fire`, `targeting.best_in_range`, controls.lua). Status: open, not coded. The ui dispatch is covered only by the Rust test of `one_hand` (A1); no test covers the cast path, and it needs ability names from the pack.
4. The lock sequence gate runs without the pad: js/client/movement-sequence-check.mjs stubs `pollPad` and `syncOneHand` as no-ops and passes `padMove` as null. No gate runs a stick-driven locked walk or the one-hand edge. Status: open, not coded.
5. The control scheme is undocumented: no file in docs/ lists the keys, the pad buttons (0 to 3 cast slots), the stick roles (left walks in lock, right aims a cast), or that the unlocked pad only pans the camera. Status: open, not coded.
6. Decision: `forwardKey` takes its world point from the pad aim when the right stick is held (play.js, `padAimWorld() ?? pointer`), so keyboard keys carry a pad aim. Confirm this is wanted before documenting it. Status: open, decision.
- Done and committed, not yet run: strike-on-move flag and sim tests (A2), the one-hand Rust test (A1), the strike-target hook (A3), the witness and verdict (A4, A7), the spec (A5).

## Notes for Phase B (observations, not validation)

- A hero walks about 4.7 cells per second on the live map. Reads taken one second apart can look like a stalled hero; use the position log over several seconds.
- Earlier reports of a hero "stuck" at the fountain were not established: one run ended with the hero mid-path and one with the hero at low health. Do not treat the stall as a cause until B-phase logs show it.
- The dev server for the witness was stopped by PID (2961824) in Phase A; restart it in Phase B with `node scripts/dev/serve.mjs --port=8831 .dev-site` after staging, and stop it by PID afterwards.
