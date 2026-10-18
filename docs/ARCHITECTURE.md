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

## Content without rebuilds

Each kind of content, the steps a change to it needs, and why. Engine: read at run time, no compile. Build-only: compiled into a derived file, no Rust source change. Rust-only: a Rust constant or table must change and the Rust crates rebuild.

| Content | Class | Steps for a change | Reason |
|---|---|---|---|
| `mods/*.lua` mod source | Engine | Edit; copy to the deployed `mods/` (build.sh `cp -R mods`); hot reload or restart the room | mod-host reads the source at run time (`read_to_string` in `mlua_host.rs`, `piccolo_host.rs`) and hot reload swaps functions only |
| `hero_rules.txt` | Engine | Edit; reload the room | `server/src/mods.rs` parses it when the mod directory loads; defaults apply to omitted keys |
| `js/client/doodads.json`, `assets.json`, `assets/` models | Engine | None beyond the `js/client` copy in build.sh | Static files fetched by the page at run time, with a fallback when `assets.json` is absent |
| `js/client/*.js` modules | Engine | None | ES modules through an import map, no bundler |
| `scenarios/moba/*.osd` (units, abilities, items, waves, map layout) | Build-only | `cargo run -p mapexport -- <site>/map.bin ../scenarios/moba`, then restart `oscraftd --data` with the new image | The sim loads only the OSDT image through `loader.rs`; `.osd` is compiled by `osdt-data` |
| `scenarios/moba/map.osd` (generated) | Build-only | Edit `scripts/gen-dota-map.mjs`, run it, then mapexport | The file is generated from leamare data; hand edits are overwritten |
| `map.bin`, `moba.bin` | Build-only | mapexport (build.sh line 20) | Derived from `.osd`; never committed |
| Doodad kind names (`"tree"`, `"tower_square"`, ...) | Rust-only | Add the kind to the builtin list in `rust/crates/data/src/format.rs`, rebuild sim and data, then mapexport | An unknown kind fails the load with `unknown doodad kind`; the list is a Rust builtin |
| Player start positions (`rust/crates/data/src/builtins.rs`) | Rust-only | Edit the Rust table, rebuild data | Named in docs/CAPABILITIES.md as a builtin, not `.osd` data |
| Camp leash, respawn, Roshan and rune numbers (`sim/src/camp.rs`) | Rust-only | Edit constants; rebuild sim, server and the wasm package | Hashed sim constants, marked UNVERIFIED |
| Wave spawn `p3` bit layout (`wave_spec_fits` in `rust/crates/data/src/validate.rs`) | Rust-only | Edit the validator; rebuild data | Validation of a packed field is in Rust |
| Lane routes (`scenarios/moba/routes.osd`) | Build-only | Edit `routes.osd`; mapexport | Single route source. The generator must not emit `route(...)` lines: the data validator rejects a route declared twice |

Counts: Engine 4 rows, Build-only 4 rows, Rust-only 4 rows.

Follow-ups for the remaining Rust-only content:

1. Move the doodad kind list from `format.rs` into a data table so a new kind needs only mapexport.
2. Move player start positions from `builtins.rs` into the `start` directive of `.osd` data.
3. Move camp and rune numbers from `camp.rs` into `behaviour`/`economy` data (already class (b) above).
4. Document the wave `p3` bit layout in `rust/crates/data/FORMAT.md` and derive its range from the unit table, so wave content needs no validator edit.
5. Add a generator check that `scripts/gen-dota-map.mjs` emits no `route(...)` lines while `routes.osd` exists.
6. Re-classify `.osd` as Engine if the server-side compile (`osdt_data::generate` in `server/src/map_image.rs`, not yet on main) lands.

## Moddable systems

Inventory for moving the whole game into Lua (maps and mods, hot reload, no Rust rebuild). Classes: **MOVE** = logic becomes Lua through a hook, query or command; **DATA** = values become scenario or asset data; **STAY** = Rust, with the reason it cannot move. Moddable today: yes, partly, or no. Paths are repo-relative.

| System | Where it lives now (file, language) | Moddable today | Lua boundary (hook / query / data) | Class |
|---|---|---|---|---|
| Camp (neutral leash, respawn, runes) | `rust/crates/sim/src/camp.rs` (Rust) | partly | hook `on_died`; data `scenarios/moba/neutral.osd`; constants in camp.rs | MOVE |
| Aggro and target priority | `rust/crates/sim/src/aggro.rs` (Rust) | partly | data `scenarios/moba/hero_rules.txt` (HeroRules) | DATA |
| Courier | `rust/crates/sim/src/courier.rs` (Rust) | no | hook `on_tick`; cmd move; query courier state | MOVE |
| Wards | `rust/crates/sim/src/wards.rs` (Rust) | no | effect kind `ward` in data; hook `on_enter` for vision | MOVE |
| Glyph (team duration and cooldown) | `rust/crates/sim/src/rules.rs` (Rust, line ~215) | no | data `scenarios/moba/behaviours.osd` | DATA |
| Shop (stock, buy) | `rust/crates/sim/src/shop.rs` (Rust); `js/client/ui_shop.js` | partly | query item catalog; cmd buy | MOVE |
| Hero stats and respawn | `rust/crates/sim/src/hero.rs` (Rust); `scenarios/moba/hero_rules.txt` | partly | data `hero_rules.txt`; hook `on_died` | DATA |
| XP | `rust/crates/sim/src/xp.rs` (Rust) | no | data `scenarios/moba/economy.osd`; hook `on_died` with killer | DATA |
| Gold | `rust/crates/sim/src/harvest.rs`, `inventory.rs` (Rust); `mods/creep_gold.lua` (Lua) | partly | hook `on_died`; cmd grant gold | MOVE |
| Towers and backdoor | `rust/crates/sim/src/aggro.rs`, `hero.rs` (Rust); `mods/tower_buff.lua` (Lua) | partly | hook `on_attacked`; data doodad and unit `.osd` | MOVE |
| Route and march | `rust/crates/sim/src/route.rs`, `movement.rs` (Rust); `scenarios/moba/routes.osd` | partly | data `routes.osd` (single source) | DATA |
| Win and objectives | no discrete module; `rust/crates/sim/src/modapi.rs` (victory), `mods/td_waves.lua` (defeat), `triggers.rs` | partly | hook `on_tick`; needs cmd end-of-match | MOVE |
| Spawns and waves | `mods/moba_waves.lua`, `mods/td_waves.lua` (Lua); `scenarios/moba/waves.osd`; sim `waves.rs` (deleted in the uncommitted working tree); `rust/crates/data/src/validate.rs` (p3 layout, Rust) | yes (Lua drives it) | hook `on_tick`; cmd spawn; data `waves.osd` | MOVE |
| Strike (attack resolution) | `rust/crates/sim/src/combat.rs`, `projectiles.rs` (Rust) | partly | effect kind `damage`; hook `on_attacked` | STAY: per-tick hashed combat math; the lua surface is the effect kind |
| Movement and pathing | `rust/crates/sim/src/movement.rs`, `pathfinding.rs`, `astar.rs` (Rust) | partly | query position; cmd move; speed in `UnitSpec` | STAY: per-tick hashed pathing must stay bit-identical across native and wasm |
| Bot hero AI | `mods/bot_hero.lua`, `mods/bot_lib.lua` (Lua) | yes | hook `on_tick`; cmds through `game` | MOVE (already Lua; geometry constants to data) |
| Creep behaviour triggers | `rust/crates/sim/src/triggers.rs` (Rust); `scenarios/moba/behaviours.osd` | partly | data `behaviours.osd` | DATA |
| Bot seating | `mods/bot_lib.lua` (`BOT_SLOTS`, `game.add_player`) | yes | hook `on_player_joined` | MOVE |
| Hero panel and scoreboard | `js/client/hud5.js`, `js/client/hud.js` (JS) | no | query `/state`; no UI host call | MOVE |
| Shop UI flow | `js/client/ui_shop.js` (JS) | partly | query `item_catalog_json`; cmd buy | MOVE |
| Skills bar | `js/client/ui_skills.js`, `hud.js` `ABILITY_SLOTS` (JS) | partly | data ability slots; needs UI host call | MOVE |
| Inventory UI | `js/client/ui_inventory.js` (JS) | partly | query inventory; needs UI host call | MOVE |
| Lobby and room setup | `js/client/lobby.js`, `js/client/rooms.js` (JS); `rust/crates/server/src/room.rs` (Rust) | partly | hook `on_player_joined`; room config data | MOVE |
| Mod UI overlay | `js/client/modui.js` (JS); events polled from `GET /events` | partly | the bridge itself: mod UI events | STAY: it is the host renderer mods call into; keep it as the host ABI |
| Key bindings and hotkeys | `js/client/hud.js` `ABILITY_SLOTS`, `ATTACK_KEY` (JS) | no | data key map; needs key-bind UI call | DATA |
| Order input (click, target) | `js/client/play.js` (JS); `rust/crates/server/src/order.rs` (Rust) | partly | cmd order | STAY: orders are validated and networked; the wire format is fixed |
| Camera control (bounds, zoom, follow) | `js/client/play.js` (`cameraDistance`, `CAMERA_PLAY_DISTANCE`); `js/client/render3d.js` (orbit) | no | hook for camera updates; needs camera host call | MOVE |
| Camera constants | `js/client/play.js`, `render3d.js` | no | data | DATA |
| Fog of war rules (sim) | `rust/crates/sim/src/fog.rs` (Rust) | partly | query `visible(x, y)` | STAY: visibility is hashed sim state |
| Vision and sight values | `rust/crates/sim/src/arrays.rs`, `loader.rs`; `UnitSpec.sight` in `modapi.rs` | partly | data unit `sight` | DATA |
| Fog render | `js/client/fogview.js` (JS) | no | query fog grid | MOVE |
| Effects and FX (spawn, animation rules) | `js/client/fx.js`, `js/client/anim.js` (JS) | no | hook `on_spawned` / `on_attacked` to a client fx command | MOVE |
| Map editor | `js/client/editor.js`, `js/client/editor.html`, `js/client/editor-check.mjs` (JS) | yes (client tool) | none needed beyond data | MOVE |
| Map generation | `rust/crates/mapexport/src/main.rs` (Rust); `scripts/gen-dota-map.mjs`; `rust/crates/server/src/map_image.rs` | partly | data: generated `scenarios/moba/map.osd` | DATA |
| Doodad placement and kinds | `rust/crates/sim/src/doodads.rs` (Rust); kind list in `rust/crates/data/src/format.rs` | no | data: kind table; needs list moved out of Rust | DATA |
| Scenario data (`.osd`) | `scenarios/moba/*.osd`, `scenarios/duel-routes/*.osd` (Starlark); `rust/crates/data/src/reader.rs`, `validate.rs` | yes | data | DATA |
| Player start placement | `rust/crates/data/src/builtins.rs` (Rust) | no | data: `start` directive in `.osd` | DATA |
| Asset manifest | `js/client/assets.json` (JSON) | yes | data | DATA |
| Doodad manifest | `js/client/doodads.json` (JSON); `DOODAD_KINDS` in `js/client/play.js` | partly | data | DATA |
| Effect kinds (enum and dispatch) | `rust/crates/data/src/format.rs` `EFFECT_KINDS` (40+ kinds, Rust); `rust/crates/sim/src/modapi.rs` `MOD_EFFECTS` (7 kinds, Rust) | partly: mods get 7 of the 40+ | data: effect kind names; dispatch in sim | STAY: binary effect ABI and sim dispatch table; extend `MOD_EFFECTS` in Rust first |
| Ability definitions | `rust/crates/data/src/abilityext.rs`, `rust/crates/sim/src/loader.rs`; `AbilitySpec` in `modapi.rs` | partly | hook `define_ability`; data `abilities.osd` | DATA |
| Unit and item definitions | `UnitSpec`, `ItemSpec`, `UpgradeSpec` in `rust/crates/sim/src/modapi.rs`; `scenarios/moba/units*.osd`, `items.osd` | yes | data; mod define calls | DATA |
| Lockstep and state sync | `rust/crates/server/src/lockstep.rs`, `room.rs` (Rust) | no | none | STAY: determinism and frame integrity |
| Wire frame codec | `js/wire/frame.js`, `js/client/wire/frame.js` (JS); `docs/WIRE.md` | no | none | STAY: fixed protocol, checked against the server copy in CI |
| Order rate limits and tick budget | `rust/crates/server/src/budget.rs`, `order.rs` (Rust) | partly | data: limits config | DATA |
| Relay and signalling | `rust/crates/relay/src/main.rs` (Rust); `rust/crates/server/src/relay.rs` | no | none | STAY: transport; no listening ports is policy |
| HTTP routes and static files | `rust/crates/server/src/http.rs`, `static_files.rs` (Rust) | no | none | STAY: fixed protocol surface |
| Replay record and playback | `rust/crates/server/src/replay.rs` (Rust); `js/client/replay.js` (JS) | partly | hook for playback UI; the replay file format is STAY | MOVE |
| Spectator follow | `js/client/replay.js`; state frames from `/state` and wire | partly | query state frames | MOVE |
| Save and load bytes | `rust/crates/sim/src/persist.rs` (Rust) | no | cmd save; cmd load | STAY: hash-checked binary image of sim state |
| Save and load flow | none (no module today) | no | hook and cmd on top of persist | MOVE |
| Campaign and scripted missions | none in repo; `mods/*.lua` per scenario; `mods/td_waves.lua` as the nearest mission | no | hooks `on_player_joined`, `on_tick`; data mission `.osd` (new) | MOVE |
| Menus and screens | `js/client/play.html`, `js/client/lobby.js` (JS) | no | hook for screen state; query room list | MOVE |
| Scoring and stats | `mods/killer.lua`, `mods/creep_gold.lua` (Lua); `js/client/hud5.js` scoreboard | partly | query per-player totals; kv store (none yet) | MOVE |
| Achievements | none in repo | no | hooks `on_died`, `on_enter`; data achievement `.osd` (new) | MOVE |
| Fixed tick scheduler | `rust/crates/sim/src/tick.rs` (Rust); `rust/crates/server/src/supervise.rs` | yes at the hook (`on_tick`) | hook `on_tick` | STAY: 16 ticks per second fixed step is the determinism clock |
| Client frame loop | `js/client/play.js`, `js/client/frame_timer.js` (JS) | no | none | STAY: browser frame pacing |
| 3D renderer | `js/client/render3d.js` (JS, WebGL2) | no | data: model map | STAY: GPU API boundary |
| TSL renderer | `js/client/render_tsl.js` (JS, WebGPU) | no | none | STAY: GPU API boundary |
| Terrain pipeline | `js/client/terrain.js`, `tiles.js`, `texture.js` (JS) | partly | data: map and tile sources | DATA |
| Model loading | `js/client/glb.js`, `texture.js` (JS); `js/client/assets.json` | partly | data: assets manifest | DATA |

Counts: MOVE 26, DATA 20, STAY 15 (61 systems).

Ordering. Systems that need a new host API first: the mod effect-kind set (`MOD_EFFECTS` is 7 of 40+), a UI and key-binding host call (shop, skills, inventory, hotkeys, camera), camp, courier, ward, glyph and gold hooks with commands, an end-of-match command (win and campaign), a save/load command, a per-player stats store with achievements, and a fog query. Systems that can move as data now: camp, glyph, XP, economy and hero numbers (`behaviours.osd`, `economy.osd`, `hero_rules.txt`), routes (`routes.osd`), camera constants, key maps, asset and doodad manifests, start placement, and map generation. Blockers in order: effect-kind set, UI host call, then the win and save commands.
