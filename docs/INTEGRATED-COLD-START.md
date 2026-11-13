# Integrated cold start

One user-facing command starts the current oscraft site, validates cross-project contracts, regenerates runtime evidence and serves the integrated pack set:

```bash
npm run serve:integrated
# or
scripts/serve-integrated.sh
```

Default port is `8842`. Override with `INTEGRATED_PORT`, `INTEGRATED_SITE`, `INTEGRATED_PID_FILE` and `INTEGRATED_LOG_FILE`, or `--port=`, `--site=`, `--pid-file=`. Ports `8831`, `8899` and `8916` are refused because they belong to separate servers.

## One-line commands

```bash
npm run serve:integrated                     # cold start, then serve
npm run serve:integrated -- --check          # bounded validation, no server
npm run serve:integrated -- --stop           # stop only the integrated PID
node scripts/check-integrated-cold-start.mjs # fresh-process proof
npm run check:integrated-cold-start          # same proof through npm
```

## URLs

With the default port:

- `http://127.0.0.1:8842/` redirects to the play page.
- `http://127.0.0.1:8842/js/client/play.html` is the default `scenarios/moba-acronyms` pack with 127 acronym-only Dota IDs.
- `http://127.0.0.1:8842/js/client/play.html?package=scenarios/bota` is the 26 fullname BOTA pack.
- `http://127.0.0.1:8842/js/client/play.html?package=scenarios/moba` is the historical source pack.
- Manifests live at `/scenarios/moba-acronyms/manifest.json`, `/scenarios/bota/manifest.json` and `/scenarios/moba/manifest.json`.

There is no dedicated `/health` endpoint; the fresh-process proof uses `/` as the bounded HTTP liveness/health probe.

## What cold start does

1. Runs bounded `cross:autoheal -- --once`. If peers changed manifests, only `scenarios/bota/cross-contract-snapshot.json` is refreshed. Real DRIFT exits non-zero.
2. Runs the bounded cross-runtime watcher once, which regenerates `scenarios/bota/runtime-evidence.json` and `scenarios/bota/runtime-evidence-status.json`.
3. Validates the status file.
4. Stages the current site with `SKIP_PACKS=1` and starts `scripts/dev/serve.mjs` on the chosen port with `--staged-only`.
5. Verifies root, all three play pages, all three manifests, the BOTA backdrop JPEG and one fullname hero GLB over localhost.
6. Prints `INTEGRATED OK` with URL, snapshot hash, identifier regimes, browser/WebGL evidence, extraction summaries and `ok: true`, plus `INTEGRATED PID`, `PORT` and `PIDFILE`.

## What runtime evidence proves

`runtime-evidence.json` records the current snapshot hash, identifier regimes, HTTP statuses, backdrop/hero hashes, 26/26 skinned model hashes, real CDP/WebGL proof (`uSkinned=1`, `jointFloats=448`, 28 models, 26 heroes, 28 joints, 8 clips, walk/run motion), cross-contract counts and informational peer extraction summaries. `runtime-evidence-status.json` records the bounded watcher result, healed STALE paths, read-only peer status and the same extraction summaries.

## Cleanup and separation

`--stop` kills only the PID stored in the integrated PID file, then removes that PID file. It never kills peer servers by port or process name. The cold-start proof uses a fresh ephemeral port, site and PID file, stops only its own PID, and asserts that PID is gone and the PID file is removed.

The legacy oscraft dev server on `8831`, the wscheppity extraction server on `8899`, and the extra extraction server on `8916` are separate persistent services. The integrated command refuses those ports and does not stop, start, or modify them. Peer reads are read-only; all writes are inside oscraft.

## Bounded integrated watch

```bash
npm run watch:integrated -- --once
npm run validate:integrated-watch
```

`npm run watch:integrated` runs a bounded 10-minute loop at 20-second intervals. `--once` performs one fresh-process integrated cold-start proof and exits. Each attempt runs `check:integrated-cold-start` on a fresh ephemeral port/site/PID file. If the only failure is healthy peer STALE (`EXIT=3`, explicit `STALE <path>` rows, or `refresh --check exited 3`), it runs `cross:autoheal -- --once` and `watch:cross-runtime -- --once`, then retries once. A small bounded retry allows peers that rewrite manifests during refresh to settle.

It validates `runtime-evidence.json`, `runtime-evidence-status.json` and `cross-contract-snapshot.json`, verifies HTTP 200 results, identifier regimes, browser/WebGL evidence, extraction summaries and peer status, then writes `scenarios/bota/integrated-watch-status.json` with `generatedAt`, `snapshotHash`, `attempts`, `healedPaths`, cold-start URL/PID/port, HTTP results, browser/WebGL summary, extraction summaries, peer status and `ok`.

`npm run validate:integrated-watch` checks that status against live evidence files, current snapshot, cold-start regimes, extraction summaries, peer status and process cleanup. It fails on stale/invalid/contradictory state or any survivor.

Real DRIFT, HTTP failure, WebGL/CDP failure, identifier mismatch, missing required peer manifest, or any surviving integrated/test process exits non-zero with exact evidence. Cleanup kills only owned PIDs and orphaned heal children created by the watcher; it never touches ports `8831`, `8899` or `8916`.

## Release artifact

`npm run report:oscraft-release` emits `docs/OSCRAFT_RELEASE.json` and `.md` from the full gate/runtime/evidence chain, including this cold-start result and cleanup proof. `npm run validate:oscraft-release` validates it. The bounded watch form is `npm run report:oscraft-release -- --watch --once`. See [CROSS-RUNTIME.md](CROSS-RUNTIME.md).

## Workspace pipeline context

The oscraft release artifact is peer-aware of the wscheppity → 247420 compression/vendor pipeline and the stable BitTTS/BitVoice contract. All reads are read-only and all missing prep artifacts are reported, not hard-failed.

```bash
npm run check:workspace-pipeline
npm run check:workspace-pipeline -- --watch --once
npm run report:oscraft-release
npm run validate:oscraft-release
```

`docs/OSCRAFT_RELEASE.json` has a `workspacePipeline` section with wscheppity extraction state, vendor-prep `UNAVAILABLE|NOT_READY|READY`, 247420 `WAITING|VENDORED|BLOCKED`, 12kbps Opus/Brotli/rerun counts when available, BitTTS contract schema/hash/lock, gmweb release readiness, extra status, and exact PENDING/BLOCKING rows. Its `contentContracts` section records the live identifier inversion and decision gates, and `finalAssets` records hy3d's `generated=24 blocked=30 ready=24` status with 24/24 GLB hashes and the concrete audio blocker; `validate:oscraft-release` recomputes them. `READY.json ok=true` is required before any 247420 vendored content is valid; vendored files without READY are `BLOCKING`. A missing vendor-prep directory is `PENDING`, not a failure. The pipeline checker has a bounded default `--watch` loop of 10 minutes at 20-second intervals and `--watch --once` for one cycle. It starts no servers and writes nothing to peers.
