# 05-validate: ui-layout-widgets-modui-hardcoded-offsets
session: gm-drain-1793526214-v5
sha: baa9778da21837d6cd91d28b5f414f19626008ee (origin/main, fetched 2026-11-01 ~10:42Z; shared checkout at e56dd0b6 is behind, not used)
mode: witness

expected (written before running):
- E1: widgets.js and modui.js at sha carry no fixed px fallback on --bar-h or --right-top: 0 hits.
- E2: live 844x390 screenshot, visible-above-bar > 0.5.
- E3: live 390x844, #ui-right maxH unchanged at 468px.

observed:
- E1 PASS (static, at sha): widgets.js 3 bar-h lines (107, 108, 118), modui.js 3 (8, 15, 16), all bare var(--bar-h) / var(--right-top); 0 fixed-px fallbacks. Landscape @media (max-width: 600px), (max-height: 500px) and (orientation: landscape) at widgets.js:106 wraps the clamps at 107-108. Blob reads: dispatch 1793529725345-1603-b21abee6ec270ac3 (widgets.js), 1793529725859-1604-b20f0e9ac7c40231 (modui.js); fetch dispatch 1793529712851-1591-d40110db9c99aa89.
- E2 UNWITNESSED: browser lease held by gm-drain-1793526214-10-v1 (live, heartbeat 1.9 s, pid 1727201, port 44351). No second browser started. Witness id: none.
- E3 UNWITNESSED: same lease reason.
- Executor self-check (not a witness): visible-above-bar 0.702 at 844x390 (was 0.691), rightMaxH 249px. Its dispatch 1793529607239-1550-62e342443ad094d0.

status: unwitnessed (browser lease held by another lane). Re-run E2/E3 against baa9778d when the lease frees.