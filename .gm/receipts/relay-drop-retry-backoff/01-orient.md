# 01-orient relay-drop-retry-backoff (session gm-drain-1793526214-13)

acceptance (row text, not derived): A relay that fails its handshake is retried with growing delay; a connected relay rejoins; after load the page reports the connected count.

## Row facts
- Row file .gm/prd/relay-drop-retry-backoff.yml: status pending, owner/session hud-start-1, witness_plan = headful load, console on, read retry lines and body.dataset.relaysAlive after 40 s, from a session other than the resolver.
- Row note: publish blocked at origin fetch on hud-start-1 (old observation; origin is now reachable, codeinsight fetched_ago_s under 60).

## Code already present (js/client/play.js, working tree at query time; line numbers shift because another lane has uncommitted edits in js/client)
- RELAY_RETRY_MS = 5000, RELAY_RETRY_MAX_MS = 120000, RELAY_RETRIES = 6 (about play.js:2051-2053).
- dropRelay(url, reason) (play.js:2060): removes url from relayPool.urls and relays, closes ws, clears _reconnectTimers, console.warn("relay ... dropped from the pool: ..."), calls scheduleRelayRetry.
- scheduleRelayRetry(url) (play.js:2077): delay = min(5000 * 2^tries, 120000); gives up with telemetry after 6 tries; telemetry line per retry.
- retryRelay(url) (play.js:2088): skips if established or already in urls; pushes url and calls relayPool._open(url).
- relay-status listener (about play.js:2094): connected adds to establishedRelays and clears retries and timers; error or closed without establishment schedules dropRelay(url, "its handshake failed") on a 0 ms timeout.
- After 15000 ms (about play.js:2107): body.dataset.relaysAlive = count of status connected; telemetry "relays: N of M connected after load; K retrying".
- statusLog: telemetry (play.js:225) pushes into statusLog (cap STATUS_LOG_MAX = 200, play.js:199); statusLog is exposed by the debug API (play.js ~3242).

## Symbols (codeinsight)
- dropRelay: defined play.js:2060-2076; one caller, play.js <module> relay-status setTimeout (edge unqualified-name). impact (callees, depth 2): no remote callers.
- scheduleRelayRetry and retryRelay: called only inside this block (codesearch).

## Wireweave API the retry relies on (js/client/vendor/wireweave/src/relay-pool.js)
- class RelayPool extends EventTarget (line 139); urls (142), relays Map (143), _reconnectTimers Map (148), _open(url) (285), relay._connectTimer (306).
- Emits relay-status with {url, status} for connected (313), error (300, 329) and closed (333). The page listener reads detail.url and detail.status: matches.

## Commit evidence
- origin/main tip f43b352b (ui: name wasm, world assets and relays in the loading overlay; row ui-loading-experience-text-only) touches play.js only to add bootRelays and the overlay relay item; its diff shows RELAY_RETRY_MS and the retry block as unchanged context, so the retry predates it.
- codeinsight stale_tree: HEAD e56dd0b6 is 1 ahead, 9 behind upstream f43b352b; js/client is 4 commits behind.
- git_status: js/client has 3 modified files and js/client/vendor 1 from other lanes; not this row's paths; must not be swept into any commit.

## Unknowns handed to gm-unknowns
- U1: does retryRelay + wireweave _open reopen a relay whose handshake failed (live run, no browser needed for the API half).
- U2: does the 15 s report set body.dataset.relaysAlive and statusLog with the connected count (live run).
- U3: does the row need any edit, or does the existing code satisfy acceptance (decides handoff with an edit sha vs a witness-only handoff).
- U4: which fact only a browser can supply (witness_plan says headful); state it or use gpu-free instrumentation.

## Process note
- One read-only codesearch in this session was dispatched with session_id gm-drain-1793526214-14 (another worker's id) by mistake; no writes were made under that id.
