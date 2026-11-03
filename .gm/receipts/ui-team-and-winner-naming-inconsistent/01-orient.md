# 01-orient: ui-team-and-winner-naming-inconsistent

session: gm-drain-1793526214-7
HEAD / origin/main: 032cc2db (clean for the cited paths)

## acceptance (from row, not derived)
One team-name source (UI.text) used by all three sites; win panel names the team and offers a return/leave button via a ui call. Witness: finish match, #win shows the lobby team name and a button.

## facts on the tree
- team source: `scenarios/moba/panels.lua:155-157` `team_name(side)` returns `TEXT.lobby.teamNames[side + 1]`, fallback `TEXT.teamLabel .. " " .. side`.
- lobby: `panels.lua:1307` `TEXT.lobby.teamNames[t + 1]`; `:1355` joins the same list. Lobby reads the table directly, not through `team_name`; same source.
- scoreboard: `panels.lua:772` `label(team_name(side), "team"..side)`.
- win: `panels.lua:261-274` `win_tree()`. Line 264 `team_name(side) .. " " .. TEXT.winner.suffix`. Line 267 prints `"P" .. row.player` (raw player id on the end screen). Line 272 `button(TEXT.menu.close, "win_close", "{}")`.
- win gate: `panels.lua:1400` `emit("win", "win", (not WIN_CLOSED) and win_tree() or nil)`; `:32` `WIN_CLOSED = false`; `:1455-1456` `win_close` handler sets `WIN_CLOSED = true`.
- lobby leave is a separate control: `panels.lua:1256` `button(TEXT.lobby.leave, "release", ...)`; `scenarios/moba/leave.lua:81` `on_action`.
- text: `scenarios/moba/moba_ui.json:136` `winner`; `:147` `teamLabel` ("Team"); `:235-237` `lobby.teamNames` ["The Sentinel", "The Scourge"].
- no `back_to_lobby` in `scenarios/` (scan partial, zero not a finding); no `win_close` in `js/` (partial scan).

## cite drift
Row cites `panels.lua:269`, `:1023,1074`. Current lines: win_tree 261-274 (prefix 264), lobby teamNames 1307 and 1355. Cites are stale; facts above are current.

## commit state
Row notes cite `ae39ed17` and `b2682bdd`. Both objects exist. `b2682bdd` is not an ancestor of HEAD 032cc2db; the one-source change is present in the tree anyway (`team_name` reads `TEXT.lobby.teamNames`, `moba_ui.json` lobby.teamNames present, `win_close` button present).

## status against acceptance
- clause 1 (one source, all three sites): met on tree at 155-157, 264, 772, 1307.
- clause 2a (win panel names the team): met at 264.
- clause 2b (return/leave button via a ui call): open. Only `win_close` (Close, dismiss) exists; it hides the panel and does not leave or return to lobby. No return-to-lobby ui call exists.
- witness (#win shows lobby name and a button in a finished match): not yet run; prior notes say no live match reachable without Claim/Fill/Start.
- related symptom: raw `P<id>` on the end screen (`panels.lua:267`), not in the acceptance clause.

## unknowns for gm-unknowns
1. Which existing ui call (lobby `release`, leave.lua `on_action`) returns a seat to lobby, and whether it is valid after game end.
2. Whether the win button should be Close, a leave, or both (acceptance says return/leave).
3. How to reach a finished match in a browser or a gpu-free instrument (witness surface).
