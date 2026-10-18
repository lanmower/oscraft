# Modding: the Lua mod API

A mod is one Lua chunk run by the mod host (`rust/crates/mod-host`). It reacts to events,
registers triggers, and queues commands on the game. Mods use only the names below. The native
host runs them on mlua (Lua 5.4); the wasip3 guest runs the same event names on piccolo.

Example mods: `mods/creep_gold.lua`, `mods/tower_buff.lua`, `mods/td_waves.lua` (timer, region, variables,
score, defeat), `mods/custom_spell.lua` (ability, gold, victory), `mods/custom_unit.lua` (unit),
`mods/custom_item.lua` (item, buy, inventory), `mods/tech_tree.lua` (upgrades), `mods/killer.lua`
(killer attribution, clock) and `mods/unit_queries.lua` (cooldown and ability queries, clock). CI
runs them with `mod_check` and asserts the `mod-state` line they print.

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
| `on_died` | `unit, killer, killer_player` | a unit goes from alive to dead; `killer` and `killer_player` are the unit and owner credited with the death, or `nil` when nothing is |
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
| `game.cmd_skill_up(unit, slot)` | spend a skill point on ability `slot` (0..ABILITY_SLOTS); the sim refuses it without a point or at max rank |
| `game.cmd_set_owner(unit, player)` | move `unit` to `player` (0..MAX_PLAYERS) |
| `game.cmd_set_fog(on)` | turn fog of war on or off (a boolean) |
| `game.set_team(player, team)` | put `player` on `team` (both 0..MAX_PLAYERS); players on one team are allied |
| `game.alliance(a, b, kind)` | relate players `a` and `b` as `"enemy"`, `"ally"` or `"vision"` (allied with shared sight), both ways. Without `kind` it returns the current relation. |
| `game.add_player(slot, controller)` | seat `"human"`, `"computer"` or `"open"` in player slot `slot` |
| `game.remove_player(slot)` | empty `slot`: it becomes open, takes a team of its own and drops every alliance and shared sight; its units stay |
| `game.cmd_grant_upgrade(player, name)` | complete upgrade `name` for `player` at no cost and time (see Upgrades) |
| `game.cmd_order_handle`, `game.cmd_damage_handle`, `game.cmd_buff_handle` | the same commands, taking handles from `cmd_spawn` |

Unit arguments of the commands must be living units when the command is queued. A dead or unknown
handle fails the queueing call, not the step.

### Queries

| Call | Returns |
|---|---|
| `game.units(player)` | the living unit handles, in index order. Pass a player id to keep only that player's units, or `nil` for all. |
| `game.players()` | a list of `{id, units, team, controller}` rows, one per player slot, with the count of living units each owns. |
| `game.team(player)` | the team number of `player` |
| `game.unit_kind(unit)` | the catalog name of the unit, or `nil`. Works on dead units. |
| `game.time()` | the game clock: `ticks, seconds` since the match began (16 ticks to the second) |
| `game.unit_cooldown(unit, slot)` | ticks left on ability `slot` of `unit`; 0 when ready |
| `game.unit_abilities(unit)` | the unit's abilities as `{slot, name, cooldown}` rows, empty slots left out |
| `game.unit_items(unit)` | the unit's inventory as `{slot, name, count, cooldown}` rows (the item's active ability cooldown), empty slots left out |
| `game.each(comps, fn)` | runs `fn` over every living unit now, with a view of the named components |
| `game.system(name, comps, fn)` | registers `fn` to run over every living unit each step. Names must be unique. |

The host also provides these global functions, which are not in `game`:
`unit_position`, `unit_hp`, `unit_player`, `unit_alive`, `get_unit_count`, `in_region`,
`define_region`, `spawn_unit`, `order_unit`, `order_player`, `damage_unit`, `order_attack`,
`buff_unit`, `map_size`, `schedule`, `cancel`, `list_abilities`, `ability_info`, `list_events`,
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
`game.map_size()` returns the map width and height as `width, height` in the fixed-point units `unit_position` uses (cells times 65536), so `map_size() - unit_position` is a valid mirror.

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

### Units

`game.unit(name, spec)` defines a unit kind in the catalog (or redefines the one of that name; units
already alive keep the stats they spawned with) and returns the name. `name` is a lowercase
identifier. `spec` fields, all numbers unless noted:

| field | meaning |
|---|---|
| `health` | hit points, whole units; required, above 0 |
| `speed`, `radius`, `sight` | milli-units per tick, milli-units, milli-units (defaults 0, 500, 8000) |
| `armor`, `shield` | whole units |
| `cost` | gold the unit costs to train |
| `bounty` | gold paid to the killer |
| `filter` | unit filter mask (default 1, ground) |
| `model` | model file the renderer draws it with (string) |
| `weapon` | `{damage, range, period}`: whole units, milli-units, ticks (defaults 0, 1000, 16) |
| `abilities` | list of ability names, at most 8, each already defined |

Spawn it with `game.cmd_spawn(name, x, y)`. A bad name or stat fails with `invalid_unit`.

### Items

`game.item(name, spec)` defines an item (or redefines the one of that name) and returns its catalog
index, the number `game.cmd_buy(unit, item)` takes. `spec` has `cost`, the stat bonuses `armor`,
`damage` and `health` (added to the holder when the item enters the inventory) and `ability`, the
name of an already defined ability the item casts with `game.cmd_use_item`. A bad name or an
unknown ability fails with `invalid_item`. Buying needs gold in the player pool and a shop in reach.

### Upgrades and tech trees

`game.upgrade(name, spec)` defines an upgrade, a research a building runs, and returns the name.
`spec` has `building` (the unit kind that researches it, required), `time` (ticks), `cost` and
`gas`, `effect` (one `{kind = ...}` entry, required), `filter` (unit filter mask the effect applies
to, 0 for every unit of the player), `requires` (upgrade names that must be complete first) and
`unlocks` (unit kinds that cannot be trained until it is complete). The effect reaches the player's
living units when the research completes and every matching unit built after.
`game.cmd_grant_upgrade(player, name)` completes it for `player` at once, with no cost and no
prerequisite check. Upgrade effects add to the receiving unit; besides the ability kinds there are
`shield` (`amount`) and `stat` (`stat` is `damage`, `armor`, `health`, `speed` or `range`; `amount`
in whole units, milli-units for `speed` and `range`). Both kinds work in abilities too. An unknown
upgrade fails with `unknown_upgrade`, a bad definition with `invalid_upgrade`.

### Match result and gold

| Call | Effect |
|---|---|
| `game.victory(player)` | the team of `player` wins (command) |
| `game.defeat(player)` | `player` loses: the team of its first hostile player wins (command) |
| `game.winner()` | the winning team, or `nil` while the match runs |
| `game.cmd_add_gold(player, n)` | add `n` (negative takes) to the player's gold pool |
| `game.cmd_add_hero_gold(unit, n)` | the same for a hero unit's own gold |
| `game.gold(player)`, `game.hero_gold(unit)` | read them |

The first winner stands; a later `victory` or `defeat` changes nothing.

### Client UI and cinematics

| Call | Effect |
|---|---|
| `game.message(player, text[, seconds])` | a line in the message log; `player` -1 reaches everyone; default 5 s |
| `game.banner(text[, seconds])` | a large centred banner; default 4 s |
| `game.panel(id, {title, text[, player]})` | a text panel keyed by `id`; calling again replaces it, `game.panel(id)` removes it |
| `game.camera(player, x, y[, zoom[, seconds]])` | glide the render3d camera to the Q16.16 point at `zoom` (1 = default distance) over `seconds` (default 1) |
| `game.sound(name)` | emit a sound event only; the client hook `onSound(name)` plays it |

These queue commands like `game.victory`. The room stamps each with an id and tick and keeps the last 256.
`GET /events?since=<id>` returns `{"seq":N,"events":[{"id","tick","kind",...}]}`; `js/client/modui.js` polls it
when the client page is opened with `?events=<server origin>` and draws the log, banners, panels and camera moves.
In-browser hosts call `window.__modUi.push(event)` with the same objects.

### Key bindings, panels and HUD text (client)

The MOBA controls read `js/client/moba_ui.json`: the key for each action (`keys`), the double-tap window, the
team of each player slot (`teams`) and every HUD string (`text`, `{name}` placeholders). Replacing that file
replaces the defaults.

| Event (`GET /events` or `push`) | Effect |
|---|---|
| `{kind="bind", key="f", action="stop"}` | `f` runs the built-in action `stop` (also `attackMove`, `hold`, `scoreboard`, `shop`) |
| `{kind="bind", key="f", action="<name>"}` | `f` dispatches `moba:action` with `{action: "custom", name: "<name>"}` for a mod's own listener |
| `{kind="unbind", key="f"}` | removes a runtime bind |

Key handling lives in `js/client/moba_input.js`; panels react to window events `moba:action` (`detail.action`
is `shop`, `scoreboard`, `custom` or a built-in name). A mod that adds a panel listens for those events and draws
its own DOM with `data-ui="<name>"` so clicks on it are not taken as ground clicks. The Lua call that emits
`bind` and `unbind` (`game.bind`, `game.unbind`) is not in the Lua host yet; the client side accepts the events.
A runtime change to the layer's settings: `window.dispatchEvent(new CustomEvent("moba:config", {detail: {...}}))`.

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
| Custom units (stats, weapons) | done | `game.unit`, `mods/custom_unit.lua` |
| Custom items | done | `game.item`, `mods/custom_item.lua` |
| Upgrades and tech trees | done | `game.upgrade`, `game.cmd_grant_upgrade`, `mods/tech_tree.lua` |
| Teams, alliances, shared vision | done | `game.team`, `game.set_team`, `game.alliance`, `mods/teams.lua` |
| Player slots: create, remove, controller type | done | `game.add_player`, `game.remove_player`, `game.players` |
| Per-unit and per-player resource other than gold | missing | `resource_b` has no mod call |
| Killer attribution in `on_died` | done | `on_died(unit, killer, killer_player)`, `mods/killer.lua` |
| Unit and ability queries (hp, position, abilities of a unit) | done | `unit_hp`, `unit_position`, `game.unit_cooldown`, `game.unit_abilities`, `game.unit_items`, `mods/unit_queries.lua` |
| Game clock query | done | `game.time`, `mods/killer.lua`, `mods/unit_queries.lua` |
| Cinematics, camera moves | partial | `game.camera`, `game.banner`, streamed by `GET /events`, applied by `js/client/modui.js`; no letterbox |
| UI messages, banners, text panels, sounds | partial | `game.message`, `game.banner`, `game.panel`, `game.sound` (event only), `mods/ui_demo.lua`; no multiboards or dialogs |
| AI scripts for computer players | partial | behaviours in the catalog and `game.system` per-unit logic; no player-level AI hook |
| Save and load game data across matches | missing | state ends with the match |
| Asset import (models, sounds) | missing | the scenario build owns assets |

## Backends

- mlua (native host): every call above, the trigger registry, and the query functions.
- piccolo (wasip3 guest): the event names (`EVENTS` in `rust/crates/mod-host/src/lib.rs`, with the
  killer arguments of `on_died`), triggers, the `game` command table, `game.time`,
  `game.unit_cooldown`, `game.unit_abilities` and `game.unit_items`. It does not define content:
  `game.unit`, `game.item`, `game.upgrade` and `game.ability` run on the native host only, and the
  server replays the definitions into the room.

## Limits

- One step is one `advance`. A trigger or event that runs over the budget stops the step with
  `instruction_limit`.
- A mod has no file, network or OS access. The base library is the set copied into each
  environment: `pcall`, `xpcall`, `error`, `assert`, `type`, `tostring`, `tonumber`, `select`,
  `pairs`, `ipairs`, `next`, plus the host globals above.
