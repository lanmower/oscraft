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
- Validation: `node scripts/witness/strike-while-walking-check.mjs` prints `strike-while-walking verdict checks ok (8 cases)`. The count went from 6 to 8 with F2 (kinds rule).
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
3. Lua tests for `cast_at` and one-handed targeting. Status: coded (E1, 9f0b6d87); run is Phase B.
4. The lock sequence gate ran without the pad (stubbed pollPad and syncOneHand). Status: coded (E2, 117ead67); run is Phase B.
5. The control scheme was undocumented. Status: coded (E3, d427d88b); review is Phase B.
6. Keyboard keys carry the pad aim while the right stick is held. Status: decided to keep and document (E4); no code change.
- Done and committed, not yet run: strike-on-move flag and sim tests (A2), the one-hand Rust test (A1), the strike-target hook (A3), the witness and verdict (A4, A7), the spec (A5).

## E1. Lua cast_at and one-hand targeting tests (ranked 3)

- Files: rust/crates/mod-host/tests/pad_cast_at.rs (new, six tests): one-hand off casts the unit under the aim; one-hand on ignores the aim and takes the best unit in reach; one-hand on with nothing in reach refuses noTarget and casts nothing; one-hand off with nothing under the aim refuses noTarget; a no-target ability casts at the caster whatever the aim; an empty slot refuses noAbility. Commit 9f0b6d87.
- Validation: `cargo test -p mod-host --test pad_cast_at`. The tests read `last_cast` (target, x, y) and the refusal lines from `take_ui`.
- Things to check when it runs: `cast_range` for Hellfire Blast (8203 read as milli-cells, enemy placed 4 cells away); whether the Vampiric Aura cast is accepted (it has no target and no cooldown, so the no-target test depends on it); the `position_fixed` tuple type matching `last_cast`.
- Not in the gate; cargo test runs by hand in Phase B.
- Status: written, not run.

## E2. Sequence gate with the real pad path (ranked 4)

- Files: js/client/movement-sequence-check.mjs. `pollPad`, `padAimWorld`, `castAtAim` and `syncOneHand` are extracted from play.js and run against an injected gamepad list; `padMove` is a local of the driver. A new `pad` mode walks the hero with the left stick in lock, checks one `one_hand` send on lock and one off on release, and checks button 1 casts once at the right-stick aim (108, 100) while button 4 casts nothing. A `stubPad` negative control (no-op pollPad and syncOneHand) must fail. Commit 117ead67.
- Validation: `node js/client/movement-sequence-check.mjs`. This file IS a gate entry (check-all line 58), so Phase B's gate run covers it. The existing controls still run first, so a failure in the new pad mode shows after their lines.
- Things to check when it runs: the extraction headers (`function pollPad() {`, `function padAimWorld() {`, `function castAtAim(slot) {`, `function syncOneHand() {`) still match play.js; the stick forward order has z below the start (forward is -z at yaw 0); `navigator` and `sendInput` injected under the same names.
- Status: written, not run. Changes gate behaviour, so it counts toward B1.

## E3. Control scheme doc (ranked 5)

- Files: docs/CONTROLS.md (new). Lists the lock's effect on the arrows and sticks, the pad buttons 0 to 3 as cast slots 0 to 3 (Q, W, E, R), the stick roles with and without the lock, the keyboard in resolution order (courier keys first when a courier is selected; ability hotkeys; Ctrl groups; the key table from moba_ui.json), and the ping wheel on G and V. Commit d427d88b.
- Differs from the ranked item as written: item 5 said the unlocked pad only pans the camera. The code also casts with the buttons and aims casts with the right stick while the lock is off. The doc says what the code does.
- Things to check in Phase B by reading against the code again: the courier letter keys (D A W R F G) replace the hero keys only while a courier is selected; Shift M is mute, and plain M is the move command; the ping-wheel release rule. These were read from the code, not run.
- Validation: review, no gate. The check-doc-claims checker covers a fixed list of docs that does not include this file.
- Status: written, not reviewed against a running client.

## E4. Decision on keyboard keys carrying the pad aim (ranked 6)

- Decision: keep the current behaviour and document it. `forwardKey` takes its world point from `padAimWorld()` when the right stick is held, otherwise from the pointer, and this applies with the lock on and off. No code change.
- Recorded in docs/CONTROLS.md under "Right stick and keyboard together".
- Validation: none beyond E3's review.
- Status: decided; documented.

## F1. Melee hit attribution (Phase A pass 2026-11-09, item 2)

- Files: scenarios/moba/effects.lua (`on_hit`, the `melee_hit` call now passes `source = attacker`).
- Cause: `melee_hit` was emitted without `source`, so the FxEvent kept the sim default `source: -1` (rust/crates/sim/src/fx.rs:28). The witness counted only `effect.source === heroId`, so its `kinds` stayed `{}` for every melee strike. Ranged hits already passed `source` (attack_arrow).
- Client effect: `js/client/fx.js` resolves an effect's anchor from `target` first (fx.js:415), and `melee_hit` has no entry in the kind table, so the added source does not move the drawn hit.
- Validation (Phase B): the live witness run in F2 must show a non-empty `kinds` on each lock-on leg, with `melee_hit` counted for the hero. No unit test asserts the `source` field itself; that stays open.
- Status: written, not run.

## F2. Witness requires hero-sourced kinds on lock-on legs (Phase A pass 2026-11-09, item 2)

- Files: scripts/witness/strike-while-walking.mjs (`verdict` adds a reason for a lock-on leg whose `kinds` is empty); scripts/witness/strike-while-walking-check.mjs (leg default gains `kinds: { melee_hit: 3 }`; two cases: a green leg with empty kinds fails, a red leg with empty kinds passes).
- Item: the witness fails when `kinds` is empty on a lock-on leg, so the telemetry cannot silently go blank. Lock-off legs are not held to it, because a control leg may legitimately record no hero-sourced effect.
- Validation (Phase B): `node scripts/witness/strike-while-walking-check.mjs` must print `strike-while-walking verdict checks ok (8 cases)`; then the live run (A4) must show non-empty `kinds` on both lock-on legs.
- Status: written, not run.

## F3. Neutral attributed units are not heroes (Phase A pass 2026-11-09, item 3)

- Files: scenarios/moba/core/bounty.lua (`refresh` skips attributed units whose owner is 255); rust/crates/mod-host/tests/neutral_attributed_is_not_a_hero.rs (new, two tests).
- Cause: `bounty.heroes()` listed every attributed unit with no owner check. `game.team`, `game.gold` and `game.setting` take a player through `player_arg`, which refuses 255 (`invalid_player: 255 is outside 0..16`, shared.rs:19). So a neutral attributed unit in that list sent 255 into courier `spawn_couriers`/`use_items`/`report_couriers` and kill_gold's passive-gold and wallet loops on each tick. `game.hostile` accepts 255 (diplomacy_player), which is why the error appeared only on the team and gold calls.
- Mechanism confirmed in code. Which live units are attributed with owner 255 is not confirmed; the loader enrols only placements whose kind is `player_controlled` (rust/crates/sim/src/loader.rs:1322), and the test's neutral creep is spawned attributed on purpose to reproduce the path.
- Validation (Phase B): `cargo test -p mod-host --test neutral_attributed_is_not_a_hero` (both pass). The first test must fail with the bounty change reverted. A live run must show no `invalid_player` in `hook_errors` or the browser log after the fix.
- Status: written, not run.

## F4. Lobby order in the witness doc matches the flow (Phase A pass 2026-11-09, item 4)

- Files: docs/BROWSER_WITNESSES.md (line 7 rewritten). No lobby code changed.
- Cause: the doc said Fill with bots, then Join The Sentinel. The code refuses fill until the caller is host (panels.lua fill handler, `L.host ~= owner` → not-host, line ~1476; the fill button is disabled unless host, line ~1978), and host is set only by a human seat claim by the caller (panels.lua seat handler, line ~1509). So fill before Join was refused, and the doc was wrong. The Join-then-Fill order is the one the live runs used.
- Chosen direction: the doc follows the code, because host gating keeps the witness hero human-seated. Changing the code to allow fill without a host would have needed a new host rule; the owner may redirect.
- Validation (Phase B): one bounded playwriter lobby run in the order the doc now states (mode chip, Join The Sentinel, Fill with bots, hero tile, Ready, Start match). It must show the fill accepted after the join, the host's seat human, and the match started. A run that tries Fill first must show the not-host refusal.
- Owner decision (2026-11-09 Phase B request): the docs are the side that was wrong; Join-then-Fill was accepted, and the host gate was not changed.
- Phase B measurement contradicts the premise the acceptance rested on. On a `?host=1` page the host gate is open (startHost makes the page the host), so Fill is allowed before any seat claim, and the Join button is disabled until the lobby is negotiated (`negotiated()` in scenarios/moba/panels.lua needs a mode and either Fill or every seat taken). So Join cannot be clicked first. Fill before a claim fills all ten seats with bots and leaves no human seat.
- The order that reached the live match (Phase B, 2026-11-09): a mode chip (All Pick), the claim call for seat 0 (`window.oscraft.lobby('claim', {slot: 0})`; the per-seat Claim buttons are hidden in this layout), Fill with bots, hero PUD, Ready, Start match. The page then reported `data-phase="live"`. docs/BROWSER_WITNESSES.md line 7 now states this order. The host gate is unchanged.
- Owner acceptance (2026-11-09, this pass): the owner accepted the measured order as the reachable one, and the docs record it. The Join-then-Fill order the owner first accepted cannot be run, so the accepted order is the measured one.
- My reasoning for keeping the host gate unchanged: Join is disabled until `negotiated()` holds, and that needs a mode and either Fill or every seat held, so Join cannot be the first human action. On a `?host=1` page Fill is allowed before any claim, and it then fills all ten seats with bots. Claiming seat 0 first makes the page's own seat human, and Fill then fills the other nine. The claim-then-Fill order is therefore the only one that keeps a human seat, and the host gate needs no change for it.
- Status: run in Phase B, and the order is documented. Accepted by the owner; the live run in Phase B is what shows the order reaches the live match.

## F5. First-hit latency for a stopped hero in reach (Phase A pass 2026-11-09, item 1)

- Files: rust/crates/sim/src/combat.rs (`set_strike_target_now`: after the target is set, `attack(i)` runs at once when the unit is alive, not channelling and off cooldown); rust/crates/mod-host/src/piccolo_host.rs (the `SetStrike` apply calls `set_strike_target_now`); rust/crates/mod-host/tests/strike_first_hit_latency.rs (new: a pudge and an enemy `objective` structure one cell away; the hero's first swing must be in flight within one advance of the first sampled in-reach advance).
- Cause, measured earlier (idle probe): in reach at t=1, launch at t=3, land at t=10. The two ticks before launch: `targeting.on_tick` queues `cmd_strike` each tick, and a command from on_tick is applied in the queued-command phase of the next step, after that step's unit loop, so `attack(i)` sees it one tick later. The change removes that one tick for a unit that is off cooldown; it does not change any cooldown count, because `attack` decrements only when the cooldown is above zero.
- Not changed, recorded as data: the swing's attack point is 7 ticks (6.88g pudge, unit.ini), which is 0.44 s at 16 Hz and is the game's own wind-up. The 2.7 s figure from the earlier witness was a measurement artifact (creep hits by other units were counted), not this delay. Facing (`face_target`) can add ticks when the hero is not already turned toward the creep; the test's geometry does not rule that out.
- Witness of the hero's own first hit: the live witness must read the hero's first hit from effects with `source` equal to the hero's id (available after F1), and the Rust test above checks the launch index.
- Side effect to check in Phase B: this applies to every unit with a strike command, not only heroes. Sim hashes and replays change for the same inputs.
- Validation (Phase B, 2026-11-09): `cargo test -p mod-host --test strike_first_hit_latency` passes with the fix (launch within one advance of in reach). With `set_strike_target_now` reverted to `set_strike_target`, the same test fails: `the hero came into reach at advance 0 and launched its first swing at advance 6, more than 1 advance later`. The first version of the test (creep two cells east, stopped by ORDER_HOLD) never produced a swing: separation pushes stationary units apart, so it was rewritten (recorded in DESIGN.md).
- Still open: the live first hit. The live witness (F2) shows hero-sourced `melee_hit` effects (`kinds` non-empty), but it does not time the first hit from the in-reach sample, so the live first-hit latency is not measured.
- Status: Rust test cleared; live first-hit timing open.

## G1. Audit of every Phase B item against the 2026-11-09 pass

- A1 (one-hand tests): unchanged by this pass.
- A2 (sim strikes, STATE_VERSION 68): combat.rs gained `set_strike_target_now` (F5). The `cargo test -p sim --test strikes` entry must now run after F5, because the strike start moved one tick earlier for every unit.
- A3 (strike-target hook): unchanged.
- A4 (live witness): the verdict gained the kinds rule (F2); the melee source fix (F1) is what lets that rule pass. Its lobby order is now the order F4 records. Its witness is not run.
- A5 (browser witness spec): BROWSER_WITNESSES.md line 7 changed (F4). `scripts/check-browser-witnesses.mjs` asserts on witness names and rule counts only, not on that line (checked by grep for Fill, Join and lobby: no match).
- A7 (verdict check): now 8 cases (F2). Its run command and expected output are updated in place.
- B2 (invalid earlier attempt): superseded by A4; unchanged.
- C1 (twin-stick check), D items, E1 to E4: unchanged by this pass. E2 and E3 files are not touched.
- New rows: F1 (melee source), F2 (kinds rule), F3 (neutral heroes), F4 (lobby order doc), F5 (first-hit latency).
- G2 (gate and suites, below): the Lua and Rust changes in this pass are not covered by a row before this one; G2 covers them.

## G2. Gate and suites for the 2026-11-09 pass

- Files touched: scenarios/moba/effects.lua, scenarios/moba/core/bounty.lua (Lua pack modules); rust/crates/sim/src/combat.rs; rust/crates/mod-host/src/piccolo_host.rs; new mod-host tests (neutral_attributed_is_not_a_hero, strike_first_hit_latency).
- Command (Phase B): `node scripts/check-all.mjs`, expected baseline `RESULT gates=73 ok=73 failed=0 drift=0 skipped=6` as in B1. If `check-moba-image.mjs` reports the image differs, refresh with `cd rust && cargo run -p mapexport -- ../scenarios/moba ../scenarios/moba/moba.osdt` and check the diff. The image may or may not include Lua modules; the gate decides.
- Command (Phase B): `cargo test -p mod-host` (includes the pack-loading tests that load every module, so a Lua syntax or require error in effects.lua or bounty.lua shows there), and `cargo test -p sim`.
- Status: not run.

## H1. Live timing hooks: swing point and sim tick (Phase A pass 2026-11-09, open item 1)

- Files: rust/crates/sim/src/swing.rs (`swing_point`, the remaining attack point of a unit's swing); rust/crates/web/src/lib.rs (`unit_swing_point`, bounds-checked like `unit_strike_of`); js/client/play.js (`window.__ciSwingPoint`, `window.__ciTicks`, next to `__ciStrikeOf`).
- Cause: the live witness had no sim tick and no remaining attack point, so it could not time a first hit in ticks. `localGame.ticks()` already existed in the wasm; the point did not.
- Validation (Phase B): `WASM=1 SKIP_PACKS=1 bash scripts/dev/stage.sh` must build the new export (wasm-bindgen fails if a missing export is called from play.js only at runtime, so the timed witness in H2 is the check that it exists: `__ciSwingPoint` must return a number, not null, for the hero).
- Status: written, not run.

## H2. Timed live witness: first hit latency, stopped and walking (Phase A pass 2026-11-09, open item 1)

- Files: scripts/witness/first-hit-timing.mjs (new: `firstHitLeg` page-side, `verdictFirstHit` pure, `runFirstHitTiming` runner over the lobby-run page); scripts/witness/first-hit-timing-check.mjs (new: 11 synthetic verdict cases). docs/BROWSER_WITNESSES.md is not changed, because scripts/check-browser-witnesses.mjs counts witness names.
- Measure: latency = sim tick of the first hit minus sim tick of the first sample with the enemy creep in reach (2 cells). Ticks come from `__ciTicks` (H1), the attack point from `__ciSwingPoint` (max observed while the swing is in flight), the hit from the creep's hp dropping below its hp at reach. Legs: stopped with lock on (judged), walking with lock on (judged, oscillates between the creep and a point 1.6 cells toward the stage), walking with lock off (control, recorded, not judged).
- Verdict: fails when a judged leg has no reach, no hit, no attack point, the lock not engaged, an abort, or a first hit later than the attack point plus 1 tick. Latency counts reach to hit, so the launch delay is inside the threshold; that is the defect F5 found (6 advances to launch).
- Threshold source: the attack point is the value the live swing reports, not a number from the data (the 7 ticks in F5 is the 6.88g data and is only a reference).
- Validation (Phase B): `node scripts/witness/first-hit-timing-check.mjs` prints `first-hit-timing verdict checks ok (11 cases)`. Then the live run through gm.playwriter_execute in the BROWSER_WITNESSES.md lobby order, with the page served from the staged `.dev-site` on port 8831. The quoted output must include the three legs' reachTick, launchTick, hitTick, attackPoint, latencyTicks and the verdict. A run where the stopped leg's latency is over attackPoint + 1 is a defect to fix, not a closure.
- Not wired into scripts/check-all.mjs: the existing strike-while-walking-check.mjs is not wired either, so this is a manual witness like it.
- Status: written, not run.

## E3. Control scheme doc review against the code (Phase A pass 2026-11-09, item 2)

Each row is one change, committed on its own. The review compared docs/CONTROLS.md with scenarios/moba/controls.lua, js/client/play.js, twin_stick.js, hero_view_lock.js, hero_run.js and targeting.lua, read-only.

- E3-1 (pad no-target and aim at rest): docs/CONTROLS.md, the pad table (buttons 0 to 3, lock off cell) and a new keyboard paragraph. Cause: the doc said buttons 0 to 3 cast at the aim point for every slot, but controls.lua cast_at sends no-target abilities to cast_key with no point, so they fire at the hero (controls.lua 439-441, 454). With the stick at rest the aim is the hero's own position (play.js 1317). Witness: rust/crates/mod-host/tests/pad_cast_at.rs (a no-target cast is not cast at the aim point, lines 184-200). Keyboard no-target fire point: controls.lua 443-444.
- Status: committed locally; the witness is not run in Phase A.
- E3-2 (keyboard quick-cast): docs/CONTROLS.md, the lock section's release paragraph. Cause: the doc said a keyboard targeted cast always arms for the next click. controls.lua cast_key marks the arm `quick` when the slot's quick-cast setting is on (quick_cast_on, controls.lua 396, used at 434), and release_quick_cast (controls.lua 496-508) fires it on key release at the pointer point, refusing noTarget for a unit target with no unit there. The setting is toggled by toggle_quick_cast (controls.lua 400) through the `quickcast` ui call. Witness: the controls.lua code above; no test is run in Phase A.
- Status: committed locally; not run in Phase A.
- E3-3 (stick speed): docs/CONTROLS.md, the gamepad paragraph after the table. Cause: the doc said deflection sets full speed, but with the lock on hero_view_lock.js (lines 35-38) normalises the stick vector and hero_run.js (RUN_CELLS 64, line 3) orders a point 64 cells ahead, so only the direction changes. Only the unlocked pan scales with magnitude (play.js 1338-1339, 1352). The deadzone rescale matches twin_stick.js (lines 9-10) and is per axis. Witness: none in the repo asserts the ordered point. js/client/hero-trajectory-check.mjs checks order gaps and speed, not the point's distance, so the claim rests on the code lines above. A Phase B browser run would have to show the locked hero's order is 64 cells ahead.
- Status: committed locally; not run in Phase A.
- E3-4 (lock release): docs/CONTROLS.md, the lock section's release paragraph. Cause: the doc named only the button as the release, but the code also releases on any explicit camera write: the right-button drag pan (play.js 1505-1511; camera_mouse.js maps the right button to pan), the podium and flow camera restores (play.js 1225-1229, 1726-1731, 2145-2150), a mod camera take (play.js 3564, modui.js onCameraTake), the minimap drag, and window.oscraft.centreOn (play.js 3792-3794). Every release goes through syncOneHand (play.js 1321-1326), which sends `one_hand` off. A minimap centre click writes nothing while locked. Witness: js/client/movement-lock-release-check.mjs and js/client/movement-sequence-check.mjs, both to run in Phase B.
- Status: committed locally; not run in Phase A.
- E3-5 (keyboard order): docs/CONTROLS.md, keyboard rule 0. Cause: the doc put custom keys before courier keys, but controls.lua handles a quick-cast release and Escape, then courier home and the courier keys (lines 655-667), and only then resolve_key, which reads CFG.custom (line 594). So a custom key bound to a courier letter never reaches the custom table while a courier is selected. The doc now states the code's order. Choice made: the doc follows the code, because the courier keys replace the hero action while a courier is selected (table row D A W R F G) and the custom table is documented as overriding defaults, not courier keys. Owner may redirect: moving the custom lookup above the courier block is a one-line change in controls.lua, and this row would then need to flip. Witness: none in the repo covers custom and courier keys together; not run in Phase A.
- Status: committed locally; not run in Phase A. Decision recorded for the owner.

- E3-6 (casting omissions): docs/CONTROLS.md, a new "Casting, range and refusals" section at the end. Cause: the doc did not describe the walk into range (controls.lua walk_then_fire, move_into_reach at 90 percent of the range, CFG.walkSeconds = 10 at line 29, the `range` refusal at lines 539-541), the lock-on fallback of a unit-or-point ability to the hero's position (one_hand_fire, lines 421-422), or the refusals noHero, noAbility, cooldown, noTarget (cast_key, cast_at). Witness: rust/crates/mod-host/tests/pad_cast_at.rs covers the pad no-target cases; the walk and the range refusal have no test in the repo, so they rest on the code lines above.
- Status: committed locally; not run in Phase A.

- E3-7 (locked walk and strike flag): docs/CONTROLS.md, the two walking bullets of the lock section. Cause: the doc said the arrows and stick walk the hero, without the run model. hero_run.js sets a run to a point RUN_CELLS (64) ahead, a tap under TAP_MAX_S (0.2 s) keeps it running, a hold of 0.2 s or more issues a hold (line 39), and a hostile within HERO_SIGHT_CELLS (28.125, line 5) stops the run. The strike-on-move flag is synced every tick for each player's own hero (controls.lua 523-527, 532), not only the local hero. Witness: js/client/hero-trajectory-check.mjs drives hero_run.js through press, release and re-press. It does not assert the 0.2 s tap and hold threshold or the hostile stop, so both are open for a Phase B run.
- Status: committed locally; not run in Phase A.

- E3-8 (best unit): docs/CONTROLS.md, the cast bullet of the lock section. Cause: the doc said "the best unit in reach" without defining it. scenarios/moba/targeting.lua best_in_reach sorts by class (class_order, with the aggro target as a class factor), then by distance squared, then by the lower unit id. Witness: none in the repo asserts the full tie order in a pad or key cast; the definition rests on targeting.lua lines 141-155.
- Status: committed locally; not run in Phase A.

- E3-9 (edge pan): docs/CONTROLS.md, the unlocked-camera paragraph of the lock section. Cause: the doc listed only the arrows and the left stick as camera pan. play.js edge-of-screen pan (lines 1344-1350) runs only when unlocked; the locked branch returns early (lines 1332-1335). Witness: js/client/movement-sequence-check.mjs runs the locked camera path against a fake DOM, but it does not drive a pointer at the screen edge; the edge rule rests on the play.js lines above.
- Status: committed locally; not run in Phase A.

- E3-10 (pad source and start): docs/CONTROLS.md, the gamepad section's first paragraph and the paragraph on buttons 0 to 3. Cause: the doc said "standard mapping" without saying the pad is the first connected gamepad with no layout check (twin_stick.js lines 20-22), and it did not say pad polling and casts wait for the renderer and the pan rules (play.js 1329-1330). Witness: none; the claim rests on those lines, and a Phase B run with a connected pad would have to show it.
- Status: committed locally; not run in Phase A.

## G3. Phase B items for the 2026-11-09 open-items pass

- Rows to validate: H1 (swing point and tick accessors, `STATE_VERSION` unchanged: accessors only, no layout change), H2 (timed live witness and its check), F4 (acceptance recorded, live order run), E3-1 to E3-10 (CONTROLS.md, doc changes only; the code-side check is the `pad_cast_at` and `movement-*` gates named in each row).
- Commands (Phase B): `WASM=1 SKIP_PACKS=1 bash scripts/dev/stage.sh`; `cargo test -p sim` and `cargo test -p mod-host`; `node scripts/witness/first-hit-timing-check.mjs`; `node scripts/witness/strike-while-walking-check.mjs`; the live timed witness (H2); `node scripts/check-all.mjs`.
- Status: not run.

## Notes for Phase B (observations, not validation)

- A hero walks about 4.7 cells per second on the live map. Reads taken one second apart can look like a stalled hero; use the position log over several seconds.
- Earlier reports of a hero "stuck" at the fountain were not established: one run ended with the hero mid-path and one with the hero at low health. Do not treat the stall as a cause until B-phase logs show it.
- The dev server for the witness was stopped by PID (2961824) in Phase A; restart it in Phase B with `node scripts/dev/serve.mjs --port=8831 .dev-site` after staging, and stop it by PID afterwards.
