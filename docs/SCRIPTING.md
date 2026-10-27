# Mod scripting: the game table

Mods reach the sim through the global `game` table and a few host globals. The hooks a mod defines (`on_tick`, `on_died`, `on_request` and the rest) are in [MODDING.md](MODDING.md). This file lists the calls and the order a step runs them in. The names here are the ones `rust/crates/mod-host/src/piccolo_host.rs` binds, plus the Lua helpers that its prelude and `rust/crates/mod-host/src/shared.rs` define; those files are the complete list.

## Step order

Each `advance` runs, in this order:

1. The sim tick.
2. The sim's queued decisions go to their hooks: `on_hit`, `on_march`, `on_cast`, `on_proc`, `on_behaviour`, `on_check_win`.
3. Events, in order: `on_spawned`, `on_attacked`, `on_died` (its triggers too, then the `on_unit_died` hook, which returns the respawn decision), `on_enter`, `on_leave`, `on_player_joined`, `on_player_left`.
4. Every system, in registration order, over every living unit (`run_systems`).
5. The queued commands, in the order they were queued. `on_skill_up`, `on_cast_request` and `on_request` decide a skill-up, a cast or item use, and an item request as its command applies.
6. Due timers (`fire_timers`).
7. `on_tick(state)`.

A command queued by `on_tick` or a timer waits for step 5 of the next advance. `on_action(name, args, state)` and `on_pointer(kind, x, y, button, state)` run outside the step, when the page sends an action or a pointer event.

## Components and systems

Systems and queries see only the components they name. Every unit view also carries `id`. The components are `position` (`{x = ..., y = ...}`, Q16.16), `owner` (player index) and `hp`. Any other name raises `unknown_component`.

- `game.system(name, {components}, fn)` registers `fn(unit)`, run for each living unit each step. A duplicate name raises `duplicate_system`. Registration is per load, so a reload replaces the systems.
- `game.each({components}, fn)` runs `fn(unit)` for each living unit now, from `on_tick` or a system.
- `game.trigger(event, spec)` registers a trigger. `event` is one of `on_spawned`, `on_attacked`, `on_died`, `on_enter`, `on_leave`, `on_player_joined`, `on_player_left`, `on_tick`; another name raises `unknown_event`. `spec` has `conditions` and `actions` (lists of functions that take the event's arguments) and optionally `action` (appended to the actions). A trigger with no action raises `invalid_trigger`. Conditions run in order and stop at the first falsy one; then the actions run in order.

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
| `game.is_summoned(unit)` | whether the unit is a summon: a timed unit (`timed_spawn`) or an illusion image |
| `game.alive(i)` | whether unit index `i` is alive; `false` for an index out of range |
| `game.filter_bits(unit)` | the unit's filter mask |
| `game.is_structure(unit)`, `game.is_attributed(unit)` | whether the unit is a structure; whether it is attributed |
| `game.hostile(p, q)` | whether players `p` and `q` are enemies |
| `game.within(unit, x, y, reach)` | whether the unit stands within `reach` of the Q16.16 point |
| `game.strike_reach(unit, r)` | the enemies `unit` can strike this tick whose centre is within raw Q16.16 `r` of it, as a flat list of unit id and squared distance pairs |
| `game.units_near(x, y, r)` | the ids of the living units within radius `r` of the Q16.16 point |
| `game.props_near(unit, reach)` | the kind indices of the props within `reach` of the unit |
| `game.unit_stash(unit)` | the unit's stash rows, `{slot, item, name, count}` |
| `game.item_info(item)` | an item record by catalog index, as a table (`no_buy`, `no_sell`, `no_drop` and `consumable` are the flags), or `nil` |
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
| `game.unit_info(kind)` | a unit kind by catalog name or index, as a table (`melee` is the hero melee flag), or `nil` |
| `game.unit_kinds()` | every unit kind as `{name, player_controlled, melee, model}` rows; `melee` is the data's melee-attacker flag (`unit_extra(melee=True)`), which Melee Only and Range Only read |
| `game.unit_class(unit)` | the resource class: `"mineral"`, `"geyser"` or `"actor"` |
| `game.unit_radius(unit)`, `game.unit_harvest(unit)`, `game.unit_amount(unit)` | the radius in Q16.16; the harvest per trip; the resource left in a patch or geyser |
| `game.econ(key)` | an economy setting by data name, or `nil` |
| `game.mana(unit)`, `game.max_mana(unit)` | the unit's mana, and its maximum |
| `game.max_hp(unit)` | the unit's maximum hit points in Q16.16, the denominator of `unit_hp(unit)` |
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
| `game.cmd_timed_stat(unit, stat, delta, ticks)` | a stat change that ends after `ticks` ticks; a second call refreshes it, and it never stacks with an ability's timed modifier |
| `game.cmd_add_gas(player, n)` | adds `n` gas to the player's bank |
| `game.cmd_give_item(unit, name, n)`, `game.cmd_buy(unit, item)`, `game.cmd_sell(unit, slot)`, `game.cmd_use_item(unit, slot, target, x, y)`, `game.cmd_unstash(unit, slot)`, `game.cmd_pack_stash(unit)` | item transfers |
| `game.cmd_give_ability(unit, slot, name)` | puts an ability in a slot |
| `game.cmd_respawn_as(unit, kind[, facing])` | the unit kind (catalog name) a dead spawn-group member comes back as at its respawn; queue it before `cmd_set_respawn`. A pack re-rolls a camp's composition with it. `facing` (degrees, 0 east, counter-clockwise) is the heading the member returns with, kept for later lives |
| `game.cmd_drop_ground(item, count, x, y)`, `game.cmd_drop_item(unit, slot, x, y)` | puts `count` of an item on the ground at the Q16.16 point; moves a unit's whole slot stack there. Both return the id the ground item will have (exact unless the ground holds `MAX_GROUND_ITEMS` = 256) |
| `game.cmd_pickup_item(unit, id)`, `game.cmd_remove_ground(id)` | moves a ground item into the unit's inventory by the purchase slot rules (it stays when no slot takes it; reach is the pack's rule); removes a ground item |
| `game.cmd_set_ground_charges(id, n)`, `game.cmd_set_item_charges(unit, slot, n)` | the charges of a ground item or an inventory slot; 0 removes the item |
| `game.cmd_replace_item(unit, slot, name, n)` | the item in an inventory slot becomes the named item with `n` charges (1 or more), in the same slot, with the slot's cooldown and owner and without stacking; nothing happens for an empty slot or a neutral/main slot mismatch. The pack's bottle changes its filled state this way |
| `game.item_owner(unit, slot)`, `game.cmd_set_item_owner(unit, slot, player)` | the player recorded for an inventory item (the holder's player when none was set; nil for an empty slot); `unit_items` rows carry it as `owner` |
| ground item owner | a ground item keeps the owner of the slot it was dropped from (the holder's player when none was set) and the unit that takes it up gets that owner on the new slot; `ground_items` rows carry `owner`. The pack's borrowed bottle copies read it |
| `game.cmd_use_item_on(unit, slot, kind, id)` | uses an item on a unit (`kind` "unit", the unit index) or a prop (`"prop"`, its id): the host checks the holder is alive, the slot holds an item and the target exists, then calls the pack's `on_item_use(unit, slot, item_name, kind, target, state)`. Reach and the effect are the pack's; the pack's bottle refills at the fountain this way |
| `game.ground_items()` | query: the items on the ground as rows `{id, item, name, count, x, y}` in placement order (`x`, `y` raw Q16.16). Ids are never reused. The page draws each as a marker |
| `game.cmd_set_fog(on)` | fog of war on or off |
| `game.cmd_save_order(unit, ...)`, `game.cmd_restore_order(unit)` | saves and restores a unit's order |

## Lua library

A pack's environment holds the base functions (`pcall`, `error`, `assert`, `type`, `tostring`, `tonumber`, `select`, `pairs`, `ipairs`, `next`), `math` without its seeded functions, and the pieces the host builds because they must be deterministic and bounded:

| call | effect |
|---|---|
| `math.random()`, `math.random(n)`, `math.random(m, n)` | a float in [0, 1), an integer in 1..n, an integer in m..n, drawn from the sim's own generator (hashed state), so a replay draws the same stream |
| `game.random(n)` | an integer in 0..n-1 from the same generator; `nil` for n below 1 |
| `string.format(fmt, ...)` | `%d %i %u %x %X %o %c %s %f %g %%` with flags `- + space 0 #`, a width and a precision; no `%e`, `%q` or `%a` |
| `string.rep`, `string.byte`, `string.char`, `string.find` | `find` takes plain text only (a pattern character in the needle is refused unless the fourth argument is true); results are capped at 1 MiB |
| `string.len`, `sub`, `lower`, `upper`, `reverse` | piccolo's own |
| `table.insert`, `table.remove`, `table.concat`, `table.sort` | as Lua 5.4; `sort` is a stable merge sort that takes an optional comparator; `table.pack` and `table.unpack` are piccolo's |

## Teams, damage and rules

`game.set_team(player, team)`, `game.alliance(a, b[, kind])`, `game.add_player(slot, controller)`, `game.remove_player(slot)`, `game.set_damage_reduction`, `game.set_healing`, `game.set_attack_hold`, `game.victory(player)` makes the player's team the winner. `game.defeat(player)` is a no-op (the sim's `declare_defeat`); the loss rule is `scenarios/moba/win.lua`, which returns the winning side from `on_check_win`. Each one's arguments are in `piccolo_host.rs` beside its binding.

The calls below act when they run, not in the command phase: `game.set_aggro(unit, attacker, hold_ticks)`, `game.issue_order(unit, kind, x, y)` (false when refused), `game.queue_order(unit, kind, x, y, target)` (false when refused; queues behind the unit's current order, see order_queue.rs), `game.queue_clear(unit)`, `game.chase_set(unit, target, kind, x, y)`, `game.set_skill_rank(unit, slot, rank)`, `game.refresh_attr_stats(unit)`, `game.spawn_neutral(kind, x, y[, facing])` (returns the id, or nil; `facing` in degrees sets the heading), `game.set_resources(player, gold, gas)`, `game.set_research_bit(player, name, on)`, `game.apply_research(name, unit)`, `game.set_upgrade_bit(unit, bit, on)`, `game.add_kill(unit, n)`, and `game.add_credit(unit, n)` (the post-game row reports credits as assists).

`game.set_xp_rate(player, percent)`, `game.xp_rate(player)`, `game.set_unit_scale(unit, percent)`, `game.unit_scale(unit)`: the xp handicap of a player (100 default, 0..=1000; every xp award to that player's units is multiplied by it, whole units rounded down, the 6.88g `SetPlayerHandicapXP`) and the drawn size of a unit (100 default, 0..=1000 percent of its model scale, the 6.88g `SetUnitScale`; a respawned slot returns to 100). Both queue as commands and apply at the start of the next step, are hashed once they differ from 100 and are saved with the sim (state v54). A percent outside 0..=1000 raises `invalid_percent`.

## Immediate calls

These also change the sim when they run, not in the command phase.

| call | effect |
|---|---|
| `game.tree_add(cx, cy, w, h[, prop])` | places a destructible tree over a grid cell rectangle (at most 32 cells a side), alive, drawn by map prop id `prop`; returns its id or `nil`. A live tree sets the TREE cell bit, so it blocks pathing and placement beside the static blocked cells. The map data does not paint tree cells; the pack registers each tree once at start |
| `game.tree_kill(id, regrow_seconds)`, `game.tree_restore(id)` | cuts a live tree down (its cells open, its prop leaves the page) and sets when it grows back (0 never); restores a dead tree now. Both return true when they changed a tree |
| `game.set_tree_guard(radius)` | a dead tree whose regrow tick has come waits one second at a time while a living hero (attributed unit) stands within `radius` (raw Q16.16 tiles) of its centre; 0 turns it off |
| `game.set_move_tree_rule(half, regrow_seconds)` | a teleport cuts the trees in the square of half-size `half` (raw Q16.16 tiles, 0 = under the landing point, -1 off) around its landing before it places the unit, each regrowing after `regrow_seconds`; without the rule a landing on a tree is refused. Spent temporary blockers give their slots to the next blocker |
| `game.trees_near(x, y, r)`, `game.tree_info(id)`, `game.tree_count()` | live tree ids whose cells meet the raw Q16.16 circle; one obstacle's `{x, y, w, h, alive, until, blocker}` (`until` = regrow or expiry tick, 0 none); the number of obstacles |
| `game.blocker_add(cx, cy, w, h, seconds)`, `game.blocker_remove(id)` | a temporary blocker over a cell rectangle (Sprout, Ironwood Branch, Fissure, Ice Shards), removed by the tick after `seconds` or by `blocker_remove`; returns its id or `nil` |
| `game.join_spawn_group(unit)` | makes a live unit (from `spawn_neutral`) a spawn-group member homed where it stands, so a whole camp the pack spawns can be respawned member by member; true when it joined |
| `game.set_turn_rate(unit, deg_per_s)`, `game.unit_heading(unit)`, `game.set_heading(unit, heading)`, `game.turn_step(unit)` | a unit's facing rule: it turns at most `deg_per_s` degrees a second toward its direction of travel, and a strike waits until it faces its target within 11.25 degrees (the turn comes before the attack point). Headings are 1/65536 of a turn, 0 along +x rising toward +y. A rate of 0 (the default, and after a respawn) means no heading and no waiting; a pack sets the rate from `on_spawned` |
| `game.set_hold_fire(unit, on)`, `game.holds_fire(unit)` | a unit that holds fire strikes nothing, even a target the pack commands (the Double Hold order); a respawn clears it |
| `game.camera_distance(player, zoom[, seconds])`, `game.camera_lock(player, on)` | moves only the camera distance (default distance over `zoom`, within the pack's limits), keeping the point it looks at; locks or frees the page's right-drag camera turn (the pack's `camera.rotationLock` in moba_ui.json sets the start state). Player -1 addresses every client |
| `game.player_controller(player)`, `game.is_computer(player)` | who commands a seat (`"open"`, `"human"`, `"computer"`); whether it is a bot |
| `game.props([kind])` | the map's placed props as rows `{id, kind, x, y}` (raw Q16.16), all or one named kind (a depot or a shop). The set is state |
| `game.bearing(dx, dy)` | the angle of a vector in the same unit by integer arithmetic (`nil` for the zero vector) |
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
| `game.apply_status(unit, name, ticks [, mul])` | gives the unit a named status for `ticks` (the longer of the held and new duration stands); an optional Q16.16 `mul` (65536 = none) scales a slow; a dead unit or a non-positive duration changes nothing |
| `game.clear_status(unit, name)` | clears one named status counter of the unit |
| `game.clear_timed_mods(unit, buffs)` | clears the unit's timed modifiers: its buffs when `buffs` is nonzero, the others otherwise |
| `game.clear_dots(unit)` | ends every damage over time on the unit |
| `game.apply_slow(unit, ticks, multiplier)` | slows the unit for `ticks`; `multiplier` is the Q16.16 speed factor (65536 none, the lowest held stands) |
| `game.shield_of(unit)` | the unit's shield in Q16.16 hit points |
| `game.raise_max_hp(unit, whole)` | raises a live unit's maximum hit points to `whole` (its life rises by the difference); false when it is already at or above |
| `game.set_base_period(unit, ticks)` | sets a live unit's base attack period in ticks and returns the previous one |
| `game.set_ability_cooldown(unit, slot, ticks)` | sets the cooldown left on an ability slot |
| `game.spell_damage(caster, target, amount, dtype)` | typed spell damage now (`dtype` 1 physical, 2 magical, 3 pure), `amount` whole hit points in 0..=32767, scaled by the caster's spell amplification; returns whether the hit killed |
| `game.add_stat(unit, stat, delta)` | adds `delta` to a stat of a unit now (a dead unit keeps the changed stat for its respawn) and re-derives its attribute stats, so a health change moves current health by the same amount (`cmd_add_stat` queues the same change for the command phase) |
| `game.set_hp(unit, hp)` | sets a live unit's health to the Q16.16 value, held between one hit point and its maximum; returns whether it was set |
| `game.set_mana(unit, mana)` | sets a live unit's mana to the whole value, held between zero and its maximum (`game.max_mana`); returns whether it was set |
| `game.start_cooldown(unit, slot, ticks)` | sets the cooldown of the unit's ability in the slot to `ticks` (at least 0); returns `true` |
| `game.consume_item_charge(unit, slot)` | spends one charge of the item in the inventory slot; returns `false`, changing nothing, for an empty slot |
| `game.take_amount(unit, want)` | takes up to `want` resource from the unit; returns what it took |
| `game.take_recent_hitters(unit, window)` | returns the ids of the unit's recent hitters within `window` ticks, and forgets them |
| `game.heavy_hit_age(unit)` | ticks since the unit took a hit above the `heavy_hit_min` rule (whole damage after armour and shields) from another unit, or `nil` when it never did; Kelen's Dagger reads it |
| `game.recent_hitters(unit, window)` | the same ids, and the record is kept; the kill split reads it before the assist credit takes it |
| `game.set_spell_block(unit, ticks)`, `game.spell_block_left(unit)` | gives a living unit the spell block of the `spell_block` item stat for `ticks` (it absorbs one unit-targeted enemy cast that does not pierce immunity, then ends); ticks left on it. Saved and hashed |
| `game.item_lock_left(unit, slot)` | ticks the item in the slot cannot be dropped for: the `item_form` effect p3 sets it on the new form (Black King Bar); 0 when free |
| `game.take_blocks()` | the casts an item spell block absorbed since the last call, as rows `{target, caster, ability}` (ability by name), and forgets them; an item with the `spell_block` stat absorbs one unit-targeted enemy cast that does not pierce immunity: costs and cooldown are paid, no effect runs, and `on_cast` does not fire |
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
- `game.message(player, text[, seconds[, allies]])` adds a log line; `player` -1 is everyone and 255 is the page's spectator seat (a peer without a seat; `game.panel` and `game.sound` take 255 too). With `allies` (a player number) the line goes only to the seats on that player's team, the sender included, and `player` is ignored. The page paints `|cAARRGGBB` colour codes and `|r` in the text.
- `game.player_name(player[, name])`, `game.player_colour(player[, rgb])` read or set a seat's display name (`"Player N"` until set; 1..32 bytes, no control characters) and colour (`0xRRGGBB`, `nil` until set). A setter queues a `seat` UI event `{seat, name, colour}` for every page.
- `game.setting(player, key[, value])` reads or sets one player's integer setting (`key` lowercase letters, digits, underscore, at most 32 bytes; at most 64 keys a player; `nil` until set). A setter queues a `setting` UI event `{key, value}` for that player's page. Names, colours and settings are display state: not hashed, not saved, not replayed; keep what must replay in unit counters.
- `game.banner(text[, seconds])` shows a centred banner.
- `game.panel(id, {title, text, player, columns, rows})` sets a panel; `game.panel(id)` removes it. `rows` is a table of rows, each a table of cell strings, and `columns` an optional header row of strings; the event then carries `columns` and `rows` and the page draws a table (`js/client/matrix_panel.js`) inside the panel. Cells may carry `|cAARRGGBB` colour codes and `|r`. At most 64 rows, 32 cells a row, 256 bytes a cell; anything else raises `invalid_panel` or `bad_field`. A panel is not deduplicated: resend it only when it changed.
- `game.camera(player, x, y[, zoom[, seconds]])` glides the camera.
- `game.sound(name[, player])` queues a sound event for every client, or only for `player`.

## Editor calls

These queue like any command and apply at the start of the next step; the grid and its tile ids are hashed sim state.

| call | effect |
|---|---|
| `game.edit_terrain(x, y, blocked)` | blocks or opens grid cell `(x, y)`, whole cell coordinates; a cell outside the grid raises `invalid_cell` |
| `game.set_tile(x, y, tile)`, `game.tile(x, y)` | sets or reads the tile id (0..=255) of a cell; `tile` is `nil` outside the grid |
| `game.list_abilities([prefix])` | every catalogue ability as `{index, name, cooldown, energy, flags}` in catalogue order, or those whose name starts with `prefix` |
| `game.list_events()` | the events a pack handles, as `{name, args}` rows (the list in MODDING.md) |

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
