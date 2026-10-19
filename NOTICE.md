# Third-party notices

Third-party components used by this project, with versions pinned in `rust/Cargo.toml` and
`rust/crates/*/Cargo.toml`. Licences were read from crates.io metadata on 2026-10-15. Transitive
dependencies are not listed because no `Cargo.lock` is tracked.

| Component | Pinned version | Licence | Source | Used by |
|---|---|---|---|---|
| starlark | =0.14.2 | Apache-2.0 | https://crates.io/crates/starlark/0.14.2 | rust/crates/data |
| anyhow | =1.0.104 | MIT OR Apache-2.0 | https://crates.io/crates/anyhow/1.0.104 | rust/crates/data |
| flate2 | =1.1.9 | MIT OR Apache-2.0 | https://crates.io/crates/flate2/1.1.9 | rust/crates/mpq |
| bzip2 | =0.6.0 | MIT OR Apache-2.0 | https://crates.io/crates/bzip2/0.6.0 | rust/crates/mpq |
| lzma-rs | =0.3.0 | MIT | https://crates.io/crates/lzma-rs/0.3.0 | rust/crates/mpq |
| mlua | 0.10 (manifest range; resolved version unknown, verify; 0.10.5 is MIT) | MIT | https://crates.io/crates/mlua | rust/crates/spike (feature `vendored`) |
| Lua 5.4 (vendored through mlua) | unknown, verify (lua-src crate) | MIT | https://www.lua.org/license.html and https://crates.io/crates/lua-src | built from source via mlua `vendored` |
| three.js | 0.186.1 | MIT (see `js/client/vendor/three/LICENSE`) | https://registry.npmjs.org/three/-/three-0.186.1.tgz; vendored `build/three.core.js`, `three.module.js`, `three.webgpu.js`, `three.tsl.js` | js/client/vendor/three (WebGPU and TSL build, buildless ES modules) |
| three.js libs: meshopt_decoder.module.js (29256 B), draco/draco_decoder.js (512465 B), draco/draco_decoder.wasm (192420 B), draco/draco_wasm_wrapper.js (58456 B), basis/basis_transcoder.js (57529 B), basis/basis_transcoder.wasm (527333 B) | three r186 examples/jsm/libs (draco/gltf build for Draco) | meshopt: MIT (libs/LICENSE.meshopt); Draco and Basis: Apache-2.0 (libs/draco/LICENSE, libs/basis/LICENSE); three.js MIT (libs/LICENSE) | https://github.com/mrdoob/three.js/tree/r186/examples/jsm/libs ; meshopt https://github.com/zeux/meshoptimizer ; Draco https://github.com/google/draco ; Basis https://github.com/BinomialLLC/basis_universal | js/client/vendor/three/libs; loaded lazily by js/client/decoders.js for EXT_meshopt_compression, KHR_draco_mesh_compression and KHR_texture_basisu |
| 3D model and texture pack (GLB models, colormap and terrain textures) | owner-confirmed CC0 (public domain dedication; no attribution required) | CC0 1.0 | local pack, untracked `assets/Models`; shipped copies in `js/client/models`, `rust/crates/web/site/assets` | browser client, moba and terrain scenes |
| Lane texture `js/client/textures/lane.jpg` (Ground037 colour map, resized to 512 px, JPEG) | ambientCG Ground037, CC0 1.0 (public domain dedication; no attribution required) | CC0 1.0 | https://ambientcg.com/view?id=Ground037 (download: `Ground037_1K-JPG_Color.jpg`) | browser client, lane tiles (`js/client/tiles.js`, `TILE_TEXTURES`) |
| Cliff texture `js/client/textures/cliff.jpg` (Rock035 colour map, resized to 512 px, JPEG) | ambientCG Rock035, CC0 1.0 (public domain dedication; no attribution required) | CC0 1.0 | https://ambientcg.com/view?id=Rock035 (download: `Rock035_1K-JPG_Color.jpg`) | browser client, no longer referenced: cliff tiles use the owner's `rock.jpg` since the lighting pass (`js/client/tiles.js`, `TILE_TEXTURES`) |
| Ground texture `js/client/textures/ground.png` (Grass004, albedo only, resized to 512 px and palette-quantised to 256 colours) | ambientCG Grass004, CC0 1.0 (public domain dedication; no attribution required) | CC0 1.0 | https://ambientcg.com/view?id=Grass004 (download: `Grass004_1K-JPG_Color.jpg`) | browser client, grass and dirt terrain tiles (`js/client/tiles.js`) |
| Splat layers `js/client/textures/grass.jpg`, `dirt.jpg`, `rock.jpg` and their height maps `grass_disp.jpg`, `dirt_disp.jpg`, `rock_disp.jpg` (colour and displacement maps of the owner's mapspinner set, resized to 512 px, JPEG) | internal, made by the owner, in `/workspace/spoint/packages/mapspinner/textures` (sand-color and sand-displacement become `dirt`; not CC0) | owner-made, shipped by owner decision | `/workspace/spoint/packages/mapspinner/textures` | browser client, splat ground layers (`js/client/tiles.js`, `SPLAT_TEXTURES`; `js/client/render3d.js` `SPLAT_FRAG`, `js/client/render_tsl.js`) |
| Splat lane layer `js/client/textures/lane.jpg` and `lane_disp.jpg` (Ground037 colour and displacement maps, 512 px, JPEG) | ambientCG Ground037, CC0 1.0 (public domain dedication; no attribution required) | CC0 1.0 | https://ambientcg.com/view?id=Ground037 (downloads: `Ground037_1K-JPG_Color.jpg`, `Ground037_1K-JPG_Displacement.jpg`) | browser client, lane splat layer (the mapspinner set has no lane layer) |

| Rock colour map `js/client/models/Textures/rock-color.ktx2` (KTX2 of the owner's mapspinner rock colour map `rock-color.jpg`, 1024 px, encoded with toktx 4.4.2: `--t2 --genmipmap --assign_oetf srgb --encode etc1s --qlevel 192 --clevel 2`) | internal, made by the owner, cleared to ship (not CC0) | owner-made, cleared by the owner to ship | `/workspace/spoint/packages/mapspinner/textures/rock-color.jpg` | browser client, base colour of the 25 rock-role GLB models in `js/client/models` (`rock-a`, `rock-b`, `rocks-large`, `stone-wall*`, ...); their UVs are unchanged |
| Encoded copies of the rows above: GLB models in `js/client/models` re-encoded with meshopt and KTX2 textures, and `*.ktx2` files in `js/client/textures` and `js/client/models/Textures` (made by `scripts/assets/prep-ktx.mjs`) | same as the source of each encoded file | same as the source of each encoded file | the rows above | js/client/models, js/client/textures |



This file is not legal advice. A human legal review is required before any release.
