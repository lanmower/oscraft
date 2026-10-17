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
- **moba-match-end**: the match ends with a winner within a bounded tick count. Witness: `moba match ok winner=<side> ticks=<n>`. Status: not started. Depends on: moba-win-witness.
- **moba-wave-spawn**: a timed creep wave spawns past one cooldown. Witness: `moba wave ok spawned=<n> at_tick=<n>`. Status: witnessed in CI run 39021272353 (moba job, success). It was red on `main` until `scenarios/moba/behaviours.osd` named its tag.
- **moba-base-tag**: a tag contract for the base structures that the win trigger reads. Gap `gap-moba-base-win-tag`. Status: closed. The `win_on_base_loss` behaviour names `tag="building"`, and the moba job compiles it.
- **moba-hud**: hit points, gold, a minimap, and a win or lose screen. Witness: `moba hud ok hp=<n> gold=<n>` and `moba win screen ok winner=<side>`. Status: blocked on the wasm exports for hit points, gold and win state, and on ci-pkg-commit-back.
- **moba-casts**: Q, W and E cast an ability at the cursor, and A-click attacks. Witness: `moba input ok` extended with `casts=<n> attacks=<n>`. Status: blocked on wasm cast and attack exports, and on ci-pkg-commit-back.
- **moba-camera**: arrow-key or edge pan, and zoom. Witness: `moba camera ok`. Status: pan witnessed in CI run 39021272353 (browser probe job, success). Zoom is not built and not witnessed.
- **moba-sync**: two peers over wireweave agree on unit positions and hit points after N ticks. Witness: `moba sync ok ticks=<n> diff=0`. Status: not started.
- **moba-creep-lane**: creeps path along a lane to the enemy base. Witness: `moba creeps ok lane_progress=<n>`. Status: not specified until the sim reports pathing; treat as a candidate.

## B. Build and CI

- **ci-pkg-commit-back**: CI rebuilds `js/client/pkg` on push to main and commits it back as one path-limited commit. The commit carries `[skip ci]` so it does not retrigger. Witness: the pkg diff is committed and the page's `pkg` matches the sources. Status: decided, not built.
- **ci-wasm-opt**: drop `--no-opt` once the wasm-pack pin holds. Status: witnessed. The opt-out is gone from `rust/crates/web/Cargo.toml`, and CI run 39021940544 passed on that head.
- **ci-caps-flake**: the connection-cap probe `0 held open above cap 64` has failed and then passed on rerun. Fix the root cause. Status: fix landed, witness pending. The probe drains until only cap sockets are registered, under a deadline. Five local runs pass; CI runs 39021940544 and 39022142837 pass with it. A flake needs many runs to rule out, so one more batch of CI runs is the witness.
- **ci-docs-skip**: the docs-only path skip has never run on a pull request. Witness: one PR that touches only `docs/` skips the Rust and browser jobs. Status: open; needs a PR, which the current rules do not allow.
- **ci-node-tooling**: see D3.

## C. Runtime and guest

- **wasip3-instantiate**: run the wasip3 component under a native host with a newer wasmtime. Witness: `wasip3 tick ok ticks=10 units=<n>`. Status: open. Wasmtime 27 cannot instantiate a wasip3 component.
- **wasip12-removal**: wasip1 and wasip2 are gone; the only reference was a doc line. Landed in `9ad18d2`. Witness: the sweep returns zero references. Status: landed.
- **wasip3-browser**: see D5.

## D. Network and lobby

- **turn-server**: friends behind strict NAT need a TURN server. Witness: a two-peer run with the relay forced, through `?ice=`. Status: open; no TURN credentials are configured.
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
