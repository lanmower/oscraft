# 02 unknowns: publish-on-origin-partial-clone-write-tree
session: gm-drain-1793526214-19

U1 plain `git write-tree` fails when an index entry's blob is absent. (ELIMINATED in 03: yes, exit 128.)
U2 `git write-tree --missing-ok` succeeds with the same absent blob. (ELIMINATED in 03: yes, exit 0.)
U3 GIT_NO_LAZY_FETCH=1 changes the write-tree result. (ELIMINATED in 03: no, identical tree sha.)
U4 the --missing-ok fix is on gm-mcp origin/main. (ELIMINATED: `git branch -r --contains 3df5e22` lists origin/main.)
U5 the installed bundle carries the fix. (ELIMINATED: installed 0.2.373 contains "missing-ok".)
U6 the served MCP bundle carries the fix. (OPEN, mutable: served 0.2.372 lagged installed 0.2.373 at orient; a gm MCP client restart is the only reload path.)
U7 the closing publish needs a witness from another session. (Given: verify_lane dispatch by session gm-drain-1793526214-19-v.)
