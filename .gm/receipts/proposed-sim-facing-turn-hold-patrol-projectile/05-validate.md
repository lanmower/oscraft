# 05 validate: proposed-sim-facing-turn-hold-patrol-projectile

session: gm-drain-1793526214-v3 (witness lane, not executor)
sha: f63d15a0f28483b02f3232eeea0cf6f003570642 (origin/main tip, receipts-only commit; ancestor of origin/main)
witness: exec_js 1793528715411-4568-b3bdfa4c65c5b2ee (git show <sha>:path symbol probe)
companion: exec_js 1793528748378-4597-47d372dfb2091c4c (HEAD e56dd0b6 vs origin/main: rust/ scenarios/ identical; only js/client/widgets.js differs)
timestamp: 2026-11-01T10:26:34.310Z
local checkout: HEAD e56dd0b6, 4 behind origin/main (stale_tree); not moved, no edits made.

## clauses (expected, then observed at sha)
1 facing/turn: expect set_turn_rate, turn_toward in facing.rs; turn_rates.lua generated; unit_rules applies it. observed: all present. live run: NONE (presence only).
2 hold fire: expect holds_fire in combat.rs; gate hold check. observed: combat.rs:167 present; gate hold_fire at mod_checks.rs:404. live run: NONE.
3 patrol: expect patrol.lua ORDER_PATROL, controls patrol action. observed: present; sim advance_patrol movement.rs:518; gate sim_checks.rs:172. live run: NONE.
4 backswing: expect attack_backswing recover + attack-point cancel in swing.rs. observed: present. gate/test coverage: none found. live run: NONE.
5 homing: expect missile_homing + fly_missile in swing.rs. observed: present. basic-attack live test: none (mod-host trigger_hero.rs:302 covers spell homing only).
6 hero acquire: expect hero_step, acquire_of, wave_creep_acquire in lanes.lua. observed: present (acquire.rs:43). live run: NONE.
7 projectile arc: row subject lists "projectile arc and homing" under "sim gaps". expect a sim consumer of missile_arc. observed: parsed in loader.rs and data crate only; no tick, combat, movement or hash reader. combat-parity row 17 says arc has no outcome effect. VERDICT: not satisfied as written; needs owner decision (implement, or hy3d amends row to display-only).

## closure
open. Not closed. Items 1-6 are code-present at the sha; none has a live execution witness in this lane. Item 7 is open per the row text.
