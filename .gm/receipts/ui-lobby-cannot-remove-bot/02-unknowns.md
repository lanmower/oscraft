# 02-unknowns: ui-lobby-cannot-remove-bot (session gm-drain-1793526214-9)

Recorded with mutable-add (dispatches 1793528118709-4057, 1793528119320-4058, 1793528119920-4059):

- ui-lobby-cannot-remove-bot-u1: does origin/main carry the host-only remove branch and Remove button. Answered yes: panels.lua:941 and :1258 on origin/main b8897bf5 (git show).
- ui-lobby-cannot-remove-bot-u2: does a live gpu-free instrument dispatch ui `remove` on a filled seat. Answered no, see below.
- ui-lobby-cannot-remove-bot-u3: is CI green for the sha carrying the fix. Answered yes: ci-status b8897bf5 success 3/3 (dispatch 1793528081813-4016-4414cae058bfc1c0).

u2 live check:
- codesearch literal `dispatch_action("ui"` in rust: 14 hits (dispatch 1793528108493-4044-c4988611cdcaa4c8). Drivers: mod-host/tests/lobby_claim.rs, mod-host/tests/panels_route.rs, gate chat_checks.rs, draft_checks.rs, mechanic_checks.rs. They send claim, world_ready, pick and start. None sends remove.
- codesearch literal `"remove"` in rust/gate: 2 hits (dispatch 1793528109741-4047-b916891cfd4e80be), both unrelated (draft panel removal, board removal).
- mod_check and parity_hash take a scenario and a mod file; no ui dispatch path. The only ui path is ModHost::dispatch_action (piccolo_host.rs:3804) called from Rust.
- Minting a driver means creating or editing a test file, which this worker must not do (gm §3, operator rule). So the witness has no instrument.

Result: u2 is the open fact. It cannot be closed without a test file, so the row's witness stays unrun.
