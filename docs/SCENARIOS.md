# Packs as scenarios

A scenario is a pack: a directory with a `manifest.json` at its root (the manifest and the Lua mods are in [MODDING.md](MODDING.md)). Its name says what it is, never a genre. The engine reads it with no scenario-specific code. A pack's data is `.osd` files; its rules are Lua mods listed in the manifest.

## Compiling the data

- `osdt-data <dir> <out.osdt>` evaluates every `*.osd` file directly inside `dir`, sorted by name, and writes one image. Subdirectories are not read.
- `mapexport <pack-dir> <out.bin>` writes the image the page loads. The pack's manifest `image` names it, for example `scenarios/moba/moba.osdt`. `scripts/dev/stage.sh` and the Pages workflow run `mapexport` on each pack.

## Files and calls

- `map.osd`: `grid(w, h, cell_milli=, fill=)` once, with `fill=1` to start every cell blocked. `paint(x0, y0, x1, y1, value)` is half-open; `paint(..., 0)` opens cells, so a route is a painted band of open cells. `start(player, x, y)` places one start per side. `spawn(unit, player, x, y)` places a unit in milli-units; `player` -1 is neutral.
- `units.osd`: `unit(name, health, speed_milli, radius_milli, ...)`, one call per line. Keywords such as `abilities=`, `filters=`, `weapon_damage=` and `harvest=` come from the unit schema.
- `abilities.osd`: `ability(...)` with `effect(...)` handles. Ability indices follow declaration order.
- `behaviours.osd`: `behaviour(name, trigger=, action=, ...)`. A `use_ability` behaviour needs `ability=`; an `ability_ready` trigger needs `ability=`.
- `economy.osd`: one `economy(...)` call with saturation counts of at least 1.

## Objectives

An objective is a unit with hit points and no weapon. Declare it with `unit(...)` and place it on a route with `spawn`. Its hit points change when other side's units damage it through the existing combat mechanics.

## The moba pack

`scenarios/moba` is a match: two sides, three lanes, towers, a base per side, heroes, workers and creep waves. Its `.osd` files hold the data. Its rules are Lua mods in the manifest's `mods` list (`lanes.lua`, `kill_gold.lua`, `hero.lua`, `store.lua`, `win.lua`, `controls.lua` and others). Each rule uses the `game` table, so the engine has no match code. [REFERENCE.md](REFERENCE.md) tabulates the pack's data.

A side loses when its last living building falls. `win.lua` defines `on_check_win`, which returns the side that outlives it.

## The duel-routes pack

`scenarios/duel-routes` is the smallest pack: two sides, three objective routes and a changed hit point value on an objective. It has no mods. `stage.sh` builds its image the same way as the moba pack's.

## Checking a pack

- `mod_check <pack-dir> <mod.lua> [ticks]` loads one mod into a pack's image and runs the ticks (see [MODDING.md](MODDING.md)).
- `scripts/dev/stage.sh` lays out the site with both packs, for a local run.
