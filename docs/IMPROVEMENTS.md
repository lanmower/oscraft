# Improvement plan

One ranked list. `docs/ARCHITECTURE.md` points here; `.gm/architecture-plan.json` is a historical receipt of an earlier pass.

Produced by a ten-angle read-only survey (Workflow `wf_5af67763-75c`, 53 findings from nine completed units; the browser-ux unit was interrupted after its evidence was gathered, so its angle is covered by the local reads below), plus local execution of every Node check, every generator `--check`, a temp mirror of the drifted outputs, the git/CI state and the Pages workflow. Each item carries its survey evidence; the ones marked landed are additionally witnessed by an executed check. Unmarked items are unverified claims, not confirmed defects.

Status: **landed** (in this run, with a witness below), **verified** (survived the vote), **planned**, **classified, needs decision**, **reported to gm** (the gm session owns the fix).

## The previous queue

| id | decision | status |
| --- | --- | --- |
| generator-check-gates | CI gates were deliberately retired in `9a17155e`; a local runner replaces them. | landed as `scripts/check-all.mjs` |
| client-seam-extraction | No broad rewrite of `play.js`; split only behind a named test. | deferred, see client-js group |
| single-site-layout | Pages and local staging share `scripts/layout-site.sh`; served-hash comparison is still manual. | partly landed, remainder planned |
| browser-witness-rescope | Browser witnesses need a fresh dev-origin row and a bounded playwriter run with its own context. | deferred |
| local-receipt-hygiene | Receipts are ignored, volatile gm state is untracked, and five stray ignore patterns were deleted. | landed |

## Landed in this run

| change | witness |
| --- | --- |
| `scripts/check-all.mjs`: one local validation entry point | `RESULT gates=39 ok=39 failed=0 drift=3 skipped=5 strict=false`; `--strict` exits 1 |
| `scripts/check-doc-claims.mjs`: the retired-CI and stale-API claims cannot come back | `doc claims ok 17/17` |
| `scripts/check-staged-secrets.mjs`: pure matcher, self-test on every run, git errors fail the gate | `secret patterns ok 5/5`, `selftest=0` |
| `scripts/check-hero-models.mjs`: 127 heroes all have a model, with a negative fixture | `hero models ok: 127 heroes, all with a model`, `hero models negative fixture caught` |
| Three drifted generated outputs classified instead of regenerated | temp mirror diff_lines 38 / 96 / 15; `manifest.json` loads `turn_rates.lua` and `ability_fx.lua` |
| Twelve docs corrected for retired CI, gate, server and hook claims | single-term `gm.grep` sweeps now show only historical mentions |
| `docs/PACKAGING.md` rewritten against `scripts/package/build.sh` | `rust/crates/server` and `rust/gate` absent; workspace members are `crates/*` |
| Receipt and gm-state hygiene | `git rm --cached` staged four deletions; `sources/` and `.gm/drain-*.txt` ignored |
| Two gm frictions reported and fixed by the gm session | `gm.verbs` batched-body throw, `gm.grep` alternation; SHAs recorded in DESIGN.md |
| Camera drags keep the hero selected | native probe: `right_drag selection=None`, `middle_drag selection=None`; empty left drag box now re-sends the current selection; `camera-mouse-check.mjs` asserts a camera drag forwards no pointer kinds |
| `moba_probe.rs` mode count derived from `modes.lua` SUPPORTED | `assert_eq!(modes.len(), 19)` was stale at 23 and blocked every probe |

## All 53 findings

### Documentation and instructions

| id | risk | finding | status |
| --- | --- | --- | --- |
| `docs-hygiene-retired-ci-gate-docs` | medium | Twelve tracked docs still describe the CI gate suite that commit 9a17155e retired | landed |
| `docs-hygiene-build-deploy-provenance` | medium | README, HOST, ARCHITECTURE and QUICKSTART disagree on wasm, maps, bundling, the dev URL and a live-build check that does not exist | landed |
| `docs-hygiene-stale-hook-and-behaviour-notes` | low | Four finished-work claims and the hand-copied HOOK_KEYS list contradict the current code | landed |
| `docs-hygiene-naming-spec-retired` | low | docs/NAMING.md is an unapplied rename spec that declares its own deletion | planned |
| `docs-hygiene-agents-budget-duplicates` | low | AGENTS.md is at 83% of its 30 KB cap and carries four troubleshooting receipts already in DESIGN.md | landed |
| `docs-hygiene-gitignore-tracked-state` | medium | The applied .gitignore receipt hygiene does not cover the paths that keep git status dirty, and ignore rules cannot clean tracked runtime files | landed |

### Pages build and deploy

| id | risk | finding | status |
| --- | --- | --- | --- |
| `pages-scout-postdeploy-compare-gate` | medium | Pages deploys with no post-deploy comparison, and the docs claim one exists | planned, deploy path |
| `pages-scout-cache-key-omits-vendor` | medium | Cached wasm/maps can come from an older commit while version.json stamps HEAD | planned, deploy path |
| `pages-scout-post-bundle-artifacts-unchecked` | low | Bundling rewrites _site after its only file check, and chunk/Lua outputs are never asserted | planned, deploy path |
| `pages-scout-observer-preload-blind-spot` | low | Live probe skips 24 of 63 modulepreload URLs, exactly the vendor files layout prunes | planned, deploy path |
| `pages-scout-version-stamp-lacks-content-hash` | medium | version.json is a claim string, not a content hash; a mixed-generation deploy passes the probe | planned, deploy path |
| `pages-scout-release-duplicates-site-layout` | medium | Release archive re-stages the site with a third, unrelated build identity | planned, deploy path |

### gm MCP tooling

| id | risk | finding | status |
| --- | --- | --- | --- |
| `gm-tooling-read-file-binary` | low | read_file serves binary files as text, with an inflated totalBytes and a bogus 9258-line pagination hint | reported to gm |
| `gm-tooling-query-log-kind-typo` | low | A mistyped query_log kind is silently accepted as an empty result and even advances the paging cursor | reported to gm |
| `gm-tooling-grep-budget-signal` | low | A scan-budget-truncated grep answers ok:true with a "no more" footer and only prose in hint | reported to gm |
| `gm-tooling-search-lock-and-cache` | medium | One mutex covers the whole scan and the 4-root cache evicts by path order, so fan-out blocks and rebuilds indexes | reported to gm |
| `gm-tooling-list-dir-pagination` | low | list_dir has no filter or paging, so growing .gm state directories dump every entry on each call | reported to gm |
| `gm-tooling-tracked-runtime-state` | low | Three tracked .gm runtime receipts are rewritten every dispatch, so more .gitignore rules cannot clean the tree | landed |

### First draw and asset pipeline

| id | risk | finding | status |
| --- | --- | --- | --- |
| `firstdraw-stale-modulepreloads` | medium | The deployed bundled page still preloads 39 unbundled local modules and loads rooms.js loose | planned, needs a named test |
| `firstdraw-probe-median-worldbuilt-request-unit` | low | The first-draw probe reports the slower sample as the median, calls distinct paths requests, and measures worldBuilt rather than a frame | planned, needs a named test |
| `firstdraw-boot-critical-path` | medium | loadSim starts 99 Lua fetches and the 944 KB image alongside the 601 KB wasm, and the Pages Lua bundle is dead output | planned, needs a named test |
| `firstdraw-cold-warm-cache-probe` | low | The first-draw probe is cold-only and uses different headers and MIME types from the pack-cache probe, so cache reuse is never measured | planned, needs a named test |
| `firstdraw-shipped-asset-prune-check` | medium | layout-site.sh ships the whole 113 MB scenarios tree while a measured first draw transfers 3 MB, with no check of which loose files are actually requested | planned, needs a named test |

### Generators and committed data

| id | risk | finding | status |
| --- | --- | --- | --- |
| `data-generators-offline-gate` | medium | Replace the generic generator-check idea with one manifest-driven offline gate | landed |
| `data-generators-current-drift` | high | Regenerate three shipped outputs that already fail their own checks | classified, needs decision |
| `data-generators-external-input-contract` | medium | Pin external hy3d inputs instead of relying on a machine path or local checkout | planned |
| `data-generators-temp-copy-no-check` | medium | Compare no-check generators through a temp copy rather than adding 25 ad hoc flags | planned |
| `data-generators-osdt-exclusion` | low | Exclude the committed stale .osdt snapshot from source-drift checks | planned |
| `data-generators-hero-model-completeness` | high | Assert nonempty hero models in the same gate that checks hero bodies | landed |

### Rust crates and sim

| id | risk | finding | status |
| --- | --- | --- | --- |
| `rust-sim-ci-test-job-retired` | medium | Restore a Rust test workflow and correct the stale ci.yml / rust-gate documentation | needs decision |
| `rust-sim-item-generation-global` | medium | Make item_generation_total per-Sim instead of a process-wide atomic | planned |
| `rust-sim-replay-format-untested` | low | Add replay format-3 parse/render round-trip tests to the only format code with no tests | planned |
| `rust-sim-state-version-contract` | medium | Pin STATE_VERSION with a full-plane save/load contract test and an in-tree version log | planned |
| `rust-sim-wasip3-command-wit-parity` | low | Replace the hand-synced wasip3 COMMANDS and WIT contract with a checked parity script | planned |
| `rust-sim-relay-tie-order` | low | Give relay REQ responses a total order when created_at ties | planned |

### Client JS

| id | risk | finding | status |
| --- | --- | --- | --- |
| `client-js-frame-chain-guard` | high | rAF chain dies on one unguarded Game call | planned, needs a named test |
| `client-js-safe-storage-seam` | medium | Only 2 of 9 localStorage sites are guarded | planned, needs a named test |
| `client-js-hud-hot-state` | medium | hmrReload rebuilds the HUD and drops selection and courier state | planned, needs a named test |
| `client-js-input-route-seam` | medium | sendInput routing is pure branching mixed with I/O and seven globals | landed as `lobby_link.js` with `lobby-link-check.mjs` (12 cases) |
| `client-js-key-names-seam` | low | Two key normalizers disagree on pack Numpad bindings | planned, needs a named test |
| `client-js-codec-copy-gate` | medium | Deployed wire codec is a 421-line copy with no drift gate | planned, needs a named test |

### Validation checks

| id | risk | finding | status |
| --- | --- | --- | --- |
| `scripts-check-runner-wiring-and-external-skip` | medium | The new check runner is untracked, unwired, and permanently skips editor-check | landed |
| `scripts-check-doc-command-drift` | low | Stale gate commands need a documentation path and command check | landed |
| `scripts-check-secret-self-test` | high | The secret scanner is a gate that can pass without scanning a line | landed |
| `scripts-check-effect-mapping-depth` | medium | Ability label mapping passes after six of seven damage effects disappear | planned |
| `scripts-check-perf-server-not-a-check` | medium | An ignored perf server is named as a check but asserts nothing | planned |
| `scripts-check-acronym-roster-coverage` | low | Acronym check validates values but never asserts roster coverage | planned |

### Lua pack and image build

| id | risk | finding | status |
| --- | --- | --- | --- |
| `lua-pack-image-order-divergence` | high | manifest.data order and build_image sorted order produce different moba images | landed: `generate` uses manifest.data; images now hash identically |
| `lua-pack-archive-module-omission` | high | pack-package.mjs omits 98 module files and all 10 tables from the tar.gz | landed: audit missing 108 -> 0; `check-pack-archive.mjs` gates it |
| `lua-pack-hook-order-unpinned` | medium | init.lua death-hook order is documented but has no test | planned, needs a pack build witness |
| `lua-pack-generated-lua-ungated` | medium | Three generated Lua tables have no banner or --check and CI runs no generator check | planned, needs a pack build witness |
| `lua-pack-ci-lua-bundle-unused` | low | Pages writes a package.preload Lua bundle that nothing loads and omits subdir modules | planned, needs a pack build witness |
| `lua-pack-example-mods-unloadable` | medium | Two of 15 example mods fail mod_check and no test exercises mods/ | planned, needs a pack build witness |

## Verification

The adversarial vote was capped at six claims and started, but no verifier returned before the fan-out was stopped; the resumed pass re-ran the survey instead of reusing cached results. No claim is therefore reported as verified by vote. Instead, the claims this run acted on were checked by execution: `scripts/check-doc-claims.mjs` re-checks the retired-CI and stale-API phrases, `camera-mouse-check.mjs` checks the camera-drag decision, the native pointer probe checks the selection rule, and `check-all.mjs` runs every gate. The infra failure (a verification fan-out that did not complete) is separate from the research result (53 listed findings).

## Needs a decision

- `data-generators-current-drift`: regenerating `ability_fx.lua`, `turn_rates.lua` and `routes.osd` changes values a live match reads through `manifest.json`. The committed data is 6.88g-era; the generators now consume later `heroes_688g.tsv` web columns and a newer pathing map. Reconcile the source of truth before regenerating.
- `rust-sim-ci-test-job-retired`: `9a17155e` deliberately retired the test workflows. Re-adding one is an owner decision, not a defect fix.

## Deferred with a reason

- Client JS seam extraction and the rAF/hot-reload guards need a named Node check and a browser witness on the seam; a broad `play.js` rewrite is explicitly out of scope.
- Browser witnesses (HMR, finished match, two-tab dispatch) need a bounded playwriter run in the worker's own context with a finally cleanup.
- First-draw and Pages changes need a measurement first: CI medians swing between runs and the deployed page carries a 10-minute `max-age=600`.
- gm MCP fixes are owned by the `gm` session; the seven findings were forwarded with their evidence.

## Validation

    node scripts/check-all.mjs
    node scripts/check-all.mjs --strict
    node scripts/check-staged-secrets.mjs --self-test
    node scripts/check-hero-models.mjs --negative-fixture
    git diff --check

The runner reports every gate, every drifted output and every external dependency with its reason; no check is silently skipped.
