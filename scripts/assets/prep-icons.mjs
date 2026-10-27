#!/usr/bin/env node
import sharp from "sharp";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..", "..");
const defaultDir = path.join(root, "scenarios", "moba", "assets", "icons");
const manifestPath = path.join(here, "prep-manifest-icons.json");

const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const force = args.includes("--force");
const sizeFlag = args.find((a) => a.startsWith("--size="));
const size = Number(sizeFlag ? sizeFlag.slice("--size=".length) : 128);
if (!Number.isInteger(size) || size < 16 || size > 1024) throw new Error(`--size must be an integer 16..1024, got ${sizeFlag}`);
const files = args.filter((a) => !a.startsWith("--"));

const sha256 = (buf) => crypto.createHash("sha256").update(buf).digest("hex");
const walk = (dir) => {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walk(abs));
    else if (entry.name.toLowerCase().endsWith(".png")) out.push(abs);
  }
  return out;
};

const targets = files.length ? files.map((f) => path.resolve(f)) : walk(defaultDir).sort();
const manifest = fs.existsSync(manifestPath)
  ? JSON.parse(fs.readFileSync(manifestPath, "utf8"))
  : { schema: 1, size, files: {} };

let prepared = 0;
let skippedDone = 0;
let skippedSmall = 0;
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
  const image = sharp(original, { limitInputPixels: false });
  const meta = await image.metadata();
  const longest = Math.max(meta.width ?? 0, meta.height ?? 0);
  if (longest <= size) {
    skippedSmall += 1;
    beforeBytes += original.length;
    afterBytes += original.length;
    if (!dryRun) manifest.files[rel] = { sha256: hash, bytes: original.length, afterBytes: original.length, unchanged: true, at: new Date().toISOString() };
    continue;
  }
  const out = await image
    .resize(size, size, { fit: "inside", withoutEnlargement: true })
    .png({ compressionLevel: 9, adaptiveFiltering: true })
    .toBuffer();
  beforeBytes += original.length;
  afterBytes += out.length;
  if (!dryRun) {
    fs.writeFileSync(file, out);
    manifest.files[rel] = { sha256: sha256(out), bytes: original.length, afterBytes: out.length, source: hash, at: new Date().toISOString() };
  }
  prepared += 1;
  console.log(`${rel}: ${meta.width}x${meta.height} ${original.length} -> ${out.length} (${((out.length - original.length) / original.length * 100).toFixed(1)}%)`);
}

if (!dryRun) {
  manifest.size = size;
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
}

const pct = beforeBytes ? ((afterBytes - beforeBytes) / beforeBytes) * 100 : 0;
console.log(
  `\n${dryRun ? "would prepare" : "prepared"} ${prepared}, already done ${skippedDone}, already small enough ${skippedSmall}\n` +
    `${(beforeBytes / 1048576).toFixed(1)} MiB -> ${(afterBytes / 1048576).toFixed(1)} MiB (${pct.toFixed(1)}%)`,
);
