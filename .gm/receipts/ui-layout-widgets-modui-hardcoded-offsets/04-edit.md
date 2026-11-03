# 04-edit: ui-layout-widgets-modui-hardcoded-offsets
session: gm-drain-1793526214-17

base: origin/main 226a623a (the shared checkout is dirty and diverged with other lanes' edits; the edit was made on the remote blobs and published through the Git API, the working tree untouched).
open unknowns before edit: none (mutable-list pending for ui-offsets-*: 0).

changed paths (commit baa9778da21837d6cd91d28b5f414f19626008ee, parent 226a623a, lanmower):
- js/client/widgets.js: the #ui-left and #ui-right max-height clamps move out of the (max-width: 600px) block into @media (max-width: 600px), (max-height: 500px) and (orientation: landscape) (same condition as play.html's phone block); var(--bar-h,160px) and var(--right-top,8px) fallbacks removed from the clamps and #ui-modal.
- js/client/modui.js: var(--bar-h, 160px) and var(--bar-h, 120px) fallbacks removed (lines 8, 15, 16).
callers: ensureStyle has one caller, createWidgets (widgets.js:531 at 032cc2db); the STYLE strings are module constants with no symbol callers beyond their module. No signatures changed.

self-check (not the witness; the verifier lane witnesses the pushed sha): git archive of c75eb8753c42 (the same tree content as baa9778d) on a local server, widgets mounted via createWidgets, chrome dispatches 1793529607239-1550-62e342443ad094d0 (capture 1793529607239-1550-*): 844x390 natural rightMaxH 249px (was none), bottom 127; with a 900px stress pad rightBottom 284 (was 1033), visibleAboveBar 0.702 (was 0.691); 390x844 stress maxH 468px, bottom 714, unchanged.
publish: Git API, blobs inline in trees POST base_tree 226a623a; commit baa9778da218 parent 226a623a; PATCH refs/heads/main force false -> ref at baa9778da218; contents ?ref=main match both files byte for byte.
ci: ci-status baa9778da218 run 41298517643 pages pending (2 jobs) at the time of this receipt.

handoff: verifier (mode=witness) and CI lane are spawned by the orchestrator per gm-orchestrate. Witness spec for the verifier on baa9778d: git archive of that sha, 844x390 and 390x844, cold start once; expect landscape rightMaxH 249px under the stress pad, rightBottom <= barTop 294, visibleAboveBar > 0.5; portrait rightMaxH 468px; grep of the two files for bar-h fallbacks returns none.

open items for the orchestrator (not this row's edit): (1) the shared working tree has uncommitted play.js edits with a TDZ error (bootRelays read at play.js:2054 before its const at 2948) that breaks the served dirty tree; the owning lane must see it. (2) play.html's landscape --bar-h rule is max-height 500px, while the parent row text says 480px; the cascade still gives 96px at 844x390.