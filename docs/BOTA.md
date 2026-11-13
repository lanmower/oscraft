# BOTA local launch

Cross-project identifiers, required/optional peer manifests, bounded autoheal and DRIFT semantics are documented in [CROSS-CONTRACTS.md](CROSS-CONTRACTS.md).

Continuous runtime evidence, bounded watch semantics and status validation are documented in [CROSS-RUNTIME.md](CROSS-RUNTIME.md). The one-command integrated cold start and bounded integrated watch are documented in [INTEGRATED-COLD-START.md](INTEGRATED-COLD-START.md) and [CROSS-RUNTIME.md](CROSS-RUNTIME.md).

BOTA is the opt-in alternative pack at `scenarios/bota`. The default pack is now `scenarios/moba-acronyms`; the historical source pack `scenarios/moba` is unchanged.

## One-line launch

From the repository root:

```bash
scripts/dev/serve-bota.sh
```

Optional port/site overrides:

```bash
BOTA_PORT=8791 scripts/dev/serve-bota.sh
BOTA_PORT=8791 BOTA_SITE=/tmp/oscraft-bota-site scripts/dev/serve-bota.sh
```

The command stages `moba`, `duel-routes`, and `bota`, serves the static site, and prints the exact URL and process/port. Open the printed URL:

```text
http://127.0.0.1:<port>/js/client/play.html?package=scenarios/bota
```

`?package=scenarios/bota` selects the BOTA pack. A URL without that parameter loads the acronym-only derived default pack `scenarios/moba-acronyms`.

## What is in the BOTA pack

- 26 active fullname hero IDs: `neckbeard`, `chad`, `wojak`, `bloomer`, `tradwife`, `boomer`, `coomer`, `gondola`, `spurdo`, `karen`, `howard`, `sanic`, `lankdad`, `finger_dan`, `willie`, `noscope`, `dino`, `ecce`, `bot_bernard`, `shoop`, `npc`, `forever`, `cole`, `grey`, `ghillie`, `gothgirl`.
- Each hero has a real 28-joint skinned GLB with `JOINTS_0`/`WEIGHTS_0` and eight retargeted clips from `assets/anim/library.json`.
- `bota-skinned.json` records source static-mesh provenance and the skinned model hashes.
- `menuBackdrop: "textures.menu_backdrop"` in `bota_ui.json` gives BOTA the 2752x1536 supplied menu backdrop; `moba_ui.json` names the same key in both default packs (`moba` and the derived `moba-acronyms`), which carry a 1920x1072 copy (436,951 bytes). `check-bota-pack.mjs` asserts the JPEG header, the 800 KiB budget and that the derived copy matches the source.
- The two neutral centaur models are genuinely skinned; legacy `centaur_skinned` hero wiring is removed.
- Slot `003` Gigachad is retired/Reserve and excluded. Slots `028-032` are Reserve and excluded.

## Verification

From the repository root:

```bash
node scenarios/bota/tools/gen-hero-acronyms.mjs --check
node scenarios/moba/tools/gen-moba-acronym-pack.mjs --check
node scenarios/bota/tools/build-bota-pack.mjs --check
node scenarios/bota/tools/check-bota-pack.mjs
node scenarios/bota/tools/check-bota-pack.mjs
npm run check:cross-contracts
node scripts/check-cross-project-contracts.mjs --watch-once
npm run refresh:cross-contracts
npm run refresh:cross-contracts -- --check
npm run check:identifier-runtime
node scripts/check-bota-launch.mjs
node scripts/check-bota-browser.mjs --all
npm run check:cross-runtime
npm run validate:runtime-evidence
node scripts/check-all.mjs
```

`check-bota-browser.mjs --all` launches a real local CDP/WebGL2 browser, proves the backdrop fetch, all 26 hero animations, the two neutral centaur animations, and `uSkinned=1` through the production renderer path. It is deliberately not part of `check-all.mjs` because it launches Chromium and takes about 13–17 seconds.

`npm run refresh:cross-contracts` regenerates `scenarios/bota/cross-contract-snapshot.json` from the current peer manifest contents, mtimes and generatedAt stamps; its `--check` mode verifies the snapshot is not stale. The checksum used for determinism excludes `generatedAt`, so repeated refreshes are byte-identical while peer changes still produce DRIFT/STALE. `npm run check:identifier-runtime` is a fast HTTP-only check of the default acronym pack, opt-in fullname BOTA pack and historical source pack; it is deterministic enough for `check-all.mjs`.

`npm run check:cross-contracts` is an explicit read-only cross-project gate: it reads only the four bota export manifests and four hy3d output manifests listed in the checker, verifies the identifier regimes, all 26 BOTA skinned-model hashes, and every hash-bearing available/ready content row. It is deliberately not in `check-all.mjs` because it depends on external worktrees. `node scripts/check-cross-project-contracts.mjs --watch-once` reports whether those manifests are already ready; `node scripts/dev/wait-cross-contracts.mjs` polls for at most 10 minutes at 10-second intervals.

`check-all.mjs` remains the fast deterministic gate. `scripts/check-bota-launch.mjs` is also fast and deterministic; it starts the launch wrapper, fetches the key URLs, verifies the backdrop bytes/hash and one hero GLB, then cleans up.

## Release artifact

The one-command oscraft release report is `npm run report:oscraft-release`; validate it with `npm run validate:oscraft-release`. See [CROSS-RUNTIME.md](CROSS-RUNTIME.md).

## Workspace pipeline context

The oscraft release artifact is peer-aware of the wscheppity → 247420 compression/vendor pipeline and the stable BitTTS/BitVoice contract. It also carries the live BOTA/hy3d content-contract status, identifier inversion and final asset status (`generated=24`, `blocked=30`, `ready=24`) from [CROSS-CONTRACTS.md](CROSS-CONTRACTS.md). All reads are read-only and all missing prep artifacts are reported, not hard-failed.

```bash
npm run check:workspace-pipeline
npm run check:workspace-pipeline -- --watch --once
npm run report:oscraft-release
npm run validate:oscraft-release
```

`docs/OSCRAFT_RELEASE.json` has a `workspacePipeline` section with wscheppity extraction state, vendor-prep `UNAVAILABLE|NOT_READY|READY`, 247420 `WAITING|VENDORED|BLOCKED`, 12kbps Opus/Brotli/rerun counts when available, BitTTS contract schema/hash/lock, gmweb release readiness, extra status, and exact PENDING/BLOCKING rows. `READY.json ok=true` is required before any 247420 vendored content is valid; vendored files without READY are `BLOCKING`. A missing vendor-prep directory is `PENDING`, not a failure. The pipeline checker has a bounded default `--watch` loop of 10 minutes at 20-second intervals and `--watch --once` for one cycle. It starts no servers and writes nothing to peers.
