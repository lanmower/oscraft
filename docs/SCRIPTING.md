# Mod scripting: the game table

Mods reach the sim through the global `game` table and a few host globals. The hooks a mod defines (`on_tick`, `on_died`, `on_request` and the rest) are in [MODDING.md](MODDING.md). This file lists the calls and the order a step runs them in. The names here are the ones `rust/crates/mod-host/src/piccolo_host.rs` binds, plus the Lua helpers that its prelude and `rust/crates/mod-host/src/shared.rs` define; those files are the complete list.

## Step order

Each `advance` runs, in this order:

1. The sim tick.
2. The sim's queued decisions go to their hooks: `on_hit`, `on_march`, `on_cast`, `on_proc`, `on_behaviour`, `on_check_win`.
3. Events, in order: `on_spawned`, `on_attacked`, `on_died` (its triggers too, then the `on_unit_died` hook, which returns the respawn decision), `on_enter`, `on_leave`, `on_player_joined`.
4. Every system, in registration order, over every living unit (`run_systems`).
5. The queued commands, in the order they were queued. `on_skill_up`, `on_cast_request` and `on_request` decide a skill-up, a cast or item use, and an item request as its command applies.
6. Due timers (`fire_timers`).
7. `on_tick(state)`.

A command queued by `on_tick` or a timer waits for step 5 of the next advance. `on_action(name, args, state)` and `on_pointer(kind, x, y, button, state)` run outside the step, when the page sends an action or a pointer event.

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

## Variables, score and regions

| call | effect |
|---|---|
| `game.var(key[, value])` | reads a global variable, or sets it when given a value (a number, string or boolean) |
| `game.player_var(player, key[, value])` | the same, for a player's variables |
| `game.unit_var(unit, key[, value])` | the same, for a unit's variables; raises for an unknown unit |
| `game.add_score(player, amount)` | adds to the player's score; returns the new total |
| `game.score(player)` | the player's score, 0 when none |
| `game.region(name, x0, y0, x1, y1[, handlers])` | defines the region as `define_region` does; `handlers.enter` and `handlers.leave`, each taking a unit, run for it on entry and exit; returns `name` |
| `game.region_bounds(name)`, `game.region_center(name)` | the region's bounds, `x0, y0, x1, y1`; its centre, `x, y` |
| `game.region_units(name)` | the living units inside the region |

## Queries

| call | returns |
|---|---|
| `game.units(player)` | living unit handles; `nil` for all players |
| `game.players()` | one row per player slot, with its living unit count |
| `game.team(player)` | the team of `player` |
| `game.unit_kind(unit)` | the catalog name, or `nil`; works on dead units |
| `game.time()` | the match clock, in ticks and seconds (16 ticks a second) |
| `game.unit_cooldown(unit, slot)` | ticks left on an ability slot; 0 when ready |
| `game.unit_abilities(unit)` | ability rows `{slot, name, cooldown}` |
| `game.unit_items(unit)` | item rows `{slot, name, count, cooldown}` |
| `game.gold(player)`, `game.unit_gold(unit)` | gold of a player pool or of a unit |
| `game.winner()` | the winning side, or `nil` while the match runs |
| `game.rule(key)` | a value of the pack's `rules` file, or `nil` |
| `game.weapon(unit)` | the unit's damage and weapon reach, fixed point; (0, 0) for a unit with no weapon |
| `game.can_target(unit, other)` | whether `unit` may strike `other` on the sim's terms, ignoring reach |
| `game.strike_of(unit)` | the enemy the unit is set to strike, -1 for none |
| `game.order_of(unit)` | the order kind the unit is on and its destination, fixed point |
| `game.acquire_of(unit)` | the unit kind's acquire range in milli-units; 0 when the kind has none |
| `game.chase_get(unit)` | the chase target (-1 for none) and the order the unit restores when the chase ends |
| `game.taunter(unit)` | the unit that taunted `unit` while the taunt runs and that unit is alive, or `nil` |
| `game.ticks()` | the sim's tick count |
| `game.research_bit(player, name)` | true once `player` holds the research `name`; false for an unknown research |
| `game.seat_start(player)` | the player's start point as the map gives it, `{x, y}`, or `nil` |
| `game.xp(unit)` | the xp the unit holds |
| `game.xp_bounty(unit)`, `game.deny_xp_pct(unit)`, `game.assist_radius_milli(unit)` | the xp the unit is worth when destroyed; its deny percent; its assist radius in milli-units |
| `game.xp_curve()` | the cumulative xp thresholds of the level curve, as a list |
| `game.stat_value(unit, code)` | the unit's current bonus for stat `code` |
| `game.item_bonus(unit, stat)` | the sum of one item stat over the unit's active slots |
| `game.upgrade_bits(unit)` | the unit's upgrade bits |
| `game.attr_field(unit, name)` | a field of the unit's `unit` line data, or `nil` |
| `game.counter(unit, name)` | a named counter the mod keeps on the unit; 0 when never set |
| `game.alive(i)` | whether unit index `i` is alive; `false` for an index out of range |
| `game.filter_bits(unit)` | the unit's filter mask |
| `game.is_structure(unit)`, `game.is_attributed(unit)` | whether the unit is a structure; whether it is attributed |
| `game.hostile(p, q)` | whether players `p` and `q` are enemies |
| `game.within(unit, x, y, reach)` | whether the unit stands within `reach` of the Q16.16 point |
| `game.units_near(x, y, r)` | the ids of the living units within radius `r` of the Q16.16 point |
| `game.doodads_near(unit, reach)` | the kind indices of the doodads within `reach` of the unit |
| `game.unit_stash(unit)` | the unit's stash rows, `{slot, item, name, count}` |
| `game.item_info(item)` | an item record by catalog index, as a table, or `nil` |
| `game.status_left(unit, name)` | ticks left on the named status; raises for an unknown name |
| `game.channel_left(unit)` | ticks left in the unit's channel |
| `game.ability_gate(index)` | the ability's flags, target filter mask and target kind, or `nil` |
| `game.cast_in_range(unit, index, x, y)` | whether the ability is in cast range of the point |
| `game.cast_point(unit)`, `game.cast_target(unit)` | the x and y of the unit's last cast; its target, or `nil` |
| `game.visible_to(unit, other)` | whether `other` is visible to the unit's player |
| `game.meets_filter(unit, mask)` | whether the unit meets the filter mask |
| `game.class_override(unit)` | the target class override, 0 to 3, or -1 for none |
| `game.aggro_target(unit)` | the unit that drew aggro from `unit`, or -1 |
| `game.can_strike(unit, other)` | whether `unit` may strike `other` this tick: not stunned or disarmed, and in reach |
| `game.strike_lands(other)` | whether a weapon hit can reduce `other`'s life |
| `game.skill_rank(unit, slot)`, `game.skill_gate(unit, slot)` | the rank of an ability slot; the level gate of its next rank, or `nil` |
| `game.skill_info(unit, slot)` | the slot's skill tree as a table, or `nil` |
| `game.is_spawn_group_unit(unit)` | whether the unit belongs to a spawn group |
| `game.is_night()` | whether it is night, as the pack's clock set it |
| `game.resources(player)` | the player's gold and gas |
| `game.capacity_limits()` | the economy's capacity base and maximum, as `{base, max}` |
| `game.walkable(x, y)` | whether the grid cell under the Q16.16 point is walkable by ground units |
| `game.research_info(name)` | a research record as a table, or `nil` |
| `game.prereqs(name, kind)` | what a unit or research (`kind` `"unit"` or `"research"`) needs, as `{kind, name}` rows |
| `game.unit_info(kind)` | a unit kind by catalog name or index, as a table, or `nil` |
| `game.unit_kinds()` | every unit kind as `{name, player_controlled}` rows |
| `game.unit_class(unit)` | the resource class: `"mineral"`, `"geyser"` or `"actor"` |
| `game.unit_radius(unit)`, `game.unit_harvest(unit)`, `game.unit_amount(unit)` | the radius in Q16.16; the harvest per trip; the resource left in a patch or geyser |
| `game.econ(key)` | an economy setting by data name, or `nil` |
| `game.mana(unit)`, `game.max_mana(unit)` | the unit's mana, and its maximum |
| `game.kills_of(unit)`, `game.deaths_of(unit)`, `game.credits_of(unit)` | the kills, deaths and credits the unit holds |
| `game.map_size()`, `map_size()` | the map's width and height, in Q16.16 |
| `unit_position(unit)`, `unit_hp(unit)`, `unit_player(unit)`, `unit_alive(unit)`, `get_unit_count()` | host globals over the sim |
| `ability_info(name)` | the ability's catalog row; raises `unknown_ability` for an unknown name |

## Commands

Commands queue during a step and apply in step 5. Unit arguments must be living when the command is queued. A bad argument raises at queue time (`invalid_unit`, `invalid_damage`, `invalid_buff`, `invalid_slot`, `unknown_unit` and others); a unit that dies before its command applies makes the command do nothing.

| call | effect |
|---|---|
| `game.cmd_spawn(kind, x, y[, owner, lane])` | spawns a catalog unit at Q16.16 `x, y` for `owner` (default 0) in `lane` (default 0) |
| `game.cmd_order(unit, x, y)` | moves a unit |
| `game.cmd_attack(attacker, target)` | attacks. The sim never chases: the attacker moves toward the target and strikes once in reach, so a mod keeps a target engaged by queueing the attack again each step |
| `game.attack_move(unit, x, y)` | queues an attack-move to the Q16.16 point |
| `game.cmd_strike(unit, target)` | the enemy `unit` strikes `target` each tick, a unit index or -1 for none |
| `game.cmd_hold(unit)`, `game.cmd_remove(unit)` | holds a unit in place; removes it |
| `game.cmd_damage(caster, target, dmg)` | damage from the caster's player |
| `game.cmd_buff(unit, kind, ticks)` | a buff of behaviour `kind` |
| `game.cmd_cast(unit, slot, target, x, y)` | casts an ability at a unit, or at the point when `target` is `-1`; the sim refuses a cast it cannot make |
| `game.cmd_skill_up(unit, slot)` | raises an ability rank; `on_skill_up` may refuse it |
| `game.cmd_set_owner(unit, player)`, `game.cmd_set_home(unit, x, y)`, `game.cmd_set_respawn(unit, seconds or false)` | ownership, home point, and the respawn decision for a dead unit |
| `game.cmd_add_gold(player, n)`, `game.cmd_add_unit_gold(unit, n)`, `game.cmd_add_xp(unit, n)`, `game.cmd_add_stat(unit, stat, n)` | gold, experience and stats |
| `game.cmd_add_gas(player, n)` | adds `n` gas to the player's bank |
| `game.cmd_give_item(unit, name, n)`, `game.cmd_buy(unit, item)`, `game.cmd_sell(unit, slot)`, `game.cmd_use_item(unit, slot, target, x, y)`, `game.cmd_unstash(unit, slot)`, `game.cmd_pack_stash(unit)` | item transfers |
| `game.cmd_give_ability(unit, slot, name)` | puts an ability in a slot |
| `game.cmd_set_fog(on)` | fog of war on or off |
| `game.cmd_save_order(unit, ...)`, `game.cmd_restore_order(unit)` | saves and restores a unit's order |

## Teams, damage and rules

`game.set_team(player, team)`, `game.alliance(a, b[, kind])`, `game.add_player(slot, controller)`, `game.remove_player(slot)`, `game.set_damage_reduction`, `game.set_healing`, `game.set_attack_hold`, `game.victory(player)` makes the player's team the winner. `game.defeat(player)` is a no-op (the sim's `declare_defeat`); the loss rule is `scenarios/moba/win.lua`, which returns the winning side from `on_check_win`. Each one's arguments are in `piccolo_host.rs` beside its binding.

The calls below act when they run, not in the command phase: `game.set_aggro(unit, attacker, hold_ticks)`, `game.issue_order(unit, kind, x, y)` (false when refused), `game.chase_set(unit, target, kind, x, y)`, `game.set_skill_rank(unit, slot, rank)`, `game.refresh_attr_stats(unit)`, `game.spawn_neutral(kind, x, y)` (returns the id, or nil), `game.set_resources(player, gold, gas)`, `game.set_research_bit(player, name, on)`, `game.apply_research(name, unit)`, `game.set_upgrade_bit(unit, bit, on)`, `game.add_kill(unit, n)`, and `game.add_credit(unit, n)` (the post-game row reports credits as assists).

## Immediate calls

These also change the sim when they run, not in the command phase.

| call | effect |
|---|---|
| `game.spawn(player, kind, x, y)` | spawns a unit of `kind` (name or index) for `player` at the Q16.16 point; returns its id, or `nil` |
| `game.timed_spawn(caster, kind, life, x, y)` | places a unit of `kind` for the caster's player, lasting `life` ticks; returns its id, or `nil` |
| `game.spawn_image(caster, x, y, life, out, inc)` | one image of the caster at the point, lasting `life` ticks; returns its id, or -1 |
| `game.teleport(unit, x, y)` | places a live unit at the Q16.16 point and clears its movement |
| `game.enroll(unit, x, y)` | enrols the unit at the point with its attribute stats and passives, health full |
| `game.hide_unit(unit)` | takes a live unit out of play without a death; returns whether it did |
| `game.unhide_unit(unit, x, y)` | puts a hidden unit back at the Q16.16 point; returns whether it did |
| `game.revive_at(unit, x, y)` | puts a dead attributed unit back at the Q16.16 point with full health, mana and items; false for a live or non-attributed unit, or a full cell |
| `game.acquire(name, range)` | sets the acquire range of a unit kind, in game units |
| `game.set_counter(unit, name, value)` | sets a named counter the mod keeps on the unit |
| `game.set_stat_bounds(stat, lo, hi)` | holds a stat within bounds when a cast cost or attack period reads it; returns whether it was set |
| `game.armour_of(unit)` | the unit's effective armour in Q16.16 (base and bonuses); 0 when unknown |
| `game.set_armour_scale(unit, scale)` | the damage scale (Q16.16) of the unit: the damage it takes is multiplied by it. `unit_rules.lua` sets it each tick from the armour curve |
| `game.set_speed_bounds(unit, floor, cap)` | holds the unit's effective speed (Q16.16 per tick) between floor and cap. `unit_rules.lua` sets it each tick from the speed limits |
| `game.unit_speed(unit)` | the unit's base speed in Q16.16 per tick, before slows, boosts and bounds; 0 when unknown |
| `game.type_speed(unit)` | the speed of the unit's type in the unit data, in milli-units per tick; 0 when unknown |
| `game.set_miss_chance(unit, chance)` | the chance (Q16.16) a shot by the unit misses; 0 means it cannot miss |
| `game.unit_miss_chance(unit)` | the miss chance (Q16.16) the unit data gives the unit's type; 0 when unknown |
| `game.set_status_flags(name, bits)` | sets the flags of a named status; returns whether they were set |
| `game.clear_status(unit, name)` | clears one named status counter of the unit |
| `game.clear_timed_mods(unit, buffs)` | clears the unit's timed modifiers: its buffs when `buffs` is nonzero, the others otherwise |
| `game.clear_dots(unit)` | ends every damage over time on the unit |
| `game.consume_item_charge(unit, slot)` | spends one charge of the item in the inventory slot; returns `false`, changing nothing, for an empty slot |
| `game.take_amount(unit, want)` | takes up to `want` resource from the unit; returns what it took |
| `game.take_recent_hitters(unit, window)` | returns the ids of the unit's recent hitters within `window` ticks, and forgets them |
| `game.recent_hitters(unit, window)` | the same ids, and the record is kept; the kill split reads it before the assist credit takes it |
| `game.add_resources(player, gold, gas)` | adds gold and gas to the player; negative amounts take them |
| `game.set_clock(night, ticks_into_cycle, day_ticks, night_ticks)` | stores the day and night clock the pack computes for this tick |
| `spawn_unit(kind, x, y)` | spawns a catalog unit by name at the Q16.16 point; does nothing for an unknown name |
| `order_unit(unit, x, y)` | orders a live unit to the Q16.16 point |

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
