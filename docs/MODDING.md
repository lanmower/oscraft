# Packs and the Lua mod API

The engine is generic. A game is a pack: a directory, a `.tar.gz` or a `.zip` with a `manifest.json` at its root. A pack holds its Lua modules, its data (`.osd`, `.json`, `.txt`, `.tsv`), the compiled map image and its assets. The engine names no genre: `scenarios/moba` and `scenarios/duel-routes` are two packs that the same engine reads.

The browser is the only build. `rust/crates/web` compiles the sim and the mod host to wasm, and the mod host runs the pack's Lua on piccolo (`rust/crates/mod-host`, the default `piccolo` feature).

## Loading a pack

- The page opens `play.html?package=<dir or archive>` (`pack=` is the same). The default is `scenarios/moba` (`js/client/pack_load.js`). A directory, a `.tar.gz`/`.tgz` and a `.zip` go through the same reader.
- The page reads only the paths the manifest names.
- A pack holds no code the page runs. A `.js`, `.mjs`, `.cjs`, `.html`, `.wasm`, `.ts` or `.map` path is refused on load, on edit and on transfer.
- `node scripts/pack-package.mjs <package-dir> <out-dir> [--build] [--part-size=<MiB>]` writes `<name>-<version>.tar.gz`. `--build` runs `mapexport` first, so the manifest's image is in the archive. An archive larger than `--part-size` MiB (25 by default) is published as numbered pieces plus a `<archive>.parts.json` index; see [HOST.md](HOST.md).

### Manifest

`scenarios/moba/manifest.json` is the reference. Its fields:

| field | meaning |
|---|---|
| `schema` | must be `1` |
| `name`, `version` | identity |
| `map` | the map source `mapexport` reads (`map.osd`) |
| `data` | the `.osd` files that compile into the image |
| `image` | the compiled image, written by `mapexport <pack-dir> <pack-dir>/<image>` |
| `mods` | the root Lua modules, in load order (`init.lua`) |
| `modules` | every module by name, mapped to its file; `require` finds a module here |
| `rules` | a `key=value` text file; `game.rule(key)` reads it |
| `tables`, `fixtures` | data tables and test fixtures |
| `ui`, `effects` | JSON the page reads for controls and display |
| `assets` | texture, model and catalogue paths, under `assets.root` |

## Editing and sharing a pack

The host opens the pack editor with the `pack` button on the play page (`js/client/pack_editor.js`). It has a file tree, a text editor for `.lua .osd .json .txt .tsv`, a drop zone for asset files, Save, and a `download pack (.zip)` link.

- Only the host edits. A peer sees the same pack read-only.
- Save sends each changed file to every peer as `PACK_FILE` frames (`pack_sync.js`). A file over 256 KiB travels by reference: the peer asks for it with `PACK_WANT`, by content hash.
- A peer applies only the frames from the host.

## Modules

- Each module is one Lua file, named by its file stem (`hero.lua` is `hero`). A module is a chunk that returns a table of hooks, such as `return { on_died = function(u, s) ... end }`, and may call `require("<module>")` to use another module of the same pack. `require` takes a string literal; a require cycle is an error that names the cycle. A module loads once and its returned table is cached. Each module runs in its own environment over the match's sim, and its hooks run in load order.
- `load_mods` loads the root modules from `mods` and every module they reach through `require`. A hook that raises an error is turned off for that module only and the error is logged. A module that fails to load (compile, top-level error, a cycle or a missing require) keeps its last-known-good version when it loaded before, else is skipped; the other modules stay live. A chunk that returns nothing and defines global hooks still works (the old style, for migration), and a global read falls through to the other modules' environments.
- A load error names the file and line: `<file>:<line>: <message>`.
- Every hook is optional, `on_tick` included.
- `state` is the host's table of scalar fields, shared by the hooks. `state.ticks` is seeded by the host.
- A hook call runs under a budget of 2,500 VM steps. Over it, the call fails with `instruction_limit`.
- Mod code has no file, network or OS access. The globals are `pcall`, `error`, `assert`, `type`, `tostring`, `tonumber`, `select`, `pairs`, `ipairs`, `next`, `math`, plus the `game` table and the host globals listed in [SCRIPTING.md](SCRIPTING.md).

## Hooks

A mod defines a hook as a function in its returned table (or as a global, in the old style). The engine calls each one that exists, in load order.

| hook | arguments | returns / effect |
|---|---|---|
| `on_tick` | `state` | once per step, after the events and the systems. |
| `on_spawned` | `unit` | a unit appears |
| `on_attacked` | `unit` | a unit's hit points fall |
| `on_died` | `unit, killer, killer_player, bounty, victim_player, killer_attributed` | a unit dies; `killer` and `killer_player` are `nil` for an attackerless death. Its triggers run after it. |
| `on_unit_died` | `unit, state` | returns the respawn seconds, `false` to leave the unit dead, or `nil` for the engine's timer |
| `on_enter`, `on_leave` | `region, unit` | a unit enters or leaves a region from `define_region` |
| `on_player_joined` | `player` | the host reports a seat taken (the page's `peer_back` action) |
| `on_player_left` | `player` | the host reports a seat closed (the page's `peer_left` action) |
| `on_behaviour` | `unit, trigger, action, ability, slot, target_kind, state` | a behaviour trigger of a unit fired (names as strings, ids as integers) |
| `on_cast_request` | `caster, slot, ability, index, target, x, y, is_item, state` | returns `true` to let a cast reach the sim, `false` to refuse it |
| `on_cast` | `caster, ability_name, state` | an ability is cast |
| `on_proc` | `caster, ability_name, state` | a passive of the caster fired; the sim has already run its engine effects |
| `on_hit` | `attacker, victim, dealt, state` | a hit that dealt damage lands; `attacker` is -1 for none |
| `on_march` | `unit, player, state` | a unit with no destination asks for a point; return `x, y` in fixed point to order it there |
| `on_check_win` | `state` | returns the winning side, or `nil`/`false` while none has won |
| `on_request` | `kind, unit, item, state` | the item rule; `kind` is `acquire`, `release`, `take_out` or `carry`; returns true, or false and a reason |
| `on_skill_up` | `unit, slot, state` | returns `true` to raise the rank, `false` to refuse; without the hook a skill-up is allowed |
| `on_action` | `name, args, state` | called by the page's `dispatch_action(name, args)` |
| `on_pointer` | `kind, x, y, button, state` | called by the page's `pointer(kind, x, y, button)`; `kind` is `down`, `up` or `move`; `x, y` are tile coordinates |

The host also dispatches `on_tick`, `on_attacked`, `on_spawned`, `on_enter`, `on_leave` and the others in the order [SCRIPTING.md](SCRIPTING.md) gives. A trigger registered with `game.trigger` runs after the hook of the same name.

## The page's entry points

These are methods of the wasm `Game` (`rust/crates/web/src/lib.rs`). The page calls them; mods do not.

| method | what it does |
|---|---|
| `load_mods(json)` | loads `[{name, text}, ...]` (the root modules and the ones they require); returns `""` or the error |
| `dispatch_action(name, args)` | runs `on_action` of each mod; `args` is a JSON string |
| `pointer(kind, x, y, button)` | runs `on_pointer` of each mod |
| `drain_ui()` | returns the UI objects queued since the last call, as a JSON array: `game.ui_event`, `camera`, `message`, `banner`, `panel`, `sound` |
| `drain_effects()` | returns the display effects queued by `game.effect`, as a JSON array |

The page renders what it drains and decides nothing about it. Controls, key bindings and HUD text are pack data: the manifest's `ui` file and the mods that read it.

## Display effects

`game.effect(kind, x, y, params)` queues a display event; the page draws it from the kind's data and decides nothing. `x, y` are a fixed-point ground point (0, 0 when the event names a unit). `params` fields, all optional: `source`, `target` (unit ids; an effect anchored to a unit follows it, and keeps its last position once it has died), `colour` (a name in `effects.colours`; the kind's own when absent), `duration` in **milliseconds** (0 or absent: the kind's `seconds`), `scale` in **thousandths** (0 or absent: 1000), `text` (float kinds), `name`, `value` (flag kinds).

Kinds live in `scenarios/moba/moba_ui.json` `effects.kinds`, each `{renderer, colour, ...params}`; colours in `effects.colours` (`[r,g,b]` 0..1). Renderers (`js/client/fx.js`, geometry in `fx_shapes.js`) read these params, defaults in brackets. Scale multiplies the starred params. Every renderer is a fixed-capacity pool: a spawn past the cap is dropped, nothing allocates per frame.

| renderer | params (default) |
|---|---|
| `ring` | `r0`, `r1`, `seconds` (0.6), `mesh` `thin`\|`thick` (thin) |
| `burst` | `count` (14), `speed` (2.5), `lift` (0.6), `seconds` (0.6) |
| `float` | `text`, `seconds` (1.1), `rise` px (48), `size` px (16); the event's `text` wins |
| `flash` | `seconds` (0.18); optional `count`, `speed`, `lift` add a burst |
| `flag` | none; sets page flag `name` to `value` (0 off) |
| `projectile` | `model` `orb`\|`arrow`, `speed` (14), `size` (0.25), `trail` `{count, seconds}` (none; count capped 12), `arrive` kinds drawn on arrival |
| `beam` | `width`* (0.12), `seconds` (0.35), `segments` (8, max 24), `jitter`* (0.15), `flicker` (0.3); from source to target, else to the point; both ends follow units |
| `column` | `r`* (1), `height`* (6), `seconds` (0.8), `grow` (0.2, fraction of life spent rising); at target, else the point, else source |
| `nova` | `r0`* (0.5), `r1`* (5), `spokes`* (8, max 32), `seconds` (0.6), `spin` (0) |
| `aura` | `r`* (1.2), `rise`* (1.6), `motes`* (8, max 48), `seconds` (3), `pulse` Hz (1); follows target, else source |
| `zone` | `r`* (3), `spokes`* (6), `motes`* (10), `seconds` (4), `spin` (0.5); ground decal at the point |
| `orbit` | `count`* (3), `r`* (1), `height`* (1), `speed` (2), `seconds` (3); follows target, else source |
| `spiral` | `turns` (2), `height`* (2.5), `motes`* (16), `seconds` (1.2); follows target, else source |
| `cone` | `length`* (4), `angle` degrees (60), `count`* (10), `seconds` (0.5); from source toward target or point |

School colours: `fire frost lightning nature shadow holy arcane poison blood earth wind physical`; the table also holds `spell missile kill level respawn flash gold damage` for the base kinds.

`scenarios/moba/ability_fx.lua` (generated by `tools/gen-ability-fx.mjs`) returns `{[ability_name] = {school, form, size, speed, dur, hits}}`: `school` a school name, `form` one of `bolt beam chain nova zone column aura cone summon blink orbit spiral`, `size` the scale in thousandths, `dur` the duration in ms, `hits` the chain length. `effects.lua` `on_cast` looks the cast ability up there: a hit composes a cast event (`fx_cast_ring`, `fx_cast_nova` for nova/zone/column/summon, `fx_cast_spiral` for orbit/spiral), the form's travel event (`fx_bolt_<school>`, `fx_beam`, `fx_nova`, `fx_zone`, `fx_column`, `fx_aura`, `fx_cone`, `fx_orbit`, `fx_spiral`, two `fx_flash` for blink; chain re-aims `fx_beam` at the next hostile within 6 units, `hits` links) and an impact event `fx_hit_<school>`; an unknown school falls back to `arcane`, and an ability with no signature shows the generic ring, burst and orb. `fx_bolt_<school>` and `fx_hit_<school>` exist for every school in `effects.lua` `SCHOOLS`.

`scenarios/moba/buff_fx.lua` (generated by `tools/gen-buff-fx.mjs`) maps buff kinds to `{school, r, motes}` and abilities to `{kind, ticks}`. `buff_fx.aura_for(ability_name)` returns `kind, school, ms` (ticks at 16 per second converted to milliseconds), or `nil` for an ability that holds no buff, ready to pass as an `fx_aura` colour and `duration`.

`node scenarios/moba/tools/check-fx.mjs` (CI runs it) exits 1 with `file:line` messages when `effects.lua` emits a kind `moba_ui.json` lacks, a signature or buff names a school or form that is not defined, a kind names a missing colour or arrival kind, or a kind's renderer is not drawn by `fx.js`.

## Checking a mod offline

`mod_check [--fog on|off] <pack-dir> <mod.lua> [ticks]` (`rust/crates/mod-host/src/bin/mod_check.rs`) builds the pack's image, loads one mod into it, runs the ticks (default 100), and prints one `mod-check` line. It exits 0 when the mod loads and every tick runs. Build it with `cargo build -p mod-host --bin mod_check`. Fog of war is on unless `--fog off`.

`oscraft-lsp` (the `mod-lsp` crate) gives editors diagnostics for `.osd` and `.lua` files over stdio.

## Examples

`mods/` holds example mods (`waves_example.lua`, `unit_bounty_example.lua`, `upgrade_tree_example.lua`, and others). Each `game.*` name they use is bound by the host (the natives in `piccolo_host.rs` and the Lua prelude in `shared.rs`). The moba pack (`scenarios/moba/*.lua`) is the full reference: it is a pack of modules required from `init.lua`.

## Debugging

The page exposes the pack's Lua state on `window.oscraft` for the browser console. Every view is a read-only copy; `oscraft.debug.run` is the only dispatch. The calls work on a local match (a page that runs the sim);
`actions` and `run` are answered by the local sim, and a joined page sends `run` to the host like the input layer does.

- `oscraft.debug.actions()`: the actions `scenarios/moba/controls.lua` answers, as an array of names. The pack
  returns them: `on_action("debug_actions")` emits the list as a `debug` UI event, which the page reads straight away.
  A pack that adds an action lists it in its own `ACTIONS` table, so the page needs no change. Returns `null`
  without a local sim.
- `oscraft.debug.effects()`: the last 64 display effects the page drained from the sim (`game.effect`), each with
  `kind`, `x`, `y`, `source`, `target`, `team`, `colour`, `duration`, `text`, `scale`, `name` and `value`. The queue
  itself is drained once per frame, so this is the page's recent copy, not a peek at the queue.
- `oscraft.debug.modErrors()`: the mod error log of this page, newest last, as `[{tick, text}]` (the last 50 entries). It
  holds every error the page logged as `[mods]`: a failed load, a failed `setup`, a failed `tick` or action.
- `oscraft.debug.swap()`: `{applied, pending, rejections}`. `applied` is the last data swap this page applied or
  failed (`tick`, `file`, `ok`, `carried`, `refused`, `error`, `readiness`); `pending` is the swap still waiting for
  its tick (`tick`, `file`, `hash`, `deferrals`, `ready`), or `null`; `rejections` are the last refused announcements.
- `oscraft.debug.run(action, args)`: dispatches `action` with `args` through the same path the keys and clicks take
  (`on_action(name, args)`), for example `oscraft.debug.run("view", {fog: false})`. It returns `true` when it
  sent the dispatch and `false` without a sim or with an empty name. The pack's error text goes to the `[mods]` log.
  A dispatch changes the match, so use it on a debug match only.
