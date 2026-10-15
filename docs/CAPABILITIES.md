# Capabilities: parity checklist for mods on the oscraft server

Measure: a capability is at parity when its row is `implemented` and its gate check passes with placed units. Presentation rows count as parity when they are `unsupported-headless` with a named refusal.

Status values: `implemented`, `partial`, `stub` (no code), `unsupported-headless`.
Paths are relative to `rust/crates/` unless shown otherwise. Gate names are planned checks; no gate check exists yet (`rust/crates/gate/src/main.rs` is a placeholder).

## Map and terrain

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-MAP-1 | Load a map grid and its walkability from map data | implemented | sim/src/arrays.rs (set_grid, grid), sim/src/loader.rs (load_file) | gate:load-map |
| CAP-MAP-2 | Report walkability of a cell and find the nearest walkable cell | implemented | sim/src/pathfinding.rs (walkable, nearest_walkable) | gate:walkable-cells |
| CAP-MAP-3 | Edit terrain at runtime from a mod | implemented | sim/src/arrays.rs (set_cell_blocked); sim/src/pathfinding.rs (Grid::set_blocked, Pool::invalidate) | gate:edit-terrain |
| CAP-MAP-4 | Clamp positions to map bounds | implemented | sim/src/arrays.rs (clamp_to_grid) | gate:map-bounds |

## Units and their stats

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-UNIT-1 | Spawn a unit of a catalog kind for a player at fixed-point coordinates | implemented | sim/src/arrays.rs (spawn, spawn_for); mod-host/src/lib.rs (spawn_unit) | gate:spawn-units |
| CAP-UNIT-2 | Read a unit's position, hp, owner and alive state | implemented | sim/src/arrays.rs (position_fixed, hp, player, alive) | gate:unit-stats-read |
| CAP-UNIT-3 | Load a unit catalog with per-kind stats | implemented | sim/src/loader.rs (load, unit_index) | gate:catalog-load |
| CAP-UNIT-4 | Classify units as worker, structure, air or projectile | implemented | sim/src/loader.rs (is_worker, is_structure, is_air, is_projectile) | gate:unit-classes |

## Abilities and effects

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-ABIL-1 | Cast an ability gated by cooldown and energy | implemented | sim/src/rules.rs (cast_ability; refusals Cooldown, Energy) | gate:cast-ability |
| CAP-ABIL-2 | Check target filter, range and move range on a cast | implemented | sim/src/rules.rs (meets_filter, within; refusals TargetFilter, MoveRange) | gate:ability-target-filter |
| CAP-ABIL-3 | Apply an effect list: damage, heal, move, spawn, buff, order, remove behaviour, shield bonus, vital fraction, create healer, calldown | implemented | sim/src/rules.rs (apply_effect) | gate:ability-effects |
| CAP-ABIL-4 | Refuse an effect kind with no implementation under a named refusal | implemented | sim/src/rules.rs (refuse, Refusal::EffectUnsupported) | gate:unsupported-effect-refuses |

## Behaviours and buffs

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-BEH-1 | Apply a timed buff to a unit and count it down each tick | implemented | sim/src/buffs.rs (apply_buff, tick_buffs) | gate:apply-buff |
| CAP-BEH-2 | Clear a buff from a unit | implemented | sim/src/buffs.rs (clear_buff) | gate:clear-buff |
| CAP-BEH-3 | Remove a behaviour from a unit through an effect | implemented | sim/src/buffs.rs (remove_behaviour); sim/src/rules.rs | gate:remove-behaviour |
| CAP-BEH-4 | Declare behaviours with a trigger (idle, attacked, spawned, ability_ready) in data | implemented | sim/src/loader.rs (Trigger, trigger_kind; BHVR parse); data/src/model.rs | gate:behaviour-declared |
| CAP-BEH-5 | Fire behaviour triggers during the tick | implemented | sim/src/triggers.rs (fire_triggers, note_idle, run_behaviours); sim/src/tick.rs | gate:behaviour-trigger-fires |

## Orders and pathing

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-ORD-1 | Parse and bounds-check a player order body (player, x, y) | implemented | server/src/order.rs (parse_order) | gate:order-parse |
| CAP-ORD-2 | Issue a move order to a set of units | implemented | sim/src/arrays.rs (order, order_all); sim/src/movement.rs (issue_order) | gate:issue-move |
| CAP-ORD-3 | Steer and integrate unit movement each tick | implemented | sim/src/movement.rs (steer, integrate) | gate:move-units-tick |
| CAP-ORD-4 | Path to a goal with shared flow fields | implemented | sim/src/pathfinding.rs (request, field, step, ready_count, cap) | gate:path-to-goal |
| CAP-ORD-5 | Stop, attack-move, patrol and hold orders | implemented | sim/src/movement.rs (issue_order_kind, advance_patrol); sim/src/tick.rs (hold skips movement) | gate:order-kinds |
| CAP-ORD-6 | Rate-limit and authorize orders per player | implemented | server/src/order.rs (rate, authorized, take) | gate:order-rate-limit |

## Production and resources

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-PROD-1 | Produce a unit from a building over time | implemented | sim/src/production.rs (produce_step) | gate:produce-unit |
| CAP-PROD-2 | Gather minerals and vespene through harvest steps | implemented | sim/src/harvest.rs (harvest_step, retarget_scan, arrived); sim/src/arrays.rs (add_resources, minerals, vespene) | gate:harvest-resources |
| CAP-PROD-3 | Deduct production costs before starting a unit | implemented | sim/src/production.rs (produce_step pays before the train starts; refuses cannot-afford); sim/src/arrays.rs (stats.trains_refused) | gate:spend-resources |
| CAP-PROD-4 | Cancel a queued unit with refund | implemented | sim/src/production.rs (cancel_train, resume_train; full refund) | gate:cancel-production |

## Combat and damage

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-CMB-1 | Single-target attack with damage and cooldown | implemented | sim/src/combat.rs (attack, damage_target, hurt) | gate:attack-unit |
| CAP-CMB-2 | Area damage around a point | implemented | sim/src/combat.rs (damage_area) | gate:area-damage |
| CAP-CMB-3 | Ranged damage through launched and ticked projectiles | implemented | sim/src/projectiles.rs (launch_missile, tick_projectiles, free_missile) | gate:projectile-hit |
| CAP-CMB-4 | Heal units over time through healers | implemented | sim/src/healers.rs (heal_area, heal_target, tick_healers) | gate:heal-units |

## Shields and armour

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-SHLD-1 | Absorb incoming damage with shields | implemented | sim/src/shields.rs (absorb_shield) | gate:shield-absorbs |
| CAP-SHLD-2 | Grant a shield bonus through an effect | implemented | sim/src/shields.rs (shield_bonus) | gate:shield-bonus |
| CAP-SHLD-3 | Reduce incoming damage with armour | implemented | sim/src/combat.rs (armour_reduced, used by attack; weapon hits only) | gate:armour-reduces |
| CAP-SHLD-4 | Apply a vital-fraction effect | implemented | sim/src/shields.rs (vital_fraction) | gate:vital-fraction |

## Players and alliances

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-PLR-1 | Own units by player index (16 players) | implemented | sim/src/arrays.rs (player); server/src/order.rs (ORDER_PLAYERS) | gate:player-ownership |
| CAP-PLR-2 | Report per-player unit presence and resources | implemented | server/src/room.rs (player_has_units, stats); server/src/main.rs (resources) | gate:player-resources |
| CAP-PLR-3 | Alliances and team-based target filtering | implemented | sim/src/arrays.rs (set_team, set_alliance, hostile); sim/src/combat.rs, healers.rs, buffs.rs (hostile test) | gate:alliance-filter |

## Regions, triggers and events

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-EVT-1 | Define rectangular regions on the map | implemented | mod-host/src/lib.rs (define_region, in_region) | gate:region-define |
| CAP-EVT-2 | Fire trigger zones on unit enter and leave | implemented | mod-host/src/lib.rs (advance, collect, dispatch) | gate:trigger-zone |
| CAP-EVT-3 | Deliver game events (spawned, attacked, died) to scripts | implemented | mod-host/src/lib.rs (collect, dispatch) | gate:event-delivery |

## Timers and scheduling

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-TIME-1 | Advance the sim at a fixed tick rate | implemented | sim/src/tick.rs (tick); server/src/room.rs (spawn_ticker) | gate:tick-rate |
| CAP-TIME-2 | Per-unit cooldowns and timers | implemented | sim/src/tick.rs (tick_cooldowns); sim/src/arrays.rs (wtimer, healer_timer) | gate:cooldown-expires |
| CAP-TIME-3 | Named delayed actions scheduled by mods | implemented | mod-host/src/lib.rs (schedule, cancel, fire_timers) | gate:scheduled-action |

## Script runtime (Lua)

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-LUA-1 | Load a mod file into a fresh environment and bind on_tick | implemented | mod-host/src/lib.rs (load, compile) | gate:lua-load |
| CAP-LUA-2 | Reload the mod when its file changes | implemented | mod-host/src/lib.rs (reload_if_changed) | gate:lua-reload |
| CAP-LUA-3 | Keep a state table across ticks | implemented | mod-host/src/lib.rs (state, tick) | gate:lua-state |
| CAP-LUA-4 | Report script errors with path and line | implemented | mod-host/src/lib.rs (compile, error format) | gate:lua-error-location |
| CAP-LUA-5 | Host functions spawn_unit and get_unit_count | implemented | mod-host/src/lib.rs (bind_host) | gate:lua-host-api |
| CAP-LUA-6 | Host functions for orders, damage, buffs and queries | partial | mod-host/src/lib.rs (order_unit, order_player, unit_position, unit_hp, unit_player, unit_alive); damage and buffs need a public sim API, Sim::damage_target and Sim::apply_buff are pub(crate) | gate:lua-host-api-full |
| CAP-LUA-7 | Instruction and memory limits for mod scripts | implemented | mod-host/src/lib.rs (set_limits, named, instruction hook) | gate:lua-sandbox-limits |

## Server surface

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-SRV-1 | Return the room snapshot with GET /state | implemented | server/src/main.rs (route, serve_state) | gate:http-state |
| CAP-SRV-2 | Return the unit and ability catalog with GET /catalog | implemented | server/src/main.rs (serve_catalog) | gate:http-catalog |
| CAP-SRV-3 | Accept player orders with POST /order under auth and rate limit | implemented | server/src/main.rs (serve_order); server/src/order.rs | gate:http-order |
| CAP-SRV-4 | Return changes since a tick with GET /delta | partial | sim/src/arrays.rs (changed_since, changed_count_since); sim/src/tick.rs (marks); server/src/main.rs (serve_delta still returns delta_unsupported) | gate:http-delta |
| CAP-SRV-5 | Serve several rooms per process (--room, --max-rooms) | implemented | server/src/main.rs (parse_args, rooms) | gate:multi-room |
| CAP-SRV-6 | Report process and room stats with GET /resources | implemented | server/src/main.rs (resources) | gate:http-resources |

## Determinism and state hashing

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-DET-1 | Fixed-point (Q16.16) arithmetic for sim math | implemented | sim/src/fixed.rs | gate:fixed-math |
| CAP-DET-2 | FNV-1a state hash over sim state | implemented | sim/src/hash.rs (state_hash); mod-host/src/lib.rs (state_hash) | gate:state-hash-stable |
| CAP-DET-3 | Identical mod and state reproduce identical hashes across runs | partial | sim/src/hash.rs; no replay gate yet | gate:replay-hash-equal |

## Persistence of map data

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-PERS-1 | Read a map data image and check its version | implemented | data/src/reader.rs (check_version, verify; ImageError::VersionMismatch) | gate:map-image-read |
| CAP-PERS-2 | Write map data to a versioned image | implemented | data/src/encode.rs (sections, assemble); data/src/reader.rs (read_sections); data/src/lib.rs (check_round_trip) | gate:map-image-roundtrip |
| CAP-PERS-3 | Save and load live sim state | stub | none | gate:save-load-state |

## Presentation (headless)

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-PRES-1 | Camera control | unsupported-headless | refusal name not yet in code | gate:presentation-refusal |
| CAP-PRES-2 | Sound playback | unsupported-headless | refusal name not yet in code | gate:presentation-refusal |
| CAP-PRES-3 | UI panels and dialogs | unsupported-headless | refusal name not yet in code | gate:presentation-refusal |
| CAP-PRES-4 | Chat messages | unsupported-headless | refusal name not yet in code | gate:presentation-refusal |
