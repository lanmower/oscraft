# Quickstart

All commands run from `rust/` unless noted. Rust stable and, for the browser build, `wasm-pack` are required.

## Build

```
cargo build --workspace
```

## Compile a scenario

```
cargo run -p osdt-data -- ../scenarios/moba /tmp/moba.osdt
```

`osdt-data` evaluates every `*.osd` file directly in the directory and writes one catalog image. A bad definition fails with the file and line.

## Serve and play

Native server:

```
cargo run -p server -- --data /tmp/moba.osdt --room ci --addr 127.0.0.1:8080
curl 'http://127.0.0.1:8080/state?map=ci'
```

Browser game (the same path the Pages workflow uses):

```
wasm-pack build crates/web --target web --out-dir site/pkg
cargo run -p mapexport -- crates/web/site/moba.bin ../scenarios/moba
```

Serve the repo root with any static file server and open `js/client/play.html#moba`. Add `?host=1` on one page to host the room; peers join the same hash over wireweave. See [HOST.md](HOST.md) for the query parameters.

## Mod

A mod is one Lua chunk defining `on_tick(state)`; see [MODDING.md](MODDING.md) and the examples in `mods/`. Check one without a match:

```
cargo run -p mod-host --bin mod_check -- ../scenarios/moba ../mods/creep_gold.lua 200
```

Run it on a live server with `--mod ../mods/creep_gold.lua --data /tmp/moba.osdt`. A save of the mod file hot-reloads between ticks; a failed reload keeps the old functions. `oscraft-lsp` gives editors diagnostics for `.osd` and `.lua` files over stdio.

## Edit a map

Edit `scenarios/moba/map.osd` (`grid`, `paint`, `doodad`, `start`, `spawn`), or open `js/client/editor.html`, which paints cells and doodads and shows the `.osd` text to copy back. Then regenerate and look:

```
cargo run -p mapexport -- crates/web/site/moba.bin ../scenarios/moba
```

## Regenerate the reference

From the repo root:

```
node scripts/gen-reference.mjs
```
