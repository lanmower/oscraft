# 04-edit: ui-phone-shop-panel-clipped-left-edge

session: gm-drain-1793526214-16
result: no edit made. gm-edit step 1 refused the start: unknown phone-shop-clip-rect-left-baseline is still open (deferred by unknown-defer, dispatch 1793529086266-4881-291cd16e52558e1).

## why it stays open
The only probe that answers it is the in-match top-strip Shop at 390x844 on the real Lua shop tree. No harness in the repo drives that (codesearch of scripts/perf for Shop and data-id: zero matches, and the match flow is not reachable from a bare page). The synthetic tree reproduces the mechanism but not the reported clip (panel 178px inside a 150px column, left 208).

## proposed edit, not applied
Path: js/client/play.html, phone block (upstream main 2890ae47 lines ~140-144, the `#ui-right { right: 4px; }` rule inside `@media (max-width: 600px), (max-height: 500px) and (orientation: landscape)`).
Add: `#ui-right > * { max-width: 100%; }`
Measured in the synthetic harness (dispatches in 03-eliminate.md): baseline panel 208-386 (overflows left); with the cap panel 236-386, card 247-287, inside at 390.
Local checkout is behind origin/main, and js/client/hud.js is dirty from another lane. Apply on the upstream state through the Git API route, not by editing the shared checkout.
