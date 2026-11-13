# Cross-runtime continuous evidence

The cross-runtime watcher keeps runtime evidence current while bota, hy3d, wscheppity and extra rewrite their own outputs. All peer reads are read-only. It writes only oscraft-owned evidence/status files.

## Commands

```bash
npm run watch:cross-runtime -- --once
npm run validate:runtime-evidence-status
npm run check:cross-runtime
npm run validate:runtime-evidence
```

`npm run watch:cross-runtime -- --once` runs one bounded evidence cycle and exits. The default `npm run watch:cross-runtime` is bounded to 10 minutes with a 15-second interval; it never idles indefinitely. Every cycle uses the existing `check:cross-runtime` per-run staged site and port, with 30-second bounded waits and child-process cleanup. No long-lived test server is left behind.

## Healable STALE vs real failure

A healthy stale snapshot is `EXIT=3` from `refresh --check`, `DRIFT/STALE cross-project snapshot` with exact `STALE <path>` rows, or `check:cross-runtime` reporting a healthy stale snapshot. These mean peers changed manifests after the last oscraft snapshot. The watcher runs `npm run cross:autoheal -- --once`, retries `check:cross-runtime` once, and records the healed paths.

Real failures exit non-zero with exact evidence and are not healed silently:

- real `DRIFT` hash/path/identifier mismatch after refresh;
- missing required peer manifests;
- HTTP failures or wrong identifier regimes;
- WebGL/CDP/browser failures;
- invalid `runtime-evidence.json` or `runtime-evidence-status.json`.

## Status output

On success the watcher writes `scenarios/bota/runtime-evidence-status.json` with:

- `generatedAt`, `snapshotHash`, `ok`;
- `checks`: `checkCrossRuntime`, `validateRuntimeEvidence`, identifier regimes, browser/WebGL summary, cross-contract counts and required peer manifest count;
- `peerChangesHealed`;
- read-only peer git status evidence;
- optional peer status;
- informational extraction summaries for wscheppity and extra;
- bounded-loop metadata.

`npm run validate:runtime-evidence-status` compares the status against live `runtime-evidence.json`, the current snapshot, required peer files, identifier regimes, browser/WebGL evidence, cross-contract counts, extraction summaries and peer status. It fails on stale, missing or contradictory state.

## Informational extraction evidence

`wscheppity/.local/extraction-manifest.json`, `extra/.local/extraction-manifest.json` and `extra/.local/cdn/manifest-all.json` are included as informational runtime evidence when present. Their `classes/complete/blocked` counts are recorded but are not hard oscraft gates. Previously-present peer extraction files disappearing are still reported as snapshot STALE. A required peer manifest disappearing fails immediately.

## Fresh-process proof

```bash
node scripts/check-integrated-cold-start.mjs
npm run check:integrated-cold-start
```

The fresh-process proof stops only its own integrated PID file, starts `scripts/serve-integrated.sh` on a free ephemeral port and site, fetches `/`, the default/BOTA/historical play pages and manifests, the backdrop JPEG, one fullname hero GLB, and the `/` liveness probe, then validates `runtime-evidence.json` and `runtime-evidence-status.json`. It stops the owned PID and asserts no owned child survived. It does not touch the persistent oscraft server on `8831` or the wscheppity/extra extraction servers. See [INTEGRATED-COLD-START.md](INTEGRATED-COLD-START.md).

## Bounded integrated watch

`npm run watch:integrated -- --once` combines the fresh-process cold-start proof, cross-runtime evidence regeneration, cross-contract snapshot validation and cleanup into one bounded self-verifying cycle. The default loop is bounded to 10 minutes at 20-second intervals. Healthy STALE peer changes are healed with `cross:autoheal -- --once` and `watch:cross-runtime -- --once`; real DRIFT, HTTP, WebGL, identifier or cleanup failures are printed and exit non-zero. The watcher writes `scenarios/bota/integrated-watch-status.json`, and `npm run validate:integrated-watch` validates it against live state. See [INTEGRATED-COLD-START.md](INTEGRATED-COLD-START.md).

## Operator release artifact

```bash
npm run report:oscraft-release
npm run validate:oscraft-release
npm run report:oscraft-release -- --watch --once
```

`report:oscraft-release` runs the full deterministic gate set plus hidden-token and survivor scans and writes `docs/OSCRAFT_RELEASE.json` and `docs/OSCRAFT_RELEASE.md`. It records identifier regimes, `PACK_PARAM`, 26 model hashes, backdrop hash/size, WebGL/CDP evidence, cross-contract snapshot hash and hash-bearing rows, peer extraction summaries, read-only peer git status, cold-start URL/PID/port, cleanup/survivor count, and gate counts. `validate:oscraft-release` revalidates the artifact against live files and checks. `--watch` is bounded to 10 minutes at 20-second intervals; `--watch --once` performs one cycle. Healthy STALE peer changes may be healed only by `cross:autoheal --once`; real DRIFT and runtime failures exit non-zero. All peer reads are read-only.

## Workspace pipeline context

The oscraft release artifact is peer-aware of the wscheppity → 247420 compression/vendor pipeline and the stable BitTTS/BitVoice contract. All reads are read-only and all missing prep artifacts are reported, not hard-failed.

```bash
npm run check:workspace-pipeline
npm run check:workspace-pipeline -- --watch --once
npm run report:oscraft-release
npm run validate:oscraft-release
```

`docs/OSCRAFT_RELEASE.json` has a `workspacePipeline` section with wscheppity extraction state, vendor-prep `UNAVAILABLE|NOT_READY|READY`, 247420 `WAITING|VENDORED|BLOCKED`, 12kbps Opus/Brotli/rerun counts when available, BitTTS contract schema/hash/lock, gmweb release readiness, extra status, and exact PENDING/BLOCKING rows. It also has `contentContracts` with the live BOTA/hy3d integration, demand, progress, coordination and identifier-inversion evidence, and `finalAssets` with hy3d's `generated=24 blocked=30 ready=24` status, 24/24 generated GLB SHA-256 proofs and the pooled-email/audio blockers. See [CROSS-CONTRACTS.md](CROSS-CONTRACTS.md); `validate:oscraft-release` recomputes both statuses. `READY.json ok=true` is required before any 247420 vendored content is valid; vendored files without READY are `BLOCKING`. A missing vendor-prep directory is `PENDING`, not a failure. The pipeline checker has a bounded default `--watch` loop of 10 minutes at 20-second intervals and `--watch --once` for one cycle. It starts no servers and writes nothing to peers.
