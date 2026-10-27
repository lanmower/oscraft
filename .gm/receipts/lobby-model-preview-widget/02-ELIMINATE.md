# 02 ELIMINATE lobby-model-preview-widget
lobby-model-preview-glb-loader: resolved. assetUrl (pack_load.js:350), fetchRetrying (pack_load.js:305), resolveGlbExtensions (glb_resolve.js:14), parseGlb (glb.js:128), fitModel (tiles.js:125). play.js parsedModel (540-557) uses the same chain on the same URL. Dispatches 1792927663930, 1792927671894, 1792927598971, 1792927592558, 1792927672552.
lobby-model-preview-render3d-single-mesh: resolved. createRenderer returns beginFrame (574), setCamera with fovY, aspect, minDistance (559), drawMesh (620), endFrame (592); upload accepts uvs null. Dispatches 1792927650532, 1792927664617, 1792927574834.
lobby-model-preview-ci-module-rules: resolved. scripts/check-modulepreload.mjs follows static imports only; a dynamic import() is a deliberate deferral. No .github workflow names js/client. Dispatches 1792927845845, 1792927844939, 1792927846739, 1792927853430.
Edit gate: no unknown open. EDIT starts now.
