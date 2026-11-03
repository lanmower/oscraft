# 03-eliminate

row: ui-phone-lobby-grid-hud-lobby-1
session: gm-drain-1793526214-15

## Unknowns
- u1 a830d270 on origin/main for widgets.js: KNOWN. git_log origin/main path=js/client/widgets.js lists 49b5ad1f, 8d971d92, a830d270 (dispatch 1793528027013-3960-555dec8571afcb69). Fit auto-fill shipped under Row ui-phone-lobby-overflows.
- u4 sibling ui-phone-lobby-overflows still pending with the same grid clause (prd-list dispatch 1793527995109-3918-bb598c76551e9e8a): KNOWN, a duplicate-scope risk.
- u3 observeFit columns fit the grid at 374px: KNOWN at grid level. Probe (mobile 390x844, dev server 8741, capture .gm/browser-cdp-capture/1793529489832-1735573.json lines 33261/33293/33336 and 1793529553734-1749738.json lines 33437/34547): modal 8..382 (374 wide); three .w-portraits.fit grids cw 336 = sw 336, columns repeat(6,52px)/repeat(5,54px)/repeat(6,52px). Control with repeat(8,56px): sw 472 > cw 336, so the probe can fail.
- u2 every hero tile inside the modal right edge: OPEN. The probe counts 84 of 127 .w-portrait cells in #ui-modal with right edges up to 1024px, unexplained by the grid scrollWidth. Screenshot .gm/receipts/ui-phone-lobby-grid-hud-lobby-1/grid-390x844.png shows the hero grid inside the modal; the boxes past the edge are the team cards. A per-cell dump (class, rect, transform) is needed.

## Checks that could not run
- chrome attempts refused while the shared accelerated session was held by other lanes: 1793528309709-4280-fc038dd512f356c, 1793528422548-4364-727eeb95699c51ed, 1793528837691-4680-9e4e20ff2f79a019, 1793529033584-4851-6ec4d898a9ce4925.
- dev server 8741 down (fetch ERR connection refused). Fresh stage on 8752 fails sim::load BadVersion(26) (dispatch 1793531506911-2210-546c6874d337c1a6): the staged wasm and map are stale. Server stopped after the run.
- local checkout is stale_tree: 1 ahead, 15 behind upstream adb338708222 (dispatch 1793531532942-2221-d185fcff38099b77). A local-tree witness does not measure remote main.

## Outcome
Row cannot close here. Filing blocker ui-phone-lobby-grid-hud-lobby-1-blocker-gm-drain-1793526214-15.