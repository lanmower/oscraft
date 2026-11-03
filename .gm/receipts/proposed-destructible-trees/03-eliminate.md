# 03-eliminate proposed-destructible-trees

session: gm-drain-1793526214-32
method: live gm reads of the tree (codesearch literal, fs_read, git_show at upstream 5b1d68fe33bd). No build runs: builds are off in this walk, so no mod_check or parity_hash run here; the witness is CI.

- ud-tree-restore-native: ELIMINATED, present. piccolo_host.rs:1606 native tree_restore; gate rust/gate/src/mod_checks.rs:506 and :527 check tree_restored. Dispatch 1793541712350 and 1793541771346 area (codesearch tree_restore).
- ud-timber-chain: ELIMINATED in part. trees.lua chain(): pull to the first tree on the line (CHAIN_BASE 600 + 200 per rank, link 50, reach 90). No damage and no 250 range talent in trees.lua. The 6.88g ability row is abilities_688g_perlevel.tsv:210 (data only). Still open: damage and talent range.
- ud-tango-ward-target: OPEN. No ward in scenarios/moba/trees.lua. spells.lua has ward hits (10) not read, so the target is not proven absent; carried open per the convergence doc.
- ud-per-site-move-clear: OPEN. movement.rs teleport() clears trees with the one global move_tree_half and move_tree_regrow (set by trees.lua:34, 75 units). No per-site parameter. game.teleport call sites (21 hits) not all read; carried open per the convergence doc.
- ud-state-version-record: ELIMINATED, no in-tree record. persist.rs:7 is a bare const STATE_VERSION 64. docs/ARCHITECTURE.md:45 says persist has its own version. The 63 to 64 record exists only in .gm/pool/runs.jsonl (line 95: STATE_VERSION 63 to 64, CI tests 41303853915, ci 41303850997, pages 41303828352).
- stale claim found: scenarios/moba/provenance/trees.lua.tsv said obstacle slots are never reused; obstacles.rs:74 reuses dead temporary slots. Corrected in 04-edit.

Open unknowns after elimination: ud-tango-ward-target and ud-per-site-move-clear (not fully read), ud-timber-chain (damage and talent). None is a blocker: each is code work that CI can witness.
