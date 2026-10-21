# Local development loop

Edit, stage, reload. No GitHub Pages deploy per change.

## Stage

    scripts/dev/stage.sh

Builds the mapexport maps and, when the wasm pkg is missing (or `WASM=1`), runs wasm-pack. It lays the site out at `.dev-site/` exactly as `.github/workflows/pages.yml` does (`js/client`, `js/client/pkg`, `map.bin`, `moba.bin`, `hero_rules.txt`, `models/*.glb`, `mods/`). Rerun after editing Rust or scenario data; plain JS, HTML and CSS edits only need a reload of the staged copy, so re-stage or copy `js/client` into `.dev-site/js/client`.

Needs cargo, and wasm-pack plus the `wasm32-unknown-unknown` target for the pkg step.

## Serve

    node scripts/dev/serve.mjs

Serves `.dev-site` on `http://localhost:8790` (127.0.0.1 only, no-store headers, built-in Node modules only). Stop it with Ctrl-C.

## Crawl the lobby (engine=cdp)

With `scripts/dev/serve.mjs` running, dispatch a `crawl` body (the cdp engine drives a headful Chrome, one window per project root):

    engine=cdp
    session=dev
    http://localhost:8790/js/client/play.html#moba
    wait=6000
    console

The `console` step reports messages captured by the crawl session, and an empty list means no errors were seen after navigation.

## Fast path

- JS, HTML or CSS only: edit, copy `js/client` into `.dev-site/js/client` and reload. No cargo run is needed, and `stage.sh` is not needed.
- Rust, mods or scenario data: run `scripts/dev/stage.sh` (it reruns mapexport; wasm-pack runs only when the pkg is missing or `WASM=1`), then reload.
- One Rust test: `cargo test -p <crate> <filter>` inside `rust/`, with `<crate>` from `rust/crates/`.
- CI on a push runs every job except when every changed file is a root-level `*.md`. A `docs/` change still runs every job, because `static-probes` and the log-schema check read `docs/`.
- The slowest CI path is the browser probes (`browser-direct`, `browser-two-peers`, `browser-probes`). Their Chromium install is cached per Playwright version and runs once per job. Check a probe's own step time in `gh run view --json jobs` before changing its waits.
- The parity gate (`oscraft-gate`) runs once per OS in `check`, `check-linux`; `duel-routes` reads the `check-linux` output, not a second run.

## Live debugging

Every page load exposes `window.oscraft` (no flag, no per-frame cost). Open `js/client/play.html#moba` in a browser and use the console. Camera `x` and `y` are world x and z (ground coordinates). `oscraft.help()` prints the list.

    oscraft.setCamera({ x: 44, y: 48, distance: 46, yaw: 0.6, pitch: 0.9 })   // clamped by the mouse's own limits
    oscraft.getCamera()
    const controlled = oscraft.getState().controlled;                                    // tick, units, controlled, frame counters, terrain mode
    oscraft.selectUnit(hero.id)
    oscraft.issueOrder(hero.id, 'move', hero.x - 3, hero.z)                  // 'move', 'stop', or 'attack' at (x, y)
    oscraft.centreOn(hero.id)
    oscraft.setShadows(false); oscraft.setFog(false)                         // fog is render-side only
    oscraft.setTerrain('tiles'); oscraft.setRenderer('tsl')                  // reload with ?terrain= / ?renderer=

Orders go through the same command path the mouse takes, so the sim sees a real order and its refusal lands in `document.body.dataset.lastRefusal`. `window.__ci*` probes still need `?ci=1`.
