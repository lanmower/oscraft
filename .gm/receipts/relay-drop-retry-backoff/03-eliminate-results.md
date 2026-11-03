# 03-eliminate results relay-drop-retry-backoff (session gm-drain-1793526214-13)

Expected values were written to 03-eliminate.md before the run. Observed values match on every field.

## Live JIT dispatches
- 1793530143024-1761-2bf5d49e13165332: real play.js relay block (from "const relayPool = new RelayPool({" to "const ROOM_KIND") over the real wireweave RelayPool, fake WebSocket, setTimeout scaled 1000x.
  - Run A (always fail): retryDelays [5000,10000,20000,40000,80000,120000] ok; urls [] ok; established [] ok; retryTimersLeft 0 ok; socketsOpened 7 ok; relaysAlive "0" ok; telemetry "gave up after 6 retries" ok.
  - Run B (fail once, then open): retryDelays [5000] ok; urls [wss://relay-a.test] ok; established [wss://relay-a.test] ok; retryTimersLeft 0 ok; socketsOpened 2 ok; relaysAlive "1" ok; telemetry "relays: 1 of 1 connected after load; 0 retrying" ok.
- 1793530149451-1765-23ccdd4e137e7601: real telemetry function from play.js: 2 lines pushed to statusLog, cap 200 ok.

## Unknowns
- relay-retry-u1-reopen: known (dispatch 1793530143024-1761-2bf5d49e13165332).
- relay-retry-u2-relays-alive: known (dispatches above).
- relay-retry-u3-edit-needed: known; no code edit is required for acceptance.
- relay-retry-u4-browser-fact: known; only real WebSocket event ordering and live reachability need a browser, which is the verifier lane's headful witness.

## Caveats
- The harness fakes the WebSocket, so the check covers the page's retry, rejoin and report logic against the real wireweave RelayPool API, not the real socket event timing.
- Relays in the working tree carry another lane's uncommitted play.js edits (bootRelays). The harness supplies bootRelays as a binding because the slice starts after its declaration.

Next: gm-edit (no edit needed for acceptance; see U3).
