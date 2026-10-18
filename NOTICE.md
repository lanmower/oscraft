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
| 3D model and texture pack (GLB models, colormap and terrain textures) | owner-confirmed CC0 (public domain dedication; no attribution required) | CC0 1.0 | local pack, untracked `assets/Models`; shipped copies in `js/client/models`, `rust/crates/web/site/assets` | browser client, moba and terrain scenes |


This file is not legal advice. A human legal review is required before any release.
