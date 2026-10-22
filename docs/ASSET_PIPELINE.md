# Asset pipeline: meshopt geometry and KTX2 textures

`scripts/assets/prep-ktx.mjs` prepares a folder of GLB models and PNG or JPEG textures for the browser client.
GLBs get lossless `EXT_meshopt_compression` geometry and `KHR_texture_basisu` KTX2 base-colour images. Each
raster image becomes a `.ktx2` file. The client loader reads both forms: `js/client/glb.js` and
`js/client/glb_resolve.js` decode meshopt, and `js/client/texture.js` decodes KTX2 through the vendored Basis
transcoder (`js/client/vendor/three/libs/basis`).

The script runs offline once its dependencies are installed. It makes no network call.

## What you need

| Tool | Version used | Where to get it |
|---|---|---|
| KTX-Software (`toktx`, `ktx`) | 4.4.2 | https://github.com/KhronosGroup/KTX-Software/releases/tag/v4.4.2 |
| Node.js | 24.x (any 20 or newer should work) | https://nodejs.org |
| npm packages `@gltf-transform/core`, `extensions`, `functions` | 4.5.1 | `npm ci` in `scripts/assets` (locked in `package-lock.json`) |
| npm package `meshoptimizer` | 0.22.0 | same |

KTX-Software is not stored in this repository. Download the release archive that matches your OS and CPU
(macOS, Windows, Linux; x64 or arm64). Unpack it anywhere, then either put its `bin` folder on `PATH` or pass
the full path with `--toktx`. On Windows the script looks for `toktx.exe` on `PATH`.

Install the script's npm dependencies once:

```sh
cd scripts/assets && npm ci
```

`scripts/assets/node_modules/` is git-ignored.

## Commands

Inventory every GLB (reads only each file's JSON chunk):

```sh
node scripts/assets/prep-ktx.mjs inventory scenarios/moba/assets/models scripts/assets/glb-inventory.json
```

Prepare a folder. GLBs and PNG or JPEG files are read from `<inDir>`, and the results go to `<outDir>`:

```sh
node scripts/assets/prep-ktx.mjs prep scenarios/moba/assets/models /tmp/out/models --toktx /path/to/toktx
node scripts/assets/prep-ktx.mjs prep scenarios/moba/assets/textures /tmp/out/textures --toktx /path/to/toktx
```

Each `prep` run writes `<outDir>/manifest.json`, which records every file's input and output sizes, the
texture encodings and, for GLBs, the geometry checks. It exits nonzero when any check fails.

Refresh the byte counts in an assets manifest from the files on disk:

```sh
node scripts/assets/prep-ktx.mjs sync scenarios/moba/assets/assets.json --root .
```

`sync` rewrites each item's `bytes` from the file on disk, keeps the file's indentation, and recomputes `count` and `total_bytes`.

## The exact toktx commands

Colour maps (sRGB, ETC1S, mipmaps):

```sh
toktx --t2 --genmipmap --assign_oetf srgb --encode etc1s --qlevel 192 --clevel 2 out.ktx2 in.png
```

Height maps (`*_disp`, `*_height`, `*_displacement`; linear, ETC1S):

```sh
toktx --t2 --genmipmap --assign_oetf linear --encode etc1s --qlevel 192 --clevel 2 out.ktx2 in.jpg
```

Normal, roughness, metal and occlusion maps (linear, UASTC, zstd):

```sh
toktx --t2 --genmipmap --assign_oetf linear --encode uastc --uastc 2 --zcmp 18 out.ktx2 in.png
```

GLB base-colour and other images use the first form, except that a GLB image in a material's normal, occlusion
or metal-rough slot uses the UASTC form.

## Decision rules

- **Keep the source when KTX2 is not smaller.** A standalone PNG or JPEG whose KTX2 is not smaller than its source is
  left as it is (GLB-embedded images are always converted). The loader falls back to the raster file, so nothing breaks. Small JPEG maps often stay JPEG for this
  reason. The manifest marks them `kept: true`.
- **External images stay external.** A GLB that points at a PNG (for example `Textures/colormap.png`) gets a
  `.ktx2` file at the same relative path (`Textures/colormap.ktx2`). The GLB refers to it by URI with
  `image/ktx2`, so the file is shared rather than copied into every model.
- **Embedded images stay embedded.** An image inside the GLB stays in the GLB as KTX2.
- **Geometry is lossless.** The script writes `EXT_meshopt_compression` with no quantization and no filter, keeps
  every vertex (no `reorder`, which would drop vertices no index uses and change the footprints the client fits
  over), and widens 8-bit indices to 16-bit, because the meshopt triangle encoder takes only 16-bit or 32-bit indices.
- **Already-prepared files are left alone.** A GLB that already uses KTX2 and meshopt is not re-encoded. Check
  its manifest entry first if you add a file like it.

## Checks the script runs

For each GLB, the script re-reads the output and compares it with the input:

- `meshesMatch`: the mesh and primitive counts are equal.
- `indexCountsMatch`: every primitive has the same index count.
- `trianglesMatch`: the triangles, as their corners' positions and TEXCOORD_0 values, are the same set.
- `boundsMatch`: each primitive's extent over the vertices its triangles reference is the same.
- `texturesMatch`: the texture count is unchanged.

Check an output file yourself with the KTX-Software validator. A validated file exits 0:

```sh
ktx validate out.ktx2
```

## Loader behaviour

- **GLB images:** `js/client/glb.js` reads `KHR_texture_basisu`, and `js/client/texture.js` decodes `image/ktx2`.
- **Raster URIs:** `loadBaseColorImage` in `js/client/texture.js` tries the `.ktx2` sibling of a PNG or JPEG
  URI first, when the asset catalogue (`assets.json`) lists that sibling, and falls back to the raster file
  otherwise. Splat layers and tile textures use this path, so they load KTX2 without any caller change.

## Verifying an output

1. `ktx validate` on every `.ktx2` file: exit 0.
2. `manifest.json`: every entry's `checks.pass` is `true`.
3. Load the GLB through the client loader (`resolveGlbExtensions` then `parseGlb`) and check its bounds and
   index count against the original. The client's Basis transcoder can decode the KTX2 base colour to RGBA for a
   colour comparison.
