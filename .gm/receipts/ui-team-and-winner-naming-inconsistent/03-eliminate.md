# 03-eliminate: ui-team-and-winner-naming-inconsistent

session: gm-drain-1793526214-7

## ui-team-win-witness-surface (unknown 3)
expected (contract): a finished match with #win is reachable gpu-free through a mod, or it needs a browser arm.
run: `rust/target/debug/mod_check scenarios/moba .gm/scratch/probe-panels-reach.lua 1` (probe: `require("panels")`, then error with the answer)
observed: `mod-check load-error probe-panels-reach: unknown_module: panels is not a module of this pack`
result: panels.lua is not reachable from a mod, so the ui_call / emit surface cannot be driven gpu-free through mod_check. Answer: browser or a new harness is needed. Status: partly known (the gpu-free route is closed; the browser route is open, see below).
browser: lease held by lane `gm-drain-1793526214-15` (verb chrome, dispatch gm-drain-1793526214-15-824759-1793528212490-1679, budget 600000 ms). Not opened here.
owner constraint (row notes, hud-match-1 2026-10-25T20:48:41Z): no live match is reachable in a browser without Claim, Fill, Start or a room join, all forbidden by the owner; owner must approve a local route.

## ui-team-win-leave-call (unknown 1)
expected (contract): an existing ui call returns to lobby after game end, or none exists.
live run: not possible here (the handler is in panels.lua, unreachable from mod_check, see unknown 3).
code facts (read at dispatch ids 1793528274891-4240-a0bf3a35a383100c, 1793528256109-4225-74824b99a9b3e21f; file reads at panels.lua:855-861, 1428-1483, 1516-1520, 1553):
- ui_call "release" / "claim" / "remove" / "pick" / "ready" / "fill" / "start" / "world_ready" / "mode" all route to `lobby_act` (panels.lua:1477-1478).
- `lobby_act` refuses with `match-started` when `LOBBY.started` (panels.lua:858-861).
- The win button `win_close` only sets `WIN_CLOSED` (panels.lua:1455-1456); it does not call lobby_act.
- The only ui entry is `on_action("ui", ...)` (panels.lua:1516-1519), exported as `{ on_action = panels_on_action }` (:1553).
Status: unknown stays open for a live check; code facts above point to "no return path after start" but are not a run.

## ui-team-win-lobby-after-end-started (new unknown, mutable-add)
question: after game end, is LOBBY.started still true so every lobby_act refuses match-started (panels.lua:858-861)?
live run: not possible here, same reason as unknown 3.

## ui-team-win-button-semantics (unknown 2)
question: should the win button stay win_close (dismiss) or become return/leave?
This is an owner-level decision about the acceptance text, not a probe-able fact. The row acceptance says return/leave, so a return/leave action is the contract; a probe cannot decide its semantics.

## verdict
- Unknown 3 probed live: browser-only route, owner route constraint.
- Unknown 1 and the new unknown: cannot be probed here (panels unreachable from mod_check, browser lease held).
- Unknown 2: owner decision.
Per gm-eliminate step 4, the unprobed unknowns go to a blocker row with these dispatch ids. Stop before gm-edit.
