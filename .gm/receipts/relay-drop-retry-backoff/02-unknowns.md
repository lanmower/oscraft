# 02-unknowns relay-drop-retry-backoff (session gm-drain-1793526214-13)

Open unknowns (mutable-add, row relay-drop-retry-backoff). The orient receipt proves the code exists; these are the facts it does not prove.

- relay-retry-u1-reopen: does retryRelay (push url, wireweave _open) reopen a relay whose handshake failed, and does a later connected status clear the retry state? Live node run of RelayPool against a refused then accepting WebSocket; no browser.
- relay-retry-u2-relays-alive: does the 15000 ms report set body.dataset.relaysAlive and push the connected count into statusLog via telemetry? Live run of the report path; no browser.
- relay-retry-u3-edit-needed: does the row need any edit, or does the existing block already satisfy acceptance? Decides edit handoff vs witness-only handoff.
- relay-retry-u4-browser-fact: which witness_plan fact (headful load, console retry lines, relaysAlive after 40 s) only a browser can supply, versus what gpu-free instrumentation of play.js and RelayPool answers.

Next: gm-eliminate (no unknown is closed here).
