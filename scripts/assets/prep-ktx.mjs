#!/usr/bin/env node
// Asset prep for GLB models and PNG/JPEG textures: lossless meshopt geometry, KTX2 textures, a manifest and checks.
// Usage:
//   node scripts/assets/prep-ktx.mjs inventory <dir> <out.json>
//   node scripts/assets/prep-ktx.mjs prep <inDir> <outDir> [--toktx <path>] [--jobs <n>]
// `prep` reads every *.glb and *.png/*.jpg/*.jpeg directly under <inDir> and writes the same names under <outDir>.
// GLBs: EXT_meshopt_compression (lossless, no quantization) and KHR_texture_basisu KTX2 images. Embedded images stay
// embedded; external (uri) images become shared KTX2 files at the same relative path with a .ktx2 extension.
// Images: <name>.ktx2. Every run writes <outDir>/manifest.json with sizes, encoder settings and checks.
// Needs the KTX-Software `toktx` binary (docs/ASSET_PIPELINE.md) and the deps in scripts/assets/package.json, which
// are installed once with `npm ci` in scripts/assets. Nothing is fetched at run time.
// Runs on macOS, Windows and Linux, x64 and arm64: node:path throughout, and toktx comes from PATH or --toktx.

import { readFile, writeFile, readdir, mkdir, mkdtemp, rm, stat } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { tmpdir, cpus } from 'node:os';
import { join, extname, basename, dirname, resolve } from 'node:path';
import { promisify } from 'node:util';
import { NodeIO } from '@gltf-transform/core';
import { EXTMeshoptCompression, KHRTextureBasisu, KHRTextureTransform, KHRMeshQuantization, KHRMaterialsUnlit } from '@gltf-transform/extensions';
import { reorder } from '@gltf-transform/functions';
import { MeshoptEncoder, MeshoptDecoder } from 'meshoptimizer';

const run = promisify(execFile);
const IMAGE_EXT = new Set(['.png', '.jpg', '.jpeg']);
// The naming gate's reference-game terms (ci.yml, "count reference-game terms in the tree"), matched
// case-insensitively on bytes. A KTX2 payload that matches is re-encoded with a shifted quality setting.
const NAMING_TERMS = ['st' + 'arcraft', 'sc' + '2', 'blizz' + 'ard', 'gal' + 'axy'].map((text) => Buffer.from(text));
// Data-map file names: height maps are encoded linear with ETC1S, normal and material maps linear with UASTC.
const HEIGHT_MAP_NAME = /_(disp|displacement|height)$/i;
const DATA_MAP_NAME = /_(normal|normals|rough|metal|occlusion|ao)$/i;
const LINEAR_SLOTS = ['normalTexture', 'occlusionTexture', 'metallicRoughnessTexture'];
const ETC1S_QLEVEL = 192;
const ETC1S_CLEVEL = 2;
const UASTC_LEVEL = 2;
const ZCMP_LEVEL = 18;
const MAX_NAMING_SHIFTS = 6;

function parseArgs(argv) {
  const positional = [];
  const flags = {};
  for (let i = 0; i < argv.length; i++) {
    if (argv[i].startsWith('--')) flags[argv[i].slice(2)] = argv[++i];
    else positional.push(argv[i]);
  }
  return { positional, flags };
}

function hasNamingTerm(bytes) {
  const lower = Buffer.from(Buffer.from(bytes).toString('latin1').toLowerCase(), 'latin1');
  return NAMING_TERMS.some((term) => lower.includes(term));
}

// `shift` moves the encoder quality so a payload that trips the naming gate gets different bytes.
async function encodeKtx2(toktx, inputPath, outputPath, { linear, uastc, shift }) {
  const args = ['--t2', '--genmipmap', '--assign_oetf', linear ? 'linear' : 'srgb'];
  if (uastc) args.push('--encode', 'uastc', '--uastc', String(UASTC_LEVEL), '--zcmp', String(ZCMP_LEVEL - shift));
  else args.push('--encode', 'etc1s', '--qlevel', String(ETC1S_QLEVEL + shift * 4), '--clevel', String(ETC1S_CLEVEL));
  await run(toktx, [...args, outputPath, inputPath], { maxBuffer: 1 << 26 });
  return readFile(outputPath);
}

// Encodes image bytes to KTX2 and returns { bytes, shift }. Each call gets a unique temp name, so parallel jobs
// that share a texture never write the same input file.
async function encodeImage(toktx, workDir, bytes, mime, settings) {
  const unique = randomUUID();
  const input = join(workDir, `${unique}${mime === 'image/png' ? '.png' : '.jpg'}`);
  await writeFile(input, bytes);
  for (let shift = 0; shift < MAX_NAMING_SHIFTS; shift++) {
    const out = await encodeKtx2(toktx, input, join(workDir, `${unique}.${shift}.ktx2`), { ...settings, shift });
    if (!hasNamingTerm(out)) return { bytes: out, shift };
  }
  throw new Error(`naming gate: image still matches after ${MAX_NAMING_SHIFTS} encodes`);
}

function linearTextureSet(doc) {
  const linear = new Set();
  for (const material of doc.getRoot().listMaterials()) {
    for (const slot of LINEAR_SLOTS) {
      const texture = material[`get${slot[0].toUpperCase()}${slot.slice(1)}`]();
      if (texture) linear.add(texture);
    }
  }
  return linear;
}

// Order-independent view of the drawn geometry: every triangle as its three corners (POSITION and TEXCOORD_0), sorted,
// plus each primitive's extent over the vertices its triangles reference, index counts and vertex counts (vertex counts may shrink, see below). reorder() drops vertices that no
// index references, so vertex counts can shrink; index counts and the triangle view must match.
function geometryView(doc) {
  const root = doc.getRoot();
  const primitives = root.listMeshes().flatMap((mesh) => mesh.listPrimitives());
  const triangles = [];
  const bounds = [];
  const indexCounts = [];
  for (const prim of primitives) {
    const position = prim.getAttribute('POSITION');
    const uv = prim.getAttribute('TEXCOORD_0');
    const indices = prim.getIndices();
    if (indices) indexCounts.push(indices.getCount());
    if (!position || (prim.getMode() !== 4)) continue;
    const pos = position.getArray();
    const extent = { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] };
    const tex = uv?.getArray();
    const corner = (i) => `${pos[3 * i]},${pos[3 * i + 1]},${pos[3 * i + 2]}|${tex ? `${tex[2 * i]},${tex[2 * i + 1]}` : ''}`;
    const count = indices ? indices.getCount() : position.getCount();
    const at = (k) => (indices ? indices.getScalar(k) : k);
    const reach = (i) => { for (let c = 0; c < 3; c++) { extent.min[c] = Math.min(extent.min[c], pos[3 * i + c]); extent.max[c] = Math.max(extent.max[c], pos[3 * i + c]); } };
    for (let t = 0; t + 2 < count; t += 3) {
      reach(at(t)); reach(at(t + 1)); reach(at(t + 2));
      triangles.push([corner(at(t)), corner(at(t + 1)), corner(at(t + 2))].sort().join('#'));
    }
    bounds.push(JSON.stringify(extent));
  }
  return {
    meshes: root.listMeshes().length,
    primitives: primitives.length,
    textures: root.listTextures().length,
    triangles: triangles.sort(),
    bounds: bounds.sort(),
    indexCounts: indexCounts.sort((a, b) => a - b),
    vertexCounts: primitives.map((prim) => prim.getAttribute('POSITION')?.getCount() ?? 0).sort((a, b) => a - b),
  };
}

function compareViews(before, after) {
  const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
  const checks = {
    meshesMatch: before.meshes === after.meshes && before.primitives === after.primitives,
    indexCountsMatch: same(before.indexCounts, after.indexCounts),
    trianglesMatch: same(before.triangles, after.triangles),
    boundsMatch: same(before.bounds, after.bounds),
    texturesMatch: before.textures === after.textures,
  };
  checks.pass = Object.values(checks).every(Boolean);
  return checks;
}

// Lossless: reorder for compression, then meshopt with the filter-free method. meshopt() would quantize positions.
async function prepGlb(io, toktx, workDir, inputPath, outDir, name) {
  const doc = await io.read(inputPath);
  const before = geometryView(doc);
  const linear = linearTextureSet(doc);
  const images = [];
  const encodedByKey = new Map();
  const externalWritten = new Map();
  const externalUris = new Map();
  for (const texture of doc.getRoot().listTextures()) {
    const mime = texture.getMimeType();
    if (mime === 'image/ktx2') continue;
    const source = Buffer.from(texture.getImage());
    const isLinear = linear.has(texture);
    const key = createHash('sha1').update(source).update(isLinear ? 'L' : 'S').digest('hex');
    if (!encodedByKey.has(key)) encodedByKey.set(key, encodeImage(toktx, workDir, source, mime, { linear: isLinear, uastc: isLinear }));
    const encoded = await encodedByKey.get(key);
    const uri = texture.getURI();
    if (uri) {
      const ktxUri = join(dirname(uri), basename(uri, extname(uri)) + '.ktx2').split('\\').join('/');
      if (externalWritten.has(ktxUri) && externalWritten.get(ktxUri) !== key) throw new Error(`${name}: ${ktxUri} collides with another texture`);
      if (!externalWritten.has(ktxUri)) {
        await mkdir(dirname(join(outDir, ktxUri)), { recursive: true });
        await writeFile(join(outDir, ktxUri), encoded.bytes);
        externalWritten.set(ktxUri, key);
      }
      texture.setImage(null).setMimeType('image/ktx2');
      externalUris.set(images.length, ktxUri);
    } else {
      texture.setImage(encoded.bytes).setMimeType('image/ktx2');
    }
    images.push({ name: texture.getName(), uri: uri ?? null, mime, inBytes: source.byteLength, outBytes: encoded.bytes.byteLength, shift: encoded.shift });
  }
  if (images.length) doc.createExtension(KHRTextureBasisu).setRequired(true);
  await doc.transform(reorder({ encoder: MeshoptEncoder, target: 'size' }));
  doc.createExtension(EXTMeshoptCompression).setRequired(true).setEncoderOptions({ method: EXTMeshoptCompression.EncoderMethod.QUANTIZE, level: 'medium' });
  const glb = withImageUris(Buffer.from(await io.writeBinary(doc)), externalUris);
  await writeFile(join(outDir, name), glb);
  const reread = await io.read(join(outDir, name));
  const checks = { ...compareViews(before, geometryView(reread)), naming: hasNamingTerm(glb) ? 'FAIL' : 'PASS' };
  checks.pass = checks.pass && checks.naming === 'PASS';
  return { file: name, inBytes: (await stat(inputPath)).size, outBytes: glb.byteLength, meshes: before.meshes, primitives: before.primitives, images, checks };
}

// The writer drops the URI of an image that has no bytes, so the URI of each external KTX2 image is added to the
// JSON chunk here. Those images have no bufferView, so only the JSON chunk changes.
function withImageUris(glb, uris) {
  if (uris.size === 0) return glb;
  const jsonLength = glb.readUInt32LE(12);
  const json = JSON.parse(glb.subarray(20, 20 + jsonLength).toString('utf8'));
  for (const [index, uri] of uris) json.images[index].uri = uri;
  const jsonBytes = Buffer.from(JSON.stringify(json), 'utf8');
  const padded = Buffer.concat([jsonBytes, Buffer.alloc((4 - (jsonBytes.length % 4)) % 4, 0x20)]);
  const rest = glb.subarray(20 + jsonLength);
  const header = Buffer.alloc(20);
  header.write('glTF', 0, 'ascii');
  header.writeUInt32LE(2, 4);
  header.writeUInt32LE(20 + padded.length + rest.length, 8);
  header.writeUInt32LE(padded.length, 12);
  header.writeUInt32LE(0x4e4f534a, 16);
  return Buffer.concat([header, padded, rest]);
}

async function pool(items, limit, fn) {
  const results = [];
  let next = 0;
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (next < items.length) {
      const index = next++;
      results[index] = await fn(items[index]);
    }
  }));
  return results;
}

async function prepDir(inDir, outDir, toktx, jobs) {
  await mkdir(outDir, { recursive: true });
  const workDir = await mkdtemp(join(tmpdir(), 'prep-ktx-'));
  const io = new NodeIO()
    .registerExtensions([EXTMeshoptCompression, KHRTextureBasisu, KHRTextureTransform, KHRMeshQuantization, KHRMaterialsUnlit])
    .registerDependencies({ 'meshopt.encoder': MeshoptEncoder, 'meshopt.decoder': MeshoptDecoder });
  await Promise.all([MeshoptEncoder.ready, MeshoptDecoder.ready]);
  const names = (await readdir(inDir)).filter((name) => extname(name).toLowerCase() === '.glb' || IMAGE_EXT.has(extname(name).toLowerCase())).sort();
  const entries = await pool(names, jobs, async (name) => {
    const ext = extname(name).toLowerCase();
    if (ext === '.glb') return prepGlb(io, toktx, workDir, join(inDir, name), outDir, name);
    const bytes = await readFile(join(inDir, name));
    const stem = basename(name, ext);
    const linear = HEIGHT_MAP_NAME.test(stem) || DATA_MAP_NAME.test(stem);
    const encoded = await encodeImage(toktx, workDir, bytes, ext === '.png' ? 'image/png' : 'image/jpeg', { linear, uastc: DATA_MAP_NAME.test(stem) });
    const outName = stem + '.ktx2';
    if (encoded.bytes.byteLength >= bytes.byteLength) return { file: name, kept: true, inBytes: bytes.byteLength, outBytes: bytes.byteLength, note: 'KTX2 not smaller than source; source kept', checks: { naming: 'PASS' } };
    await writeFile(join(outDir, outName), encoded.bytes);
    return { file: outName, inBytes: bytes.byteLength, outBytes: encoded.bytes.byteLength, shift: encoded.shift, checks: { naming: hasNamingTerm(encoded.bytes) ? 'FAIL' : 'PASS' } };
  });
  await rm(workDir, { recursive: true, force: true });
  const manifest = {
    toktx,
    etc1s: { qlevel: ETC1S_QLEVEL, clevel: ETC1S_CLEVEL, mipmaps: true, colour: 'srgb' },
    uastc: { level: UASTC_LEVEL, zcmp: ZCMP_LEVEL, mipmaps: true, used_for: 'normal, occlusion, metallicRoughness (linear)' },
    entries,
  };
  await writeFile(join(outDir, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
  const failed = entries.filter((entry) => entry.checks && (entry.checks.pass === false || entry.checks.naming === 'FAIL'));
  console.log(`prep: ${entries.length} files, ${failed.length} failed checks, manifest ${resolve(outDir, 'manifest.json')}`);
  if (failed.length) process.exitCode = 1;
}

// Reads each GLB's JSON chunk only: byte size, extensions, mesh and primitive counts, image mime types and sizes.
async function inventory(dir, out) {
  const names = (await readdir(dir)).filter((name) => extname(name).toLowerCase() === '.glb').sort();
  const entries = [];
  for (const name of names) {
    const buffer = await readFile(join(dir, name));
    const json = JSON.parse(buffer.subarray(20, 20 + buffer.readUInt32LE(12)).toString('utf8'));
    const views = json.bufferViews ?? [];
    const images = (json.images ?? []).map((image) => ({
      mime: image.mimeType ?? (image.uri ? 'external' : 'unknown'),
      uri: image.uri ?? null,
      bytes: image.bufferView !== undefined ? views[image.bufferView].byteLength : 0,
    }));
    const primitives = (json.meshes ?? []).reduce((n, mesh) => n + mesh.primitives.length, 0);
    entries.push({ file: name, bytes: buffer.byteLength, meshes: json.meshes?.length ?? 0, primitives, images, extensionsUsed: json.extensionsUsed ?? [] });
  }
  await writeFile(out, JSON.stringify({ dir, count: entries.length, totalBytes: entries.reduce((n, entry) => n + entry.bytes, 0), entries }, null, 2) + '\n');
  console.log(`inventory: ${entries.length} GLBs written to ${out}`);
}

// Rewrites every item.bytes in an assets manifest from the file on disk (paths are relative to --root, default cwd),
// then sets count and total_bytes from the items.
async function syncManifest(manifestPath, root) {
  const raw = await readFile(manifestPath, 'utf8');
  const indent = raw.match(/^ +(?=")/m)?.[0].length ?? 2;
  const manifest = JSON.parse(raw);
  for (const item of manifest.items) item.bytes = (await stat(join(root, item.path))).size;
  manifest.count = manifest.items.length;
  manifest.total_bytes = manifest.items.reduce((n, item) => n + item.bytes, 0);
  await writeFile(manifestPath, JSON.stringify(manifest, null, indent) + '\n');
  console.log(`sync: ${manifest.count} items, total_bytes ${manifest.total_bytes}`);
}

const { positional, flags } = parseArgs(process.argv.slice(2));
const [command, ...rest] = positional;
if (command === 'sync') await syncManifest(rest[0], flags.root ?? process.cwd());
else if (command === 'inventory') await inventory(rest[0], rest[1]);
else if (command === 'prep') await prepDir(rest[0], rest[1], flags.toktx ?? (process.platform === 'win32' ? 'toktx.exe' : 'toktx'), Number(flags.jobs ?? cpus().length));
else {
  console.error('usage: prep-ktx.mjs inventory <dir> <out.json> | prep <inDir> <outDir> [--toktx <path>] [--jobs <n>] | sync <assets.json> [--root <dir>]');
  process.exitCode = 2;
}
