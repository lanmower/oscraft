# Packs as scenarios

A scenario is a pack: a directory with a `manifest.json` at its root (the manifest and the Lua modules are in [MODDING.md](MODDING.md)). Two packs ship in `scenarios/`: `moba` (a match) and `duel-routes` (a routes test). The engine reads them with no scenario-specific code. A pack's data is `.osd` files; its rules are Lua modules listed in the manifest.

## Compiling the data

- `osdt-data <src_dir> <out_file>` evaluates every `*.osd` file directly inside `src_dir`, sorted by name, and writes one image. Subdirectories are not read. A file with the retired `.star` extension is refused.
- `mapexport <pack-dir> <out.osdt>` writes the image the page loads. The manifest's `image` names it, for example `scenarios/moba/moba.osdt`. `scripts/dev/stage.sh` and the Pages workflow run `mapexport` on each pack.

## Files and calls

The `.osd` builtins are defined in `rust/crates/data/src/builtins.rs`; that file is the full list. The ones the packs use most:

- `map.osd`: `grid(w, h, cell_milli=, fill=)` once, with `fill=1` to start every cell blocked. `paint(x0, y0, x1, y1, value)` paints a half-open rectangle of cells, and `paint_height` sets heights. `start(player, x, y)` places one start per side. `spawn(unit, player, x, y)` places a unit at a point, with `player` -1 for neutral. `route(...)` declares a lane route.
- `units.osd`: `unit(name, health, speed_milli, radius_milli, ...)`, one call per line. Keywords such as `abilities=`, `filters=` and `weapon_damage=` set the rest of the unit's record.
- `abilities.osd`: `ability(...)` with `effect(...)` handles. Ability indices follow declaration order.
- `behaviours.osd`: `behaviour(name, trigger, action, ...)`, with `ability=` for the behaviours that name one.
- `economy.osd`: one `economy(...)` call.
- The name lists (`stat_names`, `effect_kinds`, `filter_names`, `mod_conds`, `prop_kinds`) are data too; the sim reads them from the image.

## Objectives

An objective is a unit with hit points and no weapon. `duel-routes` declares one as `unit("beacon", ...)` with the `structure` filter, and places a beacon on each lane with `spawn`. Other sides' units damage it through the existing combat mechanics.

## The moba pack

`scenarios/moba` is a match: two sides, lanes (the routes in `routes.osd`), towers, a base per side, heroes and creep waves (`waves.osd`). Its `.osd` files hold the data. Its rules are Lua modules: `init.lua` is the manifest's root and requires the rest, which the manifest's `modules` table lists (`lanes`, `kill_gold`, `heroes`, `store`, `win`, `controls` and others). Each rule uses the `game` table, so the engine has no match code. [REFERENCE.md](REFERENCE.md) tabulates the pack's data.

A side loses when its last living building falls. `win.lua` defines `on_check_win`, which returns the side that outlives it.

## The duel-routes pack

`scenarios/duel-routes` is the routes test: two starts, three lanes with a beacon objective on each, and skirmishers, wardens and tenders on the lanes. Its rules are Lua modules listed in its manifest: `clock.lua`, `economy_start.lua`, `harvest.lua`, `production.lua` and `transport.lua`. `scripts/dev/stage.sh` builds its image the same way as the moba pack's.

## Checking a pack

- `mod_check <pack-dir> <mod.lua> [ticks]` loads one mod into a pack's image and runs the ticks (see [MODDING.md](MODDING.md)).
- `scripts/dev/stage.sh` lays out the site with both packs, for a local run.
