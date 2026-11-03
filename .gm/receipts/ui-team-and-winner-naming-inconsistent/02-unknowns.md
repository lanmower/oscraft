# 02-unknowns: ui-team-and-winner-naming-inconsistent

session: gm-drain-1793526214-7

Open unknowns (mutable-add, status unknown):
1. `ui-team-win-leave-call`: which existing ui call returns a seat or the page to lobby (lobby `release` at panels.lua:1256, leave.lua `on_action` at :81), and is it valid after game end?
2. `ui-team-win-button-semantics`: should the win button stay `win_close` (dismiss, panels.lua:272/1455) or become a return/leave action? Acceptance says return/leave.
3. `ui-team-win-witness-surface`: can a finished match (#win with team name and button) be reached gpu-free through the module harness, or does it need a browser arm, and which fact only a browser supplies?

Clause 1 (one team-name source) is met on the tree (orient receipt), so no unknown is open for it.
