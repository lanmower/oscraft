# 03 edit: proposed-sim-facing-turn-hold-patrol-projectile

session: gm-drain-1793526214-22
edit: none to sim, client or data. No unknown is open (02-ELIMINATE.md).
reason: six of seven items are on main (see 01-orient.md, 02-ELIMINATE.md). The seventh, projectile arc, is display-only: missile_arc is parsed in rust/crates/sim/src/loader.rs and read by no sim path, and the sim never reads display data. A sim-side arc would add hashed state with no outcome, so it is not added.
files changed: this receipt set only (01-orient.md, 02-ELIMINATE.md, 03-EDIT.md).
successor: verifier lane (PROVE, DECIDE, COMPLETE) reads 02-ELIMINATE.md dispatch 1793528491181-4451-c30185ea62a1ea48 and closes the row as stale (AGENTS.md convention: Status: stale) with that witness, since no implementation commit is needed.
