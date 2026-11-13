# Cross-project contracts and automatic stale healing

This repository consumes peer manifests read-only. BOTA, hy3d, wscheppity and extra are allowed to rewrite their own outputs; oscraft never writes into those trees.

## Entry points

```bash
npm run check:cross-contracts
npm run cross:autoheal -- --once
npm run check:cross-autoheal
npm run refresh:cross-contracts -- --check
```

`npm run cross:autoheal -- --once` is the one-command heal path. The default `npm run cross:autoheal` is a bounded 10-minute loop at 10-second intervals; `--once` performs one attempt and exits. The only file autoheal/refresh may write is `scenarios/bota/cross-contract-snapshot.json`.

## Required inputs

The original eight peer manifests are hard requirements. Absence prints `UNAVAILABLE` with exact paths and exits non-zero:

- `bota/export/rule-demand.json`
- `bota/export/conformance.json`
- `bota/export/export-status.json`
- `bota/export/spawn-compatibility.json`
- `hy3d/output/content-for-bota.json`
- `hy3d/output/final-asset-inventory.json`
- `hy3d/output/bota-ready.json`
- `hy3d/output/asset-delivery-report.json`

The checker also uses oscraft-owned contract files: `scenarios/bota/IDENTIFIERS.md`, `bota_ui.json`, `bota-skinned.json`, `scenarios/moba-acronyms/identifier-map.tsv` and `js/client/pack_load.js`.

## Optional inputs

Optional files are annotated as `optionalPresent`/`optionalMissing`. Missing optional files do not fail:

- `bota/export/content-request-queue.json`
- `bota/export/spawn-extraction-gaps.json`
- `bota/export/integration-contract.json`
- `bota/export/progress-dashboard.json`
- `bota/export/coordination-status.json`
- `hy3d/output/content-demand-v2.json`
- `hy3d/output/spawn-cdn-probes.json`
- `wscheppity/.local/extraction-manifest.json`
- `extra/.local/extraction-manifest.json`
- `extra/.local/cdn/manifest-all.json`

The two extra files are also reported as `extraPresent`/`extraMissing`. When present, their extraction summary `complete == total` / `blocked 0` facts are informational evidence only, never a hard oscraft gate. If `/workspace/extra` is not a git repository, `git status` returns `128`; the autoheal validator records this as an annotated peer status and still requires the text to be unchanged.

## Content-contract status

`scripts/content-contract-status.mjs` reads `bota/export/integration-contract.json`, `progress-dashboard.json`, `coordination-status.json`, `conformance.json` and `hy3d/output/content-demand-v2.json` read-only. It records the identifier inversion: 26 canonical upstream acronym keys map through `acronym_to_slot` and `IDENTIFIERS.md` to 26 oscraft BOTA fullname IDs, while the default Dota pack keeps 127 acronym IDs. It also records eight `do_not_promote` integration rules, the demand row/blocked/generated counts, coordination unresolved rows, and a SHA-256 over the five source files.

`scripts/check-cross-project-contracts.mjs` requires that status to be valid and requires the snapshot's `contentContracts.sourceSha256` to match live files. Missing decision-ready values stay gated: the checker does not promote GUESS, UNVERIFIED, AMBIGUOUS, owner-decision or intentionally-open fields into game data. A changed source file is healthy snapshot STALE after the optional-manifest check; a wrong schema, inversion count, gate count or non-zero `blocked`/`generated` demand is a real failure. `docs/OSCRAFT_RELEASE.json` carries `contentContracts`, and `scripts/validate-oscraft-release.mjs` recomputes the live status before accepting the artifact.

## hy3d final asset status

`scripts/final-asset-status.mjs` reads six hy3d reports read-only: `bota-final-asset-status.json`, `.tsv`, `BOTA-FINAL-ASSET-STATUS.md`, `bota-final-asset-evidence.json`, `bota-remaining-reconciliation.txt` and `bota-remaining-slugs.txt`. The user-facing Markdown path is uppercase; the lowercase `bota-final-asset-status.md` name is not present. The reader verifies all 24 generated GLBs by declared bytes and SHA-256 and records exact `generated=24`, `blocked=30`, `ready=24`, `remaining=30`, plus 56 status rows, one owner-decision row and one intentionally-open row.

`scripts/check-final-asset-status.mjs` adds this proof to `check-all.mjs` with negative count, audio-blocker and GLB-hash controls. `scripts/check-cross-project-contracts.mjs` also requires the snapshot's `finalAssets.sourceSha256` to match live reports. `docs/OSCRAFT_RELEASE.json/.md` carries `finalAssets` and `scripts/validate-oscraft-release.mjs` recomputes it. No generated audio, fabricated bytes or peer writes are involved.

The preserved blockers are: pooled-email studio `witch_doctor/rig` service-side HTTP 500 with no worksId, recovery false and quota preserved at 30; no studio or relay audio action, absent local BitVoice service on `127.0.0.1:8765` and missing `scripts/execute-bittts-audio-bridge.mjs`; and 12 kbps Opus/Brotli requirements preserved with no uncompressed or fabricated audio published. The 30 blocked rows are 26 `hero-voice` and four `clip` rows, each with a null path, bytes and SHA-256.

## Why `EXIT=3` is healthy

Peers are actively rewriting manifests. A snapshot that no longer matches a peer file's SHA-256, mtime or `generatedAt` is classified `DRIFT/STALE` and `check:cross-contracts` exits `3`. That is a healthy signal that the local snapshot needs refresh, not necessarily a content failure. `npm run cross:autoheal -- --once` parses exact `STALE <path>` rows, refreshes only the oscraft snapshot, re-runs the checker, and reports before/after snapshot hashes plus healed stale paths.

Missing required manifests exit `2` as `UNAVAILABLE`. Other checker failures and unresolved hash/path/identifier mismatch exit `1` or `3` after a refresh and are not silently healed.

## Real DRIFT

Real DRIFT means a hash-bearing `available`/`ready` row cannot resolve to a real file with matching SHA-256 after trying:

- declared path fields,
- `content/...` links into `content-for-bota.json`,
- `legacyDestinationPath` and `IDENTIFIERS.md` fullname migration candidates.

It also includes identifier-regime mismatches: BOTA non-fullname active IDs, default Dota non-acronym active IDs, a wrong `PACK_PARAM`, or broken `bota-skinned.json` hashes. Autoheal prints exact evidence and exits non-zero for these cases.

## Snapshot determinism

`npm run refresh:cross-contracts` records peer manifest contents, mtimes, `generatedAt`, optional/extra presence and identifier mode. `generatedAt` is excluded from the comparable hash, so repeated refreshes against unchanged peers are byte-identical. `npm run refresh:cross-contracts -- --check` fails on stale snapshots.

## Read-only peers

All peer reads are read-only. `npm run check:cross-autoheal` captures `git -C <peer> status --short` before and after autoheal for bota, hy3d, wscheppity and extra, and fails if any status text changes. It also verifies `check:cross-contracts`, `refresh --check`, deterministic snapshot bytes and absence of unresolved DRIFT.

## Runtime evidence

The repeatable bounded watcher, healable STALE rules and `runtime-evidence-status.json` are documented in [CROSS-RUNTIME.md](CROSS-RUNTIME.md).

## Runtime evidence details

The runtime evidence path connects the current peer snapshot to the three served pack regimes and to the existing real CDP/WebGL browser witness:

```bash
npm run cross:autoheal -- --once
npm run check:cross-runtime
npm run validate:runtime-evidence
node scripts/check-bota-browser.mjs --all --from-cross-runtime
```

`npm run cross:autoheal -- --once` should be run when `check:cross-contracts` or `refresh --check` reports `EXIT=3`. That exit means a peer manifest changed and the oscraft snapshot is stale; it is a healthy coordination signal, not a content failure. Autoheal writes only `scenarios/bota/cross-contract-snapshot.json`.

`npm run check:cross-runtime` uses a per-run staged site and port, a 30-second bounded wait, and cleans up its child process. It proves:

- default `/js/client/play.html` resolves to `scenarios/moba-acronyms` with 127 acronym-only active IDs and `PACK_PARAM=scenarios/moba-acronyms`;
- `?package=scenarios/bota` resolves to 26 fullname BOTA IDs, `menuBackdrop` HTTP 200 with SHA-256 `431b1f861938c26edb7ce226e75d1c66a433c701d70a59463e064c402a077549`, one fullname hero GLB HTTP 200 with `glTF/v2`, and all 26 `bota-skinned.json` hash rows matching local GLBs;
- `?package=scenarios/moba` remains the historical source pack;
- the cross-contract snapshot is not stale and optional peer inputs are annotated;
- `check-bota-browser.mjs --all --from-cross-runtime` supplies the compact real CDP/WebGL summary: `uSkinned=1`, `jointFloats=448`, `models=28`, `heroes=26`, `joints=28`, `clips=8`, `motion=walk/run`, `backdropHits>=1`, `other404s=0`.

The command writes `scenarios/bota/runtime-evidence.json` with `generatedAt`, snapshot hashes, identifier regimes, pack URLs/HTTP statuses, backdrop/hero hashes, browser/WebGL/animation evidence, cross-contract counts, optional peer status and `ok: true`. `npm run validate:runtime-evidence` revalidates that file against live files and fails on stale/missing/invalid evidence.

All peer reads remain read-only. Runtime evidence writes only oscraft-owned files.
