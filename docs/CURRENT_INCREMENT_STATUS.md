# Current oscraft increment status (2026-11-16)

Scope: fresh start in `/workspace/oscraft`; no commit or push. All pre-existing dirty files and the three untracked core dumps are preserved.

## 2026-11-16 update

- The "?" in menus is fixed for the default pack: hero acronyms resolve by unit ID (panels.lua), and the acronym pack's `init.lua` loads again (generator renames hero module files and manifest entries). Witnesses are in docs/PENDING_VALIDATION.md (2026-11-16).
- Open: the pick grid lists `lycan_wolf_form` as a hero and shows "?" for it; it is a transformation form, so the roster needs a form flag, not an acronym.
- The `scenarios/bota/cross-contract-snapshot.json` drift below is still peer-owned and not committed here.

## Done and verified locally

- Identified the unfinished increment from the dirty tree: `panels.lua` acronym resolution for both active identifier regimes, plus a hardened `scripts/check-acronyms.mjs` gate.
- Upstream: `git fetch --prune origin` exit 0; `HEAD` and `origin/main` both `6fe10658`, divergence `0 0`. No pull needed; dirty files rule out a fast-forward cleanup.
- Refreshed BOTA generated output with `node scenarios/bota/tools/build-bota-pack.mjs --apply` (exit 0), resolving the only full-gate failure (`scenarios/bota/panels.lua`).
- Confirmed `scenarios/moba-acronyms/moba.osdt` matches a fresh `mapexport` of that derived pack (SHA-256 `0c113345ce639de60c9e991e5dcafa4126e1dfaa92753f3a2287f6f69a4f65d5`); the earlier dirty 945711-byte file was the historical source image, not the derived image.
- `node scripts/check-all.mjs`: exit 0, `RESULT gates=80 ok=80 failed=0 drift=0 skipped=6 strict=false`.
- Focused checks all exit 0: `build-bota-pack.mjs --check`, `check-bota-pack.mjs`, `check-acronyms.mjs`, `gen-moba-acronym-pack.mjs --check`, `check-moba-image.mjs`.
- `npm run check:workspace-pipeline`: exit 0, `state=PENDING ok=true vendorPrep=UNAVAILABLE vendor247420=WAITING`, no blocking rows.

## Blocked / held

- `npm run check:cross-runtime` exit 1. Its bounded autoheal refreshed `scenarios/bota/cross-contract-snapshot.json`, but `refresh:cross-contracts` still exits 3: peer `/workspace/hy3d/output/content-demand-v2.json` pins hero-kit SHA-256 `f7633541...` while `/workspace/bota/export/hero-kits.json` is `69a266b1...`, and demand still names retired Karen image `gallery/011-karen.jpg`. `npm run check:cross-autoheal` exit 1 says `real DRIFT is present alongside STALE; refusing to auto-heal silently`. This is peer-owned authority drift; do not rewrite either side from oscraft.
- `npm run validate:oscraft-release` exit 1: `snapshotHash is stale: artifact=652f8bbf... live=4e6156c7...`. Rebuilding the release report is held until the peer contract snapshot is authoritative, because the report validator requires `crossContracts.driftStale === 0` and a live snapshot hash.
- Six check-all gates are skipped for documented external inputs/binaries (`editor-check.mjs`, `gen-turn-rates.mjs`, `gen-ability-keys.mjs`, `gen-ability-text-688g.mjs`, `gen-neutral-688g.mjs`, `gen-heroes.mjs`). These are not local failures.

## Next actions

1. Owner/peer re-cut `/workspace/hy3d/output/content-demand-v2.json` to the 24-hero BOTA roster and current hero-kits hash; or explicitly authorize a new contract snapshot that treats those rows as retired.
2. Re-run `npm run check:cross-runtime`; if exit 0, run `npm run report:oscraft-release` then `npm run validate:oscraft-release`.
3. Re-run `node scripts/check-all.mjs` only after any further source/generated edits, and keep this ledger updated with the exact exit codes.
