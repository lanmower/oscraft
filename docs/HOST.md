# Browser host

The game is a static page. `js/client/play.html` loads committed ES modules through a committed import map; nothing is bundled or transpiled on the load path. The page's wasm package and map images are built by the Pages workflow, not committed. The page that hosts a room runs the deterministic sim (the wasm-bindgen `Game` built from `rust/crates/web`) and sends its state to the other pages over wireweave: WebRTC data channels, with Nostr relays carrying signalling and room announcements. There is no server process.

## Files

| File | Role |
| --- | --- |
| `js/client/play.html` | The page. Its import map maps `wireweave` to the vendored copy in `js/client/vendor/wireweave/src/index.js`, and `nostr-tools` 2.10.0 and `xstate` 5.19.0 to jsDelivr. `<body data-pkg="pkg/web.js">` names the sim. |
| `js/client/play.js` | The page's main module: the WebGL2 view, the sim loop, the host loop, room join, peer frames and the orders. |
| `js/client/rooms.js` | The room list: renders the rooms announced on the relays and joins one. |
| `js/client/ice.js` | `iceServersFrom`: the ICE server list, from `?ice=` or the default. |
| `js/client/delta.js` | The state encoder (STATE or DELTA per tick) and the decoder for the client. |
| `js/client/pack_load.js` | Loads the pack (`?pack=`) from a directory or a `.tar.gz` or `.zip` archive. |
| `js/client/pack_sync.js` | Live pack propagation: PACK_FILE and PACK_WANT frames. |
| `js/client/swap_protocol.js` | The data swap rules (announce, ready, cancel), shared with `js/wire/test.js`. |
| `js/client/wire/frame.js` | The wire codec, a copy of `js/wire/frame.js` ([WIRE.md](WIRE.md)). |
| `js/client/pkg/` | The wasm-bindgen output of `rust/crates/web` (`web.js`, `web_bg.wasm`). Built, never committed (see below). |

## Query parameters

- `#<room>`: the room name, default `lobby`.
- `host=1`: this page owns the sim and hosts the room.
- `relay=<ws-url>[,<ws-url>]`: the relays. Without it the page uses wireweave's default relays with its fallbacks. With it, the page uses only the listed relays and no fallbacks.
- `ns=<name>`: the namespace for room announcements and sessions, default `oscraft`.
- `ice=<JSON list>`: the ICE servers. With `relay` set and no `ice`, the page passes no ICE servers.
- `pack=<path or archive>` (or `package=`): the pack to load, default `scenarios/moba`. A directory or a `.tar.gz` or `.zip` archive, relative to the site root.
- `shadows=0`: turns the sun shadow pass off.
- `events=<base URL>`: opt-in poll of `GET <base>/events` for mod UI events. No server in this repo serves it, so leave it unset.
- `ci=1`: also exposes the `window.__ci*` test hooks. No script in the repo reads them.

The live debug API on `window.oscraft` is listed in [DEVELOPMENT.md](DEVELOPMENT.md).

## Hosting and joining

- The host steps the sim at 16 ticks per second (`SIM_TICKS_PER_SECOND`), driven by a 10 ms poll that accumulates elapsed time. Each tick it encodes the units into one frame: STATE every 20 ticks, DELTA otherwise, and it broadcasts the bytes to every open data channel. A ROSTER frame goes to each peer every 10 ticks.
- A peer that joins takes the lowest free seat (`MAX_PLAYERS` is 10). A peer with no free seat is a spectator. A reconnecting peer gets its seat back by its key.
- The host applies an ORDER, CAST, ATTACK or other unit frame only when the unit belongs to the sender's player. Otherwise the status line reads `order refused not_owner peer=<key> unit=<id>` and `body.dataset.lastRefusal` is set to the same reason.
- A room is announced every 15 s as a parameterized replaceable event (kind 30078, tag `t=oscraft-room`, with the namespace and room tags). An announcement expires after three missed announcements (45 s), and the room list is pruned every 5 s. Room names match `^[A-Za-z0-9_-]{1,64}$`.
- A dropped connection retries with backoff from 1 s, capped at 16 s, for up to 6 attempts.

## Status attributes

The page sets these on `<body>`, for the browser tests and for debugging:

- `data-ready`: set to `1` once the sim is loaded.
- `data-units`, `data-ticks`: the unit count and the tick.
- `data-center`: the camera target as `x,z`.
- `data-error`: the message of a failure that stopped the page.
- `data-last-refusal`: the reason of the last refused command.

## Loading and deployment

- The page is buildless: `play.html` loads `play.js` directly as a module. There is no `dist/` bundle.
- The wasm package and the map images are not in git. `.github/workflows/pages.yml` builds them from source on each push to `main` (`wasm-pack build crates/web --target web --out-dir site/pkg` and `mapexport` for each pack), lays out the site, and deploys it to GitHub Pages.
- The site root is the Pages URL. `index.html` redirects to `js/client/play.html`.
- The page loads the pack's map from its manifest's `image` (`scenarios/moba/moba.osdt`, built by `mapexport`).
- The same workflow packs the moba pack into `packs/<name>-<version>.tar.gz` with a `packs/latest.json` that names it.
- After the deploy, `scripts/observe/compare.mjs` fetches the live page and compares it with the built tree (`--expect-sha`).

## Node guest

`js/host/guest.js` is a standalone Node script that decodes ORDER frames for 1 to 32 simulated peers, to check the codec's per-tick cost. It is not a host and no CI job runs it. Run it by hand with `node js/host/guest.js`.
