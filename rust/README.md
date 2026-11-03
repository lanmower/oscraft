# rust/ workspace

The Rust side of oscraft: the deterministic simulation, the data compiler, the mod host and the tools. All of it is one Cargo workspace (`rust/Cargo.toml`).

## Run the tests

From `rust/`:

```
cargo test --workspace
```

This is the `rust` job in `.github/workflows/ci.yml`. Run one crate's tests with `cargo test -p <package> <filter>`, using a package name from the table below.

## Crates

Package names are from each `rust/crates/*/Cargo.toml`.

| Package | Path | What it is |
| --- | --- | --- |
| `sim` | `crates/sim` | The deterministic simulation: Q16.16 fixed point, struct-of-arrays state, the fixed tick, and the engine mechanics and primitives mods call. |
| `osdt-data` | `crates/data` | Compiles a pack's `.osd` files into the OSDT image. Layout in `crates/data/FORMAT.md`. |
| `mod-host` | `crates/mod-host` | The mod host: the piccolo Lua VM, the natives packs call, and the hook dispatch. Binary `mod_check`. |
| `web` | `crates/web` | The wasm build the browser page loads (`js/client/pkg`). |
| `mapexport` | `crates/mapexport` | Writes a pack's map image: `mapexport <pack-dir> <out.bin>`. |
| `oscraft-lsp` | `crates/mod-lsp` | Language server for `.osd` and `.lua` files, over stdio. |
| `replay` | `crates/replay` | Replay files: the recorded command log and its playback. |
| `mpq` | `crates/mpq` | MPQ archive reader. |
| `oscraft-relay` | `crates/relay` | A small Nostr relay for local tests. |
| `wasip3-guest` | `crates/wasip3` | WASI component guest for the mod host. Excluded from the workspace. |

`rust/gate` (package `oscraft-gate`) is a workspace member too: the check suite that runs against the sim and the packs.

## Two engine rules

- The crates hold mechanics only. The sim, the data format and the host provide primitives; they carry no game rules and no genre tables.
- The game is Lua and data in `scenarios/` (the packs, with their `.osd` data and Lua modules) and `mods/` (example mods). Change a rule there, not in Rust.
