# Quickstart

The browser is the only build. Rust stable and `wasm-pack` are required. Commands run from `rust/` unless noted.

## Build the page

```
wasm-pack build crates/web --target web --out-dir site/pkg
```

## Compile a pack's data

```
cargo run -p mapexport -- ../scenarios/moba ../scenarios/moba/moba.osdt
```

`mapexport <pack-dir> <out.bin>` evaluates the pack's `.osd` files and writes the map image. The image path is the pack's manifest `image`. `osdt-data <dir> <out>` does the same for one directory of `.osd` files and reports a bad definition with its file and line.

## Serve and play

From the repo root:

```
node scripts/dev/stage.sh
node scripts/dev/serve.mjs
```

`stage.sh` lays the site out in `.dev-site/` with both packs, the page wasm and the mods. `serve.mjs` serves it. Open `js/client/play.html?package=scenarios/moba`. The `package` parameter takes a directory or a `.tar.gz`/`.zip` archive; the default is `scenarios/moba`. Add `?host=1` on one page to host the room; peers join the same room over wireweave. See [HOST.md](HOST.md) for the other query parameters.

## Edit a pack

Open the pack editor from the host's menu on the play page. It edits the pack's `.lua`, `.osd`, `.json`, `.txt` and `.tsv` files, takes asset drops, and downloads the pack as a `.zip`. Saved changes go to peers. See [MODDING.md](MODDING.md#editing-and-sharing-a-pack).

To paint cells and doodads by hand, `js/client/editor.html` is the map editor. It shows the `.osd` text to copy back. Regenerate the image after an edit with the `mapexport` command above.

## Check a mod

```
cargo run -p mod-host --bin mod_check -- ../scenarios/moba ../mods/ui_demo.lua 200
```

`mod_check` builds the pack's image, loads one mod into it and runs the ticks. See [MODDING.md](MODDING.md) and [SCRIPTING.md](SCRIPTING.md) for the API.

`oscraft-lsp` gives editors diagnostics for `.osd` and `.lua` files over stdio.

## Regenerate the reference

From the repo root:

```
node scenarios/moba/tools/gen-reference.mjs
```
