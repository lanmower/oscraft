# 01-orient proposed-destructible-trees

session: gm-drain-1793526214-32
row: proposed-destructible-trees (pending, owner hy3d, status proposed)

acceptance (stated, from the row): Trees can be cut and regrow per 6.88g, the state version is bumped and recorded, and a native and wasm state-hash witness agrees.

tree state: local e56dd0b68283 is 1 ahead and 21 behind upstream tip 5b1d68fe33bd. Blobs cited as "upstream" were read with git_show at 5b1d68fe33bd (obstacles.rs, persist.rs, trees.lua, hash.rs). Other cites are the local checkout (rust/crates/sim/src is 1 behind).

row premise vs tree (row says absent; tree has it):
- "no alive flag": Obstacle.alive at rust/crates/sim/src/obstacles.rs:15
- "no regrow tick in state or hash": Obstacle.until obstacles.rs:16; hashed at rust/crates/sim/src/hash.rs:149-155 (tree_guard, per-obstacle x0 y0 w h prop until alive temp); saved at persist.rs:491-492; STATE_VERSION 64 at persist.rs:7 (upstream)
- "no TREE cell bit": cover_cells sets pathfinding::TREE (obstacles.rs cover_cells); docs/SCRIPTING.md:185 documents game.tree_kill / tree_restore
- "no kill or restore commands": kill_tree obstacles.rs:102, restore_tree obstacles.rs:116; native tree_kill at rust/crates/mod-host/src/piccolo_host.rs:1598; scenarios/moba/trees.lua:40 calls game.tree_kill(id, trees.regrow_seconds())
- "temporary blockers": add_blocker and expire_blocker/remove_blocker in obstacles.rs (temp=true); push_obstacle reuses a dead temp slot at obstacles.rs:74 (the convergence doc says slot reuse is missing: that doc line is stale)
- "clear-under-blink", Tango, Quelling Blade: trees.cut in scenarios/moba/trees.lua:157 via spells.lua:122 HANDLERS; move clear rule set at trees.lua:34 (game.set_move_tree_rule, MOVE_CLEAR_MIN_UNITS 75); engine moves clear through movement.rs:265 move_tree_half check
- regrow per 6.88g: trees.lua upstream lines 5-8: GUARD_UNITS 175, REGROW_S 300, REGROW_FIRST_S 450, REGROW_FIRST_DEATHS 20; tick_obstacles regrows after until and defers while a hero is within tree_guard

what the tree doc (docs/CONVERGENCE_688G.md:362) still lists as missing, unverified against code:
1. per-site clear radius for the 57 hy sites whose move is an .osd effect (engine move effects have no tree hook; only the global rule at trees.lua:34)
2. Timber Chain damage and the 250 range talent
3. ward target of Tango and Quelling Blade

upstream CI on 5b1d68fe33bd (ci-status): 9 jobs, 8 success, 1 pending; run 41313887236 pending.

verdict on acceptance clauses: (1) cut and regrow per 6.88g: met by existing code for the 6.88g timings and guard; the three listed gaps are open. (2) state version bump and record: STATE_VERSION is 64 and obstacles are hashed and saved; the record of which version added obstacles is not found (git history is one squashed commit for obstacles.rs). (3) native/wasm state-hash witness: CI parity exists (tests workflow and pages parity job); a witness on the pushed sha is still required.

unknowns for stage 02 are in 02-unknowns.md.
