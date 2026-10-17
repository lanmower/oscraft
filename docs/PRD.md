# Remaining plans

Status is as of main `62af78b0`. Each row names its acceptance witness: the line that must print in CI before the row closes. "Landed" means the commit is on main; "witnessed" means its CI line was read.

## Decisions needed from the user

- **D1. Release the gm session lease.** Another live process (`mcp-codemode-276876`) owns the PRD store and holds 40 pending rows that are unrelated to the MOBA. Until it is released, rows can't be filed there, so this document is the PRD of record.
- **D2. Reset the local checkout.** `/workspace/oscraft` is stale at `7b4dbcf` with about 6,800 uncommitted entries. It must not be reset without your approval, because it may hold work.
- **D3. Node in CI.** Playwright drives headless Chromium and the wire tests run under Node. The game and relay no longer need Node. Replacing the driver is a choice, not a requirement.
- **D4. Superseded CI runs.** Concurrency now cancels the older run on the same ref. Superseded shas get no verdict; the tip does.
- **D5. wasip3 in the browser.** wasip3 cannot run in a browser without a transpile step, which breaks the buildless target. The browser keeps running `rust/crates/web`. Revisit only if you want the browser to run the guest.

## A. MOBA (browser game)

- **moba-kill-bounty**: a general bounty on a destroyed unit is paid to the opposing side's resource pool. Witness: `kill bounty ok gold_before=<n> gold_after=<n>`. Status: witnessed. Gate line `kill bounty ok gold_before=0 gold_after=25` in CI run 39021272353 (duel-routes job, success).
- **moba-win-witness**: confirm `win trigger ok winner=<side>` in CI. Status: witnessed. Gate line `win trigger ok winner=0` in CI run 39021272353 (duel-routes job, success).
- **moba-match-end**: the match ends with a winner within a bounded tick count. Witness: `moba match ok winner=<side> ticks=<n>`. Status: witnessed by the gate (`mechanic checks moba-match-end`): destroying side 1's only base declares side 0 the winner in 1 tick, bound 200. The witness is wired into the `duel-routes` CI job's gate step.
- **moba-wave-spawn**: a timed creep wave spawns past one cooldown. Witness: `moba wave ok spawned=<n> at_tick=<n>`. Status: witnessed in CI run 39021272353 (moba job, success). It was red on `main` until `scenarios/moba/behaviours.osd` named its tag.
- **moba-base-tag**: a tag contract for the base structures that the win trigger reads. Gap `gap-moba-base-win-tag`. Status: closed. The `win_on_base_loss` behaviour names `tag="building"`, and the moba job compiles it.
- **moba-hud**: hit points, gold, a minimap, and a win or lose screen. Witness: `moba hud ok hp=<n> gold=<n>` and `moba win screen ok winner=<side>`. Status: witnessed in CI run 39024499055 (browser probe job, success). The minimap draws each unit as a dot and is witnessed as `moba minimap ok dots=<n>` (dots equal the unit count, CI run 39026016132). The live Pages page still serves the committed wasm until the `ci-pkg-commit-back` decision is made.
- **moba-casts**: Q, W and E cast an ability at the cursor, and A-click attacks. Witness: `moba input ok` extended with `casts=<n> attacks=<n>`. Status: witnessed in CI run 39024499055 (`moba input ok orders=1 casts=1 attacks=1`). Casts and attacks are local-preview only; the relay path does not carry them yet.
- **moba-camera**: arrow-key or edge pan, and zoom. Witness: `moba camera ok`, `moba zoom ok`. Status: witnessed. Pan in CI run 39021272353, zoom in CI run 39022142837 (browser probe job, success).
- **moba-sync**: two peers over wireweave agree on unit positions and hit points after N ticks. Witness: `moba sync ok ticks=<n> diff=0`. Status: witnessed at the sim level (`moba-sync`): two sims from the moba image, given the same orders, agree on every unit's position and liveness after 300 ticks. The wireweave transport is not part of this witness; the browser peer job covers one order over the relay. Wireweave agreement is witnessed by the browser relay job (`browser relay agreement ok ticks=<n>`): over a TURN relay, both peers hold the same unit positions for every host tick they both logged. CI run 39028364097 logged `ticks=299` with no mismatch.
- **moba-creep-lane**: creeps path along a lane to the enemy base. Witness: `moba creeps ok lane_progress=<n>`. Status: witnessed by the gate (`moba-creeps`): a side 0 creep ordered at the rival base closes the distance in 200 ticks. `lane_progress` is in raw fixed-point units (about 31 tiles). The lane shape is witnessed by `moba lane ok ticks=<n> connector=true`: a side 0 creep ordered to a point in the north lane walks the connector column and stands on an open cell at every tick. The straight line to that point crosses blocked terrain.

## B. Build and CI

- **ci-pkg-commit-back**: CI rebuilds `js/client/pkg` on push to main and commits it back as one path-limited commit. The commit carries `[skip ci]` so it does not retrigger. Decision: `pages.yml` builds the page wasm and maps from source and deploys that build, and nothing is committed back to main, so the live page cannot go stale. Witness: pages run 39028364087 built and uploaded the site from source on this branch (success). The deploy step runs on main only, so its witness waits for the next push to main. Status: built and witnessed up to the deploy.
- **ci-wasm-opt**: drop `--no-opt` once the wasm-pack pin holds. Status: witnessed. The opt-out is gone from `rust/crates/web/Cargo.toml`, and CI run 39021940544 passed on that head.
- **ci-caps-flake**: the connection-cap probe `0 held open above cap 64` has failed and then passed on rerun. Status: fix landed and witnessed. The probe drains until only cap sockets are registered, under a deadline. Six local runs pass; CI runs 39021940544 and 39022142837 pass with it. Whether the original race is gone is inferred from the code, not reproduced, so a red run on a later push would reopen it.
- **ci-docs-skip**: the docs-only path skip has never run on a pull request. Witness: one PR that touches only `docs/` skips the Rust and browser jobs. Status: witnessed in run 39022366371 (PR #2, docs-only). `decide` reported success, and every Rust and browser job was skipped.
- **ci-node-tooling**: see D3.

## C. Runtime and guest

- **wasip3-instantiate**: run the wasip3 component under a native host with a newer wasmtime. Witness: `wasip3 tick ok ticks=10 units=<n>`. Status: witnessed. The native host runs on wasmtime 50 with component-model-async, feeds the guest the map image and three commands, and prints `wasip3 tick ok ticks=10 units=30` locally and in CI (wasip3 job, run 39025271922). The host builds on stable; the guest builds on nightly.
- **wasip12-removal**: wasip1 and wasip2 are gone; the only reference was a doc line. Landed in `9ad18d2`. Witness: the sweep returns zero references. Status: landed.
- **wasip3-browser**: see D5.

## D. Network and lobby

- **turn-server**: friends behind strict NAT need a TURN server. Witness: `browser turn relay ok local=relay` from the browser-turn-relay job, with both pages forced to relay candidates through `?ice=` and `ice_policy=relay`. Status: a CI-local coturn with throwaway credentials is wired in that job. Witnessed in CI run 39028364097: `browser turn relay ok local=relay pairs=1/1 unit=0 moved=true`. The runner's default coturn service held port 3478 in earlier runs, so the install step now stops it.
- **room-expiry**: a closed page drops off the room list after 45 s. Witness: `room expiry ok dropped=<name>`. Status: witnessed.
- **lobby-rooms**: the room list is fed from Nostr announcements. Witness: `lobby room select ok rooms=2 joined=beta`. Status: witnessed.
- **friends-guide**: the guide has no CLI or Node text. Witness: `friends guide ok command_lines=0 cli_flags=0`. Status: witnessed.
- **ice-and-delta**: ICE from `?ice=` and delta frames. Status: landed; delta frames are produced by the browser host.

## E. Engine and modding

- **structure-damage**: a beacon loses hit points while an enemy is in range. Witness: `structure damage ok hp_before=300 hp_after=270`. Status: witnessed in `39c5593`.
- **reload-latency**: a mod edit reaches the server in about 60 ms; ceiling 404 ms. Witness: `reload latency ms=<n> ceiling_ms=404 ok`. Status: witnessed.
- **frame-bytes**: a full frame is 582 bytes and deltas average 168; ceilings 2328 and 672. Witness: `browser frame bytes full=582 delta_avg=168 ... ok`. Status: witnessed.
- **mod-walkthrough**: a data file defines a new unit kind, a mod spawns it, and `/state` shows the name. Witness: `mod walkthrough ok kind=modprobe_unit`. Status: witnessed.
- **hot-error-relay**: LSP hover, Lua diagnostics, client error relay, and rollback on rejected reloads. Witnesses: `lsp hover ok`, `lua-diagnostic:`, `check: one ev=client_error line`, `reload-check rollback ok`. Status: witnessed.

## F. Process

- **gm-gripes**: the gm gripes were filed in the `/workspace/gm` store as `gm-gripe-*` items. Status: filed, for the gm project.
- **pool-rows**: the 40 pending rows in the other session's PRD store are unrelated to this work and are left alone. See D1.

## Decisions taken

The user delegated these to the agent. Each is the default a reviewer can overturn.

- **D1.** Keep `docs/PRD.md` as the record. The other session's store is not touched.
- **D2.** `/workspace/oscraft` is not reset. Its uncommitted work is left for the user to inspect.
- **D3.** Node stays in CI for the wire tests and Playwright. Replacing it is not worth the churn while the rows above are open.
- **D4.** Keep the per-ref cancel. The tip gets the verdict.
- **D5.** The browser keeps running `rust/crates/web`. wasip3 stays native.

Status: `d27b59f` restores the workspace build. `9c7b9d3` fixes the moba scenario. CI run 39021272353 on that head passed all 22 jobs. The `ci` run on `main` at `60d881b` failed in the `build` job on the missing `UnitExtRaw.bounty` field, and every probe behind it was skipped. The branch has no CI trigger, because `ci.yml` runs on `main` and on pull requests into `main`. The next verdict needs `main` to carry `d27b59f`.

Status: PR #5 merged as `46a9eed`, witnessed by CI run 39024499055 (all 22 jobs green). The two-peer probe and the moba probe both pass there. Still open: `ci-pkg-commit-back` (needs the user's decision), `turn-server` (needs TURN credentials or a CI-local TURN server with a relay-only page flag), and relay-carried casts and attacks, which need a wire-protocol change.

Status: the open rows above are built and witnessed in CI run 39028364097 (23 of 23 jobs success on commit 3e0fc5e). Relay-carried casts and attacks use frame types 6 (`CAST`) and 7 (`ATTACK`) in `js/wire/frame.js`, with the host applying them through the same ownership check as orders. The browser-turn-relay job witnessed `browser relay commands ok attacks=1 casts=1` over the relay.
