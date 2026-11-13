# Oscraft release artifact

Generated: 2026-11-11T21:12:50.261Z
Status: ok=true
Snapshot: 652f8bbf5c616e8b3e67d2e43280269366714ae1777f3c05684eacad1283f728
Identifier regimes: default moba-acronyms/127 acronyms, BOTA bota/26 fullnames, historical moba
PACK_PARAM: scenarios/moba-acronyms
Gates: 80/80 ok, failed=0, drift=0, skipped=6
BOTA models: 26 hashes, joints=28, clips=8
menuBackdrop: bytes=3654481 sha256=431b1f861938c26edb7ce226e75d1c66a433c701d70a59463e064c402a077549
Browser/WebGL: real CDP page, uSkinned=1, jointFloats=448, models=28, heroes=26, clips=8, motion=walk/run, backdropHits=1, other404s=0
Cross contracts: required=8, optionalPresent=10, extraPresent=2, hashBearingRowsVerified=837, driftStale=0
Content contracts: schema=oscraft.content-contract-status/v1, ok=true, inversion=26 acronym keys -> 26 fullname IDs, default=127, doNotPromote=8, demandRows=382
Final assets: schema=oscraft.hy3d-final-asset-status/v1, ok=true, generated=24, blocked=30, ready=24, remaining=30, generatedGlbHashes=24/24, sourceSha256=6bb183f38cffc02ed4c35d947e5a3a731aafba5d3fba8d023d95434e2779eba5
Final asset blockers: pooled-email=studio witch_doctor/rig service-side HTTP 500 no worksId; recovery watcher gated probe; quota preserved; audio=no studio/relay audio action; local BitVoice service absent and production bridge missing; requirements=12 kbps Opus and Brotli requirements were not relaxed; no uncompressed or fabricated audio was published
Cold start: http://127.0.0.1:40809/js/client/play.html pid=1114701 port=40809; pid=1114701 stopped; pidFile removed; peer servers untouched
Cleanup: survivors=0, ownedPidAlive=false, peerServersUntouched=true
Hidden-token scan: status=0, hits=0

## Check exits

- cross-contract snapshot stability: EXIT=0 stable peer snapshot reached
- check-all: EXIT=0 RESULT gates=80 ok=80 failed=0 drift=0 skipped=6 strict=false
- check:acronyms: EXIT=0 check-acronyms: default 127 acronym-only active IDs, BOTA 26 fullname active IDs; authority 127 unique 1-3 letter labels
- check:identifier-runtime: EXIT=0 identifier-runtime PASS: HTTP 200 for default, BOTA and historical play pages; PACK_PARAM=scenarios/moba-acronyms
- check:bota-pack: EXIT=0 bota animation: real skins active; source static mesh bytes preserved in bota-ready.json provenance
- check:cross-contracts: EXIT=0 read-only sources: required 8, optional 10, oscraft 5 contract files
- refresh:cross-contracts --check: EXIT=0 read-only sources: required 8, optional 10, oscraft 5 contract files
- check:cross-autoheal: EXIT=0 read-only peer status extra exit=128 evidenceLines=2
- check:cross-runtime: EXIT=0 extraction evidence extraCdnManifest: present=true classes=? complete=? blocked=? informational=true
- validate:runtime-evidence: EXIT=0 validate:runtime-evidence PASS: default 127 acronyms, BOTA 26 fullnames, historical moba, backdrop/hero hashes, 26/26 skinned hashes, WebGL summary, cross-contract counts
- watch:cross-runtime --once: EXIT=0 extraction evidence extraCdnManifest: present=true classes=? complete=? blocked=? informational=true
- validate:runtime-evidence-status: EXIT=0 validate:runtime-evidence-status PASS: checks match runtime evidence, live snapshot, identifier regimes, WebGL summary, cross-contract counts, extraction summaries and peer status
- check:integrated-cold-start: EXIT=0 check:integrated-cold-start PASS: url=http://127.0.0.1:40657/js/client/play.html pid=1108566 port=40657 ok=true
- watch:integrated --once: EXIT=0 watch:integrated PASS: owned PID cleaned; survivors=0; browser uSkinned=1 jointFloats=448 models=28 heroes=26 clips=8
- validate:integrated-watch: EXIT=0 validate:integrated-watch PASS: coldStart pid=1114701 port=40809 httpResults=10; no survivors; evidence, snapshot, browser, extraction and peer state agree
- check:bota-browser --all: EXIT=0 render evidence: live __ci hooks do not expose per-draw state; CDP WebGL2 createRenderer.drawSkinned probe proved uSkinned=1 with a 448-float joint palette through render3d.js:1007-1025
- check:workspace-pipeline: EXIT=0 check:workspace-pipeline PASS: state=PENDING pending=1 blocking=0 cycles=1
- git status scenarios/moba: EXIT=0 lines=2
- hidden-token scan: EXIT=0 hits=0
- ps survivor scan: EXIT=0 survivors=0

## Peer extraction summaries

- wscheppity: present=true classes=23 complete=23 blocked=0 informational=true
- extra: present=true classes=22 complete=22 blocked=0 informational=true
- extraCdnManifest: present=true classes=? complete=? blocked=? informational=true

## Workspace pipeline

- state: PENDING, ok=true, vendorPrep=UNAVAILABLE, 247420=WAITING
- wscheppity extraction: ok=true, classes=23, complete=23, blocked=0
- READY.json ok=false, handoffGate=null
- BitTTS/BitVoice contract: schema=1.0.0, stability=stable, sha256=f26ca47439d610e244dff4166b5061e94f39039e29be16d83967309ceefdd567, lockMatches=true
- gmweb readiness: ok=true
- PENDING: vendor-prep missing: wscheppity/.local/vendor-prep/manifest.json,wscheppity/.local/vendor-prep/READY.json,wscheppity/.local/vendor-prep/handoff.json,wscheppity/.local/vendor-prep/247420-HANDOFF.md

## Peer git status (read-only)

- bota: exit=0 lines=84 readOnly=true
- hy3d: exit=0 lines=21 readOnly=true
- wscheppity: exit=0 lines=2 readOnly=true
- extra: exit=128 lines=2 readOnly=true
