# Mod parity

The endpoint: the setup can do what the reference real-time strategy engine's mods and games can do. This list names each
capability, its status, and the evidence. Status is one of `supported` (the engine does it and a
check covers it), `partial` (it works in part), or `missing` (the engine refuses it or has no
path). A `supported` row names its check. Rows change only with a check or a code path behind them.

## Triggers and events

| capability | status | evidence |
|---|---|---|
| Run Lua when a unit is spawned, attacked, destroyed or leaves or enters a region | supported | `scenarios/moba/behaviours.osd`, `list_events` in mod-host |
| Timed triggers that fire later and can be cancelled | supported | `schedule`, `cancel` in mod-host |
| Persistent variables across ticks | supported | the state table, `docs/SCRIPTING.md` |
| Systems that run over every unit each step | supported | `game.system`, `game.each` |

## Units and data

| capability | status | evidence |
|---|---|---|
| Define unit kinds, abilities, behaviours and upgrades from data files | supported | `rust/data`, `scenarios/*/units.osd` |
| Reload data and mods while the server runs | supported | `reload_harness` in the hot reload CI job |
| Read unit position, hit points and owner from Lua | supported | `unit_position`, `unit_hp`, `unit_player` |
| Change a unit's owner from Lua | supported | `game.cmd_set_owner`, `set_owner` in the sim, mod-host test `cmd_set_owner_moves...` |
| Damage, buff and move units from Lua | supported | `damage_unit`, `buff_unit`, `order_unit` |

## Abilities and combat

| capability | status | evidence |
|---|---|---|
| Cast an ability from Lua | supported | `game.cmd_cast` |
| Spend a skill point from Lua | supported | `game.cmd_skill_up` |
| List abilities and their costs | supported | `list_abilities`, `ability_info` |
| Projectile abilities | supported | `projectile` fields in the ability record |
| Automatic attack on the nearest enemy in reach | supported | `attack_nearest` action, mod-host test `answer_attack_runs...` |
| Commanded attack that keeps a target engaged | partial | `game.cmd_attack` strikes once in reach; the sim never chases |
| Flee, or move away from a threat | missing | the loader refuses the `flee` action at load |

## Map and terrain

| capability | status | evidence |
|---|---|---|
| Regions and region checks | supported | `define_region`, `in_region` |
| Map size (fixed-point, as unit_position) | supported | `game.map_size`, `map_size` (both hosts) |
| Edit terrain tiles | supported | `set_tile`, `edit_terrain` |
| Place and remove doodads | supported | `place_doodad`, `remove_doodad`, `list_doodads` |
| Fog of war, controlled by a mod | supported | `game.cmd_set_fog`, mod-host test `cmd_set_fog_turns_fog_off...` |

## Presentation and players

| capability | status | evidence |
|---|---|---|
| Teams, alliances with shared vision, player slots and controllers from Lua | supported | `game.set_team`, `game.alliance`, `game.add_player`, `game.remove_player`, mod-host tests `shared_vision_shows_player_zero...` and `team_and_slot_commands_reach_the_sim...`, CI `mods/teams.lua` |
| Custom units, items and upgrades from Lua; killer in `on_died`; game clock; cooldown and inventory queries | supported | `game.unit`, `game.item`, `game.upgrade`, `game.cmd_grant_upgrade`, `game.time`, `game.unit_cooldown`, `game.unit_abilities`, `game.unit_items`, CI `mods/custom_unit.lua`, `custom_item.lua`, `tech_tree.lua`, `killer.lua`, `unit_queries.lua` |
| Read a unit's bounty and a player's rival slot from Lua | supported | `game.unit_bounty`, `game.rival_of` (read-only queries in both mod hosts); the sim tests `kill_pays_the_usual_bounty...` and `kill_pays_no_rust_bounty...` cover the payout they read |
| Move or control a player's camera | missing | refused as `camera-unsupported` |
| Play sound | missing | refused as `sound-unsupported` |
| Custom UI panels | missing | refused as `ui-panel-unsupported` |
| Chat and messages to players | missing | refused as `chat-unsupported` |
| Dialogs and cinematics | missing | no path in the engine |

## Gold and kill payouts

The Rust payout is `pay_bounty` in `rust/crates/sim/src/combat.rs`, with `hero_bounty` in `hero.rs` for the team
share. The Lua payout is the `on_died` trigger of `mods/moba_match.lua`. With `GOLD_BY_MOD = true` the room's
sim queues each payable death (`Sim::drain_payouts`, `combat.rs` `Payout`) instead of paying it. The server
(`room.rs`) drains that queue each tick and hands the deaths to the mod thread (`mods.rs` `ModLink::push_payouts`),
which calls `ModHost::feed_payout` (mod-host `mlua_host.rs`) before its step. The mod's `on_died` receives the room
ids and bounty (docs/MODDING.md, on_died), and its queued gold commands return to the room, which applies them to
its sim with room ids and records them as `ag`/`ah` replay lines. The mod's own world keeps its own on_died gold
and does not forward it.

Verified with `cargo run -p server --example moba_match -- scenarios/moba 40000 <replay> --mod mods/moba_match.lua`:
every payable death fed (2165) produced the gold the formula gives (`matched=2165 mismatched=0`), every `ah` (13)
targets a room hero, and the replay holds 2165 `ag` and 13 `ah` lines. The no-mod run is unchanged (replay hash
`2140ec87bfd4ec53`, winner side 0 at tick 29057).

| capability | status | evidence |
|---|---|---|
| Kill gold rule: the Lua payout matches the Rust payout (killer gets bounty less share, the killer's side gets the share) | partial | `pay_bounty`, `hero_bounty`; `mods/moba_match.lua` `on_died`; the hero split matches `hero_bounty`; a creep killer pays its whole bounty to its side in the Lua formula, where Rust pays its own gold slot and the side the share |
| Live room feeds kills to the mod's on_died with room ids | done | `Sim::drain_payouts`, `ModHost::feed_payout`, `ModLink::push_payouts`; example `moba_match` witness above |
| Team share: 20 percent of the bounty to the killer's side | partial | Rust reads `team_share_pct` from `hero_rules.txt` (default 20); the mod hard-codes `TEAM_SHARE_PCT = 20` because `hero_rules` is not in the Lua API |
| Attackerless kill pays the rival slot's gold | done | `Payout` with `killer = None` from `pay_bounty`; mod `on_died` with `killer == nil` pays `game.rival_of(victim_player)` |
| Denied kill pays nothing, in the Lua payout too | done | `pay_bounty` returns before queueing a payout when `was_denied`, so no payout is fed |
| `gold_by_mod` switches the Rust payout off | done for rooms | `room.rs` and `lib.rs` set it from the mod's `GOLD_BY_MOD`; `combat.rs` `pay_bounty` queues instead of paying; the replay header carries it (`gold_by_mod 1`) |
| piccolo (wasip3 guest) on_died | missing | `piccolo_host.rs` still passes three arguments; a mod that reads `bounty` gets nil there |

## Determinism, replays and saves

| capability | status | evidence |
|---|---|---|
| Deterministic sim, the same result on every peer | supported | the state hash and the two-peer CI probe |
| Save and load the sim state | supported | `rust/crates/sim/src/persist.rs` |
| Record and replay a match | supported | `rust/crates/server/src/replay.rs` |

## Tooling for agents and modders

| capability | status | evidence |
|---|---|---|
| Offline mod check with named refusals and a fog switch | supported | `mod_check` in the hot reload CI job |
| Language server diagnostics and hover | supported | `rust/crates/mod-lsp`, the language server CI job |
| Reload harness that shows a broken mod is refused and the old one keeps running | supported | `reload_harness` in CI |
| Drive a running server from a script | partial | HTTP order and state endpoints exist; there is no Lua test runner |
| Package and distribute a mod | partial | `--mod` loads a directory; `docs/PACKAGING.md` describes the release |

## Open items, in order

1. Commanded attacks that keep a target engaged (a sim change with a save-format bump).
2. Flee (needs a definition of its direction and distance in data).
4. Camera, sound, UI panels, chat and cinematics (presentation layer; each needs a design, since the sim is headless).
5. A Lua test runner for running server scenarios from the command line.
