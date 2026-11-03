# 05-validate: relay-drop-retry-backoff

session: gm-drain-1793526214-v6 (witness lane; executor session not used)
sha: f43b352be4a4511740a840ca10615ef46b153a18 (origin/main tip per executor; git_show dispatch 1793530263252-1851-2df5d20d83d174b8)

## Expected per acceptance clause
1. Handshake-failed relay retried with growing delay: retry delays 5 s doubling to 120 s over 6 attempts, then give up.
2. Connected relay rejoins the pool: fail-once relay reconnects and page reports relaysAlive "1".
3. After load the page reports connected count: body.dataset.relaysAlive and statusLog after 40 s on the real page at the sha.

## Observed
- Browser lease held by browser-918924-48 (pid 918924, port 43651, heartbeat_age_ms 179, live true). Tab: localhost:8731 play.html?host=1&ci=1&nocache=gmv3#moba. Dispatch 1793530249535-3591-825d55f9636ec983.
- Headful witness not started: lease held by another lane; no second browser started.
- Clauses 1-3 not observed in this lane. Executor evidence (not witnessed here): always-fail delays 5 s to 120 s over 6 attempts; fail-once relaysAlive "1" (dispatches 1793530143024-1761-2bf5d49e13165332, 1793530180534-1782-1c8bd7fbf040009e).

## Verdict
unwitnessed: browser lease held; live witness pending. Re-dispatch this lane once the lease is released.

## Notes
- git_show on f43b352 shows commit subject for row ui-loading-experience-text-only (Witness 1793529734589-1608-3d3b9dfbf32dfac0); the relay row's acceptance is met by code already on that main tip per executor.
- prd-list shows relay-drop-retry-backoff open with a publish blocker note from hud-start-1 (publish_on_origin fetch stall); not evaluated here.