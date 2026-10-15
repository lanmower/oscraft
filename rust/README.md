# rust/ mod-runtime spike

A contract proof for the Lua 5.4 mod runtime, not a port of the simulation. It embeds Lua through `mlua` (vendored, no system Lua) and proves:

1. A host loads a mod chunk into a fresh environment and calls `on_tick(state)` once per tick.
2. Hot reload: the mtime is polled between ticks. A reload replaces only the functions. The host-owned `state` table survives. A reload that fails to compile keeps the old functions and reports `<file>:<line>`.
3. Host functions `spawn_unit(kind, x, y)` and `get_unit_count()` run against a Rust `Vec` state, with Q16.16 `i32` positions.
4. Determinism: 2000 ticks run twice from identical state, and both FNV-1a hashes of the final state must match.
5. A Lua line hook counts the lines executed in one `on_tick`.
6. A runtime error in `on_tick` is reported with file and line, and the tick loop continues.

## Run

```
cargo run -p oscraft-spike
```

From `rust/`. CI runs the same command on ubuntu and windows (`rust` job in `.github/workflows/ci.yml`). The determinism hash is printed in the log.

The reload demo writes its mod versions to the system temp directory. It does not touch `spike/mods/unit_mod.lua`, which is the file the determinism and hook runs load.
