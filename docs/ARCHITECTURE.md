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

## Lobby

`js/client/lobby.js` is a pure reducer over `(state, action)`; the host's page renders it and announces each accepted action on `oscraft:lobby`. Actions: `claim`, `release`, `pick`, `ready`, `start`, and `fill`. `fill` is host-only and seats a computer ("bot:<slot>", first roster hero, ready) on every empty slot; it is refused with `nothing-to-fill` when no seat is open. The Start frame carries the seats' owner keys and heroes unchanged, so computer seats need no extra frame. The lobby's "Fill with bots" button sends `fill`.

## The wire

Peers talk over wireweave data channels (WebRTC), with Nostr relays for signalling and room announcements. Frames: ORDER, STATE, HELLO, ERROR, two-byte header, little-endian ([WIRE.md](WIRE.md)). `js/wire/frame.js` is the codec; CI compares it with the client copy.

## The mod host

A mod defines `on_tick(state)` and event handlers (`on_spawned`, `on_attacked`, `on_died`, and more) and queues commands through host globals. Mods run under instruction and memory budgets, and hot reload replaces only functions, keeping `state`. Mods interface with existing units, items and abilities as if they were data defined ones. Reference: [MODDING.md](MODDING.md).

## Runtime-controlled logic

Where each control decision lives. Classes: **(a)** already Lua or movable to Lua now; **(b)** needs a host API or data field first; **(c)** stays Rust (deterministic core, serialization, networking, test harness).

| Site | Decision | Class | Reason |
|---|---|---|---|
| `mods/bot_lib.lua`, `mods/bot_unit_example.lua` (`Bot.think`, `pick_target`, `shop`, `retreat`, `walk_lane`) | Bot hero decisions: retreat, buy, target, lane push | (a) done | Already Lua through the mod API. `BOT_SLOTS` seats via `game.add_player`. |
| `mods/waves_example.lua` | Wave timer, leak counting, lives, defeat | (a) done | Already Lua. Spawn point and goal region duplicate scenario geometry. |
| `mods/unit_bounty_example.lua`, `structure_buff_example.lua`, `team_rules_example.lua`, `upgrade_tree_example.lua`, `killer.lua`, `custom_*.lua`, `unit_queries.lua` | Gold, tower buffs, teams, tech, kill rules, custom content | (a) done | Already Lua. |
| `mods/bot_lib.lua` `LANE_Y`, `BASE_X`, `FOUNTAIN`, `PUSH_X`, `SHOPPING` | Map geometry and shop list used by bots | (b) | Hardcoded in Lua. Needs a map/region query and a shop-list data field to remove. |
| `mods/waves_example.lua` `SPAWN_X/Y`, `GOAL` | Wave spawn and goal geometry | (b) | Needs a region-by-name lookup from scenario data. |
| `sim/src/camp.rs` `CAMP_LEASH_MILLI`, `CAMP_RESPAWN_S`, `ROSHAN_RESPAWN_S`, `RUNE_*` | Camp leash, respawn timers, Roshan, bounty rune numbers | (b) | Marked UNVERIFIED placeholders in Rust. Designer-tunable, so they belong in scenario data (`behaviour`/`economy`). Moving them changes the data image format, so this is not done here. |
| `sim/src/camp.rs` `tick_camps`, `camp_died`, `tick_rune_pickups`, `respawn_camp` | Camp and rune mechanics | (c) | Hashed sim state. |
| `sim/src/aggro.rs` target priority (plain > hero > structure), backdoor | Auto-target choice | (c) | Core combat. Enabled and tuned through `HeroRules` (`hero_rules.txt`), which is already data. |
| `sim/src/hero.rs` `HeroRules` defaults, `set_hero_rules_text` | Hero rule defaults | (a) done | Already read from `hero_rules.txt`; defaults only apply when the file omits a key. |
| `sim/src/rules.rs` `cast_ability`, `cast_index`, `channel_ai` | Cast validation, channel interrupt | (c) | Determinism core. |
| `sim/src/behaviours.rs` `fire_triggers`, `fire_trigger` | Behaviour trigger dispatch | (c) | Trigger definitions come from data; dispatch is core. |
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
| Kill gold and team share in `scenarios/moba/kill_gold.lua` | Engine | Edit the file; copy to `mods/`; hot reload or restart the room | Lua pays the kill gold only while the sim flag `gold_by_mod` is set; no loader sets it at `369c3ba`, so the Rust payout (`pay_bounty`) also pays. The 20 percent share is hard-coded in the file |
| `hero_rules.txt` | Engine | Edit; reload the room | `server/src/mods.rs` parses it when the mod directory loads; defaults apply to omitted keys |
| `scenarios/moba/assets/doodads.json`, `scenarios/moba/assets/assets.json`, `scenarios/moba/assets/` models | Engine | None beyond the `js/client` copy in build.sh | Static files fetched by the page at run time, with a fallback when `assets.json` is absent |
| `js/client/*.js` modules | Engine | None | ES modules through an import map, no bundler |
| `scenarios/moba/*.osd` (units, abilities, items, waves, map layout) | Build-only | `cargo run -p mapexport -- <site>/map.bin ../scenarios/moba`, then restart `oscraftd --data` with the new image | The sim loads only the OSDT image through `loader.rs`; `.osd` is compiled by `osdt-data` |
| `scenarios/moba/map.osd` (generated) | Build-only | Edit `scenarios/moba/tools/gen-reference-map.mjs`, run it, then mapexport | The file is generated from leamare data; hand edits are overwritten |
| `map.bin`, `moba.bin` | Build-only | mapexport (build.sh line 20) | Derived from `.osd`; never committed |
| Doodad kind names (`"tree"`, `"tower_square"`, ...) | Rust-only | Add the kind to the builtin list in `rust/crates/data/src/format.rs`, rebuild sim and data, then mapexport | An unknown kind fails the load with `unknown doodad kind`; the list is a Rust builtin |
| Player start positions (`rust/crates/data/src/builtins.rs`) | Rust-only | Edit the Rust table, rebuild data | Named in docs/CAPABILITIES.md as a builtin, not `.osd` data |
| Camp leash, respawn, Roshan and rune numbers (`sim/src/camp.rs`) | Rust-only | Edit constants; rebuild sim, server and the wasm package | Hashed sim constants, marked UNVERIFIED |
| Wave spawn `p3` bit layout (`wave_spec_fits` in `rust/crates/data/src/validate.rs`) | Rust-only | Edit the validator; rebuild data | Validation of a packed field is in Rust |
| Lane routes (`scenarios/moba/routes.osd`) | Build-only | Edit `routes.osd`; mapexport | Single route source. The generator must not emit `route(...)` lines: the data validator rejects a route declared twice |

Counts: Engine 5 rows, Build-only 4 rows, Rust-only 4 rows.

### Rust-only game rules: final classes

Classes: MOVE = rule logic moves to a Lua hook or scenario data. DATA = number or table content moves to `.osd` data. STAY = stays in Rust, reason given. Source: symbol inventory of `rust/crates/sim/src` at origin `855d02c`. Signatures were classified without reading every body where marked.

| Rule | Where now | Class | Reason |
|---|---|---|---|
| Gold grants on kill | `scenarios/moba/kill_gold.lua` (`on_died`, `GOLD_BY_MOD`); engine `combat.rs` `pay_bounty` stays as the fallback while the flag is unset | MOVE | Rule logic in mod code |
| Lane creep and siege waves | `scenarios/moba/lanes.lua` (`on_tick`, timers); engine `waves.rs` and the spawn command | MOVE | Rule logic in mod code |
| Hero respawn time | `scenarios/moba/hero_respawn.lua` (`on_unit_died`) | MOVE | Rule logic in mod code |
| Backdoor and glyph protection | `scenarios/moba/protection.lua` (`on_tick`, `on_cast`); no longer in the engine | MOVE | Rule logic in mod code |
| Route march target | `scenarios/moba/march.lua` (`on_march`); engine `waypoints.rs` keeps the route records | MOVE | Rule logic in mod code |
| Win condition | `scenarios/moba/win.lua` (`on_check_win`); engine `behaviours.rs` `winner` is the generic victory primitive | MOVE | Rule logic in mod code |
| Shop buy and sell, courier delivery | `scenarios/moba/store.lua` (`on_buy`, `on_sell`, `on_unstash`, `on_deliver`); engine `store.rs` and `order_memory.rs` keep the primitives | MOVE | Rule logic in mod code |
| Camp respawn, aggro target priority, buyback | still engine: `camp.rs` `respawn_camp`, `aggro.rs`, `hero.rs` `buyback` | MOVE | Not yet moved; needs a hook or a command |
| Camp leash and clear numbers, ward duration, XP curve, item tables | still engine constants and tables | DATA | Not yet moved to `.osd` data |
| Determinism core, serialization, wire and dispatch codes | engine | STAY | Hashed or persisted formats |

Current state: the scenario mod code covers kill gold, waves, hero respawn, protection, march, win and the store. Generic primitives (hooks, commands, `pay_bounty` fallback, `winner`, `waypoints.rs`, `store.rs`) stay in the engine. The rows still in the engine are the MOVE rows marked not yet moved.

Follow-ups for the MOVE and DATA rows:

1. Move the doodad kind list from `format.rs` into a data table so a new kind needs only mapexport.
2. Move player start positions from `builtins.rs` into the `start` directive of `.osd` data.
3. Move camp and rune numbers from `camp.rs` into `behaviour`/`economy` data.
4. Document the wave `p3` bit layout in `rust/crates/data/FORMAT.md` and derive its range from the unit table.
5. Add a generator check that `scenarios/moba/tools/gen-reference-map.mjs` emits no `route(...)` lines while `routes.osd` exists.
6. Re-classify `.osd` as Engine if the server-side compile (`osdt_data::generate` in `server/src/map_image.rs`) lands.
7. Add Lua hooks for the MOVE rows above (camp respawn, aggro priority, backdoor/glyph, courier, buyback, shop, route march, win rules, gold/XP grants) in `mlua_host.rs`, `piccolo_host.rs` and `shared.rs`, once their owning files are clean.

## Moddable systems

Inventory for moving the whole game into Lua (maps and mods, hot reload, no Rust rebuild). Classes: **MOVE** = logic becomes Lua through a hook, query or command; **DATA** = values become scenario or asset data; **STAY** = Rust, with the reason it cannot move. Moddable today: yes, partly, or no. Paths are repo-relative.

Verification: verified against `lanmower/oscraft` main `88adede5` (a descendant of `73219a65`) by reading a tarball of that tree, not the local checkout. **CONFIRMED** = cited file and symbol exist and the class matches the code. **CORRECTED** = the row was wrong; the cell says what changed. **MISSING-FILE** = a cited file does not exist on main (a subset of CORRECTED). **ADDED** = a system the table did not list, found by a sweep of every layer.

| System | Where it lives now (file, language) | Moddable today | Lua boundary (hook / query / data) | Class | Verification |
|---|---|---|---|---|---|
| Camp (neutral leash, respawn, runes) | `rust/crates/sim/src/camp.rs` (Rust) | partly | hook `on_died`; data `scenarios/moba/neutral.osd`; constants in camp.rs | MOVE | CONFIRMED (`respawn_camp`, `respawns_free`, `CAMP_LEASH_MILLI` exist; no `on_camp_cleared` hook exists anywhere, the hook is `on_died`) |
| Aggro and target priority | `rust/crates/sim/src/aggro.rs` (Rust) | partly | hook for target priority (none exposed); data `scenarios/moba/hero_rules.txt` holds numbers only | MOVE | CORRECTED: target priority is code in aggro.rs (`backdoor_protected`, `glyph_protects`), not data; class changed DATA to MOVE to match the Rust-only table |
| Courier | `rust/crates/sim/src/courier.rs` (Rust) | no | hook `on_tick`; cmd move; query courier state | MOVE | CONFIRMED |
| Wards | `rust/crates/sim/src/timed_life.rs` (Rust) | no | effect kind `ward` in data; hook `on_enter` for vision | MOVE | CONFIRMED (`ward_left`; `"ward"` in `rust/crates/data/src/format.rs`) |
| Glyph (team duration and cooldown) | `rust/crates/sim/src/rules.rs` (Rust, `glyph_effect` near line 566); `rust/crates/sim/src/aggro.rs` (protection) | no | data `scenarios/moba/behaviours.osd` for durations; protection logic has no hook | MOVE | CORRECTED: protection is `glyph_protects` in aggro.rs, which is rule logic (MOVE); only the duration and cooldown numbers are DATA |
| Shop (stock, buy) | `rust/crates/sim/src/shop.rs` (Rust); `js/client/ui_shop.js` | partly | query item catalog; cmd buy | MOVE | CONFIRMED (`buy_item`; `item_catalog_json` is in `rust/crates/web/src/lib.rs`) |
| Hero stats and respawn | `rust/crates/sim/src/hero.rs` (Rust); `scenarios/moba/hero_rules.txt` | partly | data `hero_rules.txt`; hook `on_died` | DATA | CONFIRMED (`MANA_PER_INT`, `ARMOUR_PER_AGI_DIV`, `HP_REGEN_PER_STR_X100`) |
| XP | `rust/crates/sim/src/xp.rs` (Rust) | no | data `scenarios/moba/economy.osd`; hook `on_died` with killer | DATA | CONFIRMED (`xp_curve`) |
| Gold | `rust/crates/sim/src/combat.rs` `pay_bounty`, `hero.rs` `hero_bounty` (Rust); `mods/moba_match.lua` (Lua, `on_died`); `mods/unit_bounty_example.lua` (Lua) | partly | hook `on_died`; cmd `add_gold`, `add_unit_gold`; query `unit_bounty`, `rival_of`; the Rust payout is off only while `gold_by_mod` is set | MOVE | UPDATED: the payout moved to `mods/moba_match.lua` behind `gold_by_mod`; no loader sets that flag at `369c3ba`, so both payers run |
| Towers and backdoor | `rust/crates/sim/src/aggro.rs`, `hero.rs` (Rust); `mods/structure_buff_example.lua` (Lua) | partly | hook `on_attacked`; data doodad and unit `.osd` | MOVE | CONFIRMED |
| Route and march | `rust/crates/sim/src/waypoints.rs` (`start_route`, `tick_route`, `march_to_enemy_base`), `movement.rs` (Rust); `scenarios/moba/routes.osd` | partly | data `routes.osd` (single source) | DATA | CONFIRMED (the march symbol is `march_to_enemy_base`, not `march`) |
| Win and objectives | no discrete module; `rust/crates/sim/src/modapi.rs` (victory), `mods/waves_example.lua` (defeat), `rust/crates/sim/src/behaviours.rs` (`winner`) | partly | hook `on_tick`; needs cmd end-of-match | MOVE | CONFIRMED |
| Spawns and waves | `mods/moba_waves.lua`, `mods/waves_example.lua` (Lua); `scenarios/moba/waves.osd`; sim `rust/crates/sim/src/waves.rs`; `rust/crates/data/src/validate.rs` (p3 layout, Rust) | yes (Lua drives it) | hook `on_tick`; cmd spawn; data `waves.osd` | MOVE | CORRECTED: `waves.rs` exists on main; the deletion is only in the local uncommitted working tree |
| Strike (attack resolution) | `rust/crates/sim/src/combat.rs`, `projectiles.rs` (Rust) | partly | effect kind `damage`; hook `on_attacked` | STAY | CONFIRMED: per-tick hashed combat math; the Lua surface is the effect kind |
| Movement and pathing | `rust/crates/sim/src/movement.rs`, `pathfinding.rs`, `astar.rs` (Rust) | partly | query position; cmd move; speed in `UnitSpec` | STAY | CONFIRMED: per-tick hashed pathing must stay bit-identical across native and wasm |
| Bot hero AI | `mods/bot_unit_example.lua`, `mods/bot_lib.lua` (Lua) | yes | hook `on_tick` (confirmed at `mods/bot_unit_example.lua:27`); cmds through `game` | MOVE | CONFIRMED (geometry constants to data) |
| Creep behaviour triggers | `rust/crates/sim/src/behaviours.rs` (Rust); `scenarios/moba/behaviours.osd` | partly | data `behaviours.osd` | DATA | CONFIRMED |
| Bot seating | `mods/bot_unit_example.lua` (`BOT_SLOTS`, `game.add_player`, line 19) | yes | hook `on_player_joined` | MOVE | CORRECTED: the file is `mods/bot_unit_example.lua`; `mods/bot_lib.lua` has neither symbol |
| Hero panel and scoreboard | `js/client/hud5.js`, `js/client/hud.js`, `js/client/ui_scoreboard.js` (JS) | no | query `/state`; no UI host call | MOVE | CONFIRMED |
| Shop UI flow | `js/client/ui_shop.js` (JS) | partly | query `item_catalog_json`; cmd buy | MOVE | CONFIRMED |
| Skills bar | `js/client/ui_skills.js` (JS); `ABILITY_SLOTS = 8` in `rust/crates/sim/src/lib.rs` | partly | data ability slots; needs UI host call | MOVE | CORRECTED: `ABILITY_SLOTS` is not in `hud.js`; it is `rust/crates/sim/src/lib.rs:77` |
| Inventory UI | `js/client/ui_inventory.js` (JS) | partly | query inventory; needs UI host call | MOVE | CONFIRMED |
| Lobby and room setup | `js/client/lobby.js`, `js/client/rooms.js` (JS); `rust/crates/server/src/room.rs` (Rust) | partly | hook `on_player_joined`; room config data | MOVE | CONFIRMED |
| Mod UI overlay | `js/client/modui.js` (JS); events polled from `GET /events` (`rust/crates/server/src/lib.rs`) | partly | the bridge itself: mod UI events | STAY | CONFIRMED: it is the host renderer mods call into; keep it as the host ABI |
| Key bindings and hotkeys | `js/client/input.js` (`keydown` at line 204), `js/client/play.js` (`keydown` at line 568) (JS) | no | data key map; needs key-bind UI call | DATA | CORRECTED: `ABILITY_SLOTS` and `ATTACK_KEY` are not in `hud.js`; `ATTACK_KEY` is absent from the repo; the handlers are in moba_input.js and play.js |
| Order input (click, target) | `js/client/play.js` (JS); `rust/crates/server/src/order.rs` (Rust) | partly | cmd order | STAY | CONFIRMED: orders are validated and networked; the wire format is fixed |
| Camera control (bounds, zoom, follow) | `js/client/play.js` (`cameraDistance`, `CAMERA_PLAY_DISTANCE`); `js/client/render3d.js` (orbit) | no | hook for camera updates; needs camera host call | MOVE | CONFIRMED |
| Camera constants | `js/client/play.js`, `render3d.js` | no | data | DATA | CONFIRMED |
| Fog of war rules (sim) | `rust/crates/sim/src/fog.rs` (Rust, `visible`) | partly | query `visible(x, y)` | STAY | CONFIRMED: visibility is hashed sim state |
| Vision and sight values | `rust/crates/sim/src/arrays.rs`, `loader.rs`; `UnitSpec.sight` in `rust/crates/sim/src/modapi.rs` | partly | data unit `sight` | DATA | CONFIRMED |
| Fog render | `js/client/fogview.js` (JS) | no | query fog grid | MOVE | CONFIRMED |
| Effects and FX (spawn, animation rules) | `js/client/fx.js`, `js/client/anim.js` (JS) | no | hook `on_spawned` / `on_attacked` to a client fx command | MOVE | CONFIRMED (`on_spawned` and `on_attacked` are host hooks in `rust/crates/mod-host/src/mlua_host.rs`) |
| Map editor | `js/client/editor.js`, `js/client/editor.html`, `js/client/editor-check.mjs` (JS) | yes (client tool) | none needed beyond data | MOVE | CONFIRMED |
| Map generation | `rust/crates/mapexport/src/main.rs` (Rust); `scenarios/moba/tools/gen-reference-map.mjs`; `rust/crates/server/src/map_image.rs` | partly | data: generated `scenarios/moba/map.osd` | DATA | CORRECTED (MISSING-FILE): `rust/crates/server/src/map_image.rs` is not on main (untracked locally); the other paths exist |
| Doodad placement and kinds | `rust/crates/sim/src/doodads.rs` (Rust, `DOODAD_KIND_COUNT`); kind list in `rust/crates/data/src/format.rs` | no | data: kind table; needs list moved out of Rust | DATA | CONFIRMED |
| Scenario data (`.osd`) | `scenarios/moba/*.osd`, `scenarios/duel-routes/*.osd` (Starlark); `rust/crates/data/src/reader.rs`, `validate.rs` | yes | data | DATA | CONFIRMED |
| Player start placement | `rust/crates/data/src/builtins.rs` (Rust) | no | data: `start` directive in `.osd` | DATA | CONFIRMED |
| Asset manifest | `scenarios/moba/assets/assets.json` (JSON) | yes | data | DATA | CONFIRMED |
| Doodad manifest | `scenarios/moba/assets/doodads.json` (JSON); `DOODAD_KINDS` in `js/client/play.js` | partly | data | DATA | CONFIRMED |
| Effect kinds (enum and dispatch) | `rust/crates/data/src/format.rs` `EFFECT_KINDS` (Rust); `rust/crates/sim/src/modapi.rs` `MOD_EFFECTS` (7 kinds, Rust) | partly: mods get 7 kinds | data: effect kind names; dispatch in sim | STAY | CONFIRMED: binary effect ABI and sim dispatch table; extend `MOD_EFFECTS` in Rust first |
| Ability definitions | `rust/crates/data/src/abilityext.rs`, `rust/crates/sim/src/loader.rs`; `AbilitySpec` in `modapi.rs`; hook `define_ability` in `mlua_host.rs` and `piccolo_host.rs` | partly | hook `define_ability`; data `abilities.osd` | DATA | CONFIRMED |
| Unit and item definitions | `UnitSpec`, `ItemSpec`, `UpgradeSpec` in `rust/crates/sim/src/modapi.rs`; `scenarios/moba/units*.osd`, `items.osd` | yes | data; mod define calls | DATA | CONFIRMED |
| Lockstep and state sync | `rust/crates/server/src/lockstep.rs`, `room.rs` (Rust) | no | none | STAY | CONFIRMED: determinism and frame integrity |
| Wire frame codec | `js/wire/frame.js`, `js/client/wire/frame.js` (JS); `docs/WIRE.md` | no | none | STAY | CONFIRMED: fixed protocol, checked against the server copy in CI |
| Order rate limits and tick budget | `rust/crates/server/src/budget.rs`, `order.rs` (Rust) | partly | data: limits config | DATA | CONFIRMED |
| Relay and signalling | `rust/crates/relay/src/main.rs` (Rust); `rust/crates/server/src/relay.rs` | no | none | STAY | CONFIRMED: transport; no listening ports is policy |
| HTTP routes and static files | `rust/crates/server/src/http.rs`, `static_files.rs` (Rust) | no | none | STAY | CONFIRMED: fixed protocol surface |
| Replay record and playback | `rust/crates/server/src/replay.rs` (Rust); `js/client/replay.js` (JS) | partly | hook for playback UI; the replay file format is STAY. Format `oscraft-replay 2` (`FORMAT_HEADER`) records mod commands with handles resolved to units, so playback needs no mod; `oscraft-replay 1` still parses | MOVE | UPDATED: format version 2 at `369c3ba` |
| Spectator follow | `js/client/replay.js`; state frames from `/state` and wire | partly | query state frames | MOVE | CONFIRMED |
| Save and load bytes | `rust/crates/sim/src/persist.rs` (Rust, `MAGIC`, `STATE_VERSION`) | no | cmd save; cmd load | STAY | CONFIRMED: hash-checked binary image of sim state |
| Save and load flow | client snapshot in `js/client/play.js` (`snapshotKey`, `localStorage`, lines 707-752); no server or Lua save command | partly | hook and cmd on top of persist | MOVE | CORRECTED: a client save path exists (localStorage snapshot); the Lua save and load command does not |
| Campaign and scripted missions | none in repo (only `mods/*.lua` per scenario; `mods/waves_example.lua` as the nearest mission) | no | hooks `on_player_joined`, `on_tick`; data mission `.osd` (new) | MOVE | CONFIRMED (grep for campaign, chapter and mission finds nothing in code) |
| Menus and screens | `js/client/play.html`, `js/client/lobby.js` (JS) | no | hook for screen state; query room list | MOVE | CONFIRMED |
| Scoring and stats | `mods/killer.lua`, `mods/unit_bounty_example.lua` (Lua); `js/client/hud.js` (post-game), `js/client/ui_scoreboard.js`; `post_game_json` in `rust/crates/web/src/lib.rs` | partly | query per-player totals; kv store (none yet) | MOVE | CONFIRMED |
| Achievements | none in repo | no | hooks `on_died`, `on_enter`; data achievement `.osd` (new) | MOVE | CONFIRMED |
| Fixed tick scheduler | `rust/crates/sim/src/tick.rs` (Rust, `SYSTEM_COUNT`); `rust/crates/sim/src/lib.rs` (`TICKS_PER_SECOND = 16`); `rust/crates/server/src/supervise.rs` | yes at the hook (`on_tick`) | hook `on_tick` | STAY | CONFIRMED: 16 ticks per second fixed step is the determinism clock |
| Client frame loop | `js/client/play.js` (`localTickAcc`), `js/client/frame_timer.js` (JS) | no | none | STAY | CONFIRMED: browser frame pacing |
| 3D renderer | `js/client/render3d.js` (JS, WebGL2) | no | data: model map | STAY | CONFIRMED: GPU API boundary |
| TSL renderer | `js/client/render_tsl.js` (JS, WebGPU) | no | none | STAY | CONFIRMED: GPU API boundary |
| Terrain pipeline | `js/client/terrain.js`, `tiles.js`, `texture.js` (JS) | partly | data: map and tile sources | DATA | CONFIRMED |
| Model loading | `js/client/glb.js`, `texture.js` (JS); `scenarios/moba/assets/assets.json` | partly | data: assets manifest | DATA | CONFIRMED |
| Pause, settings and result screens | `js/client/replay.js` (pause only); `post_game_json` in `rust/crates/web/src/lib.rs`; no settings screen anywhere | no | query `post_game_json`; hook for screen state; no UI host call | MOVE | ADDED: missing from the table; no Lua call reaches the result JSON |
| Selection, drag select and control groups | `js/client/ui_selection.js` (JS) | no | query selection; cmd group assign; no UI host call | MOVE | ADDED: client-only selection logic with no host boundary |
| Minimap | `js/client/ui_minimap.js` (JS) | no | query map and vision grid; needs UI host call | MOVE | ADDED |
| UI layout data | `js/client/moba_ui.json`, read by `hud.js`, `ui_scoreboard.js`, `ui_inventory.js`, `ui_shop.js` | partly | data: layout file | DATA | ADDED: layout is data but not in the table |
| Spell casting and resolution | `rust/crates/sim/src/spell.rs` (Rust, `parse_ext`, 45 tables) | partly | hook `define_ability` (exposed in `mlua_host.rs`, `piccolo_host.rs`); data `abilities.osd`; cast rules stay Rust | MOVE | ADDED |
| Status effects (stun, root, silence, slow) | `rust/crates/sim/src/status.rs` (Rust, `stunned`, `rooted`, `silenced`, `slow_multiplier`) | partly | data durations; effects via `MOD_EFFECTS` `stat`; no status query or clear command exposed | MOVE | ADDED: rule logic is in Rust with no Lua query |
| Upgrades | `rust/crates/sim/src/upgrades.rs` (`complete_upgrade`, `apply_upgrade_effect`); `UpgradeSpec` in `modapi.rs` | partly | data: upgrade table; cmd research | DATA | ADDED |
| Teleport | `rust/crates/sim/src/point_move.rs` (`point_move_effect`); `RING_POINTS` numbers | no | data: ring points; cmd teleport via effect | MOVE | ADDED |
| Diplomacy and teams | `rust/crates/sim/src/diplomacy.rs` (`set_relation`, `add_player`, `relation`); `mods/team_rules_example.lua` | partly | cmd set relation; query relation; controller codes stay STAY | MOVE | ADDED |
| Lua host ABI and dispatch | `rust/crates/mod-host/src/mlua_host.rs`, `piccolo_host.rs`, `shared.rs`; `rust/crates/sim/src/modapi.rs` | yes (hooks `on_tick`, `on_died`, `on_attacked`, `on_enter`, `on_spawned`, `on_player_joined`, `define_ability`) | the boundary itself | STAY | ADDED: this is the boundary every MOVE row calls through |
| Mod loading and hot reload | `rust/crates/server/src/mods.rs` (`open`, `stage_image`, `take_commands`) | partly | cmd reload; data mod image | STAY | ADDED: host loader must stay in Rust |
| Delta state sync (client) | `js/client/delta.js` (JS) | no | none | STAY | ADDED: coupled to the wire format |
| ICE and WebRTC server list | `js/client/ice.js` (`iceServersFrom`) (JS) | no | data: ICE server list | STAY | ADDED: transport |
| Audio | none in repo (no audio code under `js/client`) | no | hook `on_spawned` / `on_attacked` to a client sound command; none exists | MOVE | ADDED: absent from the table and from the code |

Counts (75 systems): MOVE 36, DATA 20, STAY 19.

Verification counts: CONFIRMED 53, CORRECTED 8 (of which MISSING-FILE 1), ADDED 14.

Original 61 rows after verification: MOVE 28, DATA 18, STAY 15. ADDED 14 rows: MOVE 8, DATA 2, STAY 4.

Lua-boundary gaps by impact (the Lua host exposes none of these today):
1. Campaign and scripted missions: no mission state, no mission hook or `.osd` mission data.
2. Achievements: no hook or data.
3. Pause, settings and result screens: `post_game_json` is not reachable from Lua; no screen-state hook.
4. UI host call: HUD, skills, inventory, shop, scoreboard and minimap all need one (rows for Hero panel, Skills bar, Inventory UI, Shop UI, Minimap).
5. Camera host call: bounds, zoom and follow have no Lua surface.
6. Save and load command: only the client localStorage snapshot exists; no Lua `save`/`load` command.
7. Fog grid query: fog render and minimap cannot read visibility from Lua.
8. Spell casting and status queries: `define_ability` exists, but cast resolution and status state are not queryable.
9. Teleport and diplomacy commands: `set_relation` and teleport have no Lua command.
10. Audio: no sound command or host call exists.

## Doodad scale

Every doodad model is fitted to a footprint in cells (one cell is one world unit, one 64-unit GridNav cell) from `scenarios/moba/assets/doodad_sizes.json`, written by `scenarios/moba/tools/gen-reference-map.mjs` from `scenarios/moba/tools/doodad-sizes.mjs`. The rule: upright models take their role's target height in hero heights, so their crown or footprint follows their own proportions; flat models (patches, logs, fountain basins) take twice the role target as their footprint; the footprint is clamped to 0.05 to 4 cells. The hero reference is `character-male-a` as `play.js` draws it (its first mesh fitted to 2 x UNIT_HALF), 0.864 cells tall. Role targets: tree 2.0, building 2.5, prop 0.75, nature 0.4, rock 0.35, item 0.25, decoration 0.5, character 1.0 hero heights. Unit-only models (`roshan-*`, `abaddon-*`) are not scenery and have no footprint. The sizes are measured from the first mesh only, the same vertices `glb.js` reads.

Placement (`gen-reference-map.mjs`): every jungle tree cell (a blocked cell) gets a tree; bushes sit on every 40th open field cell; rocks on cliff edges; props at towers, fountains and camps; items beside shops; castle kit (`wall*`, `tower-*`, `brick-wall*`, `stone-wall*`) on the diagonal corners of a ring just outside each base and tower structure zone. Nothing is placed on a lane cell or inside a base or tower footprint (`structureZone`); lane rims stay open for bushes. Doodads never block, so walkability and spawns are unchanged.
