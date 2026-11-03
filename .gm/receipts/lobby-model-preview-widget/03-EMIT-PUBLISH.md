# 03 EMIT-PUBLISH lobby-model-preview-widget (step 1)
Files: js/client/widgets.js (model kind, portrait then preview on oscraft:model-loaded), js/client/render3d.js (drawModelPreview appended).
Publish: f356dd23d58f76c629456c313c848fcbea488db5 on origin/main (publish_on_origin dispatch 1792928028035; dry run 1792928016266). Parent 6d8e7388 (preload lane, play.js) untouched.
CI: ci success 2/2 (ci-status 1792928238137). pages 3/4 success, first draw of the built page failed the 10000 ms ceiling (median 11772 ms), same failure at 6d8e7388 (run 40189057060).
Witness: git_show 1792928333698-5611-3e7cba534aa91bb3 (verify session).
Step 2: not started (blocked_by lobby-modal-reemitted-every-refresh-flashes-hero-grid, pending).
Dream-RSI: dream_replay_cycle dispatch none, reply: data: |   cycle: |     baseline_score: 0.6525013885298642 |     beta1: 0.01 |     beta2: 0.05 |     candidates: 8 |     contract: |       post_selected_ge_baseline: true |       pre_incumbent_in_candidates: true |     
