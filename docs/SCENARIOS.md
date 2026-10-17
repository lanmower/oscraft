# Authoring a scenario as data

A scenario is a directory of `.osd` files. Its name says what it is (`scenarios/duel-routes`), never a genre. The engine reads it with no scenario-specific code.

`osdt-data <dir> <out.osdt>` evaluates every `*.osd` file directly inside the directory, in name order, and writes one catalog image. `oscraftd --data <out.osdt>` loads that image. Subdirectories are not read.

## Files and calls

- `map.osd`: `grid(w, h, cell_milli=, fill=)` once, with `fill=1` to start every cell blocked. `paint(x0, y0, x1, y1, value)` is half-open; `paint(..., 0)` opens cells, so a route is a painted band of open cells. `start(player, x, y)` places one start per side. `spawn(unit, player, x, y)` places a unit in milli-units; `player` -1 is neutral.
- `units.osd`: `unit(name, health, speed_milli, radius_milli, ...)`, one call per line. Keywords such as `abilities=`, `filters=`, `weapon_damage=` and `harvest=` come from the existing unit schema.
- `abilities.osd`: `ability(...)` with `effect(...)` handles. Ability indices follow declaration order.
- `behaviours.osd`: `behaviour(name, trigger=, action=, ...)`, for example `guard_patrol` (idle units use an ability).
- `economy.osd`: one `economy(...)` call with saturation counts of at least 1.

## Objectives

An objective is a unit with hit points and no weapon. Declare it with `unit(...)` and place it on a route with `spawn`. Its hit points change when other side's units damage it through the existing combat mechanics.

## Checking a scenario

The `duel-routes` CI job compiles `scenarios/duel-routes`, starts `oscraftd` on it, advances 200 ticks, and reads `/state?map=ci`. It requires units from two sides, three objective routes, and a changed hit point value on an objective. Its last line is `scenario ok sides=<n> routes=<n> objectives=<n> hp_changed=<bool>`.
## The moba scenario

`scenarios/moba` is a data-only match: two sides, three lanes, a tower per side per lane, one hero and one base per side, workers, and a `creep_wave` ability on each base that spawns creeps on its cooldown timer. The engine reads it with no scenario-specific code, like `duel-routes`. The `moba` CI job compiles it, serves it, advances 200 ticks, and derives every count it prints from the `.osd` files.
