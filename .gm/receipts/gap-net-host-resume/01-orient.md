# 01-orient: gap-net-host-resume (session gm-orch-2-9-r3)

acceptance: from row (acceptance_criteria): a browser room runs N ticks and writes a snapshot; after a host reload or re-create from storage a reconnecting wireweave peer receives a state frame whose state_hash equals the pre-restart hash, and the tick resumes from snapshot.tick. Missing or corrupt snapshot yields an explicit refusal line.

symbols (codeinsight, unqualified-name edges):
- sim::save_state  rust/crates/sim/src/persist.rs:270-443 (impl Sim, pub fn save_state(&self) -> Vec<u8>; read-only). callers: mod-host lib.rs:1044, server replay.rs:136,250, gate sim_checks.rs:556.
- sim::load_state  rust/crates/sim/src/persist.rs:447-650 callers: mod-host lib.rs:1052, server replay.rs:142,235, gate sim_checks.rs:558.
- gate save_load_state rust/gate/src/sim_checks.rs:551-572 is the existing round-trip witness (hash after load, then 200-tick divergence check).
- web Game rust/crates/web/src/lib.rs:10-119 (struct Game { sim }, #[wasm_bindgen] impl). Exposes tick, ticks, state_hash, but NOT save_state or load_state. This is the gap.
- page startHost rust/js... js/client/play.js:395-411: setInterval tick, computes hash (Number(localGame.state_hash() & 0xffffffffn)>>>0), encodes frame, session.broadcast. No snapshot write.
- js/client/pkg/web.js is the generated wasm-bindgen glue (Game class at line 3, state_hash at 42, tick at 46). Generated; not rebuilt locally (wasm32 target unavailable).

verified facts:
- sim persist already provides save_state/load_state; the wasm Game is the only missing exposure.
- rows' prior blocker gap-net-host-resume-blocker-gm-orch-2-9-r2 is the environment limit (no local wasm32 target); witness moves to the CI wasm build.
- git status at start has untracked assets/, rust/crates/web/site/*.bin, rust/data/moba/, rust/data/rpg/: other lanes; do not stage.
