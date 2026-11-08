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
| `firstdraw-stale-modulepreloads` | medium | The deployed bundled page still preloads 40 unbundled local modules that play.bundle.js already inlines, and loads rooms.js loose | landed: scripts/bundle-client.mjs bundles play.js and rooms.js, removes the 40 stale modulepreloads and keeps the 24 vendor preloads, and check-site-bundle.mjs fails a site that still carries them. Row: firstdraw-stale-modulepreloads. Witness: on HEAD 1bf621a the site check exits 1 with "40 stale modulepreloads of modules play.bundle.js already inlines" and the loose rooms.js; the after site passes ("24 kept"). First draw, same job on both sites (3 browser contexts, cold and warm, 6 loads each, worldBuilt in ms): HEAD median of six 1517 (cold 1518, 1516, 1536; warm 1542, 1502, 1481), 126–127 requests, 3.15 MB; after median 1470.5 (cold 1483, 1448, 1459; warm 1475, 1466, 1481), 88–90 requests, 3.06 MB. The worldBuilt gap is inside the run spread; the request and byte drop is the measured effect |
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
| `data-generators-osdt-exclusion` | low | Exclude the committed stale .osdt snapshot from source-drift checks | resolved differently: the image was stale by 4,086 bytes, so it is refreshed from mapexport and compared by the manual gate `scripts/check-moba-image.mjs`, not excluded. Witness: the gate exited 1 on the stale image and passes after the refresh (`moba image ok: 944977 bytes sha256 aea61d47...`) |
| `data-generators-hero-model-completeness` | high | Assert nonempty hero models in the same gate that checks hero bodies | landed |

### Rust crates and sim

| id | risk | finding | status |
| --- | --- | --- | --- |
| `rust-sim-ci-test-job-retired` | medium | Restore a Rust test workflow and correct the stale ci.yml / rust-gate documentation | decided: no automated checks in CI (owner); not restoring |
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

Held and not in this list: GP-CHT-15 repick window (docs/GAMEPLAY_PARITY.md).

| rank | item | status | witness |
| --- | --- | --- | --- |
| 1 | Hero area abilities run their effects: Hoof Stomp (aoe, damage, stun) and Reverse Polarity (aoe, damage, stun, displace toward the caster) | landed as witnesses: rust/crates/sim/tests/hero_area_abilities.rs | CI https://github.com/lanmower/oscraft/actions/runs/42476198386 (sim-witness.yml): 2 passed; local negative control (enemy moved outside the radius) fails |
| 2 | Reincarnation Leoric's own passive is unwitnessed (gates 6, 11, 16) | resolved | rust/crates/mod-host/tests/leoric_reincarnation.rs witnesses the revive after 48 ticks (±3), where he died, the 160 mana charge, the cooldown from the death and the 900-unit slow; CI run https://github.com/lanmower/oscraft/actions/runs/42477054117 passed (1 passed, 1.72 s) |
| 3 | aoe excluded-filter mask (Hoof Stomp) | resolved for the land-only filter: 6.88g Ubertip names enemy land units; aoe p1 bit 32 skips airborne units | rust/crates/sim/tests/hero_area_abilities.rs hoof_stomp_leaves_air_units_in_its_radius_alone (failed first: the hawk took damage and died; passes with the flag); CI https://github.com/lanmower/oscraft/actions/runs/42478080527 (sim-witness.yml) passed, 4 passed; other excluded filters are not checked |
| 4 | Aggro attack-move-back and visibility (yU) not ported | resolved: right-click gate (yU) and attack-back to the order on release, ported from war3map.j 31437 onward | rust/crates/web/tests/aggro_right_click.rs: HEAD fails at lines 189 and 200 (left 127 and -1); the port passes 6 of 6 |
| 5 | Regeneration sources other than the rune are not checked for ending at full | resolved for the water sip: it ends at full while a regeneration rune runs (war3map.j QXV removes B04A); the other timed sources already match 6.88g; the rune ends when attacked (6.88g I008), not on any HP loss | rust/crates/mod-host/tests/bottle_sip_full_health.rs: before the port left 3666 right 0 at full; after, 2 passed. rust/crates/mod-host/tests/rune_regen_attack_dispel.rs: before the port a_spell_hit_does_not_dispel_the_regeneration_rune failed; after, 2 passed |
| 6 | Courier controls unchecked: item exclusions, owner-return point, per-owner filter, zoom and rotation-lock keys | resolved for the drop exemptions, the owner-return point and the per-owner filter; zoom and rotation-lock keys stay GUESS (GP-CTL-8), and the page's own key handling is witnessed | rust/crates/mod-host/tests/courier_owner_return.rs: before the port a_basic_courier_drops_an_owners_item_at_the_owners_home_point_from_anywhere failed (got 6062080, 15761408: the courier's own point, not the home 17891328, 4128768) and a_loan_copy_of_a_rune_stays_on_the_courier_when_it_is_dropped failed (the loan copy dropped); the flying-courier and pickup-filter tests passed before and after as guards; after the port 4 passed. js/client/zoom-check.mjs: zoom keys ok |
| 7 | Wolf form E015 not modelled | resolved for the morph, the E015 stats and damage point (0.26 s), the 18 s timer, Crit Strike (already the ability's own timed crit, 30/35/40% at 1.7x) and death ending the form (6.88g TAO); open: the Ghost (Aeth) ability, which needs an ethereal state the sim lacks; the model (GUESS placeholder, `neutral-giant-wolf-skinned`); and the death clear of every positive timed modifier (GUESS, no per-source clear) | rust/crates/mod-host/tests/lycan_wolf_form.rs: before the port the Lycan stayed `lycan` after the cast (left "lycan", right "lycan_wolf_form") and the wolf swung on 550 ms (left 550, right 260); the revive after death came back as the wolf (left "lycan_wolf_form", right "lycan"). After: 5 passed |
| 8 | Live feel of the hero view lock at a usable frame rate | resolved: with the lock engaged and ArrowUp held, the live page runs at 59.5 fps median (p95 34 ms) and the camera stays a median 0.33 cells behind the hero (p95 0.51, max 0.61 over a 3 s hold); the rate had been held at 19.9 fps by the fog cost, row 15. The released control was not re-run this time (earlier runs: 50 to 75 units behind) | live runs through playwriter inside gm (nocache=13 for the rate, nocache=16 for the lag); the lock gates stay logic proxies; DESIGN.md 2026-11-08 records the probe |
| 9 | rust-sim CI test job retired | resolved: the owner decided no automated checks in CI, only manual checks; sim-witness.yml, mod-host-witness.yml and lua-wire-size.yml are removed, pages.yml and release.yml stay as builds | 9a17155e (decision); this commit (removal) |
| 10 | pages-scout post-deploy compare gate | stale: retired, not built (decided 2026-11-08) | the owner removed automated checks from CI (row 9), and a post-deploy compare would be one; freshness stays a manual check (AGENTS: compare `version.json` contentSha256 by hand after a deploy). A cache-busted fetch would avoid the max-age=600 stamp cache, but the owner's rule against automated checks decides it. Witness: node scripts/check-all.mjs unchanged, no gate added |
| 11 | Courier pickup target: 6.88g SKV (war3map.j 19315) adds the owner's items to the owner's courier Md, not the hero | resolved: the caller's items on couriers in reach go to the owner's courier creation point (SAV 19145), then the owner's basic courier takes them (the Md of SKV: the map's courier at its home point); the Lua range gate is the only reach rule; the hand-over to the hero is the courier's auto-delivery (GUESS, GP-CTL-10) | rust/crates/mod-host/tests/courier_owner_return.rs a_pickup_moves_the_callers_owned_items_onto_the_owners_basic_courier: before the port it failed at line 187 (the owner's basic courier held nothing after 3 ticks); after, 4 passed |
| 12 | Aggro SME owner filter: the redirect took any hostile creep or tower, not only those players 0 and 1 own (war3map.j 31391) | resolved: candidates must be non-heroes owned by players 0 and 1 (targeting.lua creep_owned) | rust/crates/web/tests/aggro_right_click.rs: before the port a_hostile_creep_that_no_lane_player_owns_is_not_redirected failed (left 127, right -1); after, 7 passed |
| 13 | Aggro IsUnitIllusion (SUE 31437) and the SME `Aw` filter (u00R, umtw, e026, ebal) not ported | resolved: an illusion order target is not redirected (targeting.lua note_attack_order, `game.is_illusion`); the Aw siege ids are outside AGGRO_KINDS, so no siege creep is redirected, but the filter itself is not ported (a siege kind added to AGGRO_KINDS needs its own check) | rust/crates/web/tests/aggro_right_click.rs a_right_click_on_an_enemy_illusion_does_not_redirect_the_lane_creep: red on HEAD (left 127, right -1) with the image enrolled as hero-typed; after the port 9 passed. a_right_click_on_an_enemy_hero_does_not_redirect_a_siege_creep is a guard, green before and after |
| 14 | A2ML ground-cast retargeting (Living Armor, GROUND_RETARGET in spells.lua) only took heroes, not the allied xZ structures (6.88g ALE 54120-54200, VQO 54133) | resolved: the ground cast goes to the nearest allied hero or xZ structure (tower tiers and both barracks types, by kind name, LIVING_ARMOR_STRUCTURE_KINDS); the O_V exclusions for A04R and Ty() are not ported | rust/crates/mod-host/tests/living_armor_ground_retarget.rs: red on HEAD spells.lua (both tests fail at the tower assertion: the tower gets no regen stat); after the port 2 passed |
| 15 | Fog layer cost during movement: rebuildLayer projected every vision-grid corner each frame the camera moved, and worldClip rebuilt the view-projection and allocated per call (play.js, fogview.js) | resolved: worldViewProjection caches the matrix on the camera state and aspect, worldClip builds its result without a per-call closure, and the fog grid is bounded to the visible ground plus a 3-cell margin (fogview.js cellRange; render3d.js visibleGroundBounds; the full grid when the horizon is in view) | live page through playwriter inside gm, same probe before and after: lock engaged 17.8 fps in the earlier untouched build and 19.9 fps (p95 117 ms) with the cache alone, 59.5 fps (p95 34 ms) after; lock off 13.6 fps untouched, 59.9 fps after; worldClip 23.7% to 2.4% of a locked profile; node scripts/check-all.mjs RESULT gates=73 ok=73 failed=0 drift=0 |
| 15 | Living Armor's ground cast (O_V, war3map.j 6022–6023) excluded no units with ability A04R (Marker) and had no Ty() check | resolved: the Marker exclusion is ported as `excludes_marked` on the Living Armor rule (spells.lua ground_retarget); Ty() (typed or dead, war3map.j 4894) is covered by the existing unit_alive check, so no port was needed | rust/crates/mod-host/tests/living_armor_ground_retarget.rs: on HEAD 6ff0510 the marker case fails (left 400, right 0: the marked tower takes the armour); after the port 4 passed. The Ty case (dead tower nearest the point) passes on HEAD and after the port, as a guard. Fixture: `marker` ability in abilities_688g_creeps.osd |

Landed in this pass: data-generators-temp-copy-no-check (e02044a2, check-generators-temp-copy.mjs: 3 regenerations identical, tamper control caught) and firstdraw-cold-warm-cache-probe (46c85c32, check-first-draw-passes.mjs: cold and warm, 17 MIME types match, negative controls rejected). Both rows were stale as planned.

## Needs a decision

- `data-generators-current-drift`: resolved. The owner approved reconciling the three outputs to their generators; DESIGN.md 2026-11-06 records hy3d's patch and the zero-drift gates.
- `rust-sim-ci-test-job-retired`: `9a17155e` deliberately retired the test workflows, and the owner has since decided there are no automated checks in CI. Checks run by hand (`node scripts/check-all.mjs`); builds stay in CI.

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
