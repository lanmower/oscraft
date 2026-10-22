#!/usr/bin/env node
// Prepares the pack's GLB models for the page: dedup, quantize and reorder, then meshopt compression with the
// QUANTIZE method. Measured on two character models, 841 KB -> 355 KB and 840 KB -> 358 KB, about 58% smaller.
//
// Every model in the pack already carries EXT_meshopt_compression, so re-encoding alone changes nothing (0.0%):
// the saving is in the quantization and the reordering that happen before it.
//
// Usage:
//   node scripts/assets/prep-glb.mjs                 # prepare every model the manifest does not already list
//   node scripts/assets/prep-glb.mjs --dry-run       # report what would change, write nothing
//   node scripts/assets/prep-glb.mjs --force         # prepare every model again, manifest or not
//   node scripts/assets/prep-glb.mjs path/to/a.glb   # prepare just those files
//
// The manifest (prep-manifest-glb.json) records each file it has prepared, by content hash. A file whose hash is
// already there is skipped, and a file whose hash changed is prepared again, so adding one model to a pack of
// hundreds costs one model's worth of work. That is the "do not redo models that are already done" rule: the
// manifest is the memory, not the file's mtime, which a checkout or a copy does not preserve.
//
// A model is only written when its primitive structure is unchanged: the same semantics, vertex counts and index
// counts per primitive. Anything else is reported and left alone, because the page reads the first mesh's
// positions, normals, indices, joints and weights, and a model that does not keep them is not the same model.
import { NodeIO } from "@gltf-transform/core";
import { EXTMeshoptCompression, KHRMeshQuantization, KHRTextureBasisu } from "@gltf-transform/extensions";
import { MeshoptDecoder, MeshoptEncoder } from "meshoptimizer";
import { dedup, quantize, reorder } from "@gltf-transform/functions";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const here = path.dirname(new URL(import.meta.url).pathname);
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

// What the page reads from a model: one entry per primitive, of its attribute semantics and its counts.
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
  const out = Buffer.from(await io.writeBinary(doc));
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
