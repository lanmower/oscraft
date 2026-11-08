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
| browser-witness-rescope | Browser witnesses need a bounded playwriter run with its own context. | landed: docs/BROWSER_WITNESSES.md and check-browser-witnesses.mjs |
| local-receipt-hygiene | Receipts are ignored, volatile gm state is untracked, and five stray ignore patterns were deleted. | landed |
| generated-lua-uncovered | Left uncovered on purpose: `gen-ability-keys` needs an external g688 LNI dir; `gen-ui-fill` and `gen-hero-acronyms` have no `--check`. turn_rates.lua is content-checked (417af5d9). | accepted gap, owner-approved |

## Landed in this run

| change | witness |
| --- | --- |
| `scripts/check-all.mjs`: one local validation entry point | `RESULT gates=59 ok=59 failed=0 drift=0 skipped=5 strict=false`; `--strict` exits 1 |
| `scripts/check-doc-claims.mjs`: the retired-CI and stale-API claims cannot come back | `doc claims ok 17/17` |
| `scripts/check-staged-secrets.mjs`: pure matcher, self-test on every run, git errors fail the gate | `secret patterns ok 5/5`, `selftest=0` |
| `scripts/check-hero-models.mjs`: 127 heroes all have a model, with a negative fixture | `hero models ok: 127 heroes, all with a model`, `hero models negative fixture caught` |
| Three drifted generated outputs reconciled by owner decision | hy3d patch sha 18eead38... applies cleanly; "RESULT gates=42 ok=42 failed=0 drift=0" |
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
| `docs-hygiene-naming-spec-retired` | low | docs/NAMING.md is an unapplied rename spec that declares its own deletion | landed: retired with git rm; check-naming-doc.mjs |
| `docs-hygiene-agents-budget-duplicates` | low | AGENTS.md is at 83% of its 30 KB cap and carries four troubleshooting receipts already in DESIGN.md | landed |
| `docs-hygiene-gitignore-tracked-state` | medium | The applied .gitignore receipt hygiene does not cover the paths that keep git status dirty, and ignore rules cannot clean tracked runtime files | landed |

### Pages build and deploy

| id | risk | finding | status |
| --- | --- | --- | --- |
| `pages-scout-postdeploy-compare-gate` | medium | Pages deploys with no post-deploy comparison, and the docs claim one exists | deferred with reason: max-age=600 makes an automated fetch unsafe; docs/HOST.md now says to check by hand |
| `pages-scout-cache-key-omits-vendor` | medium | Cached wasm/maps can come from an older commit while version.json stamps HEAD | landed: both keys hash js/client/vendor/** |
| `pages-scout-post-bundle-artifacts-unchecked` | low | Bundling rewrites _site after its only file check, and chunk/Lua outputs are never asserted | landed: check-site-bundle.mjs runs after both bundle steps |
| `pages-scout-observer-preload-blind-spot` | low | Live probe skips 24 of 63 modulepreload URLs, exactly the vendor files layout prunes | refuted for HEAD (64/64 present); regression gate in check-site-bundle.mjs |
| `pages-scout-version-stamp-lacks-content-hash` | medium | version.json is a claim string, not a content hash; a mixed-generation deploy passes the probe | landed: contentSha256 stamped over the built artifacts |
| `pages-scout-release-duplicates-site-layout` | medium | Release archive re-stages the site with a third, unrelated build identity | landed: build.sh delegates to layout-site.sh |

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
| `firstdraw-stale-modulepreloads` | medium | The deployed bundled page still preloads 39 unbundled local modules and loads rooms.js loose | planned: at HEAD the count is 64 and rooms.js loads loose at play.html:228; the fix (scripts/bundle-client.mjs, check-site-bundle.mjs bundled-page check) is held outside the tree because it needs the pages.yml bundle step replaced, which only a real Pages deploy can prove |
| `firstdraw-probe-median-worldbuilt-request-unit` | low | The first-draw probe reports the slower sample as the median, calls distinct paths requests, and measures worldBuilt rather than a frame | landed: scripts/check-first-draw-median.mjs (5 fixtures) |
| `firstdraw-boot-critical-path` | medium | loadSim starts 99 Lua fetches and the 944 KB image alongside the 601 KB wasm, and the Pages Lua bundle is dead output | landed: named overlap test (js/client/boot-overlap-check.mjs) and one bundle request for the Lua closure (scripts/check-lua-bundle.mjs; live counts in DESIGN.md). CI wire size (run 42475227556): live bundle 282,168 bytes vs 1,167,040 for the 99 .lua files; wasm 497,938 bytes on the wire (the 601 KB figure was a local build's gzip) |
| `firstdraw-cold-warm-cache-probe` | low | The first-draw probe is cold-only and uses different headers and MIME types from the pack-cache probe, so cache reuse is never measured | landed: check-first-draw-passes.mjs (cold and warm passes; 17 MIME types match; negative controls rejected) |
| `firstdraw-shipped-asset-prune-check` | medium | layout-site.sh ships the whole 113 MB scenarios tree while a measured first draw transfers 3 MB, with no check of which loose files are actually requested | landed as a report gate: scripts/check-shipped-assets.mjs (38.7% of 113.1 MB unreferenced; deletes nothing) |

### Generators and committed data

| id | risk | finding | status |
| --- | --- | --- | --- |
| `data-generators-offline-gate` | medium | Replace the generic generator-check idea with one manifest-driven offline gate | landed |
| `data-generators-current-drift` | high | Regenerate three shipped outputs that already fail their own checks | landed: owner approved hy3d's patch; all three --check now pass |
| `data-generators-external-input-contract` | medium | Pin external hy3d inputs instead of relying on a machine path or local checkout | landed: unit.ini and ability.ini pinned by sha256 in scenarios/moba/tools/external-688g.tsv; scripts/check-external-inputs.mjs fails on a mismatch |
| `data-generators-temp-copy-no-check` | medium | Compare no-check generators through a temp copy rather than adding 25 ad hoc flags | landed: check-generators-temp-copy.mjs (3 regenerations identical; tamper control caught) |
| `data-generators-osdt-exclusion` | low | Exclude the committed stale .osdt snapshot from source-drift checks | planned |
| `data-generators-hero-model-completeness` | high | Assert nonempty hero models in the same gate that checks hero bodies | landed |

### Rust crates and sim

| id | risk | finding | status |
| --- | --- | --- | --- |
| `rust-sim-ci-test-job-retired` | medium | Restore a Rust test workflow and correct the stale ci.yml / rust-gate documentation | needs decision |
| `rust-sim-item-generation-global` | medium | Make item_generation_total per-Sim instead of a process-wide atomic | landed: Sim field, existing tests pass |
| `rust-sim-replay-format-untested` | low | Add replay format-3 parse/render round-trip tests to the only format code with no tests | landed: replay/tests/format_three.rs, 2 tests |
| `rust-sim-state-version-contract` | medium | Pin STATE_VERSION with a full-plane save/load contract test | landed: sim/tests/state_version_contract.rs, 1 test |
| `rust-sim-wasip3-command-wit-parity` | low | Replace the hand-synced wasip3 COMMANDS and WIT contract with a checked parity script | landed: check-wasip3-parity.mjs gates 20 exports vs 20 commands |
| `rust-sim-relay-tie-order` | low | Give relay REQ responses a total order when created_at ties | landed: REQ responses total-ordered on created_at+id; live subscription EVENT delivery stays arrival-ordered by design |

### Client JS

| id | risk | finding | status |
| --- | --- | --- | --- |
| `client-js-frame-chain-guard` | high | rAF chain dies on one unguarded Game call | landed: frame_chain.js schedules in finally; frame-chain-check.mjs |
| `client-js-safe-storage-seam` | medium | Only 2 of 9 localStorage sites are guarded | landed: safe_storage.js, zero raw calls in play.js, safe-storage-check.mjs |
| `client-js-hud-hot-state` | medium | hmrReload rebuilds the HUD and drops selection and courier state | landed: hot_state.js, hmrReload resets it, hud-hot-check.mjs |
| `client-js-input-route-seam` | medium | sendInput routing is pure branching mixed with I/O and seven globals | landed as `lobby_link.js` with `lobby-link-check.mjs` (12 cases) |
| `client-js-key-names-seam` | low | Two key normalizers disagree on pack Numpad bindings | refuted as active defect; key_names.js shared, key-names-check.mjs only, no behaviour change |
| `client-js-codec-copy-gate` | medium | Deployed wire codec is a 421-line copy with no drift gate | landed: check-wire-codec.mjs gates sha256 parity |

### Validation checks

| id | risk | finding | status |
| --- | --- | --- | --- |
| `scripts-check-runner-wiring-and-external-skip` | medium | The new check runner is untracked, unwired, and permanently skips editor-check | landed |
| `scripts-check-doc-command-drift` | low | Stale gate commands need a documentation path and command check | landed |
| `scripts-check-secret-self-test` | high | The secret scanner is a gate that can pass without scanning a line | landed |
| `scripts-check-effect-mapping-depth` | medium | Ability label mapping passes after six of seven damage effects disappear | landed: 7-label floor and mutation case |
| `scripts-check-perf-server-not-a-check` | medium | An ignored perf server is named as a check but asserts nothing | landed: renamed to asset-packs-server.mjs, check-perf-probes.mjs |
| `scripts-check-acronym-roster-coverage` | low | Acronym check validates values but never asserts roster coverage | landed: 127 acronyms for 127 heroes |

### Lua pack and image build

| id | risk | finding | status |
| --- | --- | --- | --- |
| `lua-pack-image-order-divergence` | high | manifest.data order and build_image sorted order produce different moba images | landed: `generate` uses manifest.data; images now hash identically |
| `lua-pack-archive-module-omission` | high | pack-package.mjs omits 98 module files and all 10 tables from the tar.gz | landed: audit missing 108 -> 0; `check-pack-archive.mjs` gates it |
| `lua-pack-hook-order-unpinned` | medium | init.lua death-hook order is documented but has no test | landed: check-hook-order.mjs gates 61 requires |
| `lua-pack-generated-lua-ungated` | medium | Three generated Lua tables have no banner or --check and CI runs no generator check | landed: 9 banners, 16 generator gates; ability_text_688g.lua external |
| `lua-pack-ci-lua-bundle-unused` | low | Pages writes a package.preload Lua bundle that nothing loads and omits subdir modules | landed: bundle-lua.mjs writes bundle/moba.modules.json, a JSON map of the manifest modules; the client reads it once for the Lua closure (js/client/lua_closure.js) |
| `lua-pack-example-mods-unloadable` | medium | Two of 15 example mods fail mod_check and no test exercises mods/ | landed: both fixed, check-example-mods.mjs runs all 15 |

## Verification

The adversarial vote was capped at six claims and started, but no verifier returned before the fan-out was stopped; the resumed pass re-ran the survey instead of reusing cached results. No claim is therefore reported as verified by vote. Instead, the claims this run acted on were checked by execution: `scripts/check-doc-claims.mjs` re-checks the retired-CI and stale-API phrases, `camera-mouse-check.mjs` checks the camera-drag decision, the native pointer probe checks the selection rule, and `check-all.mjs` runs every gate. The infra failure (a verification fan-out that did not complete) is separate from the research result (53 listed findings).

## Remaining work, ranked (2026-11-08)

Held and not in this list: GP-CHT-15 repick window (docs/GAMEPLAY_PARITY.md) and firstdraw-stale-modulepreloads (modulepreload fix).

| rank | item | status | witness |
| --- | --- | --- | --- |
| 1 | Hero area abilities run their effects: Hoof Stomp (aoe, damage, stun) and Reverse Polarity (aoe, damage, stun, displace toward the caster) | landed as witnesses: rust/crates/sim/tests/hero_area_abilities.rs | CI https://github.com/lanmower/oscraft/actions/runs/42476198386 (sim-witness.yml): 2 passed; local negative control (enemy moved outside the radius) fails |
| 2 | Reincarnation Leoric's own passive is unwitnessed (gates 6, 11, 16) | resolved | rust/crates/mod-host/tests/leoric_reincarnation.rs witnesses the revive after 48 ticks (±3), where he died, the 160 mana charge, the cooldown from the death and the 900-unit slow; CI run https://github.com/lanmower/oscraft/actions/runs/42477054117 passed (1 passed, 1.72 s) |
| 3 | aoe excluded-filter mask (Hoof Stomp) | resolved for the land-only filter: 6.88g Ubertip names enemy land units; aoe p1 bit 32 skips airborne units | rust/crates/sim/tests/hero_area_abilities.rs hoof_stomp_leaves_air_units_in_its_radius_alone (failed first: the hawk took damage and died; passes with the flag); CI https://github.com/lanmower/oscraft/actions/runs/42478080527 (sim-witness.yml) passed, 4 passed; other excluded filters are not checked |
| 4 | Aggro attack-move-back and visibility (yU) not ported | resolved: right-click gate (yU) and attack-back to the order on release, ported from war3map.j 31437 onward | rust/crates/web/tests/aggro_right_click.rs: HEAD fails at lines 189 and 200 (left 127 and -1); the port passes 6 of 6 |
| 5 | Regeneration sources other than the rune are not checked for ending at full | planned; use the rune witness pattern | AGENTS regeneration line |
| 6 | Courier controls unchecked: item exclusions, owner-return point, per-owner filter, zoom and rotation-lock keys | planned | AGENTS courier lines |
| 7 | Wolf form E015 not modelled | planned | AGENTS |
| 8 | Live feel of the hero view lock at a usable frame rate | needs a bounded browser run | the lock gates are logic proxies |
| 9 | rust-sim CI test job retired | blocked: owner decision | 9a17155e |
| 10 | pages-scout post-deploy compare gate | deferred: max-age=600 makes it unsafe | row text above |

Landed in this pass: data-generators-temp-copy-no-check (e02044a2, check-generators-temp-copy.mjs: 3 regenerations identical, tamper control caught) and firstdraw-cold-warm-cache-probe (46c85c32, check-first-draw-passes.mjs: cold and warm, 17 MIME types match, negative controls rejected). Both rows were stale as planned.

## Needs a decision

- `data-generators-current-drift`: resolved. The owner approved reconciling the three outputs to their generators; DESIGN.md 2026-11-06 records hy3d's patch and the zero-drift gates.
- `rust-sim-ci-test-job-retired`: `9a17155e` deliberately retired the test workflows. Re-adding one is an owner decision, not a defect fix.

## Deferred with a reason

- Client JS seams remain open; hy3d supplied read-only witnesses (rAF partly guarded, 6 of 9 localStorage sites unguarded, no hot-state check, key-name normalizers disagree on Numpad1/End, codec copies byte-identical but ungated). Each needs a named Node check; a broad `play.js` rewrite is out of scope.
- Browser witnesses (HMR, finished match, two-tab dispatch) need a bounded playwriter run in the worker's own context with a finally cleanup.
- First-draw measurement supplied by hy3d, no code change: the retired CI job's last band is 6007-11772 ms median with 21,797,827 B per cold load in the latest green run 40189655805. Any change still needs a fresh measurement; Pages sends `max-age=600`.
- gm MCP fixes are owned by the `gm` session; the seven findings were forwarded with their evidence.

## Validation

    node scripts/check-all.mjs
    node scripts/check-all.mjs --strict
    node scripts/check-staged-secrets.mjs --self-test
    node scripts/check-hero-models.mjs --negative-fixture
    git diff --check

The runner reports every gate, every soft drift check and every external dependency with its reason; no check is silently skipped.
