# 03-eliminate: ui-phone-shop-panel-clipped-left-edge

session: gm-drain-1793526214-16
browser: lease held by gm-drain-1793526214-16 at port 38631 (waited behind 14 and 15); released at the end of this stage
probe scripts (outside the repo): /tmp/gm-16-probe/probe.mjs, probe2.mjs, probe3.mjs (synthetic shop tree through the real widgets.js renderer and the real play.html CSS, served from the checkout)
dispatch ids of the gm reads: 1793527830639-3795-602085caee0274b2 (row), 1793528030867-3965-35089468081dcc59 (commit 8d971d92)

## unknown phone-shop-clip-flex-end-overflow: answered yes (mechanism)
Expected from the contract (row acceptance): panel.left >= 0 and panel.right <= 390 at 390x844.
Observed, phone column (--right-w 150px, right 4px, viewport 390, column left 236, right 386), synthetic panel 178px wide:
- baseline (align-items flex-end): panel 208 to 386, width 178. Overflows the 150px column by 28px to the LEFT.
- forced align-items flex-start: panel 236 to 414. Overflows to the RIGHT.
- forced align-items stretch: panel 236 to 386, width 150. Inside.
- cap `#ui-right > * { max-width: 100% }`: panel 236 to 386, width 150. Inside; first card 247 to 287.
Result: a child wider than the column overflows the column's free edge; with flex-end that edge is the left one.

## unknown phone-shop-clip-render-path: answered yes
- scenarios/moba/panels.lua:1404 `emit("shop", "right", OPEN.shop and shop_tree() or nil)`; shop_tree (panels.lua:386) returns `panel(title, sections)` (panels.lua:451), title "Shop  Gold <n>".
- widgets.js createWidgets.apply (widgets.js:605-631) mounts the tree under `hostFor("right")` = #ui-right; the DOM is `#ui-right > .w-root[data-id=shop] > .w-panel.w-foldable` (measured).
- The top-strip Shop button and the in-game opening were NOT driven (needs a running match; see residual below).

## unknown phone-shop-clip-width-owner: partly answered
- Panel width = fit-content of its children in the column: min(max-content, max(min-content, column width)). Measured min-content in the synthetic tree: 178 = title 156 + padding 22; the grid tracks measured 49.3 px each (buttons wrap at min-target).
- Labels wrap anywhere (overflow-wrap), `.w-grid`/`.w-cell` are min-width 0, so long item names do not widen the panel. The forced-nowrap arm (`#ui-right .w-label{white-space:nowrap}` with a 50-char token) did not move the width, so nothing in the widget CSS lets a single token widen it.
- The real shop tree's widest contributor is NOT measured here: the synthetic tree never reaches the clipped width (real symptom: heading starts left of 0). Residual, see below.

## unknown phone-shop-clip-rect-left-baseline: NOT answered on the real tree
- Expected (contract): panel.left >= 0. Synthetic baseline gives 208 (inside), so the synthetic does not reproduce the reported clip.
- Exact reason: the real shop tree needs a running match (the Lua shop_tree output is produced by the mod host inside the wasm, and the Shop button exists only in-match). Local pkg is `rust/crates/web/site/pkg/web_bg.wasm`, but the match flow to reach the top-strip Shop was not driven in this run.
- Consequence: the fix below is proven by construction for any panel width (cap), not by a measured real-tree before/after.

## residual for the verifier lane
- In-game witness at 390x844 (top-strip Shop, screenshot, panel rect left >= 0, heading and first card inside) on the pushed sha. Not run here.
