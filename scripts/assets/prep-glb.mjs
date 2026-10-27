#!/usr/bin/env node
import { NodeIO } from "@gltf-transform/core";
import { EXTMeshoptCompression, KHRMeshQuantization, KHRTextureBasisu } from "@gltf-transform/extensions";
import { MeshoptDecoder, MeshoptEncoder } from "meshoptimizer";
import { dedup, quantize, reorder } from "@gltf-transform/functions";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..", "..");
const defaultDir = path.join(root, "scenarios", "moba", "assets", "models");
const manifestPath = path.join(here, "prep-manifest-glb.json");

const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const force = args.includes("--force");
const files = args.filter((a) => !a.startsWith("--"));

const sha256 = (buf) => crypto.createHash("sha256").update(buf).digest("hex");
const walk = (dir) => {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walk(abs));
    else if (entry.name.toLowerCase().endsWith(".glb")) out.push(abs);
  }
  return out;
};

const targets = files.length ? files.map((f) => path.resolve(f)) : walk(defaultDir).sort();
const manifest = fs.existsSync(manifestPath) ? JSON.parse(fs.readFileSync(manifestPath, "utf8")) : { schema: 1, files: {} };

await MeshoptEncoder.ready;
await MeshoptDecoder.ready;
const io = new NodeIO()
  .registerExtensions([EXTMeshoptCompression, KHRMeshQuantization, KHRTextureBasisu])
  .registerDependencies({ "meshopt.encoder": MeshoptEncoder, "meshopt.decoder": MeshoptDecoder });

const skeleton = (doc) => {
  const out = [];
  for (const mesh of doc.getRoot().listMeshes()) {
    for (const prim of mesh.listPrimitives()) {
      out.push([
        prim.listSemantics().sort().join("|"),
        prim.getAttribute("POSITION")?.getCount() ?? 0,
        prim.getIndices()?.getCount() ?? 0,
        prim.getAttribute("JOINTS_0")?.getCount() ?? 0,
      ].join("/"));
    }
  }
  return out.join(";");
};

const NORMALIZE_DIVISOR = { 5120: 127, 5121: 255, 5122: 32767, 5123: 65535 };

function glbChunks(bytes) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const total = view.getUint32(8, true);
  let json = null;
  let bin = null;
  for (let at = 12; at + 8 <= total;) {
    const length = view.getUint32(at, true);
    const type = view.getUint32(at + 4, true);
    const start = at + 8;
    if (type === 0x4e4f534a) json = JSON.parse(new TextDecoder().decode(new Uint8Array(bytes.buffer, bytes.byteOffset + start, length)));
    else if (type === 0x004e4942) bin = bytes.subarray(start, start + length);
    at = start + length;
  }
  return { json, bin };
}

function buildGlb(json, bin) {
  let jsonBytes = Buffer.from(JSON.stringify(json), 'utf8');
  const jsonPad = (4 - (jsonBytes.length % 4)) % 4;
  if (jsonPad) jsonBytes = Buffer.concat([jsonBytes, Buffer.alloc(jsonPad, 0x20)]);
  const binPad = (4 - (bin.length % 4)) % 4;
  const binBytes = binPad ? Buffer.concat([Buffer.from(bin), Buffer.alloc(binPad)]) : Buffer.from(bin);
  const total = 12 + 8 + jsonBytes.length + 8 + binBytes.length;
  const head = Buffer.alloc(12);
  head.writeUInt32LE(0x46546c67, 0);
  head.writeUInt32LE(2, 4);
  head.writeUInt32LE(total, 8);
  const jsonHead = Buffer.alloc(8);
  jsonHead.writeUInt32LE(jsonBytes.length, 0);
  jsonHead.writeUInt32LE(0x4e4f534a, 4);
  const binHead = Buffer.alloc(8);
  binHead.writeUInt32LE(binBytes.length, 0);
  binHead.writeUInt32LE(0x004e4942, 4);
  return Buffer.concat([head, jsonHead, jsonBytes, binHead, binBytes]);
}

function withFloatPositionBounds(bytes, doc) {
  const { json, bin } = glbChunks(bytes);
  if (!json || !bin) return bytes;
  const docMeshes = doc.getRoot().listMeshes();
  let changed = false;
  (json.meshes ?? []).forEach((mesh, mi) => {
    (mesh.primitives ?? []).forEach((prim, pi) => {
      const index = prim.attributes?.POSITION;
      if (index === undefined) return;
      const accessor = json.accessors?.[index];
      const attribute = docMeshes[mi]?.listPrimitives()[pi]?.getAttribute('POSITION');
      if (!accessor || !attribute || !accessor.normalized) return;
      const divisor = NORMALIZE_DIVISOR[accessor.componentType];
      if (!divisor) return;
      const data = attribute.getArray();
      const min = [Infinity, Infinity, Infinity];
      const max = [-Infinity, -Infinity, -Infinity];
      for (let i = 0; i < data.length; i += 3) {
        for (let c = 0; c < 3; c++) {
          const value = data[i + c] / divisor;
          if (value < min[c]) min[c] = value;
          if (value > max[c]) max[c] = value;
        }
      }
      if (min.some((v) => !Number.isFinite(v)) || max.some((v) => !Number.isFinite(v))) return;
      accessor.min = min;
      accessor.max = max;
      changed = true;
    });
  });
  return changed ? buildGlb(json, bin) : bytes;
}

let prepared = 0;
let skippedDone = 0;
let skippedNoGain = 0;
let skippedUnsafe = 0;
let beforeBytes = 0;
let afterBytes = 0;

for (const file of targets) {
  const rel = path.relative(root, file);
  const original = fs.readFileSync(file);
  const hash = sha256(original);
  const done = manifest.files[rel];
  if (done && done.sha256 === hash && !force) {
    skippedDone += 1;
    beforeBytes += done.bytes;
    afterBytes += done.afterBytes;
    continue;
  }
  let doc;
  try {
    doc = await io.read(file);
  } catch (err) {
    console.error(`${rel}: skipped, could not be read (${err.message})`);
    skippedUnsafe += 1;
    continue;
  }
  const was = skeleton(doc);
  await doc.transform(dedup(), quantize(), reorder({ encoder: MeshoptEncoder }));
  doc.createExtension(EXTMeshoptCompression)
    .setRequired(true)
    .setEncoderOptions({ method: EXTMeshoptCompression.EncoderMethod.QUANTIZE });
  const out = withFloatPositionBounds(Buffer.from(await io.writeBinary(doc)), doc);
  if (skeleton(doc) !== was) {
    console.error(`${rel}: skipped, the primitive structure changed`);
    skippedUnsafe += 1;
    continue;
  }
  if (out.length >= original.length) {
    skippedNoGain += 1;
    beforeBytes += original.length;
    afterBytes += original.length;
    if (!dryRun) manifest.files[rel] = { sha256: hash, bytes: original.length, afterBytes: original.length, unchanged: true, at: new Date().toISOString() };
    continue;
  }
  beforeBytes += original.length;
  afterBytes += out.length;
  if (!dryRun) {
    fs.writeFileSync(file, out);
    manifest.files[rel] = { sha256: sha256(out), bytes: original.length, afterBytes: out.length, source: hash, at: new Date().toISOString() };
  }
  prepared += 1;
  console.log(`${rel}: ${original.length} -> ${out.length} (${((out.length - original.length) / original.length * 100).toFixed(1)}%)`);
}

if (!dryRun) {
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
}

const pct = beforeBytes ? ((afterBytes - beforeBytes) / beforeBytes) * 100 : 0;
console.log(
  `\n${dryRun ? "would prepare" : "prepared"} ${prepared}, already done ${skippedDone}, no gain ${skippedNoGain}, left alone ${skippedUnsafe}\n` +
    `${(beforeBytes / 1048576).toFixed(1)} MiB -> ${(afterBytes / 1048576).toFixed(1)} MiB (${pct.toFixed(1)}%)`,
);
