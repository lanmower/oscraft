# workspace-tests-red-d31028ab-aegis-respawn-and-antimage-bar: stale

Stage reached: SPECIFY, ELIMINATE (live read), CLOSE. Session drain-full-29a-2-r1.

- The failing sha d31028ab is not an ancestor of main (origin/main = fafb8689).
- Live read: tests.yml run 40942127165 on main tip fafb8689, job workspace tests (123687956383): mod-host::aegis (both tests) and mod-host::ability_bar PASS.
- Remaining main failure, not filed (163 open rows, prd-add gate at 20): mod-host::moba_pack_loads every_module_loads_and_runs_300_ticks, crates/mod-host/tests/moba_pack_loads.rs:14, left 88 right 92.
- Witness dispatch (verify session): 1793303627993-2639-e3c4aeba2f2a97e0 (exec_js over gh run view, prints the failing and passing lines).
- Dream-RSI cycle: deferred (not dispatched in this lane; close stage only).
