# 01-orient: ui-lobby-cannot-remove-bot (session gm-drain-1793526214-9)

acceptance (row text): host-only `remove` on a bot or other seat, with not-host refusal for others. Witness: fill, remove one bot, seat shows open.

row read: prd-list {"id"} dispatch 1793527792339-3755-d01f000b91d25e35.

codeinsight:
- lobby_seat: defined scenarios/moba/panels.lua:1236-1261; callers 0 (dispatch 1793527803660-3769-723f5e2e454afea6).
- release: ambiguous (bottle.lua:123, store.lua:142); not usable (dispatch 1793527813591-3778-4d5b1f3d56f1ae16).
- remove: ambiguous name, 26 callers repo-wide; not a lobby signal (dispatch 1793527821491-3785-81ddedf8490caec3).

codesearch dual (`lobby remove bot seat release owner`): top BM25 panels.lua:1236 lobby_seat, panels.lua:855 lobby_act, panels.lua:786 owner_text, panels.lua:1229 my_seat (dispatch 1793527978362-3895-8a59f021fb5153). literal_matches 0 (nearest neighbours only).

tree facts (checked against origin/main b8897bf5, which is also local HEAD):
- scenarios/moba/panels.lua:941 `elseif call == "remove"`: refuses not-host unless owner == L.host; refuses slot-empty; clears owner, hero, ready, bot; reassigns host.
- scenarios/moba/panels.lua:1257-1258 lobby_seat: `elseif LOBBY.host == OWNER` draws the Remove button on any seat with an owner other than self.
- scenarios/moba/panels.lua:1477 router routes "remove" to lobby_act.
- scenarios/moba/draft.lua:999 draft.blocks lets "remove" through.
- scenarios/moba/panels.lua:725 holds the same branch in the ae39ed17 object; ae39ed17 is NOT an ancestor of origin/main, so the fix reached main under another sha. Row notes cite ae39ed17.
- CI: ci-status b8897bf5 run 41296428793 success 3/3 (dispatch 1793528081813-4016-4414cae058bfc1c0).

the row relies on: the remove branch and button (present), and a live fill, remove, seat-open run (absent, see 02-unknowns).
