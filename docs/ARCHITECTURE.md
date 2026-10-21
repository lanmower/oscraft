# Architecture

One deterministic simulation, compiled twice: native for tests and tools, wasm for the browser page. The same inputs give the same state hash on every platform.

The browser page is the host. There is no server process. `js/client/play.html` loads the wasm sim and a pack, applies the local seat's orders, and sends state frames to the other pages over WebRTC. Nostr relays carry signalling and room announcements. See [HOST.md](HOST.md).

## Layers

- **Sim** (`rust/crates/sim`): engine mechanics only. Q16.16 fixed point, struct-of-arrays state, a fixed tick of 16 per second, and the primitives mods call. It has no match rules.
- **Data** (`rust/crates/data`, package `osdt-data`): compiles a pack's `.osd` files (Starlark) into one binary image. The layout is in `rust/crates/data/FORMAT.md`.
- **Mod host** (`rust/crates/mod-host`): the Lua runtime. It runs a pack's modules on piccolo, a pure Rust Lua VM, and dispatches the hooks.
- **Packs** (`scenarios/<name>/`, and the example mods in `mods/`): data plus Lua rules. `scenarios/moba` and `scenarios/duel-routes` are the two shipped packs. [SCENARIOS.md](SCENARIOS.md) and [MODDING.md](MODDING.md) describe them.
- **Browser host** (`rust/crates/web`, `js/client`): the wasm `Game` the page drives, and the page itself.
- **Wire** (`js/wire`): the frame codec the peers exchange. [WIRE.md](WIRE.md).

## Crates

The workspace is `rust/Cargo.toml`: every directory under `rust/crates/` plus `rust/gate`. `rust/crates/wasip3` is excluded.

| Package | Path | Role |
|---|---|---|
| `sim` | `rust/crates/sim` | The simulation (see below). `examples/bench.rs` is a benchmark. |
| `osdt-data` | `rust/crates/data` | Evaluates `.osd` files into the OSDT image. Binary `osdt-data`: `osdt-data <src_dir> <out_file>`, plus `check` and `roundtrip` on an image. |
| `web` | `rust/crates/web` | wasm-bindgen wrapper of the sim as `Game` for the page. Its `site/` holds the page's wasm package build output (`site/pkg`), the unit models and the asset index. |
| `mapexport` | `rust/crates/mapexport` | `mapexport <pack-dir> <out.bin>`: writes a pack's image. |
| `mod-host` | `rust/crates/mod-host` | The mod host on piccolo. Binary `mod_check` loads one mod into a pack and runs ticks. |
| `oscraft-lsp` | `rust/crates/mod-lsp` | Language server for `.osd` and `.lua` over stdio. |
| `replay` | `rust/crates/replay` | Replay files: the command log and its playback. The header is `oscraft-replay 3`. |
| `oscraft-relay` | `rust/crates/relay` | A small Nostr relay for local tests (NIP-01 `EVENT`, `REQ`, `CLOSE`). No CI job starts it. |
| `mpq` | `rust/crates/mpq` | An MPQ archive reader, used by the gate's round-trip checks. |
| `wasip3-guest` | `rust/crates/wasip3` | A WASI component guest for the mod host (WIT in `wit/`). Outside the workspace; no CI job builds it. |
| `oscraft-gate` | `rust/gate` | The check suite run against the sim and the packs. Run it with `cargo run -p oscraft-gate`. |

## The sim

`tick.rs` steps the systems in a fixed order each tick:

1. rebuild the per-cell unit buckets;
2. the fog refresh, every 8 ticks when fog is on (`VIS_PERIOD`);
3. the flow-field paths;
4. statuses and buffs;
5. healers, then projectiles, then timed-life units, respawn timers and spawn groups, then spells;
6. for each living unit: cooldowns, behaviour triggers, shield and health regeneration, healing, channels, stun, hold or engagement, then walking, steering and integration, patrol, idle notes and the attack.

The sim keeps the engine's mechanics: combat, movement and pathing, status and buff timers, projectiles, shields, healers, inventory, fog, spawn-group timers, and the commands and queries that mods use. The rules that decide a match (kill payouts, waves, respawn decisions, the win, shops and couriers, camps, targeting and aggro policy, protection) are Lua in the pack. The sim's state is hashed with FNV (`hash.rs`) and saved through `persist.rs`, which has its own state version.

Positions are Q16.16 (`fixed.rs`). A pack's data loads through `loader.rs`, which keeps its own copy of the image layout constants and does not use the data crate at run time.

## Data format

A pack's `.osd` files are read in name order and compiled into one OSDT image, with the map. The image is a header, a section table and fixed-size little-endian records. Each section has a four-letter tag (the `TAG_*` constants in `rust/crates/data/src/format.rs`). The `.osd` builtins are in `rust/crates/data/src/builtins.rs`, and [SCENARIOS.md](SCENARIOS.md) lists the ones packs use. The sim's reader rejects an unknown version.

Build-time only: the image is written by `mapexport` and `osdt-data`, and the page loads it as a static file. Nothing compiles `.osd` at run time.

## Mod host and hooks

A pack's modules are Lua files. Each module runs in its own environment with the `game` table and the host's hooks. The hooks the host dispatches are the `HOOK_KEYS` list in `rust/crates/mod-host/src/piccolo_host.rs`: `on_spawned`, `on_attacked`, `on_died`, `on_enter`, `on_leave`, `on_player_joined`, `on_tick`, `on_behaviour`, `run_systems`, `fire_timers`, `on_unit_died`, `on_cast`, `on_cast_request`, `on_proc`, `on_hit`, `on_march`, `on_check_win`, `on_request`, `on_skill_up`, `on_action` and `on_pointer`. Each mod call runs under a step budget (`MAX_STEPS`, 2,500 steps), and a Lua memory cap (256 MiB) applies to all modules together.

A hook that fails is switched off for its module only. A module that fails to load keeps its last good version. The full natives and hook arguments are in [SCRIPTING.md](SCRIPTING.md) and [MODDING.md](MODDING.md).

## Browser host and client

`js/client/play.html` loads ES modules through an import map, with no bundler. The main pieces:

- `play.js`: the sim loop, the host loop, the orders and the room join.
- `render3d.js`, `terrain.js`, `fx.js`, `anim.js`, `glb.js`, `texture.js`: the WebGL2 view.
- `moba_input.js`, `moba_select.js`, `hud.js`, `widgets.js`, `ui_minimap.js`, `modui.js`: input, selection, the HUD and the pack's panels.
- `rooms.js`, `ice.js`, `delta.js`, `swap_protocol.js`, `pack_load.js`, `pack_sync.js`: rooms, ICE, state frames, data swaps and pack loading.
- `editor.js` and `pack_editor.js`: the map editor and the pack editor.
- `debug_api.js`: `window.oscraft`, the console API.

The page renders what the pack asks for and decides nothing about the game. Its UI calls (`ui_event`, `camera`, `message`, `banner`, `panel`, `sound`) come from the pack and are drained with `drain_ui`. Lobby seats, controls and panels are pack code (`scenarios/moba/panels.lua`, `scenarios/moba/controls.lua`).

## Wire

Peers talk over wireweave data channels (WebRTC). The frame codec in `js/wire/frame.js` has 23 frame types (ORDER, STATE, DELTA, HELLO, ROSTER, START, SWAP and the rest). `js/client/wire/frame.js` is a copy of it. [WIRE.md](WIRE.md) lists every frame.

## Build, test and deploy

- `.github/workflows/ci.yml` has two jobs. `rust` runs `cargo test --workspace` in `rust/`. `wire` runs `node test.js` in `js/wire`.
- `.github/workflows/pages.yml` builds the wasm package and the pack images from `main`, lays out the static site, deploys it to GitHub Pages, and checks the live page against the build. Nothing it builds is committed.
- `.github/workflows/release.yml` packages the release archive for each target and attaches it to the GitHub release.
- `scripts/dev/stage.sh` and `scripts/dev/serve.mjs` stage and serve the same layout locally ([DEVELOPMENT.md](DEVELOPMENT.md)).

## Determinism

The sim is integer-only: no floats, no maps, no wall clock, and no heap allocation inside `Sim::tick`. Every buffer is sized in `Sim::new` or `load`. The fixed tick and the hash make a match replayable and comparable across native and wasm builds.
