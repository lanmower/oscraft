# 01-orient: ui-layout-widgets-modui-hardcoded-offsets
session: gm-drain-1793526214-17

acceptance (row text): No fixed bar-height pixel value in widgets.js or modui.js; a 844x390 screenshot keeps the map visible above 50 percent.
witness plan (row text): Measure the panel rectangles at 390x844 and 844x390 with the eval probe.
parent row: ui-layout-constants-hard-coded-not-bar-height (pending; its note 2026-10-26 says widgets.js/modui.js offsets are this lane's).

tree: HEAD = origin/main = 032cc2db (fetched); js/client/widgets.js and js/client/modui.js clean.

facts (file:line at origin/main):
- The folded numbers named by the row (widgets.js calc(100vh - 473px), -170px, bottom 124px; modui.js bottom 180px, bottom 128px) are gone. Commit 8d971d92 'ui: place phone panel offsets from the bar token' (row ids listed in its trailer) replaced them with var(--bar-h,...) and var(--right-top,8px).
- widgets.js:107 #ui-left max-height calc(100vh - var(--bar-h,160px) - 10px); :108 #ui-right uses --right-top and --bar-h; :115 #ui-modal bottom calc(var(--bar-h,160px) + 8px).
- modui.js:8 .modui-log bottom calc(var(--bar-h, 160px) + 20px); :15 phone .modui-log bottom calc(var(--bar-h, 120px) + 8px); :16 phone .modui-panels bottom calc(var(--bar-h, 120px) + 8px); :11 .modui-panels top 60px (desktop, not a bar offset).
- play.html defines --bar-h at :root 160px (line 83), 120px (:138 phone), 96px (:150 max-height 480 landscape); --right-top 246/35 at :138/:150. grep of js/client: '473px' appears only in .gm/prd rows.
- Residual hard-coded bar-height pixel values: the var() fallbacks 160px and 120px at widgets.js:107,108,115 and modui.js:8,15,16. Acceptance says no fixed bar-height pixel value, so the fallbacks are in scope.
- widgets.js media guard is (max-width: 600px) only (line 106). At 844x390 that rule does not apply, so #ui-left/#ui-right/#ui-modal clamps are absent there. Whether the map stays above 50 percent at 844x390 is unmeasured (needs the browser probe).

codeinsight: ensureStyle widgets.js:119-125, sole caller createWidgets widgets.js:531 (edge unqualified-name). outline of widgets.js: 58 symbols, STYLE is a module template string (no symbol). The row names files and CSS text, not symbols, so no impact walk is needed beyond ensureStyle.
codesearch dual: BM25/vector hits were unrelated (statBar, later, createMessageLine); the commits channel returned 8d971d92 for modui.js and the row-named commits. literal scan: 473px only in the row/parent prd files.

derived acceptance: none needed (acceptance text present).