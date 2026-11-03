# 03-eliminate: ui-lobby-cannot-remove-bot (session gm-drain-1793526214-9)

u1 (origin/main carries the host-only remove branch and Remove button)
- expected (from row contract): a `remove` branch refusing non-hosts, and a Remove button for the host on other seats.
- run: codesearch literal `elseif call == "remove" then` in scenarios/moba, dispatch 1793528146807-4091-3cf4c195f6872c86: panels.lua:941 (1 match).
- run: codesearch literal `elseif LOBBY.host == OWNER then` in scenarios/moba, dispatch 1793528147404-4092-43085ad4b31426ab: panels.lua:1257 (1 match); button on 1258.
- observed: matches expected. Known. Tree = origin/main b8897bf5 (git show, Bash).

u2 (a live gpu-free instrument dispatches ui `remove` on a filled seat)
- expected: one driver under rust/ sends call "remove".
- run: codesearch literal `dispatch_action("ui"` in rust, dispatch 1793528108493-4044-c4988611cdcaa4c8: 14 hits; the drivers send claim, world_ready, pick, start, not remove.
- run: codesearch literal `"remove"` in rust/gate, dispatch 1793528109741-4047-b916891cfd4e80be: 2 hits, unrelated.
- observed: no driver sends remove. Unknown is answered: no instrument exists.
- check cannot run here: minting a driver is a test file (gm §3; operator rule). Exact reason recorded; blocker follows.

u3 (CI green for the fix tree)
- expected: success on the sha holding the fix.
- run: ci-status b8897bf5, dispatch 1793528081813-4016-4414cae058bfc1c0: run 41296428793 success 3/3, pending 0.
- observed: matches expected. Known.

Open unknowns: none. Unknown u2 is answered but leaves the witness without an instrument, so the row cannot close this session. Witness blocker to be recorded on the row.
