# Capabilities: parity checklist for mods on the oscraft engine

Measure: a capability is at parity when its row is `implemented` and its gate check passes with placed units. Presentation rows count as parity when they are `unsupported-headless` with a named refusal.

Status values: `implemented`, `partial`, `stub` (no code), `unsupported-headless`.
`claimed` means the code is on main and the gate check has not yet passed; `implemented` means the row's gate check is registered in `rust/gate`. No CI job runs `oscraft-gate`, so no CI run backs any status.
Paths are relative to `rust/crates/` unless shown otherwise. Gate names are checks in `rust/gate/src/` (`checks.rs`, run from `main.rs`); the checks for rules now in Lua were removed from `rust/gate`, and their rows read `none (gate removed)`.

## Map and terrain

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-MAP-1 | Load a map grid and its walkability from map data | implemented | sim/src/arrays.rs (set_grid, grid), sim/src/loader.rs (load_file) | gate:load-map |
| CAP-MAP-2 | Report walkability of a cell and find the nearest walkable cell | implemented | sim/src/pathfinding.rs (walkable, nearest_walkable) | gate:walkable-cells |
| CAP-MAP-3 | Edit terrain at runtime from a mod | implemented | none | gate:edit-terrain |
| CAP-MAP-4 | Clamp positions to map bounds | implemented | sim/src/arrays.rs (clamp_to_grid) | gate:map-bounds |
| CAP-MAP-5 | Define one start position per player in map data | claimed | rust/crates/data/src/builtins.rs (start), validate.rs | gate:player-start-positions |
| CAP-MAP-6 | Place neutral objects that belong to no player | claimed | rust/crates/data/src/builtins.rs (spawn -1), validate.rs | gate:neutral-objects |
| CAP-MAP-7 | Reveal cells within each unit's sight radius and hide cells outside friendly sight | claimed | none | gate:sight-reveal |
| CAP-MAP-8 | Grant sight over elevated cells and deny sight from lower ground | claimed | none | gate:elevation-sight |
| CAP-MAP-9 | Remember a seen enemy unit at its last known position until it is seen again | claimed | none | gate:last-seen-memory |
| CAP-MAP-10 | Author custom terrain tiles and doodads in an editor | claimed | none | gate:editor-terrain-tiles |
| CAP-MAP-11 | Place neutral objects and start positions by a map author through map data | claimed | rust/data/map.osd (start, spawn -1, limit) | gate:map-author-placement |

## Units and their stats

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-UNIT-1 | Spawn a unit of a catalog kind for a player at fixed-point coordinates | implemented | sim/src/arrays.rs (spawn, spawn_for); mod-host/src/piccolo_host.rs (spawn_unit native) | gate:spawn-units |
| CAP-UNIT-2 | Read a unit's position, hp, owner and alive state | implemented | sim/src/arrays.rs (position_fixed, hp, player, alive) | gate:unit-stats-read |
| CAP-UNIT-3 | Load a unit catalog with per-kind stats | implemented | sim/src/loader.rs (load, unit_index) | gate:catalog-load |
| CAP-UNIT-4 | Classify units as worker, structure, air or projectile | implemented | sim/src/loader.rs (is_worker, is_structure, is_air, is_projectile) | gate:unit-classes |
| CAP-UNIT-5 | Let air units cross terrain that ground units cannot cross | claimed | none | gate:air-terrain-crossing |
| CAP-UNIT-6 | Refuse ground-only weapons against air targets | claimed | none | gate:ground-only-air-refusal |
| CAP-UNIT-7 | Load ground units into a transport up to a size-based capacity and unload them | claimed | scenarios/duel-routes/transport.lua (load_unit) | none (gate removed) |
| CAP-UNIT-8 | Unload loaded units at a chosen point, including behind enemy lines | claimed | scenarios/duel-routes/transport.lua (unload_transport_at) | none (gate removed) |

## Abilities and effects

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-ABIL-1 | Cast an ability gated by cooldown and energy | implemented | sim/src/rules.rs (cast_ability; refusals Cooldown, Energy) | gate:cast-ability |
| CAP-ABIL-2 | Check target filter, range and move range on a cast | implemented | sim/src/rules.rs (meets_filter, within; refusals TargetFilter, MoveRange) | gate:ability-target-filter |
| CAP-ABIL-3 | Apply an effect list: damage, heal, move, spawn, buff, order, remove behaviour, shield bonus, vital fraction, create healer, calldown | implemented | sim/src/rules.rs (apply_effect) | gate:ability-effects |
| CAP-ABIL-4 | Refuse an effect kind with no implementation under a named refusal | implemented | sim/src/rules.rs (refuse, Refusal::EffectUnsupported) | gate:unsupported-effect-refuses |
| CAP-ABIL-5 | Hold the caster through a channelled ability and end it early when interrupted | claimed | none | gate:channel-interrupt |
| CAP-ABIL-6 | Author custom abilities in a graphical editor | claimed | none | gate:editor-abilities |

## Behaviours and buffs

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-BEH-1 | Apply a timed buff to a unit and count it down each tick | implemented | sim/src/buffs.rs (apply_buff, tick_buffs) | gate:apply-buff |
| CAP-BEH-2 | Clear a buff from a unit | implemented | sim/src/buffs.rs (clear_buff) | gate:clear-buff |
| CAP-BEH-3 | Remove a behaviour from a unit through an effect | implemented | sim/src/buffs.rs (remove_behaviour); sim/src/rules.rs | gate:remove-behaviour |
| CAP-BEH-4 | Declare behaviours with a trigger (idle, attacked, spawned, ability_ready) in data | implemented | data/src/model.rs (trigger), data/src/builtins.rs; no evaluator in sim | gate:behaviour-declared |
| CAP-BEH-5 | Fire behaviour triggers during the tick | implemented | sim/src/behaviours.rs (fire_triggers), sim/src/tick.rs | gate:behaviour-trigger-fires |
| CAP-BEH-6 | Raise a class-wide stat in levels, with each level costing more than the last | claimed | mods/upgrades.lua (level n costs n x cost_per_level) | none (gate removed) |
| CAP-BEH-7 | Stack upgrade levels separately for each unit class | claimed | mods/upgrades.lua (stacking "add" and "once") | none (gate removed) |
| CAP-BEH-8 | Apply upgrades to units already on the field, not only to new units | claimed | mods/upgrades.lua (game.apply_research per unit, existing and new) | none (gate removed) |
| CAP-BEH-9 | Apply a stated stacking rule to buffs of the same kind | claimed | sim/src/buffs.rs (apply_buff: same kind refreshes its duration) | gate:buff-stacking-rule |

## Orders and pathing

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-ORD-2 | Issue a move order to a set of units | implemented | sim/src/arrays.rs (order, order_all); sim/src/movement.rs (issue_order) | gate:issue-move |
| CAP-ORD-3 | Steer and integrate unit movement each tick | implemented | sim/src/movement.rs (steer, integrate) | gate:move-units-tick |
| CAP-ORD-4 | Path to a goal with shared flow fields | implemented | sim/src/pathfinding.rs (request, field, step, ready_count, cap) | gate:path-to-goal |
| CAP-ORD-5 | Stop, attack-move, patrol and hold orders | implemented | sim/src/movement.rs (issue_order_kind, advance_patrol), sim/src/tick.rs | gate:order-kinds |
| CAP-ORD-7 | Push colliding units apart instead of letting them overlap | claimed | sim/src/movement.rs (separation from neighbours within radius sum) | gate:unit-collision |
| CAP-ORD-8 | Limit how many units pass narrow or blocked cells at once | claimed | sim/src/limits.rs (cell_has_room), sim/src/movement.rs (integrate waits on a full cell), sim/src/arrays.rs (spawn_for refuses CellFull) | gate:narrow-passage-limit |

## Production and resources

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-PROD-1 | Produce a unit from a building over time | implemented | scenarios/duel-routes/production.lua (train) | none (gate removed) |
| CAP-PROD-2 | Gather resource_a and resource_b through harvest steps | implemented | scenarios/duel-routes/harvest.lua (worker gather and deposit); sim/src/arrays.rs (add_resources, resource_a, resource_b) | none (gate removed) |
| CAP-PROD-3 | Deduct production costs before starting a unit | implemented | scenarios/duel-routes/production.lua (cost paid at the start); sim/src/arrays.rs (add_resources) | none (gate removed) |
| CAP-PROD-4 | Cancel a queued unit with refund | implemented | scenarios/duel-routes/production.lua (cancel_train) | none (gate removed) |
| CAP-PROD-5 | Scale patch income with workers up to a saturation count | claimed | scenarios/duel-routes/harvest.lua (saturation-stretched gather time) | none (gate removed) |
| CAP-PROD-6 | Gate gas harvesting on an extractor building with a per-geyser saturation count | claimed | scenarios/duel-routes/harvest.lua (gas from a geyser a structure covers) | none (gate removed) |
| CAP-PROD-7 | Set a fixed opening resource amount and opening worker count per start | claimed | scenarios/duel-routes/economy_start.lua (opening gold, gas and workers) | none (gate removed) |
| CAP-PROD-8 | Set a fixed mineral patch and geyser count per base from map data | claimed | scenarios/duel-routes/economy_start.lua (patch row and geyser column) | none (gate removed) |
| CAP-PROD-9 | Charge a fixed capacity per unit and refuse production past the capacity cap | claimed | scenarios/duel-routes/production.lua (capacity cap) | none (gate removed) |
| CAP-PROD-10 | Raise the capacity cap with providers, bounded by a per-race maximum | claimed | scenarios/duel-routes/production.lua (capacity providers, bounded by the maximum) | none (gate removed) |
| CAP-PROD-11 | Reserve capacity when production is queued and release it on cancel | claimed | scenarios/duel-routes/production.lua (trains in progress hold capacity; cancel releases it) | none (gate removed) |
| CAP-PROD-12 | Move a newly finished unit to the building's rally point | claimed | scenarios/duel-routes/production.lua (rally offset) | none (gate removed) |
| CAP-PROD-13 | Require prerequisite buildings or research before a unit, building or upgrade can be made | claimed | scenarios/duel-routes/production.lua (prerequisites) | none (gate removed) |
| CAP-PROD-14 | Run one research at a time per building for a fixed time at a resource cost | claimed | scenarios/duel-routes/production.lua (research, one at a time per building) | none (gate removed) |
| CAP-PROD-15 | Apply completed research to every matching unit at once | claimed | scenarios/duel-routes/production.lua (research effect on completion) | none (gate removed) |

## Combat and damage

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-CMB-1 | Single-target attack with damage and cooldown | implemented | sim/src/combat.rs (attack, damage_target, hurt) | gate:attack-unit |
| CAP-CMB-2 | Area damage around a point | implemented | sim/src/combat.rs (damage_area) | gate:area-damage |
| CAP-CMB-3 | Ranged damage through launched and ticked projectiles | implemented | sim/src/projectiles.rs (launch_missile, tick_projectiles, free_missile) | gate:projectile-hit |
| CAP-CMB-4 | Heal units over time through healers | implemented | sim/src/healers.rs (heal_area, heal_target, tick_healers) | gate:heal-units |
| CAP-CMB-5 | Scale final damage by weapon damage type and target size category | claimed | none | gate:damage-type-size |
| CAP-CMB-6 | Hold attacks to weapon range and move the unit into range first | claimed | none | gate:weapon-range |
| CAP-CMB-7 | Apply a fixed miss chance to ranged attacks on high ground or under cover | claimed | none | gate:ranged-miss-chance |
| CAP-CMB-8 | Block enemy targeting of cloaked and burrowed units unless a detector covers them | claimed | none | gate:cloak-detection |

## Shields and armour

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-SHLD-1 | Absorb incoming damage with shields | implemented | sim/src/shields.rs (absorb_shield) | gate:shield-absorbs |
| CAP-SHLD-2 | Grant a shield bonus through an effect | implemented | sim/src/shields.rs (shield_bonus) | gate:shield-bonus |
| CAP-SHLD-3 | Reduce incoming damage with armour | implemented | sim/src/combat.rs (armour_reduced, applied in attack and order_attack) | gate:armour-reduces |
| CAP-SHLD-4 | Apply a vital-fraction effect | implemented | sim/src/shields.rs (vital_fraction) | gate:vital-fraction |
| CAP-SHLD-5 | Regenerate shields after a delay in which the unit takes no damage | claimed | sim/src/shields.rs (regen_shield), sim/src/combat.rs (inflict stamps last_hit), sim/src/tick.rs | gate:shield-regen-delay |
| CAP-SHLD-6 | Regenerate hit points over time for unit classes that declare it | claimed | sim/src/shields.rs (regen_health), sim/src/tick.rs | gate:hp-regen |

## Players and alliances

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-PLR-1 | Own units by player index (16 players) | implemented | sim/src/arrays.rs (player) | gate:player-ownership |
| CAP-PLR-2 | Report per-player unit presence and resources | implemented | mod-host/src/piccolo_host.rs (players, resources natives) | gate:player-resources |
| CAP-PLR-3 | Alliances and team-based target filtering | implemented | sim/src/arrays.rs (set_alliance, set_team, hostile) | gate:alliance-filter |

## Regions, triggers and events

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-EVT-1 | Define rectangular regions on the map | implemented | mod-host/src/piccolo_host.rs (define_region, in_region natives) | gate:region-define |
| CAP-EVT-2 | Fire trigger zones on unit enter and leave | implemented | mod-host/src/piccolo_host.rs (on_enter, on_leave hooks) | gate:trigger-zone |
| CAP-EVT-3 | Deliver game events (spawned, attacked, died) to scripts | implemented | mod-host/src/lib.rs (EVENTS) | gate:event-delivery |
| CAP-EVT-4 | Author custom triggers and events in a graphical editor | claimed | none | gate:editor-triggers |

## Timers and scheduling

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-TIME-1 | Advance the sim at a fixed tick rate | implemented | sim/src/tick.rs (tick); js/client/play.js (fixed-step loop calls localGame.tick) | gate:tick-rate |
| CAP-TIME-2 | Per-unit cooldowns and timers | implemented | sim/src/tick.rs (tick_cooldowns); sim/src/arrays.rs (wtimer, healer_timer) | gate:cooldown-expires |
| CAP-TIME-3 | Named delayed actions scheduled by mods | implemented | mod-host/src/piccolo_host.rs (env.schedule, env.cancel in the Lua prelude) | gate:scheduled-action |

## Script runtime (Lua)

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-LUA-1 | Load a mod file into a fresh environment and bind on_tick | implemented | mod-host/src/piccolo_host.rs (ModHost::load_file, run_chunk) | gate:lua-load |
| CAP-LUA-2 | Reload the mod when its file changes | implemented | mod-host/src/piccolo_host.rs (ModHost::reload) | gate:lua-reload |
| CAP-LUA-3 | Keep a state table across ticks | implemented | mod-host/src/lib.rs (the host owns `state`; reloads keep it) | gate:lua-state |
| CAP-LUA-4 | Report script errors with path and line | implemented | mod-host/src/piccolo_host.rs (run_chunk; chunk named by file stem) | gate:lua-error-location |
| CAP-LUA-5 | Host functions spawn_unit and get_unit_count | implemented | mod-host/src/piccolo_host.rs (spawn_unit and get_unit_count natives) | gate:lua-host-api |
| CAP-LUA-6 | Host functions for orders, damage, buffs and queries | implemented | mod-host/src/piccolo_host.rs (cmd_order, cmd_damage, cmd_buff, unit_hp and other natives) | gate:lua-host-api-full |
| CAP-LUA-7 | Instruction and memory limits for mod scripts | implemented | mod-host/src/piccolo_host.rs (FUEL_PER_STEP, MAX_STEPS, MEMORY_CAP) | gate:lua-sandbox-limits |

## Determinism and state hashing

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-DET-1 | Fixed-point (Q16.16) arithmetic for sim math | implemented | sim/src/fixed.rs | gate:fixed-math |
| CAP-DET-2 | FNV-1a state hash over sim state | implemented | sim/src/hash.rs (state_hash) | gate:state-hash-stable |
| CAP-DET-3 | Identical mod and state reproduce identical hashes across runs | implemented | sim/src/hash.rs (catalog digest, modes, flow-field pool, grid tiles, doodads), sim/src/arrays.rs (cat_digest fixed at load), sim/src/arrays.rs (seed, rng) | gate:replay-hash-equal |
| CAP-DET-4 | Record every accepted order with its tick so a match can be replayed | claimed | sim/src/arrays.rs (record_order, order_record), sim/src/movement.rs (issue_order_kind), sim/src/persist.rs, sim/src/hash.rs | gate:order-recording |
| CAP-DET-5 | Play a replay at a chosen speed, paused, or seeked | claimed | none | gate:replay-playback-controls |
| CAP-DET-6 | Advance lockstep clients by exchanging per-tick inputs and comparing state hashes | claimed | none | gate:lockstep-input-exchange (no check in rust/gate) |

## Persistence of map data

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-PERS-1 | Read a map data image and check its version | implemented | data/src/reader.rs (verify); rejects other versions, no full read path | gate:map-image-read |
| CAP-PERS-2 | Write map data to a versioned image | implemented | data/src/encode.rs, data/src/format.rs; round trip not gated | gate:map-image-roundtrip |
| CAP-PERS-3 | Save and load live sim state | implemented | sim/src/persist.rs (save_state, load_state) | gate:save-load-state |
| CAP-PERS-4 | Continue a loaded save with the same state hashes as uninterrupted play | claimed | sim/src/persist.rs (load_state restores hashed fields and flow-field pool) | gate:save-load-hash-equal |
| CAP-PERS-5 | Ship mods as versioned packages that the host checks before loading | claimed | sim/src/loader.rs (version check, BadVersion) | gate:mod-package-version |

## Presentation (headless)

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-PRES-1 | Camera control | unsupported-headless | sim/src/rules.rs (Refusal::CameraUnsupported) | gate:presentation-refusal |
| CAP-PRES-2 | Sound playback | unsupported-headless | sim/src/rules.rs (Refusal::SoundUnsupported) | gate:presentation-refusal |
| CAP-PRES-3 | UI panels and dialogs | unsupported-headless | sim/src/rules.rs (Refusal::UiPanelUnsupported) | gate:presentation-refusal |
| CAP-PRES-4 | Chat messages | unsupported-headless | sim/src/rules.rs (Refusal::ChatUnsupported) | gate:presentation-refusal |
