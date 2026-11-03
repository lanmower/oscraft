# 03-eliminate: ui-layout-widgets-modui-hardcoded-offsets
session: gm-drain-1793526214-17

base: origin/main 92633ddc (the local checkout is dirty with other lanes' edits in play.html, play.js, widgets.js and diverged; never edited for this row). Live page served from git archive of 92633ddc at /tmp/gm17-base-92633ddc2d76 on 127.0.0.1:8792 (not the dirty tree).

unknowns resolved (expected written before each run):
- ui-offsets-fallbacks-are-only-px-left. expected: exactly the six var() fallback lines. observed: modui.js:8,15,16 and widgets.js:107,108,115 (dispatch 1793528293359-4262-2e224087acdc4afd). known.
- ui-offsets-bar-h-defined-everywhere-widgets-renders. expected: --bar-h set in the unwrapped base :root. observed: play.html:82-83 :root block, no @media wrapper (same dispatch). known.
- ui-offsets-other-importers-of-widgets-modui. expected: play.html graph only. observed: widgets=[play.js], modui=[pack_editor.js, play.js] (dispatches 1793528240143-4217-4371b8d00161fac7, 1793528240832-4218-7f24e8ce0a7e9ba); only play.html among html pages. known.
- ui-offsets-844x390-map-visible-above-half. expected: above 0.5. observed (chrome dispatch 1793529504120-1513-7ab4923be0847f5 via capture 1793529503316-1509-bb14a87cd54d9a29): 844x390 bar 96px, barTop 294, visibleAboveBar 0.839 natural; 0.691 with a 900px pad in #ui-right (the number moves). known: passes, headroom finite.
- ui-offsets-844x390-widgets-clamp-missing. expected: clamp present at landscape if the row's intent holds. observed: rightMaxH none at 844x390 (stress), #ui-right bottom 1033 vs viewport 390. 390x844 rightMaxH 468px, bottom 714. widgets.js:106 clamp is max-width 600 only. known: confirmed defect.

instrument: mount = real widgets.js createWidgets({send}) via dynamic import (styleMounted true). measurement = elementsFromPoint-free rect sampling of the area above the bar against #ui-left, #ui-right, #ui-top, #ui-modal, .modui-panels, .modui-log. The #boot overlay and the map canvas are absent (no match offline), so the metric measures panel coverage only, labelled as such.
cold starts: 2 page loads (archive base, same tab reused for arm 2 via url query r=2). reloads: 0 within an arm.
lease: browser_lease_acquire then release then chrome (chrome waited on the lease when held).

open unknowns: none. Next: gm-edit (remove the six fallbacks in widgets.js and modui.js; add the landscape clamp to widgets.js:106-116 by the same media condition play.html uses for the phone block, max-height 500px and landscape).
side finding (not this row): the dirty local play.js holds an uncommitted TDZ ReferenceError (bootRelays used before its const at play.js:2948) that breaks the lobby of the served dirty tree; reported to the lane owning those edits by the witness log, not fixed here.