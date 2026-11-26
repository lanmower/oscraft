# Bevy handoff: the sim crate, the state hash and the replay API

What S1 (sim inside Bevy) and S2 (headless server) call. Every entry below is read from the source at the commit that adds this file; the file and line are given so a change shows up here.

## Build a sim

- `sim::load(bytes: &[u8], max_units: usize) -> Result<Sim, LoadError>`, rust/crates/sim/src/loader.rs:1312. Loads a compiled map image (the osdt image the pack build writes).
- `sim::load_file(path: &Path, max_units: usize) -> Result<Sim, LoadError>`, loader.rs:1336.
- `Sim::new(max_units: usize) -> Sim`, rust/crates/sim/src/arrays.rs:260. An empty sim.
- The mod host builds the image with `mod_host::build_image(dir)` (rust/crates/mod-host/src/lib.rs) and then `ModHost::new(&image)`.

## Advance one tick

- `Sim::tick(&mut self)`, rust/crates/sim/src/tick.rs:71: runs the tick with pathing and vision on.
- `Sim::tick_with_cadence(&mut self, run_paths: bool, run_vision: bool)`, tick.rs:75. S5 uses this to run pathing and fog at a lower cadence; the hash must not depend on the cadence choice unless the replay records it (see the replay API).
- `Sim::ticks(&self) -> u32`, arrays.rs:533: the current tick.

## Orders

- `Sim::order(&mut self, i: usize, x: i32, y: i32)`, arrays.rs:939: move order for unit i.
- `Sim::order_all(&mut self, player: u8, x: i32, y: i32)`, arrays.rs:1010.
- `Sim::order_attack(&mut self, i: usize, target: usize) -> Result<(), Refusal>`, rust/crates/sim/src/combat.rs:316.
- Orders enter the sim through the replay `Command` enum (rust/crates/replay/src/lib.rs:273) when a match is recorded; the server applies only the orders the sender's unit owns (docs/HOST.md, host-authoritative).

## State hash

- `Sim::state_hash(&self) -> u64`, rust/crates/sim/src/hash.rs:88. FNV-1a over the category digest, the capacity, the abilities and fog flags, the path pool, the unit count and head, the tick, the night flag, the day cycle, the RNG state, the log, the spell tables and the inventory, stats and XP arrays. The state hash is the per-tick desync check (D4).
- Native and wasm produce the same hash (docs/ARCHITECTURE.md:3). S1's witness compares the Bevy build's hash with the native hash for each tick of a replay.

## Replays

- `replay::Recording::new(image: &str, max_units: usize, sim: &Sim)`, rust/crates/replay/src/lib.rs:1000. Starts a recording at the sim's current state.
- `Recording::tick(&mut self, sim: &mut Sim, paths: bool, vision: bool)`, lib.rs:1130, and `Recording::checkpoint(&mut self, sim: &Sim)`, lib.rs:1138, which stores the tick and its hash.
- `Recording::render(&mut self, sim: &Sim) -> String`, lib.rs:1192: the oscraft-replay 3 text (`FORMAT_HEADER`, lib.rs:255).
- `replay::parse(text: &str) -> Result<Recording, ReplayError>`, lib.rs:1230: reads version 1, 2 and 3 files.
- `Replayer::new(rec: Recording, sim: &Sim)`, lib.rs:1340, and `Replayer::run_to_end(&mut self, sim: &mut Sim) -> Result<u64, ReplayError>`, lib.rs:1448: plays a recording and returns the end hash; `step` (lib.rs:1383) checks each checkpoint hash.
- `replay::play(source: &Sim, fresh: &mut Sim, checkpoints: &[(u32, u64)]) -> Result<u32, ReplayError>`, lib.rs:63, and `Playback::new` (lib.rs:109) take the per-tick (tick, hash) trace for a playback UI.

## What is not ready

- The duel-routes replay generator is rust/crates/replay/src/duel.rs (`duel::record` and `duel::verify`, bin `duel_routes_replay <pack-dir> <out-dir>`). It records scenarios/duel-routes for 1280 ticks (both armies ordered to the far start at tick 0) with a state hash after every tick, replays the text with Replayer and compares each tick, and writes `duel-routes.replay` and `duel-routes.hashes` (one `tick hash` line per tick, 0 to 1280). It is Phase A code; its committed outputs come from the gated Phase B window, which is pending.
- `parity_hash` (rust/crates/mod-host/src/bin/parity_hash.rs) prints only the final hash after N ticks; it does not write per-tick hashes.
- The sim's `tick` uses the host's advance order: the mod host runs its hooks before the sim tick (rust/crates/mod-host/src/piccolo_host.rs, `advance`). A replay that includes mod-driven changes must be recorded with the same hook order, or its hashes will not match.
