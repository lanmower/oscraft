# 01-orient: swap-deferred-reannounce-diverges

session: gm-drain-1793526214-12
ts: 2026-11-01T10:20Z
head: b8897bf5 (HEAD == origin/main at fetch time)

## acceptance (derived)
clause: a peer holding the first swap tick accepts a same-file, same-hash re-announce at a later tick in place of its pending record, so both pages apply the same tick and SWAP_CANCEL matches on both sides.
source: row subject and description (js/client/play.js re-announce path), prd row `swap-deferred-reannounce-diverges`.

## symbols (codeinsight callers/impact)
- hostSwapGate: defined js/client/play.js:2805-2823; caller startHost js/client/play.js:1921-1964 (edge at :1959). impact depth 1 only, tests_reached 0.
- failSwapNotReady: defined js/client/play.js:2825-2837; caller module scope js/client/play.js:2279. impact depth 1 only.
- prefetchSwapImage: defined js/client/play.js:2782-2797; caller module scope js/client/play.js:2271.

## code facts (codesearch dual, tree reads)
- Fix present on main: js/client/play.js:2238 `const reannounce = pendingSwap !== null && pendingSwap.file === frame.file && pendingSwap.hash === frame.hash && frame.tick > pendingSwap.tick;` and :2239 `if (pendingSwap && !reannounce)` refuses only other second announces.
- Commit 495d440a (lanmower, "Take a deferred swap re-announce in place of the pending record") carries the same hunk but is NOT an ancestor of origin/main or HEAD; the hunk reached main through squash commit 1cfc8d0d. The row note's claim that origin tip 77168a44 contains it is stale (the note's line numbers, play.js:1969/1981/2589, predate the current file).
- Working tree js/client/play.js is clean against HEAD (no other lane's hunks at orient time), so the write queue reason no longer applies.
- failSwapNotReady (2825) cancels only when pendingSwap.tick equals report.tick, the mechanism the row describes; with the reannounce branch the pending record already carries the moved tick.

## witness state
- The only open witness is a live two-tab swap run. Blocker chain: row blockedBy `live-swap-verify-2-setup` (pending); its blocker `live-swap-verify-2-setup-blocker-drain-net-shard-2-1` records that the Pages build has no swap channel (/__hmr 404, swap triggered only by the oscraft:hmr event, hmr.js:23,89,110).
- Dev origin exists: scripts/dev/serve.mjs serves /__hmr (HMR_PATH, line 21). No CI job runs a two-tab swap (ci.yml browser-smoke covers lobby, console, icons only), so no CI witness can close the row.

## unknowns for gm-unknowns
1. Does a two-peer run on a dev origin reach a deferred re-announce at all (peer must still hold the first tick when hostSwapGate fires at tick+32)? Requires throttling one page.
2. Is the WebRTC/Nostr signalling reachable from this sandbox?
3. Does the pre-fix code actually diverge (the bug is reasoned, never executed)?
