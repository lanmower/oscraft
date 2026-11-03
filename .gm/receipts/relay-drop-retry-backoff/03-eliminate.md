# 03-eliminate relay-drop-retry-backoff (session gm-drain-1793526214-13)

Expected values written before the run. Derived from the row contract (5 s doubling to 120 s, 6 attempts; connected relay rejoins; report after load), not from reading the code.

## Run A: relay that always fails (unknown relay-retry-u1-reopen)
- expected retry delays requested: [5000, 10000, 20000, 40000, 80000, 120000] (6 entries)
- expected telemetry: six "retry N of 6" lines and one "gave up after 6 retries"
- expected urls at end: [] (dropped, never re-added after the 6th failure)
- expected relays established: []
- expected retry timers left: 0
- expected fake sockets opened: 7 (initial + 6 retries)
- expected body.dataset.relaysAlive at the 15 s report: "0"

## Run B: relay that fails once then opens (unknown relay-retry-u1-reopen and relay-retry-u2-relays-alive)
- expected retry delays requested: [5000]
- expected urls at end: [wss://relay-a.test] (rejoined)
- expected relays established: [wss://relay-a.test]
- expected retry timers left: 0
- expected fake sockets opened: 2
- expected body.dataset.relaysAlive at the 15 s report: "1"
- expected telemetry contains "relays: 1 of 1 connected after load; 0 retrying"

Method: the real play.js text from "const relayPool = new RelayPool({" to "const ROOM_KIND" is evaluated with the real wireweave RelayPool (js/client/vendor/wireweave/src/relay-pool.js), a fake WebSocket that fails or opens per attempt, and setTimeout scaled 1000x (requested delays recorded unscaled). No repo file is created; the check is inline.
