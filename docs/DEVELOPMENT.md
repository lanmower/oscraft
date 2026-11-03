# Local development loop

Edit, stage, reload. No GitHub Pages deploy per change.

## Stage

    scripts/dev/stage.sh

Runs `mapexport` on the `moba` and `duel-routes` packs, and runs wasm-pack when the package is missing or `WASM=1` is set. It lays the site out at `.dev-site/` the way `.github/workflows/pages.yml` does: `js/client` (with `js/client/pkg`), `scenarios/` (with each pack's `.osdt` image and the moba models), `mods/`, `version.json` and an `index.html` that redirects to `js/client/play.html`.

Needs cargo, and wasm-pack plus the `wasm32-unknown-unknown` target for the pkg step.

## Serve

    node scripts/dev/serve.mjs

Serves `.dev-site` on `http://localhost:8790` (127.0.0.1 only, no-store headers, built-in Node modules only). Pass a directory to serve another staged site. Stop it with Ctrl-C.

## Watch JS, HTML and CSS

    node scripts/dev/hot-sync.mjs .dev-site

Copies `js/client` and `scenarios/moba` into the target directory as they change, so a reload shows the edit. The wasm package and the map images are not rebuilt; rebuild them with `stage.sh` when `rust/` or the scenario data changes. Without an argument the target is `../oscraft-site`.

## Fast path

- JS, HTML or CSS only: edit, then reload. `hot-sync.mjs` copies the change, or copy `js/client` into `.dev-site/js/client` by hand. No cargo run is needed.
- Rust, mods or scenario data: run `scripts/dev/stage.sh` (it reruns mapexport; wasm-pack runs only when the pkg is missing or `WASM=1`), then reload.
- One Rust test: `cargo test -p <package> <filter>` inside `rust/`, with `<package>` the package name in `rust/crates/*/Cargo.toml` (for example `sim`, `mod-host` or `osdt-data`).
- CI runs on every push to `main` and on demand, with two jobs: `rust` (`cargo test --workspace` in `rust/`) and `wire` (`node test.js` in `js/wire`). `.github/workflows/pages.yml` builds and deploys the site on each push to `main`. See [HOST.md](HOST.md#loading-and-deployment).

## Live debugging

Every page load exposes `window.oscraft` (no flag, no per-frame cost). Open `js/client/play.html#moba` in a browser and use the console. Check the console for errors after navigation. Camera `x` and `y` are world x and z (ground coordinates). `oscraft.help()` prints the list.

    oscraft.setCamera({ x: 44, y: 48, distance: 46, yaw: 0.6, pitch: 0.9 })   // clamped by the mouse's own limits
    oscraft.getCamera()
    const hero = oscraft.getState().hero;                                    // tick, units, hero, frame counters, terrain mode
    oscraft.selectUnit(hero.id)
    oscraft.issueOrder(hero.id, 'move', hero.x - 3, hero.z)                  // 'move', 'stop', or 'attack' at (x, y)
    oscraft.centreOn(hero.id)
    oscraft.setShadows(false); oscraft.setFog(false)                         // fog is render-side only
    oscraft.debug.run('view', { fog: false })                                // a pack action, as the page dispatches it

Orders go through the same command path the mouse takes, so the sim sees a real order and its refusal lands in `document.body.dataset.lastRefusal`. The `window.__ci*` probes need `?ci=1`. The pack's Lua state is under `oscraft.debug` ([MODDING.md](MODDING.md#debugging)).
