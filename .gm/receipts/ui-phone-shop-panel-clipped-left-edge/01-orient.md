# 01-orient: ui-phone-shop-panel-clipped-left-edge

session: gm-drain-1793526214-16
dispatch (row read): 1793527830639-3795-602085caee0274b2

## acceptance (from row)
At 390x844 the shop panel sits inside the viewport on its left edge, with its heading and first card fully visible.
Witness plan: device metrics 390x844 DPR 2, open the shop with the top-strip Shop button, screenshot, check panel rect left >= 0.
Symptom: heading reads "hop gold 1007", first card reads "ad..." (left edge clipped).

## codeinsight (callers / impact)
- buildNode js/client/widgets.js:454-538: callers foldablePanel:163, regionsOf:211, rebuild:547 (self-recursive at 493). impact d2 reaches foldablePanel, rebuild (ambiguous).
- regionsOf js/client/widgets.js:181-233: caller buildNode:479 only. impact d2 reaches buildNode, foldablePanel.
- createWidgets js/client/widgets.js:573-633: caller main js/client/play.js:3072 only.

## codesearch dual
- BM25/vector nearest neighbours: widgets.js createWidgets/regionsOf/buildNode, render3d.js, moba_input.js, scenarios/moba/panels.lua emit_all:1396, shop_tree:386, ui_call:1428, team_tree:276. No literal occurrence of the row text.
- commits: 8d971d92 "ui: place phone panel offsets from the bar token" (widgets.js and modui.js STYLE, phone @600px offsets); a830d270 "web: phone portrait grid fills available width" (widgets.js:249 region); 75617846 "chore: close the shop keyboard-only row stale".

## located layout facts (Read / literal search)
- js/client/play.html:85 `--right-w: 360px` (desktop root default); :84 `--side-w: 240px`.
- js/client/play.html:127 `#ui-right { position: fixed; right: 8px; top: var(--right-top); width: var(--right-w); display: flex; flex-direction: column; align-items: flex-end; ... }`.
- js/client/play.html:137-141 phone block `@media (max-width: 600px), (max-height: 500px) and (orientation: landscape)`: `--right-w: 150px; --right-top: 246px;` and `#ui-right { right: 4px; }`.
- js/client/play.html:147 phone min-target rule covers `#ui-right button`.
- js/client/widgets.js:6 slot map `right: "ui-right"`; :108 phone `#ui-right` max-height rule (var --right-top, --bar-h).
- scenarios/moba/panels.lua:386 `shop_tree()` builds the shop panel tree (owner per .gm/prd/shop-layout-match-dota-shop.yml line 3).

## facts the row relies on
- At 390px the phone block sets the right column to 150px. The column is `align-items: flex-end`, so a child wider than the column overflows to the left, past x=0. The clipped left edge is the expected signature of that combination (heading and first card are the widest children). This is a hypothesis for ELIMINATE, not yet measured.
- The panel width comes from the shop tree / widget CSS, not from --right-w alone. Which child sets the width (min-width, grid column, or a fixed px) is an unknown for ELIMINATE.
- Other lanes edit js/client/widgets.js, js/client/modui.js, js/client/play.html and js/client/hud.js (hud.js is in the dirty tree). Re-read at remote main before editing.

## open unknowns handed to gm-unknowns
1. Which element sets the shop panel's width at 390px (and its min-content).
2. Whether the left overflow is from align-items: flex-end on #ui-right (layout measurement at 390x844).
3. Whether the shop panel rect's left is < 0 on the committed tree (live measurement, not a reading).
