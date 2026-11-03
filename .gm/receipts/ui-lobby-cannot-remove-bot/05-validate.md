# 05-validate: ui-lobby-cannot-remove-bot (session gm-drain-1793526214-9)

expected per acceptance clause, written before the runs:
- A1 host-only remove: a `remove` branch in lobby_act that refuses a non-host. expected present.
- A2 not-host refusal for others: `L.host ~= owner` refuses with "not-host" inside the remove branch. expected present.
- A3 witness: fill, remove one bot, seat shows open. expected a live run reporting seat open.

results:
- A1 PASS: codesearch literal `elseif call == "remove" then` -> panels.lua:941 (dispatch 1793528146807-4091-3cf4c195f6872c86, from 03-eliminate).
- A2 PASS: codesearch literal `if L.host ~= owner then L.error = "not-host" return end` -> panels.lua:942 is the remove branch's line (dispatch 1793528165832-4113-1cb932350793e4e8). Lines 866, 881, 899 hold the same refusal in other branches.
- slot-empty guard: panels.lua:943 (dispatch 1793528166476-4114-24fe88512201c2fa).
- A3 NOT RUN: no live driver sends ui `remove` (dispatches 1793528108493-4044-c4988611cdcaa4c8, 1793528109741-4047-b916891cfd4e80be). A witness needs a new driver, which is a test file this lane may not create. No witness_dispatch_id exists, so no verify.

outcome: not verified. The lane cannot return `STATUS: verified`, and no fix to the code will produce a witness. Blocker filed on the row with blocked_by naming the missing witness instrument.
