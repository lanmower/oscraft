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

### Rust-only game rules: final classes

Classes: MOVE = rule logic moves to a Lua hook or scenario data. DATA = number or table content moves to `.osd` data. STAY = stays in Rust, reason given. Source: symbol inventory of `rust/crates/sim/src` at origin `855d02c`. Signatures were classified without reading every body where marked.

| Rule | Where | Class | Reason |
|---|---|---|---|
| Camp leash, clear and first-spawn numbers | `camp.rs` `CAMP_LEASH_MILLI`, `CAMP_CLEAR_MILLI`, `CAMP_FIRST_SPAWN_S` | DATA | Plain numbers; move to `behaviour` data |
| Camp respawn timing | `camp.rs` `respawns_free`, `respawn_camp` | MOVE | Rule logic; needs a Lua hook (`on_camp_cleared`) |
| Aggro target priority | `aggro.rs` target selection | MOVE | Rule logic; needs a Lua target-priority hook |
| Backdoor and glyph protection | `aggro.rs` `backdoor_protected`, `glyph_protects`, `glyph_cooldown_left` | MOVE | Rule logic; no Lua hook exists in `mods/` |
| Backdoor period | `aggro.rs` `BACKDOOR_PERIOD` | DATA | Number |
| Structure scan radius | `combat.rs` `STRUCTURE_SCAN_CELLS` | DATA | Number |
| Damage response codes | `combat.rs` `RESPONSE_REDUCED`, `RESPONSE_IMMUNE` | STAY | Persisted and wire codes |
| Courier state codes | `courier.rs` `IDLE`, `OUTBOUND`, `RETURNING` | STAY | Stored in state snapshots (`persist.rs` `STATE_VERSION`) |
| Courier delivery rules | `courier.rs` (signatures only; body not yet read) | MOVE | Rule logic; needs a Lua hook |
| Ward duration | `wards.rs` `ward_left` | DATA | Number |
| Hero stat coefficients | `hero.rs` `MANA_PER_INT`, `ARMOUR_PER_AGI_DIV`, `HP_REGEN_PER_STR_X100` | DATA | Formula coefficients |
| Buyback | `hero.rs` `buyback` | MOVE | Rule logic; needs a Lua hook |
| Shop buy rules | `shop.rs` `buy_item` | MOVE | Rule logic; needs a Lua hook |
| Shop doodad ids | `shop.rs` `SHOP_DOODAD_HOME`, `SHOP_DOODAD_SIDE` | DATA | Doodad ids belong in `.osd` data |
| Item stats and recipes | `inventory.rs` `ITEM_STATS`, `ITEM_SMODS`, `BEST_ONLY` | DATA | Tables of content |
| XP curve | `xp.rs` `xp_curve` | DATA | Table of content |
| Gold and XP grants on kill | `combat.rs` / `xp.rs` (body not yet read) | MOVE | Rule logic; needs a Lua hook |
| Teleport ring | `teleport.rs` `RING_POINTS`, `RING_STEPS` | DATA | Numbers |
| Spell, status and upgrade effect codes | `spell.rs` `K_DOT`, `K_STATUS`; `upgrades.rs` `EFFECT_*_CODE`; `status.rs` `STATUS_NAMES` | STAY | Dispatch keys shared with the Lua host ABI and persisted |
| Refusal kinds | `rules.rs` `Refusal`, `REFUSAL_KINDS`, `ALL_REFUSALS` | STAY | Wire contract with clients and replay |
| Route march rules | `route.rs` march logic | MOVE | Rule logic; route points are DATA |
| Route record layout | `route.rs` `MAX_ROUTE_POINTS`, `ROUT_RECORD` | STAY | Fixed binary record layout |
| Win and objective rules | `triggers.rs` `winner` | MOVE | Rule logic; needs a Lua hook |
| Doodad kind list | `doodads.rs` `DOODAD_KIND_COUNT`; builtin list in `format.rs` | MOVE | Follow-up 1 below |
| Determinism core | `fixed.rs`, `pathfinding.rs` cost constants, `lib.rs` `TICKS_PER_SECOND`, `DAY_TICKS`, `tick.rs` `System`, `SYSTEM_COUNT`, `COST_BUCKET_US` | STAY | Hashed fixed-point sim and scheduler order |
| Serialization | `loader.rs` `MAGIC`, `VERSION`, `Trigger`, `LoadError`; `persist.rs` `MAGIC`, `STATE_VERSION`, `StateError` | STAY | OSDT and state formats |
| Diplomacy controller codes | `diplomacy.rs` `CONTROLLER_*` | STAY | Wire codes |
| Lua host ABI vocabulary | `modapi.rs` `MOD_EFFECTS`, `STAT_NAMES` | STAY | The host enumerates the effect and stat names it dispatches on |

Counts: 28 rules: MOVE 10, DATA 9, STAY 9. No MOVE or DATA row is migrated yet. Blocked rows are in files another lane has uncommitted edits in (`camp.rs`, `aggro.rs`, `combat.rs`, `rules.rs`, `movement.rs`, `shop.rs`, `route.rs`, `lib.rs`, `loader.rs`).

Follow-ups for the MOVE and DATA rows:

1. Move the doodad kind list from `format.rs` into a data table so a new kind needs only mapexport.
2. Move player start positions from `builtins.rs` into the `start` directive of `.osd` data.
3. Move camp and rune numbers from `camp.rs` into `behaviour`/`economy` data.
4. Document the wave `p3` bit layout in `rust/crates/data/FORMAT.md` and derive its range from the unit table.
5. Add a generator check that `scripts/gen-dota-map.mjs` emits no `route(...)` lines while `routes.osd` exists.
6. Re-classify `.osd` as Engine if the server-side compile (`osdt_data::generate` in `server/src/map_image.rs`) lands.
7. Add Lua hooks for the MOVE rows above (camp respawn, aggro priority, backdoor/glyph, courier, buyback, shop, route march, win rules, gold/XP grants) in `mlua_host.rs`, `piccolo_host.rs` and `shared.rs`, once their owning files are clean.
