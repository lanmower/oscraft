# 02 eliminate: proposed-sim-facing-turn-hold-patrol-projectile

session: gm-drain-1793526214-22
JIT run: exec_js 1793528491181-4451-c30185ea62a1ea48 (reads rust/crates/sim/src/*.rs and scenarios/moba/lanes.lua from disk).

Two-sided instrument: control_negative_must_be_false = false (a nonexistent symbol is not found), control_positive_must_be_true = true (the wave_creep_acquire definition and its tick call are found). The instrument can fail, so its true results count.

## Results

- U1 landed items present on the tree (sim_files_scanned 53):
  - facing and turn rate: set_turn_rate defined (facing.rs), turn_step read by 4 sim sites. Landed.
  - hold fire: holds_fire defined (combat.rs:167). Landed.
  - patrol: scenarios/moba/patrol.lua exists. Landed.
  - backswing recover from attack_backswing: true (swing.rs). Landed.
  - homing retarget each tick: true (swing.rs fly_missile). Landed.
  - hero idle chase: wave_creep_acquire called on each tick with game.ticks() (lanes.lua:363); hero_step uses game.acquire_of (lanes.lua:345). Landed.
- U2 missile_arc reaches the sim: found in rust/crates/sim/src/loader.rs only (field parse and store). No tick, combat, movement or hash path reads it. Display-only.

## Disposition

- Leader hypothesis: every sim item of the row is already on main; the only open item, projectile arc, is display-only and has no sim outcome (combat-parity row 17; sim never reads display data).
- The probe above separates the leader from the alternative (an item missing from the tree): the alternative would show false for any landed item, and all six landed checks were true.
- No new unknown opened. Nothing blocked.
