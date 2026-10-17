# Browser host

The game is a static page. `js/client/play.html` loads committed ES modules and committed wasm.
Nothing is bundled, transpiled or installed on the load path. The page runs the deterministic sim
(the wasm-bindgen `Game` in `rust/crates/web`) in the browser and networks peers over wireweave:
WebRTC data channels, with Nostr relays carrying signalling and room announcements.

## Files

| File | Role |
| --- | --- |
| `js/client/play.html` | The page. Its import map pins `wireweave` (GitHub commit on jsDelivr), `nostr-tools` 2.10.0 and `xstate` 5.19.0. It loads `rooms.js` and `play.js` as modules. `data-pkg="pkg/web.js"` and `data-map="map.bin"` name the sim and the map. |
| `js/client/play.js` | WebGL2 map and unit view, the sim loop, room join, the host loop and the orders. |
| `js/client/rooms.js` | Room panel: lists the rooms announced on the relay and joins one. |
| `js/client/ice.js` | ICE server list. `?ice=` replaces the default STUN list. |
| `js/client/delta.js` | State and delta frame encoder and decoder. |
| `js/client/wire/frame.js` | Wire frame codec. A copy of `js/wire/frame.js`; CI compares the two. |
| `js/client/pkg/` | wasm-bindgen output of `rust/crates/web` (`web.js`, `web_bg.wasm`), committed from the CI build artifacts. |
| `js/client/map.bin` | Map image written by `mapexport`, committed from the CI build artifacts. |

## Query parameters

- `#<room>`: the room, default `lobby`.
- `relay=<ws-url>[,<ws-url>]`: the relays. Without it the page uses wireweave's default relays.
- `ns=<name>`: the namespace, default `oscraft`.
- `host=1`: this page owns the sim and hosts the room.
- `ice=<JSON list>`: ICE servers. With `relay` set and no `ice`, no ICE servers are used.
- `ci=1`: exposes `window.__ciPeers`, `__ciUnits`, `__ciOrder` and `__ciDebug` for the CI probes.

## Hosting and joining

- The host ticks the sim every 100 ms (`HOST_TICK_MS`) and sends each tick's state through the delta encoder to every open data channel.
- Each peer gets the lowest free player id (up to 256) when its channel opens. The host applies an order only when the unit belongs to the sender's player; otherwise the status line reads `order refused not_owner`.
- A follower applies the frames to its unit table, draws them, and sends its orders as order frames.
- A room is announced every 15 s as a kind 30078 event tagged `oscraft-room`. A room drops out of the list 45 s after its last announcement, and the list is pruned every 5 s.
- A dropped connection retries with backoff from 1 s, capped at 16 s, for up to 6 attempts.

## Status attributes

For headless checks the page sets these on `<body>`: `data-ready` once the sim is loaded, `data-units`, `data-ticks`, `data-center` (the camera centre), and `data-error` on a failure.

## Loading and deployment

- Buildless: `play.html` loads `play.js` directly as a module. The static CI probe fails if the page refers to a `dist/` bundle.
- Third-party modules are fetched from jsDelivr at the versions in the import map.
- GitHub Pages serves the repository as a static site, so with Pages enabled on `main` the page is at `/js/client/play.html`. Pages is configured on the repository and no CI job publishes it yet.

## Verified in CI

| Job | Witness line |
| --- | --- |
| `browser host and browser peer over wireweave` | `browser two peers ok unit=<id> host_moved=true peer_moved=true` |
| `lobby room panel lists two rooms announced on the relay` | `lobby room select ok rooms=2 joined=beta` and `room expiry ok dropped=alpha remaining=beta ttl_s=45` |
| `browser page probes` (map) | `map witness ok ticks=<n> units=<n> center=<x,z> source=js/client/map.bin` |
| `single archive builds and installs` | `package install ok ... client=ok pkg=ok map=ok` |

The jobs use `oscraft-relay`, the test relay in `rust/crates/relay`. It speaks NIP-01 `EVENT`, `REQ` and `CLOSE`, and it is started on port 7447 by the job.
Playwright (Node) drives the headless Chromium in these jobs.

## Retired

The Node host (`js/host`) is removed. `wasip3` is the only guest: CI builds it as a component, and the browser keeps running `rust/crates/web`.
