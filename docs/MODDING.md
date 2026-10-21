# Packs and the Lua mod API

The engine is generic. A game is a pack: a directory, a `.tar.gz` or a `.zip` with a `manifest.json` at its root. A pack holds its Lua mods, its data (`.osd`, `.json`, `.txt`, `.tsv`), the compiled map image and its assets. Nothing in the engine names a genre. `scenarios/moba` and `scenarios/duel-routes` are two packs that the same engine reads.

The browser is the only build. `rust/crates/web` compiles the sim and the mod host to wasm, and the mod host runs the pack's Lua on piccolo (`rust/crates/mod-host`, feature `piccolo`), the only Lua runtime.

## Loading a pack

- The page opens `play.html?package=<dir or archive>`. The default is `scenarios/moba` (`js/client/pack_load.js`). A directory, a `.tar.gz`/`.tgz` and a `.zip` go through the same reader.
- The page reads only the paths the manifest names.
- A pack holds no code the page runs. A `.js`, `.mjs`, `.cjs`, `.html`, `.wasm`, `.ts` or `.map` path is refused on load, on edit and on transfer.
- `node scripts/pack-package.mjs <pkg-dir> <out-dir> [--build]` writes `<name>-<version>.tar.gz`. `--build` runs `mapexport` first, so the manifest's image is in the archive.

### Manifest

`scenarios/moba/manifest.json` is the reference. Its fields:

| field | meaning |
|---|---|
| `schema` | must be `1` |
| `name`, `version` | identity |
| `map` | the map file `mapexport` reads |
| `data` | the `.osd` files that compile into the image |
| `image` | the compiled image, written by `mapexport <pack-dir> <pack-dir>/<image>` |
| `mods` | the Lua files, in load order |
| `rules` | a `key=value` text file; `game.rule(key)` reads it |
| `tables`, `fixtures` | data tables and test fixtures |
| `ui`, `effects` | JSON the page reads for controls and display |
| `assets` | texture, model and catalogue paths, under `assets.root` |

## Editing and sharing a pack

The host opens the pack editor from its menu (`js/client/pack_editor.js`). It has a file tree, a text editor for `.lua .osd .json .txt .tsv`, a drop zone for asset files, Save, and a `download pack (.zip)` link.

- Only the host edits. A peer sees the same pack read-only.
- Save sends each changed file to every peer as `PACK_FILE` frames (`pack_sync.js`). A file over 256 KiB travels by reference: the peer asks for it with `PACK_WANT`, by content hash.
- A peer applies only the frames from the host.

## Mods

- Each file in `mods` is one Lua chunk. The host runs them in list order in one environment shared by all of them, over the match's sim, so a later chunk sees the globals an earlier one defined. `Game.load_mods` compiles every mod before any takes effect. If one fails, the last set that loaded stays in place and the error comes back to the page.
- A load error names the file and line: `<file>:<line>: <message>`.
- Each chunk must define a global `on_tick(state)`. The host refuses a chunk without one (`<file>: on_tick is not defined`). See Known gaps below.
- `state` is the host's table of scalar fields, shared by the hooks. `state.ticks` is seeded by the host.
- A hook call runs under a budget of 2,500 VM steps. Over it, the call fails with `instruction_limit`.
- Mod code has no file, network or OS access. The globals are `pcall`, `error`, `assert`, `type`, `tostring`, `tonumber`, `select`, `pairs`, `ipairs`, `next`, `math`, plus the `game` table and the host globals listed in [SCRIPTING.md](SCRIPTING.md).

## Hooks

A mod defines a hook as a global function. The engine calls each one that exists, in load order.

| hook | arguments | returns / effect |
|---|---|---|
| `on_tick` | `state` | once per step, after the events and the systems. Required. |
| `on_spawned` | `unit` | a unit appears |
| `on_attacked` | `unit` | a unit's hit points fall |
| `on_died` | `unit, killer, killer_player, bounty, victim_player, killer_hero` | a unit dies; `killer` and `killer_player` are `nil` for an attackerless death |
| `on_enter`, `on_leave` | `region, unit` | a unit enters or leaves a region from `define_region` |
| `on_player_joined` | `player` | the host reports a seat taken |
| `on_unit_died` | `unit, state` | returns the respawn seconds, `false` to leave the unit dead, or `nil` for the engine's timer |
| `on_cast` | `caster, ability, state` | an ability is cast |
| `on_hit` | `attacker, victim, dealt, state` | a hit lands |
| `on_march` | `unit, player, state` | a unit with no destination asks for a point; return `x, y` in fixed point to order it there |
| `on_check_win` | `state` | returns the side that wins, or `nil`/`false` for none |
| `target_priority` | `unit, {target}, state` | returns a target class, or `nil`/`true` to keep the default |
| `on_request` | `kind, unit, item, state` | item rule; `kind` is `acquire`, `release`, `take_out` or `carry`; returns true, or false and a reason |
| `on_skill_up` | `unit, slot, state` | returns `true` to raise the rank, `false` to refuse |
| `on_action` | `name, args, state` | called by the page's `dispatch_action(name, args)` |
| `on_pointer` | `kind, x, y, button, state` | called by the page's `pointer(kind, x, y, button)`; `kind` is `down`, `up` or `move`; `x, y` are tile coordinates |

`on_died` receives `killer_hero` as its sixth argument. A trigger registered with `game.trigger` runs after the global of the same name.

## The page's entry points

These are methods of the wasm `Game` (`rust/crates/web/src/lib.rs`). The page calls them; mods do not.

| method | what it does |
|---|---|
| `load_mods(json)` | loads `[{name, text}, ...]` in manifest order; returns `""` or the error |
| `dispatch_action(name, args)` | runs `on_action` of each mod; `args` is a JSON string |
| `pointer(kind, x, y, button)` | runs `on_pointer` of each mod |
| `drain_ui()` | returns the UI objects queued since the last call, as a JSON array: `game.ui_event`, `camera`, `message`, `banner`, `panel`, `sound` |
| `drain_effects()` | returns the display effects queued by `game.effect`, as a JSON array |

The page renders what it drains and decides nothing about it. Controls, key bindings and HUD text are pack data: the manifest's `ui` file and the mods that read it.

## Checking a mod offline

`mod_check [--fog on|off] <pack-dir> <mod.lua> [ticks]` builds the pack's image, loads one mod into it, runs the ticks (default 100), and prints one `mod-check` line. It exits 0 when the mod loads and every tick runs. Build it with `cargo build -p mod-host --bin mod_check`. Fog of war is on unless `--fog off`.

`oscraft-lsp` gives editors diagnostics for `.osd` and `.lua` files over stdio.

## Examples and gaps

`mods/` holds example mods. Seven of them (`waves_example`, `killer`, `custom_unit`, `upgrade_tree_example`, `unit_queries`, `custom_item`, `custom_spell`) use names piccolo does not bind yet: `game.var`, `game.player_var`, `game.unit_var`, `game.region`, `game.region_center`, `game.score` and `game.add_score`. They do not load under piccolo until those names are bound. The moba pack (`scenarios/moba/*.lua`) is the working reference, and it uses only bound names.

## Known gaps

- `scenarios/moba/manifest.json` lists helper files that define no `on_tick` (`hero.lua`, `camp.lua`, `win.lua`, and others), while `protection.lua` defines the moba `on_tick`. Under the rule above the host refuses those helpers, so the moba pack does not load as listed until the host and the manifest agree on one `on_tick` rule.

## Debugging

The page exposes the pack's Lua state on `window.oscraft` for the browser console. Every view is a read-only
copy; `oscraft.debug.run` is the only dispatch. The calls work on a local match (a page that runs the sim);
`actions` and `run` are answered by the local sim, and a joined page sends `run` to the host like the input layer does.

- `oscraft.debug.actions()`: the actions `scenarios/moba/controls.lua` answers, as an array of names. The pack
  returns them: `on_action("debug_actions")` emits the list as a `debug` UI event, which the page reads straight away.
  A pack that adds an action lists it in its own `ACTIONS` table, so the page needs no change. Returns `null`
  without a local sim.
- `oscraft.debug.effects()`: the last 64 display effects the page drained from the sim (`game.effect`), each with
  `kind`, `x`, `y`, `source`, `target`, `team`, `colour`, `duration`, `text`, `scale`, `name` and `value`. The queue
  itself is drained once per frame, so this is the page's recent copy, not a peek at the queue.
- `oscraft.debug.modErrors()`: the mod error log of this page, newest last, as `[{tick, text}]` (50 entries). It
  holds every error the page logged as `[mods]`: a failed load, a failed `setup`, a failed `tick` or action.
- `oscraft.debug.swap()`: `{applied, pending, rejections}`. `applied` is the last data swap this page applied or
  failed (`tick`, `file`, `ok`, `carried`, `refused`, `error`, `readiness`); `pending` is the swap still waiting for
  its tick (`tick`, `file`, `hash`, `deferrals`, `ready`), or `null`; `rejections` are the last refused announcements.
- `oscraft.debug.run(action, args)`: dispatches `action` with `args` through the same path the keys and clicks take
  (`on_action(name, args)`), for example `oscraft.debug.run("view", {fog: false})`. It returns `true` when it
  sent the dispatch and `false` without a sim or with an empty name. The pack's error text goes to the `[mods]` log.
  A dispatch changes the match, so use it on a debug match only.
