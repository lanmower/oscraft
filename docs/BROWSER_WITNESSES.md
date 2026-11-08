# Browser witnesses

Four witnesses remain scoped to a dev origin: HMR, finished match, two-tab dispatch, and strike while walking. Each is one bounded playwriter run.

Strike while walking runs three legs in one match (one-hand on, off, on) through scripts/witness/strike-while-walking.mjs (runWitness). Its red run is the one-hand-off leg, which matches attack() before the fix: a walking hero returns early there. A leg passes only when the hero is in reach while it moves, so a zero count in the red leg proves the flag and not the geometry.

The witness hero must be human-seated. "Fill with bots" gives a bot to every open seat, including the host's, and a bot-seated hero overrides every move order the witness issues. The lobby order is the one the 2026-11-09 Phase B run reached the live match with: a mode chip (All Pick), then the lobby's claim call for seat 0 (`window.oscraft.lobby('claim', {slot: 0})`; the per-seat Claim buttons are hidden in this layout), then Fill with bots (it fills only the open seats, so the claimed seat stays human), then a hero tile (PUD), then Ready, then Start match. Join The Sentinel is disabled until the lobby is negotiated (a mode, then Fill or every seat taken), so it cannot take the first human seat; Fill before any claim fills all ten seats with bots and leaves no human seat. Check the live match with a move order that holds for several seconds before any leg runs.

Open pages in the worker own context; keep at most two pages open at a time.
Before each step, list the pages and act only on pages you opened yourself.
Close every page and stop every server in a finally block; name every page and server you closed in the report. Never reuse a page from the shared list.
A stale .dev-site keeps a bot-filled host at tick 0, so re-stage from HEAD with SKIP_PACKS=1 before a witness.
Each witness records its evidence in DESIGN.md. No witness is a CI gate.
