# Modding: the Lua mod API

A mod is one Lua chunk run by the mod host (`rust/crates/mod-host`). It reacts to events,
registers triggers, and queues commands on the game. Mods use only the names below. The native
host runs them on mlua (Lua 5.4); the wasip3 guest runs the same event names on piccolo.

Example mods: `mods/creep_gold.lua`, `mods/tower_buff.lua`, `mods/td_waves.lua` (timer, region, variables,
score, defeat) and `mods/custom_spell.lua` (ability, gold, victory). CI runs the last two with
`mod_check` and asserts the `mod-state` line they print.

## The file contract

- The chunk must define a global `on_tick(state)`. The host refuses a mod without it.
- `state` is the host-owned table that persists across reloads. A mod may write fields to it.
- Errors name the mod file and line: `<path>:<line>: <message>`.
- Mod code runs under an instruction and memory budget. A call over either stops with
  `instruction_limit` or `memory_limit`, and a script cannot catch those to keep running.

## Events

A mod handles an event by defining a global function of the same name, or by registering a
trigger on it. Both receive the same arguments. The host queues events in the step that sees them.

| Event | Arguments | When |
|---|---|---|
| `on_spawned` | `unit` | a unit appears in the sim |
| `on_attacked` | `unit` | a unit's hit points fall |
| `on_died` | `unit` | a unit goes from alive to dead |
| `on_enter` | `region, unit` | a unit enters a named region |
| `on_leave` | `region, unit` | a unit leaves a named region |
| `on_player_joined` | `player` | the host reports a player joining (the host calls it, not the sim) |
| `on_tick` | `state` | once per step, after the events above and the systems |

`unit` is a unit handle (an integer). A dead unit is still a valid handle for `on_died`:
`unit_player` and `game.unit_kind` accept it.

`list_events()` returns this table at runtime, with the same names.

## Triggers

A trigger fires on one event. It has conditions and actions, WC3 style:

```lua
game.trigger("on_died", {
  conditions = { function(unit) return game.unit_kind(unit) == "creep" end },
  actions = { function(unit) --[[ ... ]] end },
  -- action = f is a shorthand that appends one action.
})
```

- `game.trigger(event, spec)` registers the trigger and returns its index on that event.
  `event` must be one of the names in the Events table, or the call fails with `unknown_event`.
  A trigger with no action fails with `invalid_trigger`.
- Every condition must return a truthy value for the actions to run. Conditions run in order and
  stop at the first falsy one.
- Actions run in order, each with the event's arguments.
- Triggers run after the mod's own global event function of the same name.
- Triggers register at load time, usually at the top level of the chunk.

## Game object

All game calls are fields of the global `game`. Positions are Q16.16 fixed point (a whole tile
coordinate `n` is `n * 65536`); values outside the i32 range are refused.

### Commands

Commands queue during a step and apply in the next step's command phase. A command issued from
an event or trigger therefore takes effect one step later.

| Call | Effect |
|---|---|
| `game.cmd_spawn(kind, x, y)` | spawn a unit of catalog name `kind` at `x, y`; returns its handle. Unknown kinds fail with `unknown_unit`. |
| `game.cmd_order(unit, x, y)` | move `unit` to `x, y` |
| `game.cmd_attack(attacker, target)` | attack `target` with `attacker` |
| `game.cmd_damage(caster, target, dmg)` | deal `dmg` to `target` (i32) |
| `game.cmd_buff(unit, kind, ticks)` | give `unit` the behaviour kind `kind` for `ticks` (positive i32) |
| `game.cmd_cast(unit, slot, target, x, y)` | cast ability `slot` at a unit (`target`) or a point (`target = -1`) |
| `game.cmd_set_owner(unit, player)` | move `unit` to `player` (0..MAX_PLAYERS) |
| `game.cmd_set_fog(on)` | turn fog of war on or off (a boolean) |
| `game.cmd_order_handle`, `game.cmd_damage_handle`, `game.cmd_buff_handle` | the same commands, taking handles from `cmd_spawn` |

Unit arguments of the commands must be living units when the command is queued. A dead or unknown
handle fails the queueing call, not the step.

### Queries

| Call | Returns |
|---|---|
| `game.units(player)` | the living unit handles, in index order. Pass a player id to keep only that player's units, or `nil` for all. |
| `game.players()` | a list of `{id, units}` rows, one per player slot, with the count of living units each owns. |
| `game.unit_kind(unit)` | the catalog name of the unit, or `nil`. Works on dead units. |
| `game.each(comps, fn)` | runs `fn` over every living unit now, with a view of the named components |
| `game.system(name, comps, fn)` | registers `fn` to run over every living unit each step. Names must be unique. |

The host also provides these global functions, which are not in `game`:
`unit_position`, `unit_hp`, `unit_player`, `unit_alive`, `get_unit_count`, `in_region`,
`define_region`, `spawn_unit`, `order_unit`, `order_player`, `damage_unit`, `order_attack`,
`buff_unit`, `schedule`, `cancel`, `list_abilities`, `ability_info`, `list_events`,
`edit_terrain`, `set_tile`, `list_doodads`, `place_doodad`, `remove_doodad`.

### Timers

`game.timer(name, delay, action[, period])` runs `action()` after `delay` ticks (at least 1), and
every `period` ticks after that when `period` is given. A timer of the same name is replaced.
`game.cancel_timer(name)` stops it, also from inside its own action, and returns whether it was live.
At most 256 timers. The older `schedule(name, delay, fn)` and `cancel(name)` are the one-shot form.

### Regions

`game.region(name, x0, y0, x1, y1[, {enter = f, leave = g}])` defines a named rectangle and
registers `f(unit)` for `on_enter` and `g(unit)` for `on_leave` on it. `game.region_units(name)`
returns the living units inside it now. At most 64 regions. `in_region(name, x, y)` tests a point.

### Variables and score

Variables hold a number, string or boolean under a string key and survive a mod reload.
Called with a value they set it, called without they read it (`nil` clears it).

| Call | Scope |
|---|---|
| `game.var(key[, value])` | the whole game |
| `game.player_var(player, key[, value])` | one player slot |
| `game.unit_var(unit, key[, value])` | one unit handle (dead units keep theirs) |
| `game.add_score(player, n)` / `game.score(player)` | a per-player tally the mod owns; it is not part of the sim state |

### Abilities

`game.ability(name, spec)` defines an ability in the catalog (or redefines the one of that name) and
returns the name. `name` is a lowercase identifier. `spec` has `cooldown` and `energy` (ticks and
points), `target` (`"none"`, `"unit"` or `"point"`), `filter` (unit filter mask) and `effects`, a
list of `{kind = ...}` entries:

| kind | fields |
|---|---|
| `damage`, `heal` | `amount`, `radius` (milli-units, 0 = the target only) |
| `move` | `range` (milli-units) |
| `buff` | `buff` (behaviour kind), `ticks` |
| `spawn` | `unit` (catalog name), `dx`, `dy` (milli-units from the caster) |

`game.cmd_give_ability(unit, slot, name)` puts the ability in a slot of a living unit; cast it
with `game.cmd_cast`. An unknown kind fails with `unknown_effect`, a bad name with `invalid_ability`.

### Match result and gold

| Call | Effect |
|---|---|
| `game.victory(player)` | the team of `player` wins (command) |
| `game.defeat(player)` | `player` loses: the team of its first hostile player wins (command) |
| `game.winner()` | the winning team, or `nil` while the match runs |
| `game.cmd_add_gold(player, n)` | add `n` (negative takes) to the player's gold pool |
| `game.cmd_add_hero_gold(unit, n)` | the same for a hero unit's own gold, which `cmd_buy` spends |
| `game.gold(player)`, `game.hero_gold(unit)` | read them |

The first winner stands; a later `victory` or `defeat` changes nothing.

## Gap list against the map-editor modding of earlier RTS and arena games

Status: done (in the API), partial (a piece exists), missing (not in the API).

| Capability | Status | How |
|---|---|---|
| Events (unit spawn, attacked, died, enter, leave, player joined) | done | Events table, `game.trigger` |
| Triggers with conditions and actions | done | `game.trigger` |
| Timers, one-shot and periodic | done | `game.timer`, `game.cancel_timer` |
| Regions with enter and leave events | done | `game.region`, `on_enter`, `on_leave` |
| Variables, global, per player, per unit | done | `game.var`, `game.player_var`, `game.unit_var` |
| Custom abilities with effects | done | `game.ability`, `game.cmd_give_ability` |
| Victory and defeat | done | `game.victory`, `game.defeat`, `game.winner` |
| Player gold and score | done | `game.cmd_add_gold`, `game.cmd_add_hero_gold`, `game.add_score` |
| Items: buy, sell, use | done | `game.cmd_buy`, `cmd_sell`, `cmd_use_item` |
| Fog of war toggle | done | `game.cmd_set_fog` |
| Terrain, tiles, doodads placement | done | `edit_terrain`, `set_tile`, `place_doodad`, `remove_doodad` |
| Spawn, order, attack, damage, buff, ownership | done | `game.cmd_*` |
| Persistent state across reload | done | `state`, the variable stores |
| Custom units (stats, weapons) | missing | units come from the scenario catalog; a `game.unit` definition mirroring `game.ability` is next |
| Custom items | missing | items come from the catalog; same shape as `game.ability` |
| Upgrades and tech trees | missing | the catalog holds research and prerequisites; no Lua definition or grant call |
| Teams, alliances, shared vision | partial | `cmd_set_owner` moves units; no team or alliance command |
| Player slots: create, remove, controller type | missing | `on_player_joined` only reads |
| Per-unit and per-player resource other than gold | missing | `resource_b` has no mod call |
| Killer attribution in `on_died` | missing | the event gives the dead unit only |
| Unit and ability queries (hp, position, abilities of a unit) | partial | `unit_hp`, `unit_position`; no cooldown or inventory read |
| Game clock query | missing | a mod counts its own ticks |
| Cinematics, camera, letterbox | missing | the camera is client side; needs a command stream to clients |
| UI panels, messages, multiboards, dialogs | missing | needs a command stream to clients |
| AI scripts for computer players | partial | behaviours in the catalog and `game.system` per-unit logic; no player-level AI hook |
| Save and load game data across matches | missing | state ends with the match |
| Asset import (models, sounds) | missing | the scenario build owns assets |

## Backends

- mlua (native host): every call above, the trigger registry, and the query functions.
- piccolo (wasip3 guest): the same event names (`EVENTS` in `rust/crates/mod-host/src/lib.rs`),
  `player_joined`, `take_joined`, `units`, `players`, `unit_kind`, and `events`. The guest does not
  yet run mod-side triggers or the `game` command table. A mod that calls them fails to load on the guest.

## Limits

- One step is one `advance`. A trigger or event that runs over the budget stops the step with
  `instruction_limit`.
- A mod has no file, network or OS access. The base library is the set copied into each
  environment: `pcall`, `xpcall`, `error`, `assert`, `type`, `tostring`, `tonumber`, `select`,
  `pairs`, `ipairs`, `next`, plus the host globals above.
- Killer attribution is not in the API: `on_died` gives the dead unit, not what killed it. A mod
  that needs the killer must infer it from its own state.
