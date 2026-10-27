# 01 SPECIFY lobby-model-preview-widget (session drain-modelview-13)
Scope: step 1 only, js/client/widgets.js and js/client/render3d.js. Step 2 (scenarios/moba/panels.lua) waits for lobby-modal-reemitted-every-refresh-flashes-hero-grid, still pending in prd-list (dispatches 1792927518240, 1792927551123).
Stage 0 REFERENCES: prd-references dispatch 1792927549588 (MDN CustomEvent, EventTarget.addEventListener).
Premise (codesearch, codeinsight): buildNode (widgets.js:314) has no model kind; createRenderer (render3d.js:414) has no preview; play.js:1583 dispatches oscraft:model-loaded with detail {model} (preload lane, uncommitted in the parent).
Origin check: origin/main 6d8e738 is ahead of HEAD 53eecaea by one commit touching only js/client/play.js (compare 1792927777837). widgets.js and render3d.js are equal on origin and HEAD.
Writer lock: parent check returned not_clean (dirty play.js of the preload lane, 1792927778508). This lane edits only its two owned files and publishes them with publish_on_origin, which leaves play.js alone.
Unknowns: lobby-model-preview-glb-loader (1792927835023), lobby-model-preview-render3d-single-mesh (1792927835649), lobby-model-preview-ci-module-rules (1792927836313).
