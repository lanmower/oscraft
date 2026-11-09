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
