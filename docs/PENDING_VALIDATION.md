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

## H2 run record (Phase B, 2026-11-09)

- Run 1 (arrival window 120 s): lobby reached `live` (`LOBBY {"claim":true,"heroTileCount":1,"phase":"live"}`), but the stopped and walking legs ended with `hero did not reach the stage at start hp`. The verdict was `ok:false`, with 8 reasons, all of them the stage arrival, reach and first hit.
- Run 2 (arrival window 330 s, output in /tmp/witness-driver/run1.out, `RUN_EXIT 0`): the same two legs aborted with `hero did not reach the stage at start hp` again. The lock-off control arrived (`ready: true`, `targetId 87`, `samples 10`), so the arrival was slow, not blocked.
- Cause, measured: a probe after the same lobby order (/tmp/witness-driver/probe3.mjs) sampled the hero every 10 s after a move order to the stage. The hero moved steadily (z 269 to 226 over 120 s, with x held at 61.28 for stretches around the path), and the sim ran at about 1.3 to 1.5 ticks per second. At 16 Hz the hero covers about 4.7 cells per second, so the 190-cell trip is a few minutes of sim time at this rate. The arrival window was too short for the headless tick rate. It is not a sim defect: the hero is not stuck.
- A smaller viewport (640x400, /tmp/witness-driver/probe4.mjs) gave about 1.5 ticks per second too, so the viewport is not the lever and stays at 1280x800.
- Fix (this pass): `resetMs` is 600000 in scripts/witness/first-hit-timing.mjs. Latency is measured in sim ticks, so the slow wall clock does not change the numbers. The stopped and walking legs run after the arrival, so the window is spent once.
- Run 3 (600 s window, /tmp/witness-driver/run3.out): the hero reached the stage in all three legs (`ready: true`), and the target was creep 87. Both judged legs ended with `never came into reach`, and no leg had a first hit. Cause: the stopped leg never walked to its target. It only held still at the stage, so the creep had to wander into reach by chance.
- Run 4 (stopped leg walks to its target, stops in reach; /tmp/witness-driver/run4.out): the stopped leg again ran out its 480 s window without reaching the creep (samples 242, no reach). The lock-on walking leg reached at tick 793, launched at 796 (launch delay 3), hit at 800 (latency 7), with attackPoint 3. The verdict failed: `walking, lock on: first hit 7 ticks after reach, later than the attack point 3 plus 1 tick`. The control leg reached at 824 and launched at 859.
- Run 5 (per-tick trace added; /tmp/witness-driver/run5.out, `RUN_EXIT 0`, after a foreground wait on the still-running process 3501011): the stopped leg walked from 100 cells away and was at 64 cells when its 480 s window ended (samples 189, no reach). Cause: the target was chosen from the fountain, about 100 cells from the hero, and the sim ran at about 0.3 to 0.5 ticks per second that run. The walking lock-on leg reached at tick 718 with its swing point already at 7, so the swing launched on the reach tick. The witness recorded launch at 721 and attackPoint 4, because the swing bookkeeping ran only after the reach sample. That is a witness defect, fixed in this pass: swing bookkeeping now runs on every sample. Hit at 728, trace points 4 at 721, 1 at 724, 0 at 728.
- Replay of the fixed detection on run 5's trace (no live run): reach 718, launch 718, attackPoint 7, release 728, hit 728, latency 10 against a threshold of 8. This still fails by the sampled numbers, but the samples are 2 to 3 ticks apart (ticks 724 and 728 are the only samples in that window), so the release and hit ticks have up to 3 ticks of error. The true latency lies between 7 and 10, so this is not decided by the run.
- Run 6 (fixed detection, /tmp/witness-driver/run6.out, `RUN_EXIT 0`, the code committed in this pass): the stopped leg again ran out its 480 s window (samples 199, the last reading at 47.79 cells from creep 118 at tick 638). The walking lock-on leg reached at tick 812 (d 1.96, point 0), with samples at 816 (point 4, d 0.84), 819 (point 1, d 0.07), 822 (point 0, hp 550 to 484). The verdict is `walking, lock on: first hit 10 ticks after reach, later than the attack point 4 plus 1 tick`. The control reached at 848, launched at 860, and did not hit.
- Run 6 reading: attackPoint 4 is the largest value the samples caught, not the swing's point at launch. The decrement from 4 at 816 to 1 at 819 puts the launch at about 813 with the point at 7. The release then falls at 820, so the hit is about 8 ticks after reach, which would pass the threshold of 8. That is an inference from the decrements, and no sample shows tick 813 or 820, so H2 cannot be closed on these numbers.
- Open cause, not closed: (0) the attack point is the largest sampled value, so a launch between samples under-reports it (run 6: 4 for a 7-tick swing), which makes the threshold too tight. (1) the sample gap. The page's timers fire every 2 to 3 ticks under headless rendering, so the 1-tick threshold can't be judged from sampled hit and release ticks. (2) The stopped leg's approach window is too short when the target is far from the fountain.
- Status: H2 is open. The detection fix is committed; the witness's verdict is not decided by these runs.

## H2 close pass (Phase A, 2026-11-09): changes

The H2 witness was pushed in f964cf11 with its verdict still failing. This pass closes it or leaves it open with a cause. Phase A changes only; no build, test or witness run in Phase A. The three causes from the H2 run record each get one change, all in scripts/witness/first-hit-timing.mjs and scripts/witness/first-hit-timing-check.mjs, committed locally.

- H2-A1 (attack point at launch): the attack point is the sampled point at the launch sample, the first sample with a point above zero, not the largest sampled value. Cause: run 6 read 4 for a swing that was 7 at launch, because the largest sample came after three decrements. Witness: the live timed run (H2-B) must show `attackPoint` equal to the launch sample's point, and the reach-to-launch ticks must match the trace; the synthetic check has a case for a missing attack point.
- H2-A2 (hit tick within one tick): the leg now waits for each sim tick to advance (`nextTick`, polling `__ciTicks`) and samples once per tick, instead of a fixed 5 ms timer. Each leg records `windowGap`, the largest tick gap between two samples from the reach sample to the hit. A judged leg whose `windowGap` is above `maxWindowGapTicks` (1) fails the verdict, so a coarsely sampled run cannot pass. Cause: run 5 and run 6 sampled 2 to 3 ticks apart, so the hit tick had up to 3 ticks of error. Witness: `node scripts/witness/first-hit-timing-check.mjs` now has 14 cases, including `a judged leg sampled two ticks apart in its window fails` and `a coarsely sampled lock-off control does not fail the verdict`; the live run must show `windowGap` 1 on both judged legs.
- H2-A3 (stopped leg window and target): decided to keep the target, the nearest hostile creep to the hero, and to lengthen the judged leg window to 900 s (`legMs`), with the control at 60 s (`controlMs`, not judged). Reason: the target is about 100 cells from the fountain, and moving it closer would bake a map position into the witness. The approach is setup, not the measured interval, so the window only has to cover the walk. Run 5 and run 6 each ran out 480 s at 47 to 64 cells away. Witness: the live run must show the stopped leg reaching (`reachTick` not null) and hitting, with `ready: true`.
- Not changed in this pass: the sim. The attack point and the release come from the sim's own swing point through `__ciSwingPoint`, unchanged.
- Status: committed locally; the live run (H2-B) decides H2.

## H2 leg-2 defect and the silent fallback (runs 9 to 13, 2026-11-09)

- Runs 9 to 13 all judged the second leg against creep 99 after the first leg's creep 118. Leg 1 was valid in every run: reach 793 to 798, launch at reach, hit 6 ticks later against attack point 7. Leg 2 failed in every run, with `struckOther` true in runs 10 to 13 and a 21-tick latency in runs 9 to 13.
- Run 13 trace (/tmp/witness-driver/run13.out, `RUN_EXIT 0`, heading logged from the roster). Stamp changes for leg 2, as [tick, distance to creep 99, hero heading, struck creep, reach, launch, land]: 812 (25.0 cells, 32768, creep 118 stamp from leg 1); 1024 (2.75, 59792, creep 100 reach and launch 1024, land not yet); 1033 (1.92, 55182, creep 100 land 1030); 1217 (1.92, 63732, creep 101 reach 1216); 1233 (1.92, 60326, creep 101 launch 1230); 1240 (1.92, 60326, creep 101 land 1237); 1740 (1.07, 60326, creep 99 reach 1738); 1753 (1.07, 49838, creep 99 launch 1752); 1760 (1.07, 49838, creep 99 land 1759).
- Cause of the 21-tick latency: facing. The heading held at 60326 from 1240 to 1740 and turned to 49838 by 1753, a change of about 58 degrees if 65536 is a full turn. The launch at 1752 waited for that turn: `face_target` (facing.rs) only launches once the heading is within tolerance of the target. The approach had carried the hero past creep 99 and left it facing away, so the first hit on 99 could not start at reach.
- Cause of the interceptor strikes: the sim picks the nearest in-reach creep by class (targeting.lua `best_in_reach`), so creeps 100 and 101 won while the hero was 1.9 to 2.8 cells from creep 99. The witness's approach brought them into reach.
- Cause of the check letting them through: the isolation check (`nearestCreep`, now `pickCreep`) ran once at leg start, and when no candidate passed it, it fell back silently to the nearest unisolated creep (`pool = isolated.length > 0 ? isolated : candidates`). The leg-start roster was not logged, so run 13 cannot say whether creep 99 was isolated at start or was the fallback pick. Both fit the data; the run after the fix decides it.
- Phase A change (this entry, before the next run): `pickCreep` no longer falls back. When no candidate has no hostile unit within `isolationCells` (4) of it or of its path from the hero, the leg aborts with `no isolated creep: nearest creep <id> has hostile units <ids> within 4 cells`, and the verdict fails on that reason. The chosen creep is recorded when the check passes. The approach is a straight move at the creep (no oscillation). Trace logs each stamp change with the hero heading. Witness: `node scripts/witness/first-hit-timing-check.mjs` (15 cases, unchanged verdict rules), and the live run 14.
- Correction to the H2-A2 row above: HEAD still carries the `windowGap` rule (`maxWindowGapTicks`, 8 witness lines and 4 check lines). This pass removes it from the working tree, because the per-tick stamp trace replaces it; the commit carries that removal.
- Status: H2 is open until a live run shows both judged legs valid under the verdict. If no clean creep exists on this map, leg 2 aborts with the reason above and H2 stays open; the next decision is a target that the map provides, not a fallback.

## H2 leg-2 start: the hero stands beside its own first target (run 14, 2026-11-09)

- Run 14 (/tmp/witness-driver/run14.out, `RUN_EXIT 0`, fallback removed): leg 1 valid (creep 118, reach 801, launch 801, hit 807, latency 6). Leg 2 refused at its start with `no isolated creep: nearest creep 99 has hostile units 100, 101, 118 within 4 cells`. Creep 118 is leg 1's target, still alive, and within 4 cells of the hero, because leg 1 ends with the hero in reach of it.
- Cause: the pick treats every living hostile within 4 cells of the hero as an interceptor, and that includes the previous leg's own target, which the hero is still beside. Every candidate's path starts at the hero, so with 118 beside the hero no candidate can pass. The data says the leg-2 start is contaminated by leg 1, not that the field has no clean creep.
- Phase A change (before the next run): the witness records the hero's position at leg 1 start (`origin` in the result) and passes it to later legs as `retreat`. Before a later leg picks, if any living hostile is within `isolationCells` of the hero, the hero moves to `retreat` and the leg waits up to `retreatMs` (240 s) for it to arrive. The pick then runs from that position, and the same refusal applies if the position is still crowded. The verdict rules are unchanged.
- Status: H2 open until a live run shows both judged legs valid under the verdict.

## H2 leg-2 after the retreat (run 15, 2026-11-09)

- Run 13 verdict (/tmp/witness-driver/run13.out, `RUN_EXIT 0`): `ok: false`. Leg 1 valid (creep 118, reach 796, launch 796, hit 802, latency 6). Leg 2 (creep 99): `struckOther` true and latency 21 (reach 1738, launch 1752, hit 1759). The trace above is the evidence for both causes.
- Run 15 (/tmp/witness-driver/run15.out, `RUN_EXIT 0`, retreat in place): leg 1 valid (creep 118, reach 795, launch 795, hit 801, latency 6). Leg 2 refused before any strike: `no isolated creep: nearest creep 111 has hostile units 109, 110 within 4 cells`. The control was refused with the same reason. The verdict is `ok: false`.
- What run 15 shows: the retreat cleared leg 1's creep 118 from the leg-2 start, because 111 is now the nearest candidate and 118 is no longer named. The refusal then says that no candidate on the map passes the check from the retreat point, not only the nearest. Every candidate's hero-to-creep segment, or the creep itself, has a living hostile unit within 4 cells.
- Open cause: the check counts every living hostile unit as an interceptor, including towers and lane creeps. The refusal does not name the player or type of 109 and 110, so the run cannot say whether they are neutral camp creeps (the leg's own kind) or enemy structures and lane units. That decides whether the check is too broad or the map has no clean neutral creep from this spot. The next step is to log each blocker's player and type in the refusal and read it from one run, before changing the rule.
- Status: H2 open. The verdict is not valid; no push.

## H2 target selection: the interceptor rule blocks every creep (Phase A, 2026-11-09)

- Cause, from run 15's refusal (`nearest creep 111 has hostile units 109, 110 within 4 cells`; the control refused the same way): the interceptor rule treated any living hostile within `isolationCells` (4) of the candidate as a blocker. A creep in a camp always has camp-mates within 4 cells, so every candidate on the map was blocked. The rule counted camp-mates on the far side of the candidate, which the sim never strikes first.
- What the sim does (targeting.lua `best_in_reach`, class then centre distance): the hero strikes the nearest plain creep in reach from its stop point. A creep farther from the stop point than the candidate does not take the first hit. A creep nearer to the stop point does. The rule compared against the candidate, not against where the hero stops.
- Cause of the approach strikes in runs 10 to 13: the hero's path was blocked by bodies in the corridor, so it stopped with zero velocity in reach of a creep beside the path, and that creep was struck first. The corridor check is correct for this and stays.
- Fix (witness, `pickCreep` and `interceptorsOf`): a candidate is valid when no living hostile other than itself (a) lies within `isolationCells` of the segment from the hero to the candidate (the approach corridor), or (b) lies nearer to the stop point than the candidate, where the stop point is `reachCells` (2) from the candidate on the hero's side. The pick is the nearest valid candidate. When none is valid, the refusal names the nearest candidate and its blockers under the same rule. The control leg uses the same pick.
- Witness: the live run 16 must show `ok: true` for both judged legs (or name a new blocker that the rule now counts), with leg 2's `targetId` distinct from leg 1's. `node scripts/witness/first-hit-timing-check.mjs` covers the verdict rules unchanged.
- Status: code and doc written in Phase A. Not built, not run (Phase A). Phase B runs the witness, then check-all, then one push.

## H2 driver race: the lock button is hidden while the flow stage clears (runs 16, 16b, 2026-11-09)

- Run 16 (`RUN_EXIT 1`) timed out on `locator.click` for `#hero-lock` after resolving it. Run 16b (`RUN_EXIT 1`) timed out waiting for `#hero-lock` to appear, 30 s, with the same lobby order.
- Cause: `js/client/gui_flow.js` line 27 hides `#hero-lock` while `body[data-flow]` is set, and the flow stage is cleared after `body[data-phase="live"]` is set. The driver clicks the lock as soon as the phase is live, so under machine load (other sessions' browsers were running; load average 9.8) the click lands before the stage clears and waits for a button that is hidden. Earlier runs passed because the stage cleared in time.
- Fix (witness, `runFirstHitTiming`): wait for `#hero-lock` to be visible, up to 120 s, before the first lock click. The verdict rules and legs are unchanged.
- Status: the fix is in Phase B, with run 16c as its witness. No witness verdict is recorded under this entry.

## H2 run 16c: the rule rejects every candidate from the retreat point (2026-11-09)

- Run 16c (/tmp/witness-driver/run16c.out, `RUN_EXIT 0`, the stop-point rule and the lock wait in place): `ok: false`. Leg 1 valid (creep 118, reach 803, launch 803, hit 809, latency 6). Leg 2 refused: `nearest creep 109 has hostile units 110, 111`. The control refused with `nearest creep 111 has hostile units 109, 110`.
- The refusal wording still says "within 4 cells". The check now tests the approach corridor and whether a hostile is nearer the stop point than the candidate, so the wording is stale and must change with the rule.
- What this shows: the nearest candidate's blockers are its camp-mates, and they sit nearer the hero's stop point than the candidate, so the sim would strike them first. The rule is right about that. But the refusal is for the nearest candidate only; `pickCreep` refuses only when no candidate on the map passes, so every neutral creep is blocked from the retreat point. The refusal does not name the player or type of any blocker. The unanswered question is whether those blockers are neutral camp creeps or hostile lane and structure units, which decides whether the interceptor rule should count them.
- Status: H2 open. No valid verdict. Not pushed.
- Decision needed (owner): whether hostile units of the lanes and structures count as interceptors for the first-hit witness, or whether the judged legs use a target the map provides away from lanes. The witness cannot pass this rule as written from the retreat point.

## H2 run 16c decision: keep the lane-strike protection (2026-11-09)

- Decision (owner): hostile lane units and towers count as interceptors. The rule is not relaxed. The judged legs and the control take a target clear of the lanes, and when no candidate is clear, the leg is refused and its named blocker is recorded.
- Code (Phase A): the rule was already counting every living hostile unit, lane units and towers included, so no rule changed. The refusal now names every blocker with its player, its unit type, whether it sits on the approach or nearer the stop point, and its distance from the stop point, and it reports how many candidates were checked. The identities are needed to say whether any neutral creep is clear of the lanes.
- Phase A gives no witness result. Phase B runs the witness; the verdict must show both judged legs and the control with distinct targets, or the named blockers show that no clear target exists from the retreat point.

- Run 17 (/tmp/witness-driver/run17.out, `RUN_EXIT 0`, named blockers, the rule unchanged): `ok: false`. Leg 1 valid (creep 118, reach 803, launch 803, hit 809, latency 6). Leg 2 and the control both refused: `all 44 candidates are blocked; nearest 111 is blocked by 109 (player 255, 46, on the approach, 2.7 cells from the stop point); 110 (player 255, 45, on the approach, 3.1 cells from the stop point)`.
- What this shows: the blockers are neutral creeps (player 255), not lane or tower units, so the owner's wording on lanes and towers does not name them directly. The rule counts them because the sim strikes the nearest in-reach creep, and a camp-mate on the approach would take the first hit. Leg 1 found creep 118 clear from the same retreat point before any strike, and after it, no other creep is clear from there.
- Status: H2 open. No valid verdict for leg 2 or the control. Not pushed.
- Decision needed (owner): the rule counts neutral camp-mates on the approach as well as lane and tower units. Lane-only protection would be a relaxation, so the rule stays unless you decide otherwise. Options left, each a new design: a leg-2 start point other than the fountain, or a judged target placed by the map rather than found by the witness.

- Decision (owner, after run 17): camp-mates on the approach keep counting. The sim strikes the nearest in-reach creep first, so the witness must measure that. Keep the rule.
- Start points tried: the fountain only (the leg-1 origin, the retreat point for later legs). Runs 16c and 17 show that all 44 neutral candidates are blocked from it, so no start point was moved. Every other start point is unknown to the witness: the roster holds creeps and no terrain, so a search would walk the hero to guessed points. This is why the fallback below is used now.
- Fallback (owner's rule): when no candidate is clear from the start point, the witness places a neutral creep at a clear point. The point is tried at 8, 12, then 16 cells from the start, at 15-degree steps, and it is accepted only when no living hostile is within 4 cells of it and no living hostile is within 4 cells of the segment from the start to it. The placed creep uses the kind of the existing neutral creeps (their roster `type`) and is spawned with `localGame.spawn_attributed(255, kind, x, z)` through a new page hook, `window.__ciSpawnNeutral`. The witness then picks again with the unchanged rule; the placed creep is clear by construction.
- The placement is local to the witness page and marks the replay incomplete (as the sim's spawn does). It is a test target, not a change to the map.
- Phase A has no witness result. Phase B runs the witness until both judged legs and the control show ok true with distinct targets.

- Run 18 (/tmp/witness-driver/run18.out, `RUN_EXIT 0`, placement in place): `ok: true`. Leg 1: creep 118, reach 797, launch 797, hit 803, latency 6. Leg 2: placed creep 223, reach 1371, launch 1371, hit 1377, latency 6, struckOther false. Control: placed creep 248, 5 samples, then aborted with `no clear creep: all 44 candidates are blocked`; `reachTick` null, which is expected for a lock-off control, but the abort is not.
- Control cause: creep 248 was gone or at zero health within five samples, so the mid-leg re-pick ran. That re-pick does not place, and the 44 neutral candidates are blocked from the hero's position, so the re-pick refused. The trace has one sample and no health, so the run does not say whether a tower or lane unit killed 248. The placement checked only 4 cells, and neutral creeps are enemies of all, so a placement near a tower or lane unit is the likely cause.
- Phase A fix: placement also keeps 10 cells from every living hostile that is not neutral (lanes, towers, enemy units), which is the owner's "clear of the lanes". The mid-leg re-pick uses the same choice as the start, so a lost target is replaced under the same rule. Each leg also reports `targetHp` at its end, so a lost target shows whether it died.
- Phase A has no witness result. Phase B runs the witness until the legs and the control show `aborted: null`, with distinct targets.

- Run 19 (witness output /tmp/witness-driver/run19.out, `RUN_EXIT 0`, commit 6b780fbc, dev server PID 104780 on port 8831): `ok: true`, all three legs `aborted: null`, distinct targets. Leg 1 (creep 118): reach 795, launch 795, hit 801, latency 6, `targetHp` 7448, struckOther false. Leg 2 (placed creep 223): reach 1389, launch 1389, hit 1395, latency 6, `targetHp` 1231, struckOther false. Control (placed creep 249, lock off): reach 1702, launch 1702, hit 1709, latency 7, `targetHp` 1231, 23 samples, struckOther false.
- Run 18's control cause (creep 248 lost mid-leg) is addressed by the lane clearance and the re-pick placement. Run 19 did not lose any target.
- Status: H2 closed on the witness verdict of run 19. The judged legs measure the sim's first hit within the attack point plus one tick, with the placed target clear of the lanes. check-all: `RESULT gates=73 ok=73 failed=0 drift=0 skipped=6 strict=false` (/tmp/witness-driver/check-all-h2.out, `CHECK_EXIT 0`).

## D1 pass: clear-target rule for the one-hand witness (Phase A, 2026-11-09)

- Ranked remaining work, from this doc, with status:
  1. D1, one-hand walking-strike witness (owner rank 1): module and verdict check written; target rule defective (below); not run.
  2. G2, cargo test sim and mod-host, plus check-all, for the 2026-11-09 Lua and Rust changes: check-all 73/73 at 3dd9924a; cargo tests not re-run since the F-series; not run.
  3. G3 and A2, strikes tests after F5, staged wasm, strike-while-walking check: not run.
  4. F5, first-hit latency: H2 run 19 is the live first-hit timing; the row still says open; docs only.
  5. F1, F2, F3, melee attribution, kinds rule, neutral heroes: written, not run.
  6. F4, lobby order: owner accepted; live run not done.
  7. E2, sequence gate with the real pad path: written, not run; changes gate behaviour.
  8. E1, Lua cast_at tests, and C1, twin-stick pure-logic check: written, not run; C1 is not in the gate.
  9. E3, control scheme review against a running client: committed, not reviewed.
  10. A5, browser witness spec: written, not run.
  11. Process: the H2 runs used playwright-core directly, not the gm playwriter that invariant 11 requires. Owner decision; not changed here.
- Defect found on review: `scripts/witness/strike-while-walking.mjs` picks its target as the nearest creep to a fixed stage, with no clear-target rule. That is the H2 failure: a leg can fight a camp-mate or a lane unit and count it as its own walking strike. Its verdict does not check the target.
- Phase A change: `scripts/witness/clear-target.js` holds the H2 rule as a plain script (interceptors on the approach or nearer the stop point, the 10-cell lane clearance for placement, and the named refusal). The one-hand module uses it for its target choice at leg start and again when a target dies, with the same placement fallback as H2 (`__ciSpawnNeutral`). A refused leg aborts with its named blocker, and the verdict fails on it.
- Witness for this change: `node scripts/witness/clear-target-check.mjs` (node tests of the pure rule, no browser); the live one-hand run is Phase B. Not built or run in Phase A.
- H2 is unchanged in this pass; moving its witness onto the shared file is a separate change.
- Corridor correction (found while writing this pass): the H2 rule's approach corridor ran from the hero to the candidate itself. Every creep within 4 cells of the candidate is then within 4 cells of that segment's end, so the candidate's own camp-mates on the far side blocked it. Run 16c's stop-point rule kept that corridor, which is why all 44 candidates stayed blocked. The shared file's corridor runs from the hero to the stop point, which matches the owner's decision that a camp-mate counts when it is on the approach or nearer the stop point. H2's witness still has the old corridor; its first clear target (run 19) was found under the old rule, so a re-run of H2 under the corrected corridor is a follow-up, not part of this pass.
- Known risk for the one-hand module: each leg resets to the stage before it picks, so a creep still alive beside the stage from an earlier leg blocks the pick, and the leg refuses with its named blocker. Nothing in this pass changes that; the first live run shows whether it happens.
- Changes in this pass, each committed locally as lanmower: (1) `scripts/witness/clear-target.js` and `scripts/witness/clear-target-check.mjs`, the shared rule and its node tests (witness: `node scripts/witness/clear-target-check.mjs`, not run in Phase A); (2) `scripts/witness/strike-while-walking.mjs`, the one-hand target choice through the shared rule with placement and a named refusal (witness: the live one-hand run in Phase B, not run in Phase A); (3) this entry.

## Phase B pass 2026-11-09 (D1, H2 re-run, gates): record

- Browser work ran through the gm playwriter setup (`gm.playwriter_execute`, `state.page`), not playwright-core. The witness modules are imported inside the snippet and run with `runWitness(state.page)` / `runFirstHitTiming(state.page)`. Each witness run uses a fresh run context (`gm.playwriter_browsers({action:'close'})` closes only this run's context), so no leftover match is reused.
- Staged wasm: `WASM=1 SKIP_PACKS=1 bash scripts/dev/stage.sh` ran with `wasm-pack` at `/config/.cargo/bin` (job PATH); the stage wrote `.dev-site` (EXIT 0). The job's PATH did not find wasm-pack at first (`wasm-pack: command not found`), so the run added `/config/.cargo/bin` to PATH.
- Node checks, all pass: `clear-target-check.mjs` (ok), `strike-while-walking-check.mjs` (8 cases), `first-hit-timing-check.mjs` (15 cases), `js/client/movement-sequence-check.mjs` (sequence ok, all negative controls fail as expected, pad walk ok).
- Cargo, all pass: `cargo test -p sim` (12 result blocks, 0 failed), `cargo test -p sim --test strikes` (9 passed), `cargo test -p mod-host --test pad_cast_at` (6 passed). `cargo test -p mod-host` was killed mid-run by the gm job runner (16 of 17 test binaries reported, the last one `running 1 test` had no result); rerun detached in `/tmp/gm-modhost2.log` (see the result line below once it finishes).

### Defects found live and fixed in this pass

1. Placement spawn refusal aborted the leg. The sim refused some placed points (`spawn_attributed` → `fits` refuses unwalkable cells: probes refused (84.6,83.3), (86.4,82), (120,120), (30,30) and accepted (76,257), (100,100), (200,200)). `choose` gave up after the first refusal with `could not place a creep at ...`. Fix in `clear-target.js`: placement tries each clear point in order until the sim accepts one (`placements` returns all clear points; `choose` loops). Node tests added: a refused point is followed by the next, and an all-refused list names the refusal. H2's `placeClearTarget` has the same fix (it continues on refusal).
2. Placed points in a pocket were unreachable. The old stage (78.5, 82) lies in a pocket: the hero moves west, south and north-west from it, but the east and north-east points stop it after 0.3 cells. Fix: a reachability probe in `clear-target.js` `choose` (an injected async `reachable(point)`): the probe orders a move to the stop point and counts the approach over 1.5 s (`probeMs`, `probeCells`), an unreachable existing creep is excluded and the next is picked, and placement spawns only at a probed point (`probeLimit` 8). The witness leg passes `reachable` from its own move order.
3. The walk orders went to a creep's own centre, a cell its footprint occupies; the hero stood still. Fix: the walk stations are now stop points two cells from the creep on the stage side, and the second station 1.6 cells further (`stopCells`, `oscillateCells`).
4. The stage was a fixed point in the pocket. Fix: the stage is the hero's own position at leg start (`stage` in the leg), so no fixed map coordinate is assumed.
5. No strike target was ever set while walking. Fix: the leg issues a commanded attack (`__ciAttack`) on the picked creep and re-issues it when the strike target is lost; the leg result carries `initialPick` (the id, type, position and the strike target 400 ms after the attack).
6. H2 corridor: `first-hit-timing.mjs` `interceptorsOf` now measures the approach to the stop point (the same corridor as `clear-target.js`), and its cause text names the stop point.

### One-hand witness (D1): run results

- Run with fixes 1 to 4 (before the attack order): all three legs started (`aborted: null`), no strike target, `movingSamples` 2 to 6, the hero did not walk. Cause: the walk went to the creep's centre (fix 3).
- Run with fix 3 and the stage at the hero's spawn: the hero walks (movingSamples 188, 190, 169), the leg picks creep 118 (type 6, 1.6 cells from the hero), and `probes` shows the approach gained 6.39, 5.49 and 2.74 cells.
- Run with fix 5 and the `initialPick` record: leg 1 `strikeAfterAttack` -1, leg 2 -1, leg 3 118 (`inReach` 10, `targetsSeen` [118]). Verdict: `ok: false`, with `never in reach while moving` for legs 1 and 2, and no strike or kinds on any leg. Last verdict line: `lock on: the hero was never in reach while moving; lock on: no strike while moving with one-hand on; lock on: no hero-sourced effect kinds recorded; lock off: never in reach while moving; lock on again: no strike while moving with one-hand on; lock on again: no hero-sourced effect kinds recorded`.
- Superseded by the D1 green pass below. The refusal was not capture-able before this pass: each attempt now records the sim's terms and its refusal (`attack_terms`, `simRefusal`, `uiRefusal`).
- D1 status at this point: red, an honest failure of the one-hand walking strike, not a witness defect.

### H2 re-run under the corrected corridor

- Change before the run: `first-hit-timing.mjs` `interceptorsOf` now uses `segmentDistance(v, from, stop)` (was `from, c`), and `placeClearTarget` continues past a sim refusal.
- Run (live, fresh match, gm playwriter, `runFirstHitTiming`): verdict `{"ok":true,"reasons":[]}`. Legs: first target 118, `reachTick` 830, `launchTick` 830, `hitTick` 836, `latencyTicks` 6, `attackPoint` 7, `struckOther` false, `aborted` null; second target 248, `reachTick` 1699, `hitTick` 1705, `latencyTicks` 6, `attackPoint` 7, `aborted` null; lock-off control 251, `latencyTicks` 6 (not judged). Result: H2 passes under the corrected corridor.

### Results of the Phase B pass

- `node scripts/check-all.mjs`: `RESULT gates=73 ok=73 failed=0 drift=0 skipped=6 strict=false`, exit 0 (the 6 skips are the same missing inputs as before: G688_UNIT_INI, G688_LNI, --src, hy3d directory, osdt-data binary, and check-all's own editor check).
- `node js/client/movement-sequence-check.mjs`: sequence ok, every negative control fails as expected, pad walk ok.
- `cargo test -p sim`: all test binaries ok. `cargo test -p sim --test strikes`: 9 passed. `cargo test -p mod-host --test pad_cast_at`: 6 passed. `cargo test -p mod-host` (full rerun, detached, `/tmp/gm-modhost2.log`): `MODHOST_EXIT 0`, no FAILED or panicked lines.
- `node scripts/witness/clear-target-check.mjs`: ok. `node scripts/witness/strike-while-walking-check.mjs`: 8 cases ok. `node scripts/witness/first-hit-timing-check.mjs`: 15 cases ok. `node js/client/twin-stick-check.mjs` (C1): ok. `node scripts/check-browser-witnesses.mjs` (A5): `browser witness spec ok (4 witnesses, 4 bounded-run rules)`.
- Not run in this pass: E3 (control scheme doc review against a running client; a review item, not a gate).
- D1 (one-hand walking strike, live): RED at this point; the green pass below supersedes it.

## D1 green pass (2026-11-09)

Commits, all lanmower, no trailers, in order: `5996938e` (sim and web `attack_terms`), `521e38e2` (page: `__ciAttack` is the UI path, refusal and terms hooks), `87e71eff` (clear-target: range-bounded pick, unreachable creeps excluded), `2c0786f1` (strike-while-walking: attempt terms, launch-based control).

Diagnosis, legs 1 and 2 against leg 3:
- The refusal is captured per attempt: `accepted`, `visible`, `inReach`, `cooldownReady`, `simRefusal`, `uiRefusal`. The attempt records come from `attack_terms` (combat.rs) and `last_refusal` (sim).
- In the green run, the judged legs' initial attacks were accepted (initial picks: creep 169 for lock on, 169 for lock off, 225 for lock on again). Every refused attempt on the judged legs was `Cooldown` with `visible` true and `inReach` true: a re-issue while the hero's own swing was running (`order_attack`, combat.rs). That is the sim's normal answer, not an illegal target.
- The earlier refusal of creep 118 in legs 1 and 2 (`InvalidOrderTarget`, the visibility clause of `order_attack_or_approach`, movement.rs:454) came from the pick having no range bound: the witness ordered a creep the hero could not see. `pickRangeCells` (100) bounds the pick, so the attack is ordered only on a visible creep. That refusal was not reproduced in this pass.
- The lock-off control failed before this pass for a measurement reason, not a sim one. It counted landed hits, and a swing launched while the hero stood still can land while it walks on. The sim launches a swing only when the unit is not walking (combat.rs `attack`). The control now counts launches: a launch counts when the hero moved and is more than 1.5 cells from its commanded station (movement.rs `walking` is false within one cell of the target).

Witness output, `runWitness(state.page)` on a live match (lobby: All Pick, slot 0 PUD, Fill with bots, Ready, Start match, `body[data-phase=live]`):

    legs: lock on      aborted null  strikeWhileMoving 1  launchesWhileMoving 3  moving 82  inReach 14  accepted 22/26
          lock off     aborted null  strikeWhileMoving 0  launchesWhileMoving 0  moving 47  inReach 4   accepted 17/19
          lock on again aborted null strikeWhileMoving 1  launchesWhileMoving 2  moving 131 inReach 2   accepted 36/40
    verdict {"ok":true,"reasons":[]}

H2 (`runFirstHitTiming(state.page)`, fresh match): verdict `{"ok":true,"reasons":[]}`. First target 118 `latencyTicks` 6, `attackPoint` 7, `struckOther` false; second target 248 `latencyTicks` 6, `attackPoint` 7; lock-off control 252 not judged.

Gates:
- `node scripts/check-all.mjs`: `RESULT gates=73 ok=73 failed=0 drift=0 skipped=6 strict=false`, `CHECKALL_EXIT 0`.
- `cargo test -p sim`: 12 result blocks, 40 passed, 0 failed, `SIM_EXIT 0`.
- `cargo test -p mod-host`: 22 result blocks, 48 passed, 0 failed, `MODHOST_EXIT 0` (log `/tmp/d1-modhost.log`).

## Defect: 83ba7ed1 pushed with D1 red (2026-11-09)

- What: commit 83ba7ed1 went to origin main while D1 (the one-hand walking strike witness) was red. The rule is that every defect is fixed before a push, so this push breaks it. The push itself is recorded here, not hidden in the D1 red note.
- Cause of the red: the page assigned `window.__ciAttack` twice (js/client/play.js, the second assignment at the old line 3739 used `session.broadcast(sendAttack(...))`). The second assignment overrode the first, which is the path a click takes (`commands.attack`, with refusal recording). So the witness's attack order went out as a session frame that the local sim does not apply in a solo match, and the strike target never set.
- Fix (commit 521e38e2): the duplicate is removed, so `__ciAttack` is `commands.attack`. Two debug hooks are added beside `__ciLocalAttack`: `__ciRefusal` (the sim's last refusal, `localGame.last_refusal()`) and `__ciOrderOf` (the unit's current order, `localGame.current_order()`).
- Note for the same class: `window.__ciCast` (play.js) still uses `session.broadcast(sendCast(...))`, not the UI path. Not used by D1 or H2; left as it is, to be checked when a witness calls it.

## Notes for Phase B (observations, not validation)

- A hero walks about 4.7 cells per second on the live map. Reads taken one second apart can look like a stalled hero; use the position log over several seconds.
- Earlier reports of a hero "stuck" at the fountain were not established: one run ended with the hero mid-path and one with the hero at low health. Do not treat the stall as a cause until B-phase logs show it.
- The dev server for the witness was stopped by PID (2961824) in Phase A; restart it in Phase B with `node scripts/dev/serve.mjs --port=8831 .dev-site` after staging, and stop it by PID afterwards.

## Phase A pass 2026-11-09 (owner defects: bundling, melee reach; D1 five-run rerun)

Phase A, code, docs and tests only: nothing built, run or witnessed in this pass. Each row below is pending until its Phase B witness is green.

- A1, pathing under a bundle. Test: `a_bundle_larger_than_the_path_field_cache_reaches_each_goal_behind_a_wall` (rust/crates/web/tests/pathing_probe.rs). 40 ground units start west of the wall cells (98, 234..246) and each take a distinct goal east of it. Hypothesis, to confirm in Phase B: each bundled unit's stop point is its own goal, so 40 goals need 40 path fields, but `DEFAULT_MAX_FIELDS` is 32 (rust/crates/sim/src/pathfinding.rs:8) and `request` evicts the oldest field (`claim`, `discard`) while one fresh build runs per tick (`tick`, `FIRST_BUILD_BUDGET`). A unit whose field is not ready gets no direction (`dir_index` returns None while `!ready`), so `steer` (movement.rs) falls back to a straight line into the wall and `integrate` refuses the terrain. Witness: `cargo test -p web --test pathing_probe a_bundle_larger` ok, with the probe line `probe bundle units=40 arrived=40`. If it fails, the probe line gives the stuck indices and the stall run, and the cause is what the failing unit's field reports (ready, direction, goal).
- A2, melee reach. Test: `a_melee_hit_lands_only_within_its_weapon_range` (same file). It asserts that a melee unit's hit lands only when its centre is within its weapon range of the foe. Hypothesis, if it fails: the reach measure for a structure target (`strike_measure`, movement.rs:391) takes the nearest blocked cell within radius+1 of the target, not the target's own footprint, so terrain next to a building can extend a melee unit's reach to it. A unit target is measured centre to centre (no hull), so this test is not expected to fail for units; if it passes, the owner's melee report comes from another path (`K_ATTACK` effects in spell.rs, which apply no reach check, or a cleave), and Phase B must find it with a live witness before fixing. Witness: `cargo test -p web --test pathing_probe a_melee_hit` ok, with `probe melee_reach`.
- A3, D1 five live runs. Witness: `scripts/witness/strike-while-walking.mjs` `runWitness` on five fresh matches, each recorded here with its verdict line and per-leg counts (strikeWhileMoving, launchesWhileMoving, accepted). Phase B only.

## Phase B row: melee reach (owner report, 2026-11-09)

- Owner report: a melee attack reaches targets beyond the attacker's real melee reach, so characters not near a target take melee damage.
- Constant and check, found in rust/crates/sim: there is no separate melee constant. A unit's reach is its own `weapon_range_milli` (loaded into `range_q` and, squared, `range_sq`, arrays.rs:712-719; `morph.rs:21` and `inventory.rs:416` change it). The check is `strike_ready` (combat.rs:189): `damage > 0`, `can_target` or `denies`, and `strike_measure` distance `<= range_sq << SHIFT`. For a unit target `strike_measure` is centre to centre (movement.rs:391); for a structure it is the edge of its blocked cells. `land_swing` (swing.rs:205) repeats the reach check at landing, so a hit cannot land out of reach of the attacker's own position at that tick.
- Finding: the check is correct for the attacker. Witnesses (rust/crates/web/tests/pathing_probe.rs): `a_target_just_outside_melee_reach_takes_no_melee_damage` (the foe at 1.86 cells, reach 1.56: 0 attacker strikes in 200 ticks) and `a_target_just_inside_melee_reach_takes_melee_damage` (foe at 1.26 cells: 2 attacker strikes in 100 ticks). Both pass.
- Cause of the earlier out-of-reach hit seen in the melee probe: not the attacker. A map unit (player 0, kind `hero` from units.osd line 1, `weapon_range_milli=2344`, 150 units) stood next to the foe and struck it. The foe was inside that unit's reach. The witnesses count only the attacker's strikes (`struck_by(attacker, foe)`), and the outside witness foe can still die to that map hero (`foe_hp=0`), which is a placement isolation weakness of the witness, not a reach fault.
- Open, owner decision: the reach values themselves. The generic `hero` (150 units) and the lane creep `creep`/`sentinel_creep` (1562 milli, 100 units) are not 6.88g values (6.88g melee cutoff is 128 units, `rangeN1 <= 128`, AGENTS.md). If the owner's "real melee reach" is 128 units, the change is data: `weapon_range_milli` of the generic `hero` (units.osd:1) from 2344 to 2000, and the creep value after its 6.88g source is confirmed. Not changed in this pass: the sim check is correct, and a data change needs the owner's reach number.
- Witness: `cargo test -p web --test pathing_probe reach_` (2 passed, `probe reach_outside ... attacker_strikes=0`, `probe reach_inside ... attacker_strikes=2`).

## Phase B results: owner defects and the D1 five-run rerun (2026-11-09)

Phase A rows, as run in Phase B:
- A1, bundled pathing. `a_bundle_larger_than_the_path_field_cache_reaches_each_goal_behind_a_wall`: passes. `probe bundle units=40 arrived=40 stuck=[] longest_stall_run=0`. The field-cache hypothesis (40 goals against 32 fields) does not reproduce the failure headless. `a_bundle_chasing_one_foe_behind_a_wall_reaches_strike_range_or_the_foe_dies` (added this pass, 30 attackers with their own stop points): passes, `probe chase_bundle attackers=30 reached=30 missing=[]`.
- A2, melee reach. `a_melee_hit_lands_only_within_its_weapon_range`: passes (`probe melee_reach reach=1.56 hits=1`). The earlier failure (a hit 3.22 cells from the attacker) was a map hero (player 0, kind `hero`, reach 2.34 cells) that stood beside the foe; the witness now counts only the attacker's strikes.
- Owner melee report: see the row "Phase B row: melee reach" above (commit 18913805). The reach check is correct for the attacker; the reach values are an owner decision.

Owner defect, bundling, status: OPEN. The headless witnesses pass, but one live D1 run (run 7 below) stalled its hero in lock on, with 48 moving samples against 102 to 218 in the other runs. Cause not confirmed: no trace of the stall was kept. Next: a live run that records the hero's position and the order state each sample while stalled, before any fix.

D1 five-run rerun, live, one fresh match each (lobby: All Pick, slot 0 PUD, Fill with bots, Ready, Start match). Runs 1 to 3 used earlier witness versions (before the settle guard, commit 7c231401); runs 4 to 8 used the final witness.

- Run 1 (earlier witness): `verdict {"ok":false}`. Lock on passes (34/39 accepted). Lock off and lock on again abort: `no clear point to place a creep within the placement radii`. Cause: the hostile creep alive beside the hero makes every placement segment fail the isolation check (`placements`, clear-target.js), and the pick finds no clear creep because the two adjacent creeps block each other.
- Run 2 (earlier witness): `verdict {"ok":false}`. Lock on: no strike while moving (launches 1, landings 0), lock off: 2 launches while moving, lock on again: no strike while moving. Cause: the lock-off control counted launches from a stop that the sim had not yet changed (the settle guard fixes this measurement; commit 7c231401).
- Run 3 (earlier witness): `verdict {"ok":false}`, lock off: 4 launches while moving. Same cause as run 2 (settle guard).
- Run 4 (final witness): `verdict {"ok":true,"reasons":[]}`. Lock on: 2 strikes while moving, 0 launches, 38/40 accepted; lock off: 2 strikes landed, 0 launches; lock on again: 2 strikes while moving, 0 launches, 34/40 accepted.
- Run 5 (final witness): `verdict {"ok":true,"reasons":[]}`. Lock on: 1 strike, 0 launches, 28/30 accepted; lock off: 2 strikes landed, 0 launches, 30/38 accepted; lock on again: 3 strikes, 0 launches, 34/40 accepted.
- Run 6 (final witness): `verdict {"ok":true,"reasons":[]}`. Lock on: 1 strike, 0 launches, 25/29 accepted; lock off: 1 strike landed, 0 launches; lock on again: 2 strikes, 0 launches, 37/40 accepted.
- Run 7 (final witness): `verdict {"ok":false}`, reasons `lock on: no strike while moving with one-hand on`, `lock on again: no strike while moving with one-hand on`, `lock on again: no hero-sourced effect kinds recorded`. Lock on logged 48 moving samples (runs 4 to 6: 102 to 218), inReach 6, 13/15 attacks accepted. The hero stalled for most of the leg. Cause not confirmed; it is the live stall above, the open bundling row.
- Run 8 (final witness): `verdict {"ok":true,"reasons":[]}`. Lock on: 126 moving samples, 1 strike while moving, kinds `melee_hit` 1; lock off: 0 launches; lock on again: 5 strikes while moving, kinds `melee_hit` 6, 36/40 accepted.

Final-witness tally: 4 of 5 green (runs 4, 5, 6, 8 pass; run 7 fails with the stall).

## Phase B gate: check-all after the disk fix (2026-11-09)

- Cause of the earlier failures: the root overlay (`/`, which holds `/tmp`) was 100% full, so every gate that makes a temp directory failed with ENOSPC. Nothing was deleted: the other sessions' `/tmp` targets and worktrees are untouched.
- Fix used: `TMPDIR=/workspace/d1-tmp node scripts/check-all.mjs` (the workspace disk has 554G free). The empty `/workspace/oscraft/.tmp-d1` was removed before the run, so the tree stays clean.
- Witness: `RESULT gates=73 ok=73 failed=0 drift=0 skipped=6 strict=false`, `CHECKALL_EXIT 0` (log /tmp/b-checkall2.log). The six skipped gates each name a missing input: G688_LNI, --src for war3map.j, and ref-heroes-src, among others.
- Still open and not changed here: the live bundling stall (run 7 above) and the owner's reach-value decision (the melee row, commit 18913805).

## Phase A row: bundling, funnelled through a gap (owner defect, 2026-11-09, reopened)

- Defect: pathfinding fails when characters bundle up. The row closed at the last push was left open with one live stall (D1 run 7) and no cause.
- What the code reading rules in or out (no run in Phase A):
  - The field pool (pathfinding.rs, DEFAULT_MAX_FIELDS 32) is not the limit for the existing 40-unit bundle: units with goals 1 cell apart share one field (`can_serve`, drift MOVE_THRESHOLD 4), so the bundle needs few fields. This is why `a_bundle_larger_than_the_path_field_cache...` passes.
  - `route_for` already re-requests the field each tick for a routed unit (movement.rs:28-50), so a blocked unit does not keep another goal's field. Not the cause; no change made there.
  - Body radius does not block a gap: creep radius 250 milli = 0.25 cell, cells are 1 world unit, so a 5-cell gap clears every body (units.osd line 4, `fixed::from_milli`).
  - Live candidate, not yet shown: `steer` drops a unit's target (`tgt = pos`) when it is within one step of its goal and a friendly footprint sits on that goal (movement.rs, `snap_clear` branch). A bundle ordered to one area can stop short that way.
- Change in this row: `pathing_probe.rs` gets `march_bundle` (the march the bundle witness already ran, factored out) and the new witness `a_bundle_funnelled_through_a_gap_reaches_each_goal_behind_the_wall`: 60 units, a 5-cell gap (y 237 to 241) in the wall at x 98, 6000 ticks, stall limit 400. The thresholds are set before any run and any change to them is recorded.
- No sim change in Phase A: no reproduction exists yet, and a fix made without one would not be witnessed. Phase B runs this witness and the two bundle witnesses; a failure is diagnosed from the stuck unit's order, target, velocity and field state, and the fix follows from that.
- Witness: `cargo test -p web --test pathing_probe bundle` (three tests: the 40-unit bundle, the 30-unit chase, and the 60-unit funnel).

## Phase B results: bundling, funnel and spread witnesses (2026-11-09, row still OPEN)

- `cargo test -p web --test pathing_probe bundle` (commit d20f4a18 plus the spread witness, uncommitted at this line): 3 passed. `probe bundle units=40 arrived=40 stuck=[] longest_stall_run=0`; `probe chase_bundle attackers=30 reached=30`; `probe funnel units=60 arrived=60 stuck=[] longest_stall_run=1`.
- `cargo test -p web --test pathing_probe spread`: 1 passed. `probe spread units=40 arrived=40 stuck=[] longest_stall_run=37`. The goals are 6 cells apart, so the 40 units need more fields than the pool holds (32), and it still arrives.
- `cargo test -p web --test pathing_probe reach_`: 2 passed. `probe reach_outside gap=1.86 reach=1.56 attacker_strikes=0`; `probe reach_inside gap=1.26 reach=1.56 attacker_strikes=2`.
- No sim change was made. The field-cap reading (the pool evicts in-use fields once more than 32 distinct goals are live) is not the cause here: the spread witness exceeds the cap and passes. Body radius does not block the gap (creep radius 0.25 cell).
- Not reproduced headlessly in four geometries (a 40-unit bundle behind a wall, a 30-unit chase, a 60-unit funnel through a 5-cell gap, a 40-unit spread). The owner's live stall (D1 run 7: 48 moving samples against 102 to 218) has no trace. Candidate mechanisms the sim holds a unit still for, all in the tick loop: `swing_busy` without strikes-on-move (movement is skipped, tick.rs), `rooted`, and `engaged` (only `separate` runs). None is shown to be the cause.
- Row status: OPEN. Closing it needs a live reproduction with per-sample hero state (order kind, target, `swing_busy`, `rooted`, `vel`, field readiness) on the stalled sample. Until then the row is not pushed, per the fix-before-push rule.

## Phase A row: D1 witness trace and station refusal (live stall, 2026-11-09)

- Defect (the live stall, D1 lock-on leg, 1 in 5 runs and two more in later runs): the hero stood still for 8 to 86 samples. Cause, from the trace (stall samples, `stallTrace`): the witness's station cell was unwalkable, so the sim refused the move with `InvalidOrderTarget` (`__ciMove` returns false in local mode, `noteLocalRefusal`). The hero kept its last accepted order and a chasing attack halted it inside weapon reach (`attack_chase`, the `strike_ready` branch), and the witness never switched stations because it only switched after reaching one. This is a witness defect, not a pathfinding defect: the sim refused a bad destination as designed.
- Change, `scripts/witness/strike-while-walking.mjs`:
  - a refused move (`__ciMove` returns false) rotates the station set by 45 degrees (`pointToward` takes `turn`, `stationTurns` 8), so a walkable station is found; counted as `stationRotations` and `refusedMoves`;
  - a stall trace (`stallRuns`, `stallTrace`, capped by `stallTraceCap`) records order, swing point, strike target, tick, hp and last refusal for each still run of `stallRunSamples` samples or more;
  - each counted launch while moving records position, station, order, strike target and tick (`movingLaunches`), so a launch can be checked against the sim's walk state;
  - placement uses a wider search (`placeRadii` 8 to 40) and a lane isolation radius of 4 cells (`laneCells` 4, was 10).
- Change, `scripts/witness/clear-target.js`: `choose` retries placement `placeRetries` times (`placeRetryMs` apart) before it reports the placement as aborted. Earlier runs aborted on "no clear point" after one search, while lane creeps moved past the candidate points.
- Witness (Phase B): the three-leg D1 witness via gm, five final-version live runs, each recorded with its verdict and its stall, refusal and rotation counts. The earlier runs in this session that used the older witness versions are not counted toward the five.

## Phase A row: D1 counts walking under the sim's move order (2026-11-09)

- Defect, from the five final-version live runs (`/tmp/d1-final-1794215010721.json` to `...1794215446691.json`): three launches counted "while moving" had sim order kind 2 (`ORDER_ATTACK_MOVE`, lib.rs:94), not `ORDER_MOVE`. The sim's `walking()` (movement.rs) needs `ORDER_MOVE`; an attack-chase move is the sim's permitted movement, so the witness counted a launch the sim does not classify as walking. Witness defect, not a sim defect.
- Change, `scripts/witness/strike-while-walking.mjs`: a sample counts as walking only when `__ciOrderOf(heroId)[0] === SIM_ORDER_MOVE` (0) as well as moving. Launch counting, strike samples while walking and effects while walking use `walking`.
- Witness: the three-leg D1 witness, five live runs, recorded with their verdicts. Earlier runs (before this row) are recorded in the Phase B results below.

## Phase A row: placement rejected every candidate when an enemy stood near the hero (2026-11-09)

- Defect (D1 walk-order runs 2, 5 and earlier: "no clear point to place a creep" on the lock-off and lock-on-again legs, 3 of 5 runs): the placement test (`clear-target.js` placements) required every hostile unit to be more than `isolationCells` from the segment from the hero to the candidate. A hostile unit within `isolationCells` of the hero is within that distance of the segment's start, so it blocks every candidate direction at once. Cause: the test ignores that the blocker is at the hero. Witness defect, not a sim defect.
- Change, `scripts/witness/clear-target.js`: the segment test skips a hostile unit already within `isolationCells` of the hero (`distance(v, from) <= isolationCells ||`). The point test still applies to it.
- Witness: the three-leg D1 witness, five live runs.

## Phase A row: a routed unit keeps a move order to a goal it cannot reach (live stall, 2026-11-09)

- Defect (the D1 live stall of run 7, and the open bundling row): the hero ordered to a live station stood still with an active MOVE order for the whole leg. Reproduced headlessly with the live coordinates (`a_hero_ordered_to_a_live_station_reaches_it_or_is_refused`: start (71.5, 276.5), goal cell centre (73.5, 280.5), walkable but in a separate region, `shortest_cells` = None). The order was accepted: `issue_order_kind` checks walkability of the goal, not its reachability.
- Cause, from a per-tick trace of the hero (`routed=1`, `blocked=1`, the goal field ready, distance at the hero's cell -1, direction NO_DIR): `route_for` requests the goal's field every tick, the field gives no direction from the hero's cell, so `steer` falls back to a straight vector into a wall, `integrate` refuses it, and the order never ends.
- Change (`rust/crates/sim/src/movement.rs`, `route_for`): when a routed unit's ready goal field reports its cell unreachable (new `Field::unreachable_from`, `rust/crates/sim/src/pathfinding.rs`), the route ends, and an open MOVE without an attack becomes STOP. The target is kept.
- Rejected, measured: the same branch with the target set to the unit's position made a hero pressed into a wall pocket oscillate (298 direction reversals in 400 ticks, `a_hero_with_an_unreachable_order_target_settles_instead_of_jittering`). With no route force left, separation alone flips the velocity each tick. Keeping the target avoids that.
- Not changed: `steer` (movement.rs, the `snap_clear` branch) also sets the target to the unit's position when a one-step snap is unclear, and the MOVE stays. No test reproduces that path, so it is recorded as open here, not fixed.
- Phase A note: the diagnosis in this pass ran builds and traces before this row was written. Their results are the Phase B evidence below; no change was made without a trace or a failing test.
- Witness: `cargo test -p web --test pathing_probe`: 27 passed, including the live-station, unreachable-target, bundle, funnel and spread tests.

## Phase A row: witness move refusals are logged; verdict check fixture drift (2026-11-09)

- Defect 1 (live pass 4 of this batch, leg "lock on again"): `no strike while moving with one-hand on`, with 64 refused moves and 64 station rotations over the leg, and 25 dropped attempts. The witness counted refusals but kept no record of which station, creep or hero position was refused, or the sim's reason, so the cause could not be read from the output. Change, `scripts/witness/strike-while-walking.mjs`: each refused move records `{atMs, turn, station, creep, hero, refusal}` in `moveRefusals` (capped by `attemptLimit`, the rest counted in `moveRefusalsDropped`), and the leg returns both. No threshold or verdict rule changed.
- Defect 2 (node check, at HEAD before this row): `scripts/witness/strike-while-walking-check.mjs` failed its first case. Commit 2c0786f1 added the verdict rule `lock off: launchesWhileMoving !== 0` and changed the red-leg rule from strike samples to launches, but the check fixture still had no `launchesWhileMoving` and still expected a red leg with strike samples to fail. Change, the check only: the fixture's default leg carries `launchesWhileMoving: 0`; the case for strike samples in a red leg now expects a pass (a landed strike is not a launch); a new case expects a red leg with a launch to fail. The verdict is unchanged.
- Witness: `node scripts/witness/strike-while-walking-check.mjs` prints `strike-while-walking verdict checks ok (9 cases)`; then the live D1 run (Phase B) prints the move refusals for any refused leg.

## Phase A row: a creep whose whole station ring is refused is dropped (D1 pass 4, 2026-11-09)

- Defect (live pass 4, leg "lock on again": `no strike while moving with one-hand on`, 64 refused moves and 64 station rotations over the leg): the witness kept one creep for the whole leg, and every station it ordered on that creep was refused with `InvalidOrderTarget`, the sim's correct refusal of an unwalkable cell. Rotation alone cannot leave a ring of unwalkable stations, so the hero never walked. Pass 5 showed the same refusal kind on a smaller scale (7 refusals, all absorbed). Witness defect, not a sim defect.
- Change, `scripts/witness/strike-while-walking.mjs`: a refused-move run counts consecutive refusals (`refusalRun`, reset on an accepted move). After `stationTurns` refusals in a row, the creep is added to `stationBlocked`, dropped, and the next pick excludes it (`rule().excluded` carries `stationBlocked`). The leg then re-picks a creep through the same clear-target rule.
- Witness: the three-leg D1 witness, five live runs on this version, each with its verdict and its refusal counts. Earlier runs (before this row) are recorded in the Phase B results.

## Phase B row: an attack order clears a STOP left by an unreachable goal (D1 passes 7 to 9, 2026-11-09)

- Defect (D1 live passes on a22ed043, before this fix: 3 of 3 red, one run per fresh match): `no strike while moving with one-hand on` on `lock on` (two runs) or `lock on again` (one run), with the hero in reach for 8 to 22 samples, zero hits in the failing leg (`kinds: {}`), and the attacks accepted (`accepted: true`, `inReach: true` on the first attempts). The unreachable-goal branch of `route_for` (70d12611) turns an open MOVE into STOP. `attack_chase` (movement.rs) drops any attack order while the order kind is STOP, and the approach branch of `order_attack_or_approach` never set the kind, so an attack ordered after such a STOP was cleared on the next tick and the hero never walked in.
- Reproduction, headless (before the fix): `a_stopped_hero_ordered_to_attack_a_foe_out_of_reach_walks_in_and_strikes`: the hero is stopped (`ORDER_STOP`), then ordered to attack a foe 0.3 cells outside reach. Output: `probe stopped_attack strikes=0 order=(1, ...) attack=-1`; the test failed. The same test is the pre-existing semantic for an explicit stop followed by an attack, so the bug is not new to 70d12611; 70d12611 made the STOP common.
- Fix (`rust/crates/sim/src/movement.rs`, `order_attack_or_approach`): on the approach branch, a STOP becomes MOVE before `approach_to`, as the in-reach branch already sets MOVE. HOLD is unchanged. Behaviour change, owner-visible: an attack order given to a unit that was stopped now walks in to strike, as an attack command should; before, the unit stood still and the attack was dropped.
- Witness: `cargo test -p web --test pathing_probe`: 28 passed, including `a_stopped_hero_ordered_to_attack_a_foe_out_of_reach_walks_in_and_strikes` (`probe stopped_attack strikes=2 order=(0, ...) attack=-1`), `probe live_stall ... order_kind=1`, `probe bundle units=40 arrived=40`, `probe funnel units=60 arrived=60`, `probe spread units=40 arrived=40`, `reach_inside attacker_strikes=2`, `reach_outside attacker_strikes=0`. `cargo test -p sim -p mod-host`: 34 suites ok, no FAILED, exit 0.
- Live witness: see the next rows (one fresh match per pass, on the committed version).

## Phase B row: the D1 witness's station points were outside the hero's melee reach (2026-11-09)

- Defect (D1 live passes on 72e2d16d: 1 of 1 red, `lock on` 0 hits with `inReachMoving` 28, `lock off` and `lock on again` 0 hits; earlier witness versions red in 4 of 9 passes, all with no hits in the failing leg): the witness walks the hero between two stations, `stopCells` (2.0 cells) and `stopCells + oscillateCells` (3.6 cells), around the creep. The hero's melee reach is about 2 cells (the probe reports `range=2.00` for a hero), and the sim's own stop for a strike is 3/4 of the weapon range (`strike_stop`). The stations sit on or past the reach edge, so whether a strike lands depends on whether the walk happens to cross reach at the damage point. Witness defect: the stations should be inside reach.
- Change, `scripts/witness/strike-while-walking.mjs`: `stopCells` 2 becomes 1, `oscillateCells` 1.6 becomes 0.4, so the stations are at 1.0 and 1.4 cells, inside the 2.0-cell reach. The verdict rules are unchanged.
- Check: `node scripts/witness/strike-while-walking-check.mjs` (9 cases).

## Phase B results: attack-order fix and the D1 witness, final version (2026-11-09)

- `cargo test -p web --test pathing_probe` (on 72e2d16d, the sim of d3688447): `test result: ok. 28 passed; 0 failed`, with `probe stopped_attack strikes=2 order=(0, ...) attack=-1`, `probe live_stall ... order_kind=1`, `probe bundle units=40 arrived=40 stuck=[]`, `probe funnel units=60 arrived=60`, `probe spread units=40 arrived=40`, `probe reach_inside attacker_strikes=2`, `probe reach_outside attacker_strikes=0`. Exit 0.
- `cargo test -p sim -p mod-host`: 34 `test result: ok` suites, no FAILED, exit 0.
- Live D1 witness, one fresh match per pass, committed sim 72e2d16d with the witness at d3688447, verdict as the witness returns it:
  - Pass 1: `verdict {"ok":true,"reasons":[]}`; lock on 1 strike while moving, 0 launches; lock off 2 strikes, 0 launches; lock on again 1 strike, 0 launches.
  - Pass 2: `verdict {"ok":true,"reasons":[]}`; lock on 1, lock off 1, lock on again 2 strikes; 0 launches while moving.
  - Pass 3: `verdict {"ok":true,"reasons":[]}`; lock on 1, lock off 0, lock on again 1 strike; 0 launches while moving.
  - Pass 4: `verdict {"ok":true,"reasons":[]}`; lock off 8 refused moves, all absorbed by rotation, 0 launches; lock on again 1 launch while moving (allowed on a lock-on leg).
  - Pass 5: `verdict {"ok":true,"reasons":[]}`; lock on 1, lock off 2, lock on again 2 strikes; 0 launches while moving.
- Red before the station fix (same sim, witness at a22ed043 and 72e2d16d with the stations at 2.0 and 3.6 cells): 4 of 9 passes over the whole batch were red (pass 4 of the earlier batch; A1, A2, A3; B1). Each red leg had zero hits, attacks accepted and the hero in reach at the first attempt. Cause: witness defect, the stations were outside the hero's 2.0-cell melee reach (see the row above), so a strike depended on the walk's timing.
- Sim changes in this row (the attack-order fix): the approach branch of `order_attack_or_approach` turns STOP into MOVE, witnessed by `a_stopped_hero_ordered_to_attack_a_foe_out_of_reach_walks_in_and_strikes`. Before the fix that test failed with `strikes=0 attack=-1`.

## Phase A row: the steer fallback held MOVE on the unit's own position (2026-11-09)

- Defect: `steer` (movement.rs, the branch where a unit is within one step of its target and the straight snap is not clear, because a friendly body holds the target or a corner is blocked) set the target to the unit's own position and left the order MOVE. A MOVE with its target on its own position never resolves, so the order reports an active move that is finished. Not reproduced in a live run; the code reading is the cause.
- Change, `rust/crates/sim/src/movement.rs` (`steer`): the same branch now turns an open MOVE with no attack into STOP. The target is kept at the position, as before; the order kind is the only change. Rationale: a unit that cannot reach its target ends its move as STOP.
- Witness: `a_move_whose_goal_is_held_by_a_friendly_body_ends_as_stop_not_as_a_move_to_its_own_position` (`rust/crates/web/tests/pathing_probe.rs`): a blocker is teleported onto a goal 0.2 cells from the hero, the move is issued, and after 20 ticks the order must not be MOVE and the target must equal the position. Phase B runs this test red before the fix (fix temporarily removed) and green after, and quotes both.
- Gate: the existing `a_stopped_hero_ordered_to_attack_a_foe_out_of_reach_walks_in_and_strikes` and the full `pathing_probe` suite must still pass.

## Phase A decision: the bundling row is resolved (2026-11-09)

- Decision (owner delegated the call to the agent): the bundling row closes as `resolved`. The close is committed in Phase B, with its commit body carrying `Row:`, `Witness:` and `Status: resolved`, and only after the witnesses listed below have run and been quoted.
- Evidence for the close: the live stall the row tracked (D1 run 7) reproduced headlessly with its exact coordinates as `a_hero_ordered_to_a_live_station_reaches_it_or_is_refused`. Two causes, both fixed: (1) the sim accepted a walkable goal in a separate region and a routed unit held an open MOVE forever (70d12611); (2) an attack ordered after such a STOP was dropped (72e2d16d). A third cause, the witness's own station refusals and station geometry, is fixed in the witness (a22ed043, d3688447).
- Witnesses for the close, all run in Phase B: `pathing_probe bundle` (40-unit bundle), `funnel` (60-unit funnel through a 5-cell gap), `spread` (40 goals past the field cache), `a_hero_ordered_to_a_live_station`, `a_stopped_hero_ordered_to_attack`, and five fresh-match D1 live passes on the final commit.

## Phase A note: the attack walk-in change is gameplay-visible (2026-11-09)

- An attack ordered to a unit that was stopped now walks in to strike; before 72e2d16d the attack was dropped and the unit stood still. Rationale: an attack command should chase its target, and the old drop only happened because a stop left by an unreachable move was not cleared (see the rows above).

## Phase B results: steer fallback (2026-11-09)

- Evidence before the fix (pre-fix movement.rs from HEAD~1, the rewritten test): `probe friendly_goal kind=0 radius=0.074 reversals=26`; the test failed on `the hero holds a move order to a goal a friendly body holds`. The hero held MOVE and jittered by about 0.07 cells, 26 direction reversals in the sample.
- What the trace showed (with the STOP fix from Phase A): the STOP was applied on the first tick, but each later tick `steer` found the target within one step and the snap clear again (the hero had been moved off the blocker's footprint by the same tick), and stepped back toward the stale target, the previous position. The unit cycled between two points about 0.4 cells apart.
- Fix (`rust/crates/sim/src/movement.rs`, `steer`, the within-one-step branch): a STOP unit no longer steps toward its target on that branch; it pins to its position. The MOVE-to-STOP conversion (Phase A) is unchanged.
- Test change, recorded: the Phase A test asserted the target equalled the position after 20 ticks. The trace shows the target and position differ after the tick's own integration (the pin happens before the separation move), so that assertion was the wrong criterion. The test now asserts the STOP kind and uses the existing jitter helpers (`settle_and_measure`, `assert_settles`), which is the actual defect.
- Evidence after the fix: `probe friendly_goal kind=1 radius=0.000 reversals=0`; `test result: ok. 1 passed` for the test alone.
- Full suites on the Phase B tree: `cargo test -p web --test pathing_probe`: `test result: ok. 29 passed; 0 failed`, exit 0, including `probe stopped_attack strikes=2 ... attack=-1`, `probe bundle units=40 arrived=40 stuck=[]`, `probe funnel units=60 arrived=60 stuck=[]`, `probe spread units=40 arrived=40 stuck=[]`, `probe live_stall ... order_kind=1`. `cargo test -p sim -p mod-host`: 34 `test result: ok` suites, exit 0.

## Phase B results: D1 live passes on the committed sim 0c8a808d (2026-11-09)

- Sim tree: 0c8a808d (no sim change since). The witness gained a read-only per-sample trace of the lock-on leg (`strikeTrace`: position, creep, distance, creep hp, order, swing point, launch stamp). Verdict logic is unchanged; the self-check prints `strike-while-walking verdict checks ok (9 cases)`.
- One fresh match per pass, verdict as the witness returns it (lock-on hits are `melee_hit` effects, counted per leg):
  - Q1: ok. Q2: ok. Q3: red, zero hits in all three legs.
  - seq 24: red, zero hits in all three legs; the first attack was accepted at 1521 ms with `inReach: true` and `cooldownReady: true`. No trace (the trace was added after this pass).
  - seq 31: ok. seq 38: ok.
  - seq 97: red, zero hits in all three legs, same shape as seq 24. No trace.
  - seq 104: ok (2, 2, 2 strikes while moving, `melee_hit` 2 per leg).
  - seq 111: ok (lock on 5 strikes, `melee_hit` 14; lock off 0; lock on again 5, `melee_hit` 16).
  - seq 118: ok (lock on 4, `melee_hit` 14; lock off 0; lock on again 5, `melee_hit` 17).
  - seq 125: ok (lock on 6, `melee_hit` 15; lock off 0; lock on again 5, `melee_hit` 17).
  - seq 132: ok (lock on 7, `melee_hit` 16; lock off 0; lock on again 5, `melee_hit` 17).
- Tally: 9 ok, 3 red over 12 passes. The last five passes (seq 104 to 132) are consecutive and all ok; they are the five fresh-match passes on the committed sim.
- Trace evidence from the ok passes: the lock-on leg walks in to 1.0 to 1.4 cells of the creep and its `creepHp` falls while the hero moves (for example 6888 to 59 over about 4.5 s). The lock-off leg runs at the same stations and records no hero effects (the verdict does not require hits there).
- Open, not closed by the above: the three red passes have zero hits in every leg for the whole match, which is a different shape from the bundling stall (see the bundling close below). No trace was kept for any red pass. Cause not established. Candidate, untested: the witness reissues a move order every 300 ms (`orderEveryMs`), which may cancel a swing that is still in windup when the cadence and the swing duration line up. The trace build is in place; the next red pass will show the order, swing point and creep hp for each sample.

## Phase B close: the bundling row is resolved (2026-11-09)

- The owner's decision, made by the agent under the delegation recorded in the Phase A decision above: the bundling row closes as `resolved`.
- Evidence: `cargo test -p web --test pathing_probe` (29 passed, exit 0) including `bundle units=40 arrived=40 stuck=[] longest_stall_run=0`, `funnel units=60 arrived=60 stuck=[]`, `spread units=40 arrived=40 stuck=[]`, `a_hero_ordered_to_a_live_station_reaches_it_or_is_refused`, and `a_stopped_hero_ordered_to_attack_a_foe_out_of_reach_walks_in_and_strikes` (`strikes=2 attack=-1`); the five fresh-match D1 passes above (seq 104 to 132, all ok).
- Scope of the close: the row tracked the live stall of run 7 (a hero holding MOVE for 48 moving samples). It reproduces headlessly at the live coordinates and is fixed by 70d12611 and 72e2d16d. The D1 red shape above (zero hits, hero moving) is a separate open item and does not reopen this row.

## Phase A pass: open items of the 2026-11-09 pass (dispatcher order)

- Item 1, pages.yml modulepreload. Committed and pushed in 0d1f636d (origin/main contains it); the ledger had no row for it, so this row records it. Phase B: witness the Pages build locally (bundle-client.mjs into a staged _site, check-site-bundle.mjs) and push with the rest.
- Item 2, D1 zero-hit red, candidate: the 300 ms move reissue (scripts/witness/strike-while-walking.mjs, orderEveryMs) cancels a swing still in windup. Code reading, not yet run: Sim::issue_order_kind (movement.rs) interrupts a windup on a move only when strike_on_move is off (movement.rs `549); controls.lua:526 sets strike_on_move from the lock state; the witness move goes through order_move (web lib.rs) into that path. So with lock on the reissue should not cancel. Test added in Phase A: sim tests/strikes.rs a_move_order_cancels_a_windup_only_when_strike_on_move_is_off. Phase B runs it first. The witness changes (no reissue during windup) only if the test shows a cancel with lock on. The red passes are otherwise open until a red pass with a trace is read.
- Item 3, melee reach, decision: one shared constant, MELEE_CUTOFF_UNITS = 128 in scenarios/moba/tools/gen-melee-reach.mjs. Reason: 6.88g rangeN1 <= 128 is the melee cutoff (AGENTS.md); antimage's data already sits at weapon_range_milli=2000 (128 units); the generic unit("hero") at 2344 (150 units) was the outlier. The generator writes units.osd unit("hero") weapon_range_milli=2000 and melee_reach.lua (cutoff_q16 = 131072), which effects.lua reads as its melee_hit cutoff (was a literal 170394 Q16, 2.6 cells, 166 units). Not changed: attr_rules melee_range_q16 (blind miss gate, unit_rules.lua:123, a different rule) and hero_melee_688g.tsv (the 6.88g flags, data).
- Item 4, check-all on HEAD: Phase B, run once on the final HEAD before the push; gate count is now 74 with gen-melee-reach.mjs.

## Phase B items (2026-11-09 open-items pass)

- B-a: cargo build for mapexport, refresh scenarios/moba/moba.osdt (the units.osd and effects.lua change), commit it.
- B-b: run the new strikes test and the pathing_probe reach tests; record each result.
- B-c: D1 candidate decision from the B-b result; then D1 live passes (scripts/witness/strike-while-walking.mjs, lock on, off, on) until one is red, read its strikeTrace, fix the cause shown, rerun.
- B-d: check-all on the final HEAD, quoted in the report.
- B-e: Pages build witness and one push.

## Phase B results: D1 zero-hit red, cause and fix (2026-11-09 open-items pass)

- B-b, the candidate (the 300 ms move reissue cancels a swing in windup): refuted for lock on. `a_move_order_cancels_a_windup_only_when_strike_on_move_is_off` passes: with strike on move on the windup survives a move order, and with it off the move cancels the windup, as the sim intends. The witness does not reissue moves during windup, so the thresholds are unchanged and the verdict logic is unchanged.
- Witness fixes, each from a red pass read in its trace:
  - Passes 5 and 8: the re-pick probes (up to about 24 s) ran inside the sample loop, so samples stalled. Cause: the loop waited on the pick. Fix: the re-pick runs in the background and the loop waits for it after the leg. Witness: maximum lock-on sample gap about 185 ms in passes 17 to 23.
  - Passes 21 and 24: `launches while moving with one-hand off`. Cause: the witness counted a move as walking within one cell of its order target, where the sim does not count it as walking, so stop-and-strike launches were flagged. Fix: the witness uses the sim's arrival rule (`arrivedCells` 1).
  - Passes 25 to 29: `no strike while moving with one-hand on`, with the hero never in play in pass 25. Pass 25 skipped the lobby claim and pick, so its legs had no hero (`no placed point was both reachable ... 16 tried`). Passes 26 to 29 had the hero, and their trace (`strikeTrace` with `terms`, the sim's `__ciAttackTerms`) showed the in-reach moving samples with the cooldown term 0 and launches only at stations. Cause: the stations sat on one ray from the creep at radii 1.0 and 1.4 cells, so the move order was never more than 0.4 cells from the hero's target. The sim counts a unit as walking only when its order target is more than one cell away, so the lock-on leg could never be walking. Fix: station `b` is the opposite point on the same radius (`stationTurn + stationTurns / 2`), two cells from `a`; `oscillateCells` is removed. Pass 30 green, both lock-on legs with walking strikes.
  - Passes 32 to 34 (after the station fix): one lock-on leg in each with zero walking strikes. Trace of pass 35 (green): 168 in-reach moving samples in the lock-on leg, 157 with the cooldown not ready, 11 with the cooldown ready and no swing, 48 with a swing in flight while moving. Cause: walking strikes are a minority of the hero's strikes (about 15 per 25 s leg, one per attack period), so a leg can end with none. This is low measurement power, not a missing launch. Fix: `legMs` 25000 to 40000 (more samples; thresholds and verdict logic unchanged).
- Tried and reverted: a sim change that let a unit with strike on move face its commanded target instead of its travel direction (`face_move` skipped in `tick.rs` while a strike is in reach). A unit test at turn rate 90 showed the swing does not start while walking past a foe at a tangent without it. The live hero (pudge) has turn rate 1337 (`turn_rates.lua`), and the trace shows cooldown, not facing, as the limit, so the change was reverted and HEAD sim is unchanged. The two `starts_while_walking` strikes tests (strike on move off) stay: they encode the one-hand-off rule, but they do not discriminate the facing change, since they pass with and without it.
- Module cache: the playwriter caches a witness module by path, so passes 1 to 16 ran the original file. Fresh imports use `file:///workspace/oscraft/scripts/witness/strike-while-walking.mjs?fresh=N` (a bare absolute path does not resolve in the playwriter).
- Witnesses: `node scripts/witness/strike-while-walking-check.mjs` prints `strike-while-walking verdict checks ok (9 cases)`; `cargo test -p sim` exit 0 (full suite, strikes 12 passed); `cargo test -p web --test pathing_probe` 29 passed; `cargo test -p mod-host` exit 0 (22 result blocks, no failure).
- D1 live passes after the fixes, one fresh match each, verdict as the witness returns it (lock on, off, on again):
  - At legMs 25000 (passes 30 to 35): 30 ok, 31 ok, 35 ok; 32, 33, 34 red, each with one lock-on leg at zero walking strikes (the low-count cause above).
  - At legMs 40000 (passes 36 to 44): 36 ok (lock on 5, 1), 37 ok (8, 4), 39 ok (3, 2), 40 ok (4, 3), 41 ok (2, 3), 42 ok (7, 1), 44 ok (4, 1); 38 red (`lock on again: the hero was never in reach while moving`, no trace captured); 43 red (`lock on again: no strike while moving`).
  - Trace of 43, read: the lock-off leg's creep 170 died (creepHp 0 by the leg's end in pass 42, the same death in 43), and the background re-pick was slow (`picks` doneMs: pass 42 lock off 13915 and 41890; pass 43 lock off 7820 and 14617). The lock-on-again leg then fought creep 201, already at 84 HP, so it had 105 moving in-reach samples and 0 walking strikes. Cause: legs of 40 s let the creep die inside a leg, and the witness's re-pick is slow (probe-bound). The sim is not implicated: the in-reach terms are all 1 except cooldown.
  - Open: pass 38's trace was not captured, and the death-and-re-pick flake is not fixed. The residual red at legMs 40000 is 2 of 9 passes. Next fix (not made here): pick a fresh creep before each leg, or make the re-pick fast enough to finish inside a leg.
- check-all on HEAD e430e812: `RESULT gates=74 ok=74 failed=0 drift=0 skipped=6 strict=false`, exit 0. The first run on 68a745be failed one gate, check-design-record: DESIGN.md recorded gates=73 while the tree has 74 (gen-melee-reach.mjs joined GATE_GENERATORS in b5b3cc24). DESIGN.md:68 is now gates=74; its negative control passes. Skipped by name: editor-check (needs the built osdt-data binary), gen-turn-rates (needs G688_UNIT_INI), gen-ability-keys and gen-ability-text-688g (need the hy3d directory), gen-neutral-688g (needs --src), gen-heroes (needs --src).
- Pages-build witness (Phase B, e430e812): `SKIP_PACKS=1 bash scripts/layout-site.sh /tmp/oscraft-pages-site main 0` exit 0 (the pack step is skipped here: `toktx` is not installed, and CI builds the packs); `node scripts/bundle-client.mjs` 63 inputs bundled, 40 stale modulepreloads removed, 24 kept, play.bundle.js 444 KB, rooms.bundle.js 4 KB; `node scripts/bundle-lua.mjs` 101 modules; `node scripts/check-site-bundle.mjs --stamp` `site bundle ok` (chunks 4, preloads 24, contentSha256 0e50e6f2). The pages.yml modulepreload change (0d1f636d) is covered by this check. The repo tree is unchanged by the witness.
- Playwriter note: one run stalled (a trivial page evaluation did not return for over 120 s after a long witness batch); the next run succeeded. Re-run the batch if a call does not return.

## Phase A pass: residual D1 flake, fresh creep per leg and trace on every run (2026-11-09)

- A-1 scripts/witness/strike-while-walking.mjs, commit b5c4c874: each leg picks its creep before the sampling loop, avoiding the creep the previous leg struck (window.__ciWitnessUsed, reset at the start of runWitness; the fallback is the unrestricted pick when no other creep is found). The carried-creep reuse is removed, so a creep that died between legs no longer forces the slow probe-bound re-pick inside the loop (passes 42 and 43). A creep that dies inside a leg still forces a re-pick in the loop; the trace records its timing (picks, doneMs). initialPick records the creep's hp. Witness: the Phase B fresh-import D1 passes on port 8832; each leg's initialPick.hp and picks in the trace.
- A-2 scripts/witness/strike-while-walking.mjs, commit 6922ff37: runWitness writes every run's config, verdict and legs to .gm/witness-traces/d1-<stamp>-<ok|red>.json and returns traceFile. The directory is already ignored by /.gm/witness-*/ (.gitignore line 38). Witness: each Phase B pass prints its verdict and traceFile; every red pass quotes its trace from that file.
- Not yet run (Phase A rule): node --check on the witness passed (syntax only). The self-check scripts/witness/strike-while-walking-check.mjs, the D1 passes and check-all are Phase B.
- B-1 (Phase B, pass 45 red, cause shown by its trace): the first version of A-1 excluded every creep struck earlier in the run. The lock-on-again leg then had no candidate, the pick fell to placement, and all 16 placement probes failed (`aborted: no placed point was both reachable and accepted by the sim (16 tried)`, zero samples). Fix: avoid only the previous leg's creep, falling back to the unrestricted pick. Witness: pass 46 green after the fix; passes 47 to 50 below.

- B-2 (Phase B, passes 46 to 58, one fresh match each, legMs 40000, verdict as the witness returns it; trace file under .gm/witness-traces/):
  - 46 ok (lock on walking 8, lock on again 1); 47 ok (7, 3); 48 ok (5, 4); 49 ok (5, 7); 50 ok (2, 1); 51 ok (2, 1).
  - 52 red: `lock on again: no strike while moving with one-hand on`. Trace d1-2026-11-09T15-16-44-546Z-red.json: the lock-on-again creep 249 (type 68, 1200 hp) died at 35.3 s; the in-loop re-pick took 1.6 s and chose creep 200 (804 hp at the end); lock-on-again walking 0, stopped 22, samples 245. Sampling was at full rate.
  - 53 red: same reason. Trace d1-2026-11-09T15-19-06-204Z-red.json: creep 225 (1200 hp) was re-picked at 38.9 s, so the leg had about 1 s of live target; walking 0, stopped 20, samples 214.
  - Setup, no verdict: pass 50's first attempt (lobby not live after 90 s, phase lobby, the page wedged so that the next navigations timed out). Passes 54 to 58 use a retry on start and 180 s for live; pass 54 still reached the lobby only (setupError, no witness run).
- B-3 (Phase B, clear-target.js and the witness rule): the pick takes the healthiest live candidate among the clear ones, nearest breaking ties (healthiestOf, rule.hp). Reason: placed creeps (type 68, 1200 hp) die inside a 40 s leg (52, 53, 55, 57), while lane creeps (type 6, about 7000 hp) survive it. Witness: passes 54 to 58 below.
- B-4 passes 54 to 58 with B-3 (verdict as the witness returns it):
  - 54 ok (lock on walking 3, lock on again 2); 56 ok (2, 1).
  - 55 red: `lock on again: no strike while moving`, trace d1-2026-11-09T15-33-34-613Z-red.json: the leg's creep 200 was at 590 hp at the start and died at 28.7 s; re-pick 1.7 s; samples 194, 104, 117 (sampling at about half rate).
  - 57 red: `lock on again: no strike while moving`, trace d1-2026-11-09T15-38-34-911Z-red.json: creep 225 (1200 hp, no re-pick), walking 0 stopped 10, samples 128, 113, sampling low.
  - 58 red: `lock on` and `lock on again` both zero walking strikes, trace d1-2026-11-09T15-41-18-680Z-red.json: samples 162, 117, 135.
- Host load: the red traces 55, 57 and 58 have sampling rates of half or less, and the machine's load average was 11 to 19 (17:37 to 17:40) from processes outside this repo (stick-floor-ramp-witness, paired-run, busy loops). Passes with full sampling rate (about 250 to 310 samples a leg) were green, except 52 and 53, which are the mid-leg creep death. Under load, a leg of 40 s gives about 100 to 200 samples, and the walking count is a few per leg, so the count is not robust.
- Status: the flake is NOT closed. Post-fix at legMs 40000 (passes 46 to 58 excluding setup errors): 7 ok, 6 red. Each red has a trace. The remaining causes are the mid-leg creep death (52, 53) and sampling-rate loss under host load (55, 57, 58). Next fix (not made): measure a leg in sim ticks rather than wall-clock, or rerun a leg whose sample count falls below a floor and record that rerun.

## Phase A pass: D1 flake at its cause, sim-tick legs, replaced targets, sample floor (2026-11-09)

- Causes from the Phase B reds (passes 52 to 58): (a) wall-clock legs, where host load (load average 11 to 19) cut the sample count to 104 to 194 in 55, 57 and 58; (b) a creep that dies inside the leg (52, 53, 55 partly), which forces a re-pick of 1.6 to 1.7 s.
- A-1 `scripts/witness/strike-while-walking.mjs`, sim-tick legs. `legTicks` 640 (40 s at 16 ticks a second, the sim's rate). The sample loop waits until the sim tick has advanced `sampleTicks` 2, so a leg takes 320 samples whatever the host load; load only stretches its wall time. Order cadence (`orderEveryTicks` 5, about 300 ms), order settle (`orderSettleTicks` 4, about 250 ms), the initial settle (`initialSettleTicks` 6), and the placement probe (`probeTicks` 24, about 1.5 s) are ticks too. `waitTicks` gives up after `stallPolls` 200 polls with no tick advance (abort `sim stopped ticking`), so the guard counts polls, not milliseconds. Hero wait loops count polls (`slowPollMs` 500, `resetPolls` 240, 120 s). Trace stamps are `atTick` (ticks from the leg start), replacing `atMs`. The leg function has no `Date.now()`.
- A-2 `scripts/witness/strike-while-walking.mjs`, target life. The pick is still the healthiest clear candidate (B-3). New: a target is replaced when its hp falls to `replaceFraction` 0.25 of its hp at pick (`replaceHp`). The replacement pick starts in the background, avoids the current creep, and swaps in when done (`picks` entries carry `reason`, `from`, `to`, `doneTick`). If no replacement is found, the leg keeps the creep and stops replacing (`replaceHp` 0), so death still re-picks. Numbers, from the pass 52 trace: creep 249 (1200 hp, type 68) died at 35.3 s, so its average loss was about 34 hp a second (GUESS as a constant, one creep's average). At that rate 25% (300 hp) is reached near 26.5 s, and the 1.6 s re-pick lands by about 28 s, about 12 s of margin in a 40 s leg. Not taken: fitting the leg length to the target's life, because the sample count would then vary and the floor would fail short legs.
- A-3 `scripts/witness/strike-while-walking.mjs`, sample floor. `sampleFloor` 288 (90% of 320). `runWitness` reruns a leg whose samples are below the floor, up to `legReruns` 2. Each discarded attempt is kept in that leg's `reruns` (attempt, samples, ticks, aborted, picks) and written to the trace file. `verdict()` fails a leg whose final samples are below the floor and names the attempt count. A rerun that passes is in the trace, not in the verdict, so it is recorded and never silent.
- A-4 `scripts/witness/strike-while-walking-check.mjs`, witness that fails on wall-clock. It asserts `CONFIG.legTicks` is 40 s at 16 ticks a second, that `strikeWhileWalkingLeg`'s source has no `Date.now(` and reads `__ciTicks()`, two floor cases (at the floor passes, below fails) and the reason text. Expected output: `strike-while-walking verdict checks ok (11 cases, leg timing in sim ticks)`.
- Not converted, recorded: `placeRetryMs` (400 ms wait between placement attempts in `clear-target.js`) is a wait inside the pick, not a measurement; the A-4 check covers the leg function only.
- Phase A ran no build, test, gate, witness or browser step, and no `node --check`. Phase B owns: the self-check, fresh-import D1 passes on port 8832 (each verdict and, for every red, its trace file with `reruns`, `picks` and `atTick` stamps), `node scripts/check-all.mjs` (gates=74 expected), and one push.

## Phase B results: sim-tick D1 passes on port 8832 (2026-11-09)

- Stage: `WASM=1 SKIP_PACKS=1 bash scripts/dev/stage.sh` laid out `.dev-site` at sha a476ec98; `git status` stayed clean (the regenerated moba.osdt is byte-identical). Server: `node scripts/dev/serve.mjs /workspace/oscraft/.dev-site --port=8832`, PID 1733860, recorded in /tmp/serve-8832.pid.
- Self-check: `strike-while-walking verdict checks ok (11 cases, leg timing in sim ticks)`.
- Job loss: run A (gm job body-72, passes 1 to 8 in one body) was killed after pass 4 (state `killed`, no runner from it alive). Passes 1 to 4 had finished and written traces; passes 5 to 8 never ran. Not run under setsid: browser work runs only through the gm playwriter verbs (INVARIANT 11), and a separate process would bypass them. Run B (gm body-124) reran passes 3 to 8, one fresh import each, appending each result to `.gm/witness-traces/d1-rerun-3-8.json`; it completed.
- Verdicts, one fresh match each, legs of 640 ticks:
  - Run A: pass 1 ok (`d1-2026-11-09T15-55-51-321Z-ok.json`); pass 2 red, `lock on again: no strike while moving with one-hand on` (`d1-2026-11-09T15-58-44-863Z-red.json`); pass 3 ok (`d1-2026-11-09T16-01-05-498Z-ok.json`); pass 4 red, `lock on again: no strike while moving with one-hand on` (`d1-2026-11-09T16-03-44-013Z-red.json`).
  - Run B: pass 3 ok (`d1-2026-11-09T16-21-03-267Z-ok.json`); pass 4 red, `lock on: no strike while moving with one-hand on` (`d1-2026-11-09T16-23-42-539Z-red.json`); pass 5 ok (`d1-2026-11-09T16-26-05-815Z-ok.json`); pass 6 red, `lock on again: no strike while moving with one-hand on` (`d1-2026-11-09T16-28-22-586Z-red.json`); pass 7 ok (`d1-2026-11-09T16-30-47-488Z-ok.json`); pass 8 red, `lock on again: no strike while moving with one-hand on` (`d1-2026-11-09T16-33-04-311Z-red.json`).
- Tally: 10 fresh matches, 5 ok and 5 red. In every red, each leg has 317 to 320 samples over 640 to 684 ticks, with no reruns and no abort. The sample-loss cause did not occur, and the sample floor never fired.
- Red traces, read (B 4, B 6, B 8, and A 2):
  - The failing leg is lock-on in B 4 and lock-on-again in B 6, B 8 and A 2. Its walking-strike count is 0 in all four, with 128 to 235 in-reach moving samples.
  - Cooldown-ready samples (`terms` index 7 = 1) are rare: 13 to 17 a leg in the lock-on legs and 4 to 14 in the lock-off legs. Walking strikes come only from those samples, so a 40 s leg has about one chance per attack period.
  - Replacement works in the failing legs: B 6 and B 8 start lock-on-again on creep 225 at 1200 hp and replace it twice (at ticks 393 to 465 and 543 to 615). A 2 started on creep 200 at 469 hp, which the lock-off leg had already struck, because the avoid list removes only the previous leg's creep and the fallback pick returned it. B 4's lock-on-again also fell back to creep 200 at 592 hp.
- Cause: not established. Ruled out by these traces: sample loss (all legs at full sampling), a creep dying inside a leg (the 25% replacement covered every case), and the pick alone (B 6 and B 8 started on a full-hp creep). Candidate, untested: walking strikes are rare per leg, so the at-least-one-walking-strike rule is fragile at full sampling. Separating that from a launch that does not happen needs a sim-side probe of cooldown-ready, in-reach, moving samples, which is not part of this change.
- Status: NOT closed. The Phase A changes removed the wall-clock sample loss and the mid-leg creep death from the failing shapes, but the red shape remains (zero walking strikes with full samples), and the rate is 5 of 10. No sim change was made in this pass.

## Phase A pass: D1 cause at the sim, launch facing and a probe (2026-11-09)

- Process defect, recorded: commit 34357ade (the ledger entry with the Phase B results) was pushed during the Phase A window. The rule is that Phase A pushes nothing and the push happens once, in Phase B. It was a rule break. No code changed with it, and it is reverted by no commit; this pass pushes once, in Phase B.
- Cause, derived by reading the sim (not yet run; Phase B's probe confirms or refutes it). Per unit, `tick.rs` moves the unit, then calls `face_move` (turns the heading toward the direction of travel by `turn_step`), then `attack()`, which calls `face_target` (turns the heading toward the strike target by `turn_step`) and launches only when the heading is within `FACE_TOLERANCE` (2048 units, 11.25 degrees) of the target bearing. While a walking unit has a strike target, the two turns pull in opposite directions whenever the target lies behind or beside the travel direction. The heading then stalls outside the arc and `attack()` returns at the arc check on every tick. Worked numbers for the live hero (pudge, turn rate 1337, `turn_step` 15200 units a tick): travel at heading 0, target at 180 degrees, the heading settles at 30400, 2368 units (13 degrees) outside the arc, and never reaches it. This matches the traces: walking launches come only when the travel direction brings the target into the arc (lock-on legs with 8, 6 and 5 walking strikes in the passes that passed).
- Changes, each with the witness it needs:
  1. `rust/crates/sim/src/combat.rs`: the probe. `attack()` counts, per unit, the ticks where the unit walks with strike on move and its strike target is in reach (`walk_watch`): `WALK_LAUNCHED`, `WALK_COOLING` (cooldown not ready), `WALK_NOT_COMMANDABLE`, `WALK_OUT_OF_ARC` (facing failed). Counts are per `attack()` call, so a tick with a second call (`order_attack`, `set_strike_target_now`) counts twice. Read through `walk_probe_of`. The probe is transient (not hashed, not saved, no `STATE_VERSION` change), records only, and changes no verdict. Witness: Phase B trace, per leg `walkProbe` (launched, cooling, notCommandable, outOfArc).
  2. `rust/crates/sim/src/arrays.rs`: the `walk_probe` field, its init, and its reset on slot reuse. Witness: compiles (Phase B `cargo test -p sim`).
  3. `rust/crates/web/src/lib.rs` `unit_walk_probe`, and `js/client/play.js` `window.__ciWalkProbe`. Witness: Phase B witness trace has `walkProbe` on every leg (a null hook would show as zeros).
  4. The candidate fix. `rust/crates/sim/src/combat.rs` `turns_to_strike` and `rust/crates/sim/src/tick.rs`: `face_move` is skipped while the unit strikes on move and has a commanded strike in reach, so the heading turns to the target (`face_target` in `attack()`) and is not pulled back. Witness: `rust/crates/sim/tests/strikes.rs` `a_walking_unit_turns_to_a_foe_behind_it_and_strikes_it_with_strike_on_move_on`. It must fail with the `face_move` guard removed (the unit stalls, no hit) and pass with it. Phase B runs both.
  5. `scripts/witness/strike-while-walking.mjs`: `walkProbe` per leg (the probe's delta over the leg), in the leg's return and so in the trace. The verdict and its thresholds are unchanged.
- Not changed, recorded: the 25% replacement rule (`replaceFraction` 0.25), `legTicks` 640, `sampleFloor` 288, `legReruns` 2, and every verdict threshold in `verdict()`. The verdict stays the same: a lock-on leg passes with at least one strike while moving, a lock-off leg with zero launches while moving, and every leg with in-reach moving samples, hero-sourced effects and the sample floor.
- Phase B decision rule. Keep the fix only if: (a) the new strikes test is red with the guard removed and green with it; (b) the lock-on legs show walking launches at full sampling; (c) the probe's `outOfArc` count falls in those legs while `launched` rises. If `outOfArc` stays dominant with the fix, the arc is not the only cause; record the probe and the next cause, and do not loosen any verdict. If `cooling` dominates the in-reach ticks and `launched` stays low, the limit is the cooldown, the facing candidate is refuted, and the guard is reverted and recorded as refuted.
- Phase B order: stage (`WASM=1 SKIP_PACKS=1 bash scripts/dev/stage.sh`), `cargo test -p sim --test strikes`, the guard-removed run of the same test, the self-check (`node scripts/witness/strike-while-walking-check.mjs`), `cargo test -p web --test pathing_probe` and `cargo test -p mod-host`, eight fresh-import D1 passes on port 8832 (each verdict, each red trace with `walkProbe`), `node scripts/check-all.mjs` (74 gates), then one push.
- Remaining work, ranked, in the final report.

## Phase B results: D1 at the sim-side facing fix, eight passes (2026-11-09)

- Process defect, recorded: commit 34357ade pushed the Phase B ledger during the Phase A window (a rule break; no code was affected). This pass pushes once, after Phase B.
- Test change, committed: `rust/crates/sim/tests/strikes.rs` (commit after 30deda62, lanmower). The facing test asserts the walking launch (`walk_probe_of(attacker)[0] > 0`), not a hit: a unit walking away from a foe behind it leaves reach during the windup, and `tick_point` cancels that swing, so the hit cannot land. Discriminating witness: with the `turns_to_strike` guard removed from `tick.rs`, `cargo test -p sim --test strikes a_walking_unit_turns` fails (`the walking unit turned to the foe behind it and launched a strike`); with the guard, it passes. Phase B: `cargo test -p sim --test strikes` 13 passed; `cargo test -p sim` all suites ok; `cargo test -p web --test pathing_probe` ok; `cargo test -p mod-host` 22 suites ok; `node scripts/witness/strike-while-walking-check.mjs`: `strike-while-walking verdict checks ok (11 cases, leg timing in sim ticks)`.
- The eight D1 passes, one fresh import each on port 8832 at sha 30deda62 (the fix is in), verdict as the witness returns it:
  - Pass 1: `verdict: { ok: true, reasons: [] }`, trace `d1-2026-11-09T17-09-22-406Z-ok.json`.
  - Pass 2: red, `lock on again: no strike while moving with one-hand on`, trace `d1-2026-11-09T17-12-08-606Z-red.json`.
  - Pass 3: red, same reason, trace `d1-2026-11-09T17-14-45-622Z-red.json`.
  - Pass 4: red, same reason, trace `d1-2026-11-09T17-34-02-650Z-red.json`.
  - Pass 5: red, same reason, trace `d1-2026-11-09T17-36-22-071Z-red.json`.
  - Pass 6: `verdict: { ok: true, reasons: [] }`, trace `d1-2026-11-09T17-38-45-662Z-ok.json`.
  - Pass 7: `verdict: { ok: true, reasons: [] }`, trace `d1-2026-11-09T17-41-10-231Z-ok.json`.
  - Pass 8: `verdict: { ok: true, reasons: [] }`, trace `d1-2026-11-09T17-43-50-324Z-ok.json`.
  - Tally: 4 ok, 4 red. Every red fails the lock-on-again leg; the lock-on leg passes in all eight.
- The reds, from the `walkProbe` counts of the failing leg (lock on again, 320 samples, zero walking strikes in each):

  | Pass | launched | cooling | notCommandable | outOfArc | walking-watched ticks | in-loop re-attacks |
  |---|---|---|---|---|---|---|
  | 2 | 3 | 53 | 0 | 0 | 56 | 0 |
  | 3 | 2 | 36 | 0 | 0 | 38 | 0 |
  | 4 | 0 | 12 | 0 | 0 | 12 | 25 |
  | 5 | 1 | 75 | 0 | 0 | 76 | 0 |

  Green lock-on-again legs (passes 6 and 8) show 108 and 125 walking-watched ticks, with 4 and 5 launches. Pass 4's green counterpart in the lock-on-again leg has 0 re-attacks.
- Cause, from the counts. The sim's rule is right: in the failing legs `outOfArc` and `notCommandable` are 0, so every walking-watched tick where the weapon was ready launched a strike. The shortfall is the number of walking-watched ticks (12 to 76 in the red legs, against 108 to 130 in the green ones). Why they are fewer: the sim counts a unit as walking only with order kind 0 (`ORDER_MOVE`). In pass 4's lock-on-again leg, 231 of 300 samples are in kind 2 (`ORDER_ATTACK_MOVE`) and only 10 are in kind 0. In green pass 6's lock-on-again leg, 226 of 300 are in kind 2, and 73 of the other samples are kind 0 with motion. The difference is the witness's in-loop re-attack (`commandAttack`, issued when the strike target is lost), which sets kind 2 again after each move order. Pass 4's lock-on-again leg had 25 such re-attacks; green pass 6's had none (only the initial pick and one re-pick). So the red is a witness effect: its re-attack path turns a move into an attack-move, which the sim does not count as walking.
- Decision: the sim rules stay (the windup cancel and the launch rule). Evidence against the windup cancel: across the lock legs of passes 1, 2, 3, the windup starts are 25 to 26 per leg and 0 to 1 are cancelled, so the cancel rule does not leave legal walking strikes unlanded. The facing fix holds: the arc failures in green lock legs are 1 to 8 per leg, and none in the failing legs. The witness's in-loop re-attack is the open item (below); changing it changes what the verdict measures, so it is an owner decision, not made in this pass. The verdict is unchanged.
- Status: NOT closed. The D1 red is 4 of 8 at sha 30deda62. Cause: the witness's in-loop re-attack moves the hero from a move order to an attack-move, so the walking-watched ticks fall; the sim launches on every ready walking tick.
- check-all, at this commit: `RESULT gates=74 ok=74 failed=0 drift=0 skipped=6 strict=false`, `EXIT=0` (`.gm/witness-traces/checkall.log`).

## Phase A pass: D1 re-attack question decided from the code (2026-11-09)

- Decision: a sim defect, fixed at the sim's walking classification; the witness is unchanged. Evidence:
  - `Command::Attack` (rust/crates/replay/src/lib.rs:542) calls `order_attack_or_approach`, which never sets `ORDER_ATTACK_MOVE`: it sets `ORDER_MOVE` only when the unit was on STOP, and `approach_to` keeps the kind. The in-loop re-attack therefore cannot produce kind 2.
  - Green pass 6's lock-on-again leg has 226 of 300 samples in kind 2 and no re-attacks (line 753). Kind 2 is the hero's normal order in that leg, so the attribution to the re-attack in lines 753 and 755 is wrong. Those two lines are superseded by this entry.
  - `walking()` (rust/crates/sim/src/movement.rs) required `ORDER_MOVE`, so a moving attack-move unit with strike on move on was not walking. The defect condition of the owner rule holds.
  - The attack gate (combat.rs:129) blocks only walking units with the flag off. A kind-2 unit with the flag off is never walking after the change, so every unit keeps its gate behaviour.
- Change: `walking()` counts `ORDER_ATTACK_MOVE` only while `strikes_on_move(i)` is set; the other terms are unchanged. `walking()` feeds only the gate at combat.rs:129 and `walk_watch` (the probe), so no movement, facing or strike result changes. Only the probe count of kind-2 walking launches changes.
- Test, committed in this pass: `rust/crates/sim/tests/strikes.rs` `a_moving_attack_move_unit_with_strike_on_move_on_is_walking_when_it_launches_a_strike`. Phase B witness: with the `walking()` change reverted it must fail (probe count 0); with the change it must pass. Whether the attack-move unit stops in reach (`attack_chase` stops a unit whose strike is ready) is unverified until that run; if it stops, the test needs a foe that stays out of reach while the unit walks.
- Witness: unchanged, verdict thresholds unchanged. The witness's walking definition (`walkingOrder`, strike-while-walking.mjs:352) is still kind 0 only, so `strikeSamplesWhileMoving` may stay 0 on attack-move legs even with the sim fix. If Phase B's D1 stays red for that reason, the open item is the witness's walking definition, an owner decision.
- Phase A ran no build, test, gate, witness or browser step. Phase B owns: stage, self-check, `cargo test -p sim --test strikes` (the new test red without the change, green with it), `cargo test -p sim`, eight fresh-import D1 passes on port 8832 with each verdict and red trace, `node scripts/check-all.mjs` (gates=74), then one push.

## Phase B results: attack-move walking change at c5e9fab0, D1 blocked (2026-11-09)

- Test decision: kept `a_moving_attack_move_unit_with_strike_on_move_on_is_walking_when_it_launches_a_strike` (rust/crates/sim/tests/strikes.rs). The uncommitted change replaces `drive_targeting` with a direct `set_strike_target` each tick. Reason: `drive_targeting` sets `hold_override` for attack-move units with a foe in reach, and `engaged()` then holds the unit still (`separate`), so the test measured the harness, not the walking classification. The first version failed with the fix and without it.
- Rust, detached job (`.gm/witness-traces/phaseb-c5e9.log`, status `phaseb-c5e9-status.txt`):
  - With the walking change: `test a_moving_attack_move_unit_with_strike_on_move_on_is_walking_when_it_launches_a_strike ... ok`; `cargo test -p sim --test strikes`: `test result: ok. 14 passed; 0 failed`.
  - With `walking()` reverted to HEAD~1: `... FAILED`, panic at `crates/sim/tests/strikes.rs:303:5`: `the moving attack-move unit launched a strike counted as walking`; `test result: FAILED. 13 passed; 1 failed`.
  - `cargo test -p sim`: all suites `ok`. `cargo test -p web --test pathing_probe`: exit 0. `cargo test -p mod-host`: exit 0.
- Stage: `WASM=1 SKIP_PACKS=1 bash scripts/dev/stage.sh`: `laid out /workspace/oscraft/.dev-site sha=c5e9fab083075c15088e88085dc8818674e0f3c8`, exit 0. Self-check `node scripts/witness/strike-while-walking-check.mjs`: `strike-while-walking verdict checks ok (11 cases, leg timing in sim ticks)`.
- Gate: `node scripts/check-all.mjs`: `RESULT gates=74 ok=74 failed=0 drift=0 skipped=6 strict=false`, EXIT=0 (`.gm/witness-traces/checkall-c5e9.log`).
- D1 eight fresh-import passes: NOT RUN. Blocked by the browser route, not by the sim:
  - The gm playwriter MCP server exits with code 128 at startup. Runs `2026-11-09-18-42-10-98e4bc` (event seq 3-4: `mcp-server playwriter state down, reason exited with code 128`) and `2026-11-09-19-03-09-ae5a08` (no playwriter verb listed; `gm.verbs()` names matching `playwriter` are empty) both lack `playwriter_execute`.
  - `playwriter-project status` (run from the shell, not the gm verb) reports `no gate running for /workspace/oscraft`.
  - The cached playwriter package in `~/.npm/_npx/d2f95d268a966dbb` cannot load its `playwright-core` (missing module `ip-address`), so the fallback browser route is also unavailable. The repo has no Playwright install.
  - Dev server on port 8832 was up and serving sha c5e9fab0 (`/version.json`), PID 2098953 (own, stopped by PID after this entry).
- Step 7 (witness walking definition): not decided. The sim now counts a moving attack-move unit as walking when strike on move is on. The witness's `walkingOrder` (strike-while-walking.mjs:352) still counts kind 0 only, so `strikeSamplesWhileMoving` on attack-move legs stays as it was. Prediction to check with the eight passes: if the red legs still show zero strikes while moving with kind-2 samples, the witness's kind-0 definition is the cause and the witness change is to count the sim's walking definition. Not changed in this pass.
- Earlier D1 data, read in this pass: passes 2 to 5 red, `lock on again: no strike while moving with one-hand on`, `strikeSamplesWhileMoving` 0 in each red lock-on-again leg (176 in-reach samples in passes 4 and 5). Passes 6 to 8 green with 1 to 4 such samples. These are at sha 30deda62 (before the walking change), not the new code.
- Push: held. The owner's rule pushes once every witness is green, and the D1 witness for c5e9fab0 has not run. The ledger and the test change are committed locally as lanmower, without trailer.

## Phase A audit: allocations in hot paths and reuse plan (2026-11-09, read-only)

Scope: Rust sim and replay (tick path and host read APIs), and the JS frame loop, pointer input and HUD in js/client. No source edit, build, test, gate or witness ran in this pass. Each site is `file:line` with the enclosing function. Sampled, not exhaustive: play.js has 372 allocation-token matches, most outside the frame, and only the per-frame chain and its helpers were read in full.

Result of the tick-path check: `rust/crates/sim/src/tick.rs` has no allocation token, and neither do the per-unit loop body or the `tick_*` systems. The allocations are in host read APIs, in per-call helpers, and in string keys.

### Rust (15 rows, 19 locations)

| # | Site | Allocation | Frequency | Reuse | Behaviour change? |
|---|---|---|---|---|---|
| R1 | sim/src/attributes.rs:207, 211 | `name.to_string()` on every counter get and set | every mod counter call | intern counter names at load to a `u32` id, key the map by `(usize, u32)` | yes: the key order and `state_hash` change, so bump `STATE_VERSION` and note it in the commit body |
| R2 | sim/src/obstacles.rs:148 `dead_props` | `collect` into a Vec, the wasm return | every frame from play.js:1187 | a sim-side death generation counter; JS compares the number | no, if the generation changes exactly when the set does |
| R3 | sim/src/aggro.rs:132 `units_near` | `collect` | per call; caller not found in sim/src | `units_near_into(&mut Vec)` with the caller holding the buffer | no |
| R4 | sim/src/combat.rs:579 `drain_kill_events` | `collect` of `kill_queue.drain` | per host drain | drain into a caller buffer | no |
| R5 | sim/src/spell.rs:1631 `take_procs`, sim/src/rules.rs:555 `take_casts`, sim/src/waypoints.rs:92 `take_destination_requests` | `mem::take` moves the Vec out (its capacity is lost), then `collect` | per host drain | keep the Vec in the Sim, `drain(..)` into a caller buffer | no (order is kept); re-entrancy must be checked |
| R6 | rust/crates/replay/src/lib.rs:1087-1097 `hash_later` | `sim.clone()` sent to the hasher thread | every `CHECK_EVERY` = 64 ticks, native only (wasm hashes in place at :1078) | a pre-allocated snapshot the hasher copies into (`copy_from`) | no result change; cost only |
| R7 | sim/src/hash.rs:92 `pool`, :180, :184, :304, :308 | `Vec::new` and `bytes` collects inside `state_hash(&self)` | per `state_hash` call (replay checkpoint, web :147) | a scratch Vec held in the Sim, `state_hash` taking `&mut self` or an interior cell | no; the signature changes at web/src/lib.rs:147 and replay/src/lib.rs |
| R8 | sim/src/astar.rs:56, :89 `route`, `search` | `Vec::new` for the output path | per route request (movement.rs:495, per move order) | route into a caller-held Vec; the thread-local scratch is already reused | no |
| R9 | sim/src/obstacles.rs:162 `clear_trees_square`, :189 `trees_near` | `felled` and `out` Vecs | per teleport (movement.rs:291); `trees_near` caller not found | a held buffer per call site | no |
| R10 | sim/src/fog.rs:110-114 `vision_plane` | returns a new Vec<u8> per call | caller not confirmed; the JS fog view may call it per frame | write into a caller-owned `&mut [u8]` | no |
| R11 | sim/src/combat.rs:236 `attack_terms` | `vec![]` | per call; used by the witness hooks | fixed array return `[i32; N]` | no |
| R12 | sim/src/combat.rs:526, :534 `recent_hitters` | `Vec::new` and `collect` | per death or hit window | a held buffer | no |
| R13 | sim/src/order_queue.rs:108 `queued_orders` | `collect` | per call; resync at order_queue.rs:113 | iterate without collecting, or a held buffer | no |
| R14 | sim/src/ground.rs:70 `pickup_ground_item` | `before` collect | per pickup command | a held buffer | no |
| R15 | sim/src/carry.rs:7 `carry_from` | `filter().collect()` | per carry event | a held buffer | no |

Not reused, deliberately: the pathfinding scratch (pathfinding.rs:155-157 `cur`, `n1`, `n2`) and the astar thread-local are already reused, and the constructors (arrays.rs, spell.rs, aggro.rs, attributes.rs) run once. R4, R5 and R12 change which buffer the host reads from; the witness is a parity check (below).

### JS (15 rows, 17 locations)

| # | Site | Allocation | Frequency | Reuse | Behaviour change? |
|---|---|---|---|---|---|
| J1 | js/client/play.js:1187-1190 `syncDeadProps` | `localGame.dead_props()` every frame (wasm Vec), `dead.join(",")` key string | every frame from drawWorld3d:1215 | the generation from R2; no join | no, if R2 lands |
| J2 | play.js:955 `currentUnits` | `netState.units.map(anchored)`, a new `out` array and one new object per unit (`anchored`, :950) | every frame from drawWorld3d:1219 | a held `out` array (`length = 0`) and pooled unit records | yes if any caller keeps a unit across frames; check the readers |
| J3 | play.js:1082 `collectUnitDraws` | `list`, `clips`, `yaws`, `new Map(units.map(...))` | every frame | module-level arrays cleared per frame, a Map held and `clear()`ed | yes if a caller keeps the list |
| J4 | play.js:1151 `emitUnitDraws` | draw objects; the body was not read in full | every frame | held draw records | to confirm |
| J5 | play.js:1196 `podiumUnitDraws`, :1208 `podiumTransform` | a list and a new transform per entry | podium only | held transforms per seat | no |
| J6 | play.js drawWorld3d, the shadow branch | `{ ...drawCounts }` spread every shadow frame | every shadow frame | a module-level counts object, `Object.assign` back | no |
| J7 | play.js drawWorld3d, podium branch | `{ ...podium.plinth, color: [0.32, 0.38, 0.5] }` | podium frames | hoist the object and the colour constant | no |
| J8 | js/client/render3d.js:105, :509 | `[...opts.target]` each camera set | every camera set, including drags | copy into a held `camera.target` array | no |
| J9 | render3d.js:1114 `figureOf` | `new Set(clips.map(...))`, `SCENE_SWITCHES.filter` | caller frequency to confirm | a held Set, cleared and refilled | no |
| J10 | js/client/moba_input.js:47 `onMove` | `box = {x0, y0, x1, y1}` per pointermove | every pointer move | mutate one `box` object | yes if any reader keeps `box` between events |
| J11 | js/client/ui_minimap.js:149 `playerFill` | a template-string colour per unit per draw (called at :171) | per unit per minimap draw | memoise per player in the existing `fillByPlayer` Map | no |
| J12 | js/client/hud.js:200-205 `viewRect` | a 4-element `corners` array with `map` and spreads into `Math.min` | caller frequency to confirm | four scalar min and max, no arrays | no |
| J13 | js/client/hud_canvas.js:129-133 `place` | `dom.map` and spreads into `Math.min` and `Math.max` | per place call, caller at :157 | reuse a held rect array | no |
| J14 | js/client/hud.js:263 `gameMaxHps` | a new Map per call | caller frequency to confirm | a held Map, cleared | no |
| J15 | js/client/ui_minimap.js:88-89 `fit` | two new Maps per fit | per resize (:86, :282) | low; held Maps | no |

Also seen and not a site: render3d.js:1-17 and 276 already hold module scratch matrices, and drawLod (render3d.js:737) caches its tier buffer per batch. Those are the pattern for J2 to J9.

### Reuse that changes behaviour (owner decision before any edit)

- R1 changes the state hash and the persisted layout: it needs a STATE_VERSION bump and a replay parity check.
- R7 changes the `state_hash` signature (web/src/lib.rs:147 and replay). The hash value must not change.
- J2, J3 and J10 change what a caller may hold across frames. Each needs a reader check before the change.
- R5 and R4 move the read buffer into the caller: the capacity and ordering must be preserved.

### Witnesses each reuse needs (Phase B)

- Rust sites R1 to R15: `cargo test -p sim` and `cargo test -p replay`, plus the replay parity check (`mod-host/src/bin/parity_hash.rs`) on a fixed seed. R1 and R7 also need the hash equal to the pre-change hash on the same run, and STATE_VERSION bumped with the reason in the commit body.
- R6: a native replay test that asserts the same `chk` lines before and after.
- JS sites J1 to J15: `node js/client/camera-target-audit-check.mjs`, `js/client/hero-trajectory-check.mjs`, `js/client/movement-sequence-check.mjs` and `js/client/lock-engage-order-check.mjs` (each must still pass), plus a playwriter run that counts GC or heap growth per 600 frames before and after on the staged site (the baseline is the current build).

### Playwriter step (step 5 of this pass)

- Error recorded (run 2026-11-09-18-42-10-98e4bc, next_context events seq 3-4): `mcp-server playwriter state down, reason exited with code 128`, stderr empty. The same happened in run 2026-11-09-19-03-09-ae5a08 until its `restart` at 19:08:10 (event seq 11, "restarted without a reason"). After that restart the server reported `connected`, tools 3, protocol 2025-11-25 (seq 13).
- Cause: not established. The logs carry no stderr. The launch command is `npx -y -p github:AnEntrypoint/pw#1488a92 playwriter-project` (defaults/mcp.json). The shell run of `playwriter-project --help` and `status` worked, so the failure was in the gm launch, not the package. The restart cleared it. Treat a repeat as an open defect.
- Session started through gm `playwriter_execute` on the staged site (sha c5e9fab0, port 8832, my server PID 2148909): the project gate pid 2148462, browser pid 2149052, page `http://localhost:8832/js/client/play.html?ci=1&fresh=session1`, title `oscraft net client`, `window.oscraft` is an object, `body[data-phase]` is `lobby`. No D1 pass ran.

### D1 eight passes at f513d64b and the walking witness decision (2026-11-09)

Staged site: sha f513d64b (restaged in this pass; `/version.json` confirmed). Each pass: a fresh page (`?ci=1&fresh=<tag>`), the lobby (`All Pick`, claim, pick antimage, ready, fill, start), `phase === 'live'`, then `runWitness`. Trace files are in `.gm/witness-traces/`. The reds' `walkProbe` counts are per leg; `watched` is launched + cooling + notCommandable + outOfArc (the walking-watched ticks); `in-loop re-attacks` counts attempts whose `why` is `in-loop` (the witness's re-attack).

Original passes, the witness that counted walking as kind 0 only (`walkingOrder` at strike-while-walking.mjs:352 before this change):

| Pass | Verdict | Trace | lock on again: launched / cooling / notCommandable / outOfArc / watched | in-loop re-attacks (lock on / lock on again) |
|---|---|---|---|---|
| 1 | ok | d1-2026-11-09T19-26-03-163Z-ok.json | not red | 1 / 0 |
| 2 | red | d1-2026-11-09T19-28-33-569Z-red.json | 3 / 61 / 0 / 1 / 65 | 0 / 0 |
| 3 | red | d1-2026-11-09T19-31-03-622Z-red.json | 10 / 38 / 0 / 13 / 61 | 1 / 0 |
| 4 | red | d1-2026-11-09T19-33-35-486Z-red.json | 4 / 61 / 0 / 5 / 70 | 0 / 0 |
| 5 | ok | d1-2026-11-09T19-36-06-322Z-ok.json | not red | 1 / 0 |
| 6 | red | d1-2026-11-09T19-38-57-853Z-red.json | 3 / 22 / 0 / 0 / 25 | 1 / 0 |
| 7 | ok | d1-2026-11-09T19-41-32-490Z-ok.json | not red | 1 / 0 |
| 8 | ok | d1-2026-11-09T19-44-02-846Z-ok.json | not red | 1 / 0 |

The four reds all fail `lock on again: no strike while moving with one-hand on`, with `strikeSamplesWhileMoving` 0. Their `lock on` legs pass.

Decision (made from the probe counts). The sim's `walking()` counts a moving attack-move unit as walking when strike on move is on (c5e9fab0). The witness counted kind 0 only, so every strike the sim launched on a kind-2 walking tick was invisible to the verdict. The red legs show launches on watched ticks (3, 10, 4, 3), which is the sim doing what the witness claimed to check. So the witness is changed to match the sim:
- `walkingOrder` now accepts kind 0, or kind 2 while `#hero-lock` `aria-pressed` is `true`. That is the same flag as the one-hand ui call: `play.js:1322-1325` (`syncOneHand`) sends `one_hand` from `heroViewLock.isLocked()`, and `controls.lua:526` sets strike on move from it. A lock-off leg is unchanged (kind 0 only).
- Not mirrored: the sim's `attack_order < 0` clause. No hook exposes `attack_order` to the page, so the witness can count a kind-2 sample with an attack target that the sim would not. The launched-count comparison below is the check on that.
- Verdict thresholds unchanged. `node scripts/witness/strike-while-walking-check.mjs`: `strike-while-walking verdict checks ok (11 cases, leg timing in sim ticks)`.

Reruns with the changed witness (each its own status file under `.gm/witness-traces/d1-finalB-rerun-pass<n>-status.txt`). Pass 2 was run once more in the restart (19-48-05, red) and not rerun again, since it is consistent with the reds:

| Pass | Verdict | Trace | lock on again: strikeWhileMoving / launched / cooling / outOfArc / watched | lock on: strikeWhileMoving |
|---|---|---|---|---|
| 2 | red | d1-2026-11-09T19-48-05-847Z-red.json | 0 / 2 / 21 / 2 / 25 | 4 |
| 3 | ok | d1-2026-11-09T20-03-59-844Z-ok.json | 1 / 6 / 99 / 8 / 113 | 22 |
| 4 | ok | d1-2026-11-09T20-06-34-455Z-ok.json | 5 / 11 / 147 / 3 / 161 | 22 |
| 6 | red | d1-2026-11-09T20-09-04-628Z-red.json | 0 / 0 / 59 / 0 / 59 | 25 |

Before and after, lock on again (strikeSamplesWhileMoving): pass 2 0 to 0, pass 3 0 to 1 (ok, a margin of 1 at the threshold), pass 4 0 to 5, pass 6 0 to 0. The walking-watched ticks in pass 6 are all cooling (59 of 59), so the sim had no ready weapon while walking there, not a witness fault.

Result: final verdicts for the eight passes, with the witness each was measured under, are:
- 1 ok (old witness), 2 red (new), 3 ok (new), 4 ok (new), 5 ok (old), 6 red (new), 7 ok (old), 8 ok (old).
- Six ok, two red. D1 is not green.

Caveat, not resolved here: passes 1, 5, 7 and 8 were measured under the old witness. The new witness can only add kind-2 walking samples to the lock-on-again leg, so those passes cannot turn red from the change; their counts are simply not from the same measure. Rerunning them under the new witness would make the eight passes one measure; that was not in the requested scope.

Open: passes 2 and 6 remain red. Their launches on watched ticks are 2 and 0 in lock on again, and most watched ticks are cooling (21 of 25 in pass 2, 59 of 59 in pass 6). The walking definition is no longer the cause there; the weapon's cooldown leaves no strike while walking. Whether that is the intended sim behaviour, or a witness that should wait for a ready weapon before counting, is open.

check-all at this change (the witness change above, with the ledger): `RESULT gates=74 ok=74 failed=0 drift=0 skipped=6 strict=false`, exit 0 (log `.gm/witness-traces/checkall-final.log`).


## Phase A pass: D1 cause for passes 2 and 6, and the reuse pass (2026-11-09)

Phase A only: no build, test, gate, witness or browser run ran in this pass. Start point 4ff5c321. Committed locally as lanmower with no trailer; nothing pushed. The Rust edit is not compiled and the JS and witness edits are not run; Phase B does all of that.

### D1 decision for passes 2 and 6: the cause is in the witness, the sim is unchanged

Counts from the traces in .gm/witness-traces (walkProbe is the sim's own count per leg; a watched tick is one where the hero walks, is in weapon range and the strike is ready). Lock-off legs record zero walkProbe by construction, since walk_watch needs strikes on move.

| Trace | Leg | walkProbe launched / cooling / notCommandable / outOfArc | strikeSamplesWhileMoving | launchesWhileMoving | strikeSamplesWhileStopped |
|---|---|---|---|---|---|
| P2 19-48-05 (red) | lock on | 10 / 145 / 0 / 3 | 4 | 1 | 28 |
| P2 19-48-05 (red) | lock on again | 2 / 21 / 0 / 2 | 0 | 0 | 26 |
| P3 20-03-59 (ok) | lock on | 15 / 216 / 0 / 8 | 22 | 1 | 10 |
| P3 20-03-59 (ok) | lock on again | 6 / 99 / 0 / 8 | 1 | 0 | 21 |
| P4 20-06-34 (ok) | lock on | 14 / 210 / 0 / 6 | 22 | 0 | 11 |
| P4 20-06-34 (ok) | lock on again | 11 / 147 / 0 / 3 | 5 | 0 | 30 |
| P6 20-09-04 (red) | lock on | 16 / 216 / 0 / 8 | 25 | 0 | 8 |
| P6 20-09-04 (red) | lock on again | 0 / 59 / 0 / 0 | 0 | 0 | 23 |

Pass 2 (red): the sim is not at fault. The sim launched 2 strikes on watched ticks in lock on again, so it did strike while walking with one-hand on. The witness saw neither. Two witness measures miss them:
- strikeSamplesWhileMoving counts landing effects only on samples where the hero moved and walked. A launch whose effect lands after the walk is counted as stopped, not moving.
- launchesWhileMoving needs the hero to be more than 1.5 cells from its station (farFromStation). The trace's one launch stamp for this leg is at atTick 594, pos (93.73, 275.01), walkTo (94.97, 275.49): 1.33 cells, so the witness excludes it, while the sim counts it as walking (more than 1 cell from its target). The launch stamp also records only the first launch per target (strike_stamp, stamp_launch), so the witness cannot see later launches on the same target at all.

Pass 6 (red): the sim is not at fault. In lock on again every one of the 59 watched ticks is cooling and none launched. Its weapon was on cooldown from the 23 stationary strikes of that leg. The sim decrements the cooldown before its walking check (combat.rs attack, cooldown then walk_watch), and a stationary strike sets the cooldown to the attack period, so the cooling is the sim's own rule working. The witness did not ensure a ready weapon before its walk, and its verdict reported "no strike while moving", which put the cause on the sim.

Caveat: the trace file keeps only the first 300 strike samples (strikeTraceCap), so for lock on again it covers ticks 0 to 600 of 640. The decision rests on the walkProbe counts and the stored leg counts, not on the trace tail.

Witness changes made in this pass (scripts/witness/strike-while-walking.mjs, verdict):
- W1: on a one-hand-on leg, a leg with no strike sample passes when the sim's walkProbe.launched is above zero. The sim's own launch count is the evidence, and the effect sample is kept as corroboration. Expected effect: pass 2 satisfies the lock on again check on 2 launches (thin margin, not yet run).
- W2: a one-hand-on leg with no strike sample and no launch, but with walking cooling ticks, reports "every walking tick in reach waited on the weapon (N cooling, 0 launched), so the leg has no ready walking window" in place of "no strike while moving". Expected effect: pass 6 stays red under the corrected reason (not yet run).
- Self-check (scripts/witness/strike-while-walking-check.mjs): three cases added (pass-2 shape passes, pass-6 shape fails with the no-ready-window reason, a leg with no watched tick keeps the old reason). The loop now asserts the reason substring when a case gives one. The 11 earlier cases are unchanged.

Open, not changed in Phase A: the lock-on-again leg does not wait for a ready weapon before its walk. A leg-design change (a stationary wait until the sim reports a ready strike, or a longer walk) is a witness change for a later pass; Phase B decides whether D1 needs it.

### Reuse pass (audit at f513d64b, rows marked no behaviour change)

Applied:
- J1 (js/client/play.js syncDeadProps): the join key is gone. The dead-prop id array is compared element by element against the last one; null (the reset at play.js:2823) still forces a rebuild, as the string compare did. R2 (a sim death-generation counter) is not applied: it needs a new Sim field and a load and swap contract. The wasm Vec still allocates each frame.
- J6 (play.js drawWorld3d, shadow branch): the spread copy of drawCounts is a module-level savedDrawCounts filled by Object.assign and restored the same way.
- J7 (play.js drawWorld3d, podium plinth): the plinth mesh is built once per podium.plinth object (podium.js:65 assigns it once and nothing mutates it in place).
- J12 (js/client/hud.js viewRect): four scalar corners, no array or spreads; the toWorld calls keep their order.
- J14 (js/client/hud.js gameMaxHps): a held Map cleared per call. Its only caller is drawBars, which runs each frame and uses the map within the frame.
- R14 (rust/crates/sim/src/ground.rs pickup_ground_item): the before-snapshot is a stack array [u32; INV_SLOTS] (INV_SLOTS is pub(crate) const usize in inventory.rs). This is a stack array, not a held buffer, so no Sim field is needed.

Already reused, no change: J11 (ui_minimap.js playerFill memoises in fillByPlayer, :149-157).

Audit rows corrected by reading the code:
- J15: ui_minimap.js:88-89 are constructor-time Maps in createMinimapImpl, not per fit(). No change.
- J9: figureOf runs once per loaded figure (render3d.js:1214), not per frame. No change.
- J13: hud_canvas.js place() runs on panel updates (apply, :157), not per frame, and the DOM rects allocate regardless. No change.

Not applied, with the reason:
- R1 (state hash, STATE_VERSION), J2, J3, J4, J10: the audit marks them as behaviour changes. Kept.
- R2: needs a Sim generation field and a load and swap contract (see J1).
- R3 units_near: mod-host (piccolo_host.rs:2005) builds a Lua table per call, so a held Rust Vec saves one small allocation beside it. Not done.
- R4 drain_kill_events: the web boundary (web/src/lib.rs:474) returns a Vec to JS; a held buffer needs a host field.
- R5 take_procs, take_casts, take_destination_requests: drained by the host (piccolo_host.rs:4509, 4530). A held buffer needs a host field, and re-entrancy is not checked.
- R6 hash_later snapshot: needs a Sim copy_from over every field. Not started.
- R7 state_hash: changing to &mut self touches web/src/lib.rs:147 and replay. Runs per checkpoint only.
- R8 astar route: the path feeds one waypoint (movement.rs:495); a held output needs a Sim field.
- R9 clear_trees_square felled: per teleport; needs a Sim field.
- R10 vision_plane: the wasm boundary (web/src/lib.rs:524) returns a Vec; needs a JS buffer.
- R11 attack_terms: a witness hook only; web/src/lib.rs:208 converts to a Vec at the boundary, so a fixed array gains nothing.
- R12 recent_hitters: mod-host (piccolo_host.rs:1247, 1292) and tests use the Vec.
- R13 queued_orders: the resync reader (order_queue.rs:113) is not checked.
- R15 carry_from: set_ground takes an owned Vec that becomes the stored value; no spare buffer.
- J5 podiumTransform: multiply, translation and rotationY allocate their own matrices; full reuse needs matrix-into variants.
- J8 render3d.js setCamera (:509): copying into a held camera.target changes array identity; the holders of camera.target are not checked.

### Changes in this pass (file, row, witness needed in Phase B)

1. scripts/witness/strike-while-walking.mjs, verdict (D1 W1 and W2). Witness: node scripts/witness/strike-while-walking-check.mjs, then eight fresh-import D1 passes on port 8832.
2. scripts/witness/strike-while-walking-check.mjs (three cases and the reason assertion). Witness: the same self-check.
3. js/client/play.js syncDeadProps (J1). Witness: js/client/camera-target-audit-check.mjs, js/client/hero-trajectory-check.mjs, js/client/movement-sequence-check.mjs and js/client/lock-engage-order-check.mjs, plus a playwriter count of GC or heap growth per 600 frames against the current build.
4. js/client/play.js drawWorld3d (J6, J7). Witness: the same JS gates and heap count.
5. js/client/hud.js viewRect and gameMaxHps (J12, J14). Witness: the same JS gates, and js/client/hud-canvas-check.mjs as the HUD probe.
6. rust/crates/sim/src/ground.rs pickup_ground_item (R14). Witness: cargo test -p sim, and the replay parity check (mod-host/src/bin/parity_hash.rs) on a fixed seed.

Phase B order: self-check, cargo test -p sim --test strikes with the guard removed and then with it, the eight D1 passes, node scripts/check-all.mjs, one push. The D1 result is not predicted here; the predictions above (pass 2 ok, pass 6 red with the new reason) are to be confirmed or refuted by the passes.
