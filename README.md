# oscraft

A deterministic simulation in the style of the reference game, being ported to Rust. The Go implementation has been removed from `main`; the Rust workspace under `rust/` is the only code on the branch.

The goal is a headless simulation that gives the same state hash on every platform for the same inputs, with game data read from a binary file generated from Starlark sources, and mods written in Lua 5.4 that run against the host's state.

## Browser game

The playable build is a static page in `js/client/`, the GitHub Pages target. It loads committed ES modules through an import map and committed wasm, with no bundler or install step. See [docs/HOST.md](docs/HOST.md).

## Crates

All crates live in the `rust/` workspace (`rust/Cargo.toml`).

| Crate | Path | Status |
| --- | --- | --- |
| `sim` | `rust/crates/sim` | In progress. Q16.16 fixed-point, struct-of-arrays state, loader for the OSDT data image. Modules cover combat, movement, pathfinding, production, harvesting, buffs, fog, projectiles and shields. |
| `mpq` | `rust/crates/mpq` | In progress. MPQ archive reader: header and table parsing, classic hash and block tables, HET/BET lookup, and the sector codecs (PKWARE, Huffman, inflate, bzip2, LZMA, sparse). |
| `data` | `rust/crates/data` | In progress. Generator for the OSDT data image. Evaluates the `.osd` files in a directory and writes the binary layout described in `rust/crates/data/FORMAT.md` (format version 2). |
| `server` | `rust/crates/server` | In progress. HTTP and room/order handling that calls the sim API. |
| `spike` | `rust/crates/spike` | Contract proof for the Lua mod runtime, described below. Not yet verified by a CI run on this commit. |

The port is in progress. The simulation, archive reader, data generator and server are not yet at parity with the feature set they replace, and their modules are not documented beyond the code itself.

## Lua mod spike

`rust/crates/spike` embeds Lua 5.4 through `mlua` (vendored, no system Lua) and checks six things:

1. A host loads a mod chunk into a fresh environment and calls `on_tick(state)` once per tick.
2. Hot reload: the file's mtime is polled between ticks. A reload replaces only the functions; the host-owned `state` table survives. A reload that fails to compile keeps the old functions and reports `<file>:<line>`.
3. Host functions `spawn_unit(kind, x, y)` and `get_unit_count()` run against a Rust `Vec` state with Q16.16 `i32` positions.
4. Determinism: 2000 ticks run twice from identical state, and the FNV-1a hashes of the final state must match.
5. A Lua line hook counts the lines executed in one `on_tick`.
6. A runtime error in `on_tick` is reported with file and line, and the tick loop continues.

### Run

From `rust/`:

```
cargo run -p oscraft-spike
```

The reload demo writes its mod versions to the system temp directory. The determinism and hook runs load `rust/crates/spike/mods/unit_mod.lua`.

### Build

From `rust/`:

```
cargo build --workspace
```

CI runs both commands on ubuntu-latest, windows-latest and macos-latest (`.github/workflows/ci.yml`).

## License and provenance

The project's own code is licensed under the MIT licence in [LICENSE](LICENSE). Third-party
components, their pinned versions and licences are listed in [NOTICE.md](NOTICE.md).

No game assets, data or maps are distributed by this repository, and user-supplied maps are
never bundled.

This text is not legal advice. A human legal review is required before any release.
