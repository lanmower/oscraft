# 05-validate proposed-destructible-trees (witness, stale premise)

session: gm-drain-1793526214-v14 (witness lane, not the executor)
sha: cd07f7a185e9354e881f30f8623cfbbb1d8a6768 (commit gm: rescope proposed-destructible-trees; ancestor of upstream tip e9b66a7e3ec2)
timestamp: 2026-11-01T14:11:40.995Z
method: live reads of blobs at the sha via git_show; no local build; no test files written; CI not read by this lane (gm-validate step 4; the orchestrator spawns the CI lane for this sha)

check: expected (receipt cite) -> observed at sha
C1 obstacles.rs alive/until/temp fields 15-17 -> 15 alive, 16 until, 17 temp. PASS
C2 kill_tree, restore_tree -> 102 pub fn kill_tree, 115 pub fn restore_tree (receipt cited 116). PASS
C3 dead temp slot reuse -> 74 push_obstacle position(temp && !alive). PASS
C4 tick_obstacles regrow -> 216-239: until gate 220, temp expiry 223-227, tree_guard defer 233-235, restore 237. PASS
C5 TREE cell bit -> cover_cells 43; TREE set/clear at 52, 64, 253. PASS
C6 obstacles hashed -> hash.rs 149-152 (tree_guard 151). PASS
C7 obstacles saved -> persist.rs 491-492. PASS
C8 STATE_VERSION 64 -> persist.rs 7; save writes 277; load check 529. PASS
C9 natives tree_kill, tree_restore -> piccolo_host.rs present (content located at sha; receipt lines 1598, 1606; relative spacing 8 lines matches; absolute line not recounted). PASS
C10 trees.lua 6 GUARD_UNITS=175; 8 REGROW 300/450/20; 26-27 regrow_seconds; 33 set_tree_guard; 34 set_move_tree_rule 75; 40 game.tree_kill; 157 trees.cut. PASS
C11 six blockers -> trees.sprout 101, trees.branch 106 (Ironwood branch), trees.fissure 108, trees.ice_shards 115. PASS

open gaps confirmed open at sha:
G1 Timber Chain damage and 250 range: trees.lua 126-144 chain pulls only, no damage term. OPEN
G2 Tango and Quelling Blade ward target: item_tango 16 heal only, item_quelling_blade 17 empty. OPEN
G3 per-site move clear radius: only global set_move_tree_rule at 34. OPEN
G4 63 to 64 in-tree record: AGENTS.md at sha has no bump record (persist.rs:89 is the generic rule). OPEN

verdict: stale premise verified; cut and regrow per 6.88g met by existing code at the sha; the four rescoped gaps are correctly listed as open.
unverified here: CI on the sha and native/wasm parity (CI lane).