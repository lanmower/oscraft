# Packaging

One archive holds everything a fresh install needs: the mod language server, the browser page, the mods and both scenario packs with their rebuilt map images. `scripts/package/build.sh` builds it and `release.yml` runs it for tagged releases. The install check and the `ci.yml` job that ran it were retired in `9a17155e`.

## Layout

The archive has these members at its root. Paths are relative and every file is listed in `MANIFEST.sha256`.

| Member | Contents |
| --- | --- |
| `bin/oscraft-lsp` | the mod language server over stdio (crate `mod-lsp`, package `oscraft-lsp`) |
| `js/client/` | the browser page and its ES modules, loaded directly (no bundle) |
| `js/client/pkg/` | the wasm-bindgen output (`web.js`, `web_bg.wasm`), built by `wasm-pack` and never committed |
| `mods/` | the example mod packs |
| `scenarios/moba/`, `scenarios/duel-routes/` | both scenario packs; `mapexport` rebuilds each `.osdt` image into the archive |
| `scenarios/moba/assets/models/` | the unit GLB models |
| `VERSION` | the crate version the archive was built from |
| `MANIFEST.sha256` | one `sha256  path` line per member, in `sha256sum` format; lists every file except itself |

The archive is `oscraft-<version>.tar.gz`, built as a gzip tar of the staging directory.

## Build

    bash scripts/package/build.sh <out-dir>

Needs a Rust toolchain with the `wasm32-unknown-unknown` target, `wasm-pack`, and `sha256sum` or `shasum`. No node or npm is needed. The script finishes by printing `package built archive=<path> version=<v> members=<n>`.

## Verify an archive

`scripts/package/install-check.sh` and the CI job that ran it were retired in `9a17155e`. To check an unpacked archive by hand, recompute the hashes against `MANIFEST.sha256` and confirm `js/client/pkg/web.js`, `web_bg.wasm` and both `.osdt` images are non-empty.

## Running an installed copy

    tar -xzf oscraft-<version>.tar.gz -C /opt/oscraft
    node scripts/dev/serve.mjs /opt/oscraft
    /opt/oscraft/bin/oscraft-lsp

The browser page is the host: there is no server process. From the repository, `scripts/dev/serve.mjs` serves an unpacked archive on 127.0.0.1 only with no-store headers; any static file server works for a deployed copy.

## Scope

The packager ships data and binaries without reading their content: it knows no genre, unit or map. Adding a member means adding one line to `build.sh` and one to the table above. The script is bash, so it runs on Linux, macOS and Windows under a bash shell; `release.yml` builds all three, and the Windows runner produces the `.exe` members.
