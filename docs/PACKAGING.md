# Packaging

One archive holds everything a fresh install needs: the server, the mod language server, the data
directory, the browser page, and the net host and client. `scripts/package/build.sh` builds it and `scripts/package/install-check.sh`
proves it installs. CI runs both in the `package-install` job of `.github/workflows/ci.yml`.

## Layout

The archive has these members at its root. Paths are relative and every file is listed in the manifest.

| Member | Contents |
| --- | --- |
| `bin/oscraftd` | the server binary (crate `server`, release build) |
| `bin/oscraft-lsp` | the mod language server over stdio (crate `mod-lsp`, package `oscraft-lsp`) |
| `data/*.star` | the data directory; `oscraftd --data` builds the catalog image from it |
| `site/` | the browser page: `index.html`, `main.js`, `assets/`, `map.bin` (from `mapexport`), `pkg/` (wasm-pack output) |
| `js/host/` | the net host: `server.js`, `guest.js`, `test-relay.js`, `package.json` (pinned versions; `npm install` there before serving) |
| `js/wire/frame.js` | the wire frame codec the host imports |
| `js/client/play.html`, `js/client/dist/play.js` | the net client page and its esbuild bundle; built by `npm run build` in `js/client` when `dist/play.js` is absent |
| `VERSION` | the crate version the archive was built from |
| `MANIFEST.sha256` | one `sha256  path` line per member, in `sha256sum` format; lists every file except itself |

The archive is `oscraft-<version>.tar.gz`, built as a gzip tar of the staging directory.

## Build

    bash scripts/package/build.sh <out-dir>

Needs a Rust toolchain with the `wasm32-unknown-unknown` target, `wasm-pack`, and `sha256sum` or `shasum`. The client bundle needs `node` and `npm` when `js/client/dist/play.js` is not already built.
The script finishes by printing `package built archive=<path> version=<v> members=<n>`.

## Install check

    bash scripts/package/install-check.sh <archive> <empty-dir>

The destination must be an existing empty directory. The check:

1. unpacks the archive into it;
2. re-hashes every member and fails on any sha256 that differs from `MANIFEST.sha256`, or on a file the manifest does not list;
3. starts `bin/oscraftd --room ci --data data --site site` from the unpacked directory and requires `GET /state` to return JSON with `units` and `GET /site/` to return 200;
4. runs `bin/oscraft-lsp` over stdio with `initialize` and `shutdown`, and requires a reply that contains `capabilities`.
5. runs `node js/host/server.js --help` from the unpacked directory (no npm install needed) and requires exit 0 and a `usage: node js/host/server.js --wasm` line; requires `js/client/play.html` to reference `dist/play.js` and `js/client/dist/play.js` to be non-empty.

On success it prints `package install ok members=<n> sha256_checked=<n>` and then `package install ok members=<n> sha256_checked=<n> host_help=ok client=ok`, where `members` counts the manifest lines
and `sha256_checked` counts the members whose hash matched. Any failure prints `FAIL: <reason>` and exits 1.

## Running an installed copy

    tar -xzf oscraft-<version>.tar.gz -C /opt/oscraft
    /opt/oscraft/bin/oscraftd --addr 127.0.0.1:8080 --room main --data /opt/oscraft/data --site /opt/oscraft/site
    /opt/oscraft/bin/oscraft-lsp

`--room` is required. `--site` is optional; without it `/site/` returns 404 and the API still works.

## Scope

The packager ships data and binaries without reading their content: it knows no genre, unit or map. Adding a
member means adding one line to `build.sh` and one to the table above. The script is bash, so it runs on
Linux and macOS; the CI job runs on Linux. Windows `.exe` members are not produced here.
