# oscraft

A deterministic, generic game engine: a pack of data and Lua defines the game, played in the browser. The simulation is Rust (native and wasm), the game is data (`.osd` text compiled to a binary image), and mods are Lua 5.4. The same inputs give the same state hash on every platform.

## What it is

- A fixed-point simulation (`rust/crates/sim`) with combat, pathfinding, attributed units, ability ranks, items, shops, camps and status effects.
- A pack model: each scenario directory holds `.osd` data, Lua mods and a `manifest.json`; the engine has no built-in game types. `scenarios/moba` is one example pack (a 5v5 match of units, items and rules), `scenarios/duel-routes` is a routes test, and `scenarios/bota` is a selectable alternative pack cloned from the default MOBA rules.
- A static browser client in `js/client` (GitHub Pages target): local ES modules load directly, Pages builds wasm, maps and bundles from source; peers join over WebRTC and Nostr.

## Run

The packs are built and checked from `rust/`:

```
cd rust
cargo test --workspace
cargo run -p mapexport -- ../scenarios/moba ../scenarios/moba/moba.osdt
```

The browser game is the static page in `js/client/`. Stage it the way Pages does with `scripts/dev/stage.sh`, then serve it with `node scripts/dev/serve.mjs` and open `http://localhost:8790/js/client/play.html?package=scenarios/moba`. `.github/workflows/pages.yml` builds the wasm package with `wasm-pack`, the map images with `mapexport`, bundles the client JS and Lua, and deploys; nothing it builds is committed. For mods and map editing see [docs/QUICKSTART.md](docs/QUICKSTART.md) and [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Docs

- [docs/QUICKSTART.md](docs/QUICKSTART.md): build, serve, play, mod, edit a map
- [docs/BOTA.md](docs/BOTA.md): one-line local launch, BOTA features, verification
- [docs/GAME.md](docs/GAME.md): how a match plays, with numbers
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): crates, data format, sim, browser host, wasm, client, wire, mod host
- [docs/MODDING.md](docs/MODDING.md), [docs/SCENARIOS.md](docs/SCENARIOS.md), [docs/HOST.md](docs/HOST.md), [docs/WIRE.md](docs/WIRE.md)

## Browser client

The playable build is a static page in `js/client/`, the GitHub Pages target. Local ES modules load directly through an import map; Pages bundles them and builds the wasm package and map images from source. See [docs/HOST.md](docs/HOST.md).

## Crates

All crates live in the `rust/` workspace (`rust/Cargo.toml`). The engine crates hold mechanics only; the game's rules are Lua in the pack.

| Crate | Path | What it is |
| --- | --- | --- |
| `sim` | `rust/crates/sim` | The deterministic simulation: Q16.16 fixed point, struct-of-arrays state, movement, combat mechanics, status timers, hashing and persistence. |
| `data` | `rust/crates/data` | The data image generator: evaluates a scenario's `.osd` files into the binary image (format in `rust/crates/data/FORMAT.md`). |
| `mod-host` | `rust/crates/mod-host` | The mod host: the piccolo Lua VM, the natives packs call, and hook dispatch. |
| `oscraft-lsp` | `rust/crates/mod-lsp` | A language server for mods. |
| `mapexport` | `rust/crates/mapexport` | Writes a scenario's map image. |
| `web` | `rust/crates/web` | The wasm build the browser page loads (`js/client/pkg`). |
| `replay` | `rust/crates/replay` | Recording and playback of a sim's order log. |
| `mpq` | `rust/crates/mpq` | MPQ archive reader. |
| `oscraft-relay` | `rust/crates/relay` | Native test relay (NIP-01 over WebSocket) for local browser tests. |
| `oscraft-gate` | `rust/gate` | The check suite that runs against the sim and the packs. |
| `wasip3-guest` | `rust/crates/wasip3` | Excluded from the workspace: the WASI guest that runs the mod host under wasmtime. |


## Integrated cold start

Run `npm run serve:integrated` for the validated default acronym pack, opt-in fullname BOTA pack and historical source pack. Use `npm run watch:integrated -- --once` for a bounded fresh-process proof. See [docs/INTEGRATED-COLD-START.md](docs/INTEGRATED-COLD-START.md).

## Oscraft release artifact

Generate and validate the operator-facing release report with `npm run report:oscraft-release` and `npm run validate:oscraft-release`. See [docs/CROSS-RUNTIME.md](docs/CROSS-RUNTIME.md) and [docs/INTEGRATED-COLD-START.md](docs/INTEGRATED-COLD-START.md).

## Workspace pipeline context

The oscraft release artifact is peer-aware of the wscheppity → 247420 compression/vendor pipeline and the stable BitTTS/BitVoice contract. All reads are read-only and all missing prep artifacts are reported, not hard-failed.

```bash
npm run check:workspace-pipeline
npm run check:workspace-pipeline -- --watch --once
npm run report:oscraft-release
npm run validate:oscraft-release
```

`docs/OSCRAFT_RELEASE.json` has a `workspacePipeline` section with wscheppity extraction state, vendor-prep `UNAVAILABLE|NOT_READY|READY`, 247420 `WAITING|VENDORED|BLOCKED`, 12kbps Opus/Brotli/rerun counts when available, BitTTS contract schema/hash/lock, gmweb release readiness, extra status, and exact PENDING/BLOCKING rows. `READY.json ok=true` is required before any 247420 vendored content is valid; vendored files without READY are `BLOCKING`. A missing vendor-prep directory is `PENDING`, not a failure. The pipeline checker has a bounded default `--watch` loop of 10 minutes at 20-second intervals and `--watch --once` for one cycle. It starts no servers and writes nothing to peers.
