# Mod scripting: the game table

Mods reach the sim through the global `game` table and a few host globals. The hooks a mod defines (`on_tick`, `on_died`, `on_request` and the rest) are in [MODDING.md](MODDING.md). This file lists the calls and the order a step runs them in. The names here are the ones `rust/crates/mod-host/src/piccolo_host.rs` binds; that file is the complete list.

## Step order

Each `advance` runs, in this order:

1. The sim tick.
2. The sim's queued decisions go to their hooks: `target_priority`, `on_march`, `on_cast`, `on_hit`, `on_check_win`.
3. Events, in order: `on_spawned`, `on_attacked`, `on_died` (its triggers too), `on_enter`, `on_leave`, `on_player_joined`.
4. Every system, in registration order, over every living unit.
5. The queued commands, in the order they were queued.
6. Due timers.
7. `on_tick(state)`.

A command queued by `on_tick` or a timer waits for step 5 of the next advance.

## Components and systems

Systems and queries see only the components they name. Every unit view also carries `id`. The components are `position` (`{x = ..., y = ...}`, Q16.16), `owner` (player index) and `hp`. Any other name raises `unknown_component`.

- `game.system(name, {components}, fn)` registers `fn(unit)`, run for each living unit each step. A duplicate name raises `duplicate_system`. Registration is per load, so a reload replaces the systems.
- `game.each({components}, fn)` runs `fn(unit)` for each living unit now, from `on_tick` or a system.
- `game.trigger(event, spec)` registers a trigger. `event` is one of `on_spawned`, `on_attacked`, `on_died`, `on_enter`, `on_leave`, `on_player_joined`, `on_tick`; another name raises `unknown_event`. `spec` has `conditions` and `actions` (lists of functions that take the event's arguments) and optionally `action` (appended to the actions). A trigger with no action raises `invalid_trigger`. Conditions run in order and stop at the first falsy one; then the actions run in order.

## Timers and regions

- `game.timer(name, delay, action[, period])` runs `action()` after `delay` ticks and, given `period`, every `period` ticks after that.
- `game.cancel_timer(name)` stops a timer.
- `schedule(name, after, action)` and `cancel(name)` are the one-shot form (host globals).
- `define_region(name, x0, y0, x1, y1)` defines a named rectangle in Q16.16. A redefinition replaces it; more than 64 regions raises `region_limit`. `in_region(name, x, y)` tests a point, and `region_bounds(name)` returns the bounds. `on_enter` and `on_leave` fire for it.

## Queries

| call | returns |
|---|---|
| `game.units(player)` | living unit handles; `nil` for all players |
| `game.players()` | one row per player slot, with its living unit count |
| `game.team(player)` | the team of `player` |
| `game.unit_kind(unit)` | the catalog name, or `nil`; works on dead units |
| `game.reward_unit(unit)` | the gold the unit is worth when destroyed (`unit_bounty` is a deprecated alias) |
| `game.rival_of(player)` | the first player slot hostile to `player`, or `nil` |
| `game.time()` | the match clock, in ticks and seconds (16 ticks a second) |
| `game.unit_cooldown(unit, slot)` | ticks left on an ability slot; 0 when ready |
| `game.unit_level(unit)` | the unit's level |
| `game.unit_abilities(unit)` | ability rows `{slot, name, cooldown}` |
| `game.unit_items(unit)` | item rows `{slot, name, count, cooldown}` |
| `game.str_lower(s)` | `s` with its ASCII letters lowercased |
| `game.str_find_ci(haystack, needle)` | the 1-based start of the first case-insensitive match, or `nil` |
| `game.gold(player)`, `game.unit_gold(unit)` | gold of a player pool or of a unit |
| `game.winner()` | the winning side, or `nil` while the match runs |
| `game.rule(key)` | a value of the pack's `rules` file, or `nil` |
| `unit_position(unit)`, `unit_hp(unit)`, `unit_player(unit)`, `unit_alive(unit)`, `get_unit_count()` | host globals over the sim |
| `list_abilities()`, `ability_info(name)`, `list_events()` | the catalog, as rows |

## Commands

Commands queue during a step and apply in step 5. Unit arguments must be living when the command is queued. A bad argument raises at queue time (`invalid_unit`, `invalid_damage`, `invalid_buff`, `invalid_slot`, `unknown_unit` and others); a unit that dies before its command applies makes the command do nothing.

| call | effect |
|---|---|
| `game.cmd_spawn(kind, x, y[, owner, lane])` | spawns a catalog unit at Q16.16 `x, y` for `owner` (default 0) in `lane` (default 0) |
| `game.cmd_order(unit, x, y)` | moves a unit |
| `game.cmd_attack(attacker, target)` | attacks. The sim never chases: the attacker moves toward the target and strikes once in reach, so a mod keeps a target engaged by queueing the attack again each step |
| `game.cmd_hold(unit)`, `game.cmd_remove(unit)` | holds a unit in place; removes it |
| `game.cmd_damage(caster, target, dmg)` | damage from the caster's player |
| `game.cmd_buff(unit, kind, ticks)` | a buff of behaviour `kind` |
| `game.cmd_cast(unit, slot, target, x, y)` | casts an ability at a unit, or at the point when `target` is `-1`; the sim refuses a cast it cannot make |
| `game.cmd_skill_up(unit, slot)` | raises an ability rank; `on_skill_up` may refuse it |
| `game.cmd_set_owner(unit, player)`, `game.cmd_set_home(unit, x, y)`, `game.cmd_set_respawn(unit, seconds or false)` | ownership, home point, and the respawn decision for a dead unit |
| `game.cmd_add_gold(player, n)`, `game.cmd_add_unit_gold(unit, n)`, `game.cmd_add_xp(unit, n)`, `game.cmd_add_stat(unit, stat, n)` | gold, experience and stats |
| `game.cmd_give_item(unit, name, n)`, `game.cmd_buy(unit, item)`, `game.cmd_sell(unit, slot)`, `game.cmd_use_item(unit, slot, target, x, y)`, `game.cmd_unstash(unit, slot)`, `game.cmd_pack_stash(unit)` | item transfers |
| `game.cmd_give_ability(unit, slot, name)` | puts an ability in a slot |
| `game.cmd_grant_upgrade(player, name)` | completes an upgrade for `player` |
| `game.cmd_set_fog(on)` | fog of war on or off |
| `game.cmd_save_order(unit, ...)`, `game.cmd_restore_order(unit)` | saves and restores a unit's order |

The `*_handle` variants (`cmd_order_handle`, `cmd_damage_handle`, `cmd_buff_handle`) take the handles of a room; a local world treats them as no-ops.

## Teams, damage and rules

`game.set_team(player, team)`, `game.alliance(a, b[, kind])`, `game.add_player(slot, controller)`, `game.remove_player(slot)`, `game.set_damage_reduction`, `game.set_healing`, `game.set_invulnerable`, `game.set_target_class`, `game.clear_target_class`, `game.set_attack_hold`, `game.clear_attack_hold`, `game.set_group_leash`, `game.set_reincarnate_item`, `game.victory(player)` and `game.defeat(player)` are the rule calls. Each one's arguments are in `piccolo_host.rs` beside its binding.

## Definitions

`game.unit(name, spec)`, `game.item(name, spec)`, `game.ability(name, spec)` and `game.upgrade(name, spec)` define a catalog record in the world at the call. Each returns the name (or, for items, the index). Their spec fields are read in `piccolo_host.rs`. A bad definition raises `invalid_unit`, `invalid_item`, `invalid_ability` or `invalid_upgrade`.

## Effects and UI

The page draws these; the sim decides nothing from them.

- `game.effect(kind, x, y[, params])` queues a display effect. `x, y` are fixed point, or 0 when `params.source` or `params.target` anchors it to a unit. `params` may carry `source`, `target`, `team`, `colour`, `duration`, `text`, `scale`, `name` and `value`. Effects are not hashed and not replayed. The page reads them with `drain_effects()`.
- `game.ui_event(kind, json)` queues a UI object, read with `drain_ui()`.
- `game.message(player, text[, seconds])` adds a log line; `player` -1 is everyone.
- `game.banner(text[, seconds])` shows a centred banner.
- `game.panel(id, {title, text, player})` sets a panel; `game.panel(id)` removes it.
- `game.camera(player, x, y[, zoom[, seconds]])` glides the camera.
- `game.sound(name)` queues a sound event.

## Example

```lua
game.system("advance", {"position", "owner"}, function(u)
  if u.owner == 0 then
    game.cmd_order(u.id, u.position.x + 65536, u.position.y)
  end
end)

function on_tick(state)
  state.mine = 0
  game.each({"owner"}, function(u)
    if u.owner == 0 then state.mine = state.mine + 1 end
  end)
end
```

The system queues one move per player-0 unit. The commands apply after every system has run, in the order they were queued, so the result does not depend on how the systems interleave.
