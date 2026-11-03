# 04-edit relay-drop-retry-backoff (session gm-drain-1793526214-13)

Changed paths: none.

Gate: no unknown open (relay-retry-u1..u4 resolved, dispatches 1793530143024-1761-2bf5d49e13165332, 1793530149451-1765-23ccdd4e137e7601, and the mutable-resolve dispatches 1793530154707-1767-3735f5833fb313ce .. 1793530155656-1770-cc99115aca45a257).

Decision: the acceptance clauses are already implemented in js/client/play.js on origin/main (f43b352b and its ancestors). The retry block (RELAY_RETRY_MS 5000, RELAY_RETRY_MAX_MS 120000, RELAY_RETRIES 6, dropRelay, scheduleRelayRetry, retryRelay, relay-status listener, 15 s relaysAlive and telemetry report) was run against the origin blob f43b352b:js/client/play.js (dispatch 1793530180534-1782-1c8bd7fbf040009e) and matched every expected value. No edit is made, so no callers check is needed for a changed symbol.

Handoff: the executor stops here. Verification (PROVE, witness on a headful load with console, relaysAlive after 40 s) and the CI read are separate lanes for the orchestrator to spawn on sha f43b352be4a4511740a840ca10615ef46b153a18. gm-validate is not loaded by the executor (invoke only from the orchestrator).

Working tree: other lanes hold uncommitted edits in js/client (play.js carries bootRelays). This row commits nothing and sweeps none of them.
