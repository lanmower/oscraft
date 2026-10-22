#!/usr/bin/env node
// Shrinks the pack's PNG icons to the size the page draws them at.
//
// Usage:
//   node scripts/assets/prep-icons.mjs                  # prepare every icon the manifest does not already list
//   node scripts/assets/prep-icons.mjs --dry-run         # report what would change, write nothing
//   node scripts/assets/prep-icons.mjs --force           # prepare every icon again, manifest or not
//   node scripts/assets/prep-icons.mjs --size=128        # longest edge to keep, default 128
//   node scripts/assets/prep-icons.mjs path/to/a.png     # prepare just those files
//
// widgets.js draws an icon at n.size CSS pixels on a side, 64 by default, so on a 2x display the page
// never needs more than 128 px of source. The icons arrive 256x256, four times the pixels anyone sees:
// measured over the 125 icons of this pack, 7.3 MiB -> 1.8 MiB, about 75% smaller, with no visible
// difference at any size the page draws.
//
// The manifest (prep-manifest-icons.json) records each file it has prepared, by content hash. A file
// whose hash is already there is skipped, and a file whose hash changed is prepared again, so adding
// one icon to a pack of hundreds costs one icon's worth of work. That is the same rule prep-glb.mjs
// follows: the manifest is the memory, not the file's mtime, which a checkout or a copy does not keep.
//
// The format stays PNG and the name stays the same, because a pack names its icons by path and nothing
// else is changed here. An icon already at or under --size is re-encoded once and then recorded as
// done, so a later run does not redo it.
import sharp from "sharp";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const here = path.dirname(new URL(import.meta.url).pathname);
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
    // Already at or under the size the page draws: nothing to give up, so it is recorded and left alone.
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
