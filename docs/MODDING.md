# Packs and the Lua mod API

The engine is generic. A game is a pack: a directory, a `.tar.gz` or a `.zip` with a `manifest.json` at its root. A pack holds its Lua modules, its data (`.osd`, `.json`, `.txt`, `.tsv`), the compiled map image and its assets. The engine names no genre: `scenarios/moba` and `scenarios/duel-routes` are two packs that the same engine reads.

The browser is the only build. `rust/crates/web` compiles the sim and the mod host to wasm, and the mod host runs the pack's Lua on piccolo (`rust/crates/mod-host`, the default `piccolo` feature).

## Loading a pack

- The page opens `play.html?package=<dir or archive>` (`pack=` is the same). The default is `scenarios/moba` (`js/client/pack_load.js`). A directory, a `.tar.gz`/`.tgz` and a `.zip` go through the same reader.
- The page reads only the paths the manifest names.
- A pack holds no code the page runs. A `.js`, `.mjs`, `.cjs`, `.html`, `.wasm`, `.ts` or `.map` path is refused on load, on edit and on transfer.
- `node scripts/pack-package.mjs <package-dir> <out-dir> [--build]` writes `<name>-<version>.tar.gz`. `--build` runs `mapexport` first, so the manifest's image is in the archive.

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
| `on_player_joined` | `player` | the host reports a seat taken |
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
