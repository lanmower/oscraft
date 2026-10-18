# Architecture

One deterministic simulation, compiled twice: native for the server and tools, wasm for the browser. Same inputs give the same state hash on every platform.

## Crates (`rust/crates`)

| Crate | Role |
|---|---|
| `sim` | The simulation. Q16.16 fixed point (`fixed.rs`), struct-of-arrays state (`arrays.rs`), a fixed tick (16 per second), and one module per system: combat, movement, pathfinding, production, harvesting, buffs and status, fog, projectiles, shields, healers, heroes, skills, xp, inventory, camps, triggers, upgrades, transport. State hashes with FNV (`hash.rs`) and saves through `persist.rs`. |
| `data` (`osdt-data`) | Evaluates `.osd` Starlark sources and writes the OSDT catalog image. |
| `server` (`oscraftd`) | HTTP rooms, orders, state, deltas, events, replay, tick budget and shedding, optional mod host. |
| `web` | wasm-bindgen wrapper of `sim` as a `Game` for the browser. |
| `mapexport` | Writes the map/catalog image the browser loads from a data directory. |
| `mod-host` | Lua 5.4 mod runtime (mlua native, piccolo for the wasip3 guest), with `mod_check` and `reload_harness` binaries. |
| `mod-lsp` (`oscraft-lsp`) | Language server for `.osd` and `.lua` over stdio, no network port. |
| `relay` | Minimal Nostr relay for browser CI jobs. |
| `wasip3`, `spike`, `mpq` | Component-model guest, the Lua contract proof, and an archive reader. |

## Data format

A scenario is a directory of `.osd` files (see [SCENARIOS.md](SCENARIOS.md)): `unit`, `ability`, `effect`, `item`, `skill`, `behaviour`, `economy`, `grid`/`paint`/`doodad`/`start`/`spawn`. `osdt-data` evaluates them in name order and emits the binary OSDT image: a header, a section table, then fixed-size records, little-endian, no padding. Layout and version history: `rust/crates/data/FORMAT.md`. A reader rejects an unknown version or a length mismatch. The sim loads the image through `loader.rs`; the loader and `format.rs` share one set of builtin tables.

## The sim

`tick.rs` steps systems in a fixed order over arrays sized to the unit cap. Orders enter through `order.rs` (server) or the wasm `Game`. Behaviours (`trigger`, `action`) drive idle units, spawn timers and the win trigger. Everything hashed is sim state: camps, healers, status, inventory, hero timers.

## The server

`oscraftd --data image.osdt --room name` serves rooms. Endpoints: `GET /state`, `/events`, `/delta`, `/catalog`, `/mod-version`, `/mod-events`, `/resources`, `POST /order`, `POST /client-error`, and `/site/` static files. Orders are rate-limited per room; a tick budget logs overruns and can shed work. Logs are `ev=<name> key=value` lines documented in [LOG_SCHEMA.md](LOG_SCHEMA.md).

## wasm

`rust/crates/web` builds with `wasm-pack build --target web`. The output (`web.js`, `web_bg.wasm`) is never committed: Pages, CI and `scripts/package/build.sh` rebuild `js/client/pkg`, `map.bin` and `moba.bin` from source (`js/client/pkg` and the maps are gitignored; stage them as ci.yml does to run the page locally).

## The browser client

`js/client/play.html` plus ES modules loaded through an import map (no bundler): `play.js` (WebGL2 view, sim loop, orders), `render3d.js`, `terrain.js`, `fx.js`, `hud*.js`, `ui_shop.js`, `ui_inventory.js`, `ui_skills.js`, `lobby.js`, `rooms.js`, `editor.js`. One page hosts the sim; others follow its state frames. See [HOST.md](HOST.md).

## The wire

Peers talk over wireweave data channels (WebRTC), with Nostr relays for signalling and room announcements. Frames: ORDER, STATE, HELLO, ERROR, two-byte header, little-endian ([WIRE.md](WIRE.md)). `js/wire/frame.js` is the codec; CI compares it with the client copy.

## The mod host

A mod defines `on_tick(state)` and event handlers (`on_spawned`, `on_attacked`, `on_died`, and more) and queues commands through host globals. Mods run under instruction and memory budgets, and hot reload replaces only functions, keeping `state`. Mods interface with existing units, items and abilities as if they were data defined ones. Reference: [MODDING.md](MODDING.md).

## Runtime-controlled logic

Where each control decision lives. Classes: **(a)** already Lua or movable to Lua now; **(b)** needs a host API or data field first; **(c)** stays Rust (deterministic core, serialization, networking, test harness).

| Site | Decision | Class | Reason |
|---|---|---|---|
| `mods/bot_lib.lua`, `mods/bot_hero.lua` (`Bot.think`, `pick_target`, `shop`, `retreat`, `walk_lane`) | Bot hero decisions: retreat, buy, target, lane push | (a) done | Already Lua through the mod API. `BOT_SLOTS` seats via `game.add_player`. |
| `mods/td_waves.lua` | Wave timer, leak counting, lives, defeat | (a) done | Already Lua. Spawn point and goal region duplicate scenario geometry. |
| `mods/creep_gold.lua`, `tower_buff.lua`, `teams.lua`, `tech_tree.lua`, `killer.lua`, `custom_*.lua`, `unit_queries.lua` | Gold, tower buffs, teams, tech, kill rules, custom content | (a) done | Already Lua. |
| `mods/bot_lib.lua` `LANE_Y`, `BASE_X`, `FOUNTAIN`, `PUSH_X`, `SHOPPING` | Map geometry and shop list used by bots | (b) | Hardcoded in Lua. Needs a map/region query and a shop-list data field to remove. |
| `mods/td_waves.lua` `SPAWN_X/Y`, `GOAL` | Wave spawn and goal geometry | (b) | Needs a region-by-name lookup from scenario data. |
| `sim/src/camp.rs` `CAMP_LEASH_MILLI`, `CAMP_RESPAWN_S`, `ROSHAN_RESPAWN_S`, `RUNE_*` | Camp leash, respawn timers, Roshan, bounty rune numbers | (b) | Marked UNVERIFIED placeholders in Rust. Designer-tunable, so they belong in scenario data (`behaviour`/`economy`). Moving them changes the data image format, so this is not done here. |
| `sim/src/camp.rs` `tick_camps`, `camp_died`, `tick_rune_pickups`, `respawn_camp` | Camp and rune mechanics | (c) | Hashed sim state. |
| `sim/src/aggro.rs` target priority (plain > hero > structure), backdoor | Auto-target choice | (c) | Core combat. Enabled and tuned through `HeroRules` (`hero_rules.txt`), which is already data. |
| `sim/src/hero.rs` `HeroRules` defaults, `set_hero_rules_text` | Hero rule defaults | (a) done | Already read from `hero_rules.txt`; defaults only apply when the file omits a key. |
| `sim/src/rules.rs` `cast_ability`, `cast_index`, `channel_ai` | Cast validation, channel interrupt | (c) | Determinism core. |
| `sim/src/triggers.rs` `fire_triggers`, `fire_trigger` | Behaviour trigger dispatch | (c) | Trigger definitions come from data; dispatch is core. |
| `sim/src/production.rs` `train_block`, `start_train`, `finish_train` | Training rules | (b) | Refusal and timing rules could move into data. |
| `sim/src/modapi.rs` `define_ability`, `ability_in_slot` | Mod-facing ability API | (c) | Host boundary. |
| `server/src/room.rs` `respawn(old, new, unit)` | Room state diff | (c) | Serialization and networking. |
| `server/examples/moba_match.rs` scripted hero policy | Witness bot for the 5v5 milestone check | (c) | Test harness, not game runtime. Keep it Rust so its determinism witness does not depend on a mod. |
| `mod-host/src/mlua_host.rs`, `piccolo_host.rs` | Host API and command queueing | (c) | The boundary itself. Both backends must expose the same names. |
| `js/client/hud.js` `ABILITY_SLOTS`, `ATTACK_KEY` | Key bindings | (b) | Could move to a Lua UI call. Needs a key-binding host call. |
| `js/client/ui_shop.js`, `ui_skills.js`, `ui_inventory.js` | Shop, skill and inventory flow | (b) | Flow choices could move to the Lua mod UI (`modui.js`). Needs a UI call that lets a mod drive the flow. |
| `js/client/rooms.js` `MAX_PLAYERS`, `play.js` colours and camera | Presentation constants | (c) | Not designer control logic. |

Nothing in class (a) remains in Rust. The next moves need host APIs: a map/region query for bot and wave geometry, a camp and training policy block in scenario data, and a key-binding and shop-flow UI call for the client.

## Client and server separation

Boundary. The browser owns rendering (`render3d.js`, and the TSL path when it lands), input, UI (`hud*.js`, `ui_*.js`, `lobby.js`, `editor.js`) and the browser sim wasm (`pkg/web.js`, loaded through `data-pkg`). The server owns authoritative sim hosting, rooms, mods and its HTTP surface. The only contract between them is the wire protocol ([WIRE.md](WIRE.md), `js/wire/frame.js`) and static files (`map.bin`, `moba.bin`, `models/`, `doodads.json`, `assets.json`).

Audit of `js/client` (imports, `fetch`, dynamic `import()`, wasm, and `/state`, `/events`, `/replay`, `/map` routes):

| # | Where | Coupling | Status | Fix |
|---|---|---|---|---|
| 1 | `modui.js` `poll`; `play.js` `eventsBase` from `?events=` | Polls the server's `GET /events` for mod UI events | Remaining, opt-in | Carry mod UI events in the wire frame or a static feed |
| 2 | `replay.js` start hint | Named the server's `/replay` route | Fixed: hint names any static `.replay` file; `?replay=URL` unchanged | none |
| 3 | `play.html` `data-pkg`, `play.js` `loadSim`, `replay.js` `start` | Sim `pkg/web.js` and `web_bg.wasm` are wasm-pack output of `rust/crates/web`; the page cannot run without that build | Allowed by the boundary (browser owns the sim wasm); built by CI, never committed | Keep; CI already stages it |
| 4 | `play.html` `data-map`, `play.js` `mapUrl`, `editor.js` `map.bin` | Map images written by `mapexport` | Static files on Pages | none |
| 5 | `js/client/editor-check.mjs` | Node witness in the client tree that runs the Rust `osdt-data` compile | Remaining | Move to `scripts/` and update the CI path |
| 6 | `play.html` import map | `wireweave`, `nostr-tools`, `xstate` from CDN; `three/webgpu` and `three/tsl` vendored under `js/client/vendor/three` (pinned in [NOTICE.md](../NOTICE.md)) | OK | none |

Coupling count, server routes plus Rust tooling in the client tree: before 4 (routes 3: `/events` fetch, `eventsBase` wiring, `/replay` hint; tool 1), after 3 (routes 2, tool 1). No client file requires a running server to load; `/events` is needed only when `?events=` is given.
