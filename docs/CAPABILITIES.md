# Capabilities: parity checklist for mods on the oscraft engine

Measure: a capability is at parity when its row is `implemented` and its gate check passes with placed units. Presentation rows count as parity when they are `unsupported-headless` with a named refusal.

Status values: `implemented`, `partial`, `stub` (no code), `unsupported-headless`.
`claimed` means the code is on main and the gate check has not yet passed; `implemented` means the row's gate check is registered in `rust/gate`. The `oscraft-gate` job of `.github/workflows/tests.yml` (workflow_dispatch) runs the gate, so a status is backed only by a run of that job that passed the row's check.
Paths are relative to `rust/crates/` unless shown otherwise. Gate names are checks in `rust/gate/src/` (`checks.rs`, run from `main.rs`); the checks for rules now in Lua were removed from `rust/gate`, and their rows read `none (gate removed)`.

## Map and terrain

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-MAP-1 | Load a map grid and its walkability from map data | implemented | sim/src/arrays.rs (set_grid, grid), sim/src/loader.rs (load_file) | gate:load-map |
| CAP-MAP-2 | Report walkability of a cell and find the nearest walkable cell | implemented | sim/src/pathfinding.rs (walkable, nearest_walkable) | gate:walkable-cells |
| CAP-MAP-3 | Block or open a terrain cell at runtime, from the sim or the mod API (`game.edit_terrain`) | claimed | sim/src/arrays.rs (set_cell_blocked); mod-host/src/piccolo_host/editor.rs | gate:edit-terrain, gate:lua-editor-api |
| CAP-MAP-4 | Clamp positions to map bounds | implemented | sim/src/arrays.rs (clamp_to_grid) | gate:map-bounds |
| CAP-MAP-5 | Define one start position per player in map data | claimed | rust/crates/data/src/builtins.rs (start), validate.rs | gate:player-start-positions |
| CAP-MAP-6 | Place neutral objects that belong to no player | claimed | rust/crates/data/src/builtins.rs (spawn -1), validate.rs | gate:neutral-objects |
| CAP-MAP-7 | Reveal cells within each unit's sight radius and hide cells outside friendly sight | claimed | none | gate:sight-reveal |
| CAP-MAP-8 | Grant sight over elevated cells and deny sight from lower ground | claimed | none | gate:elevation-sight |
| CAP-MAP-9 | Remember a seen enemy unit at its last known position until it is seen again | claimed | none | gate:last-seen-memory |
| CAP-MAP-10 | Author custom terrain tiles in an editor: the mod API sets and reads tile ids (`game.set_tile`, `game.tile`); prop placement is map data | partial | mod-host/src/piccolo_host/editor.rs; sim/src/arrays.rs (set_cell_tile) | gate:lua-editor-api |
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
| CAP-ABIL-6 | Author custom abilities in a graphical editor: the mod API lists the catalogue (`game.list_abilities`) and defines abilities (`game.ability`); the graphical editor is the page's editor.html | partial | mod-host/src/piccolo_host/editor.rs | gate:lua-editor-api |

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
| CAP-SHLD-2 | Grant a shield bonus of p0 whole hit points through an effect; p1 above 0 removes what is left of the bonus after p1 ticks; p2 above 0 refuses an attributed target the bonus for p2 ticks after its last grant | implemented | sim/src/shields.rs (shield_bonus_timed, expire_shields) | gate:shield-bonus |
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
| CAP-EVT-4 | Author custom triggers and events in a graphical editor: the mod API lists the events (`game.list_events`) and registers triggers (`game.trigger`) | partial | mod-host/src/piccolo_host/editor.rs (list_events); game.trigger | gate:lua-editor-api |

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
| CAP-DET-3 | Identical mod and state reproduce identical hashes across runs | implemented | sim/src/hash.rs (catalog digest, modes, flow-field pool, grid tiles, props), sim/src/arrays.rs (cat_digest fixed at load), sim/src/arrays.rs (seed, rng) | gate:replay-hash-equal |
| CAP-DET-4 | Record every accepted order with its tick so a match can be replayed | claimed | sim/src/arrays.rs (record_order, order_record), sim/src/movement.rs (issue_order_kind), sim/src/persist.rs, sim/src/hash.rs | gate:order-recording |
| CAP-DET-5 | Play a replay at a chosen speed, paused, or seeked | claimed | none | gate:replay-playback-controls |
| CAP-DET-6 | Advance lockstep clients by exchanging per-tick inputs and comparing state hashes | claimed | none | gate:lockstep-input-exchange (no check in rust/gate) |

## Items and obstacles on the ground

| id | capability | status | provider | gate |
|---|---|---|---|---|
| CAP-GROUND-1 | Place, drop from a unit slot, pick up, remove and re-count items lying on the ground, kept in the state hash and the save | claimed | sim/src/ground.rs (place_ground_item, drop_item_to_ground, pickup_ground_item, remove_ground_item, set_ground_item_count); mod-host/src/piccolo_host.rs (cmd_drop_ground, cmd_drop_item, cmd_pickup_item, cmd_remove_ground, cmd_set_ground_charges, ground_items) | none |
| CAP-GROUND-2 | Set the charges of an item held in an inventory slot | claimed | sim/src/ground.rs (set_item_charges); mod-host/src/piccolo_host.rs (cmd_set_item_charges) | none |
| CAP-GROUND-3 | Swap the item held in an inventory slot for another kind in place, keeping the slot and its cooldown | claimed | sim/src/inventory.rs (replace_item); mod-host/src/piccolo_host.rs (cmd_replace_item) | mechanic-bottle-swap-in-place, host-items-and-facing |
| CAP-GROUND-4 | Per-item owner: an item slot records a player that follows the item through a replacement or a move and clears with the item; an unowned item reports its holder's player | claimed | sim/src/inventory.rs (item_owner, set_item_owner); mod-host/src/piccolo_host.rs (item_owner, cmd_set_item_owner, unit_items owner field) | host-items-and-facing |
| CAP-ITEMUSE-1 | Use an inventory item on a unit or a prop as an order the pack answers: the host validates holder, slot and target and calls the pack's on_item_use(unit, slot, item, kind, target, state); works for items with no ability of their own | claimed | mod-host/src/piccolo_host.rs (cmd_use_item_on, on_item_use) | host-items-and-facing |
| CAP-ITEM-1 | Item use that turns the used slot into another item (toggle, use-count chain, empty copy after the last charge) without a script: effect kind `item_form`, with the slot and its cooldown kept | claimed | sim/src/item_form.rs; data/src/builtins.rs (effect `item=`); data/src/validate.rs | gate:item-form-toggle |
| CAP-ITEM-2 | Item stat `spell_block`: the holder absorbs one unit-targeted enemy cast (costs paid, no effect), logged for `game.take_blocks`; item stat `true_strike`: the holder's attacks ignore evasion; item flag no_drop | claimed | sim/src/spell_block.rs, sim/src/rules.rs (cast_index), sim/src/spell.rs (strike_pct); mod-host/src/piccolo_host.rs (take_blocks, item_info no_drop) | gate:item-spell-block |
| CAP-GROUND-5 | Ground item owner: a dropped item carries the owner of its slot and the unit that picks it up gets that owner, in the state hash and the save | claimed | sim/src/ground.rs (GroundItem owner, drop_item_to_ground, pickup_ground_item), sim/src/persist.rs (STATE_VERSION 62); mod-host/src/piccolo_host.rs (ground_items owner) | mechanic-bottle-loan, mechanic-bottle-share-dose |
| CAP-TREE-1 | Destructible trees and temporary blockers: a TREE cell bit tested beside BLOCKED, per-obstacle alive flag and regrow or expiry tick in the state hash and the save, hero guard on regrow, queries by circle | claimed | sim/src/obstacles.rs, sim/src/pathfinding.rs (TREE, SOLID); mod-host/src/piccolo_host.rs (tree_add, tree_kill, tree_restore, set_tree_guard, trees_near, tree_info, blocker_add, blocker_remove); web/src/lib.rs (dead_props), js/client/play.js (syncDeadProps) | none |
| CAP-GROUP-1 | Respawn a dead spawn-group member as another unit kind, and make a spawned unit a spawn-group member homed where it stands | claimed | sim/src/spawn_groups.rs (set_respawn_kind, join_spawn_group); mod-host/src/piccolo_host.rs (cmd_respawn_as, join_spawn_group) | none |
| CAP-GROUP-2 | A spawned neutral and a respawning spawn-group member take a facing (degrees): spawn_neutral's fourth argument, cmd_respawn_as's third, kept for later lives | claimed | sim/src/spawn_groups.rs (set_respawn_heading); mod-host/src/piccolo_host.rs (spawn_neutral, cmd_respawn_as) | host-items-and-facing |
| CAP-FACE-1 | Unit heading and turn rate: turn toward travel, turn to face a strike target before the swing starts; heading and step in the state hash and the save | claimed | sim/src/facing.rs, sim/src/combat.rs (attack), sim/src/tick.rs (face_move); mod-host/src/piccolo_host.rs (set_turn_rate, unit_heading, set_heading, turn_step, bearing); web/src/lib.rs (unit_headings) | none |
| CAP-LUA-1 | Deterministic random numbers from the sim generator, and string and table helpers for pack Lua | claimed | sim/src/spell.rs (random_bits); mod-host/src/piccolo_host.rs (bind_lua_libs, HOST_PRELUDE table.sort) | none |
| CAP-HOLD-1 | Hold-fire flag: a unit that holds fire strikes nothing even when the pack commands a target; flag in the state hash and the save | claimed | sim/src/combat.rs (set_hold_fire, commanded_strike); mod-host/src/piccolo_host.rs (set_hold_fire, holds_fire) | none |
| CAP-PRES-5 | Camera distance without a point, camera rotation lock | claimed | mod-host/src/piccolo_host.rs (camera_distance, camera_lock); js/client/modui.js, render3d.js (yawLocked), camera_rules.js (rotationLock) | none |
| CAP-QUERY-1 | Player controller (bot or human) and placed prop queries for packs | claimed | mod-host/src/piccolo_host.rs (player_controller, is_computer, props) | none |

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
