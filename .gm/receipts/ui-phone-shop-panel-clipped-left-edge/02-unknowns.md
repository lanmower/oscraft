# 02-unknowns: ui-phone-shop-panel-clipped-left-edge

session: gm-drain-1793526214-16
recorded with unknown-add (mutable-add redirects there; it needs id, value, status)

## unknowns (open)
1. phone-shop-clip-width-owner: which #ui-right child (shop root, heading, first card) has the widest min-content width at 390x844, and which CSS rule sets it.
2. phone-shop-clip-flex-end-overflow: does #ui-right (align-items: flex-end, 150px in the phone block) overflow past x=0 when the shop child is wider (live measurement).
3. phone-shop-clip-rect-left-baseline: on the committed tree at 390x844 DPR 2, with the shop opened from the top-strip Shop button, is the shop panel rect left < 0 (witness baseline).
4. phone-shop-clip-render-path: does the shop panel reach widgets.js buildNode via scenarios/moba/panels.lua shop_tree into the right slot, and is the Shop button a top-strip control at phone size.

## facts already proven (orient receipt)
- acceptance text present on the row.
- callers/impact for buildNode, regionsOf, createWidgets (codeinsight).
- phone block rules at js/client/play.html:137-141 and #ui-right at :127.

## next
Load gm-eliminate: each unknown by a live JIT run (gpu-free where it answers the question; browser only for the 390x844 rect).
