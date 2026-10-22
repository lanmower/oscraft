#!/usr/bin/env node
// Writes <packs-dir>/latest.json: the pack this site published, so a loader can find it without
// knowing the version. It reads what pack-package.mjs left in the directory rather than restating
// the name, the version and the part list in shell, and it fails when a piece the index names is
// missing or the wrong size, because a pack that cannot be joined is worse than no pack.
//
// Usage: node scripts/pack-latest.mjs <packs-dir>
//   GITHUB_SHA and GITHUB_RUN_ID, when set, are recorded on the entry.
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const dir = process.argv[2];
if (!dir) {
  console.error("usage: node scripts/pack-latest.mjs <packs-dir>");
  process.exit(2);
}
const packsDir = path.resolve(dir);
const entries = fs.readdirSync(packsDir);

const indexName = entries.find((f) => f.endsWith(".parts.json"));
const split = indexName
  ? JSON.parse(fs.readFileSync(path.join(packsDir, indexName), "utf8"))
  : null;

let name;
let version;
let archive;
let size;
let sha256;
let parts = null;
let partBytes = null;

if (split) {
  if (split.schema !== 1) throw new Error(`${indexName}: schema ${split.schema} is not 1`);
  ({ name, version, archive, size, sha256 } = split);
  partBytes = split.partBytes;
  parts = split.parts.map((part) => {
    const abs = path.join(packsDir, part.file);
    if (!fs.existsSync(abs)) throw new Error(`${indexName} names ${part.file}, which is missing`);
    const onDisk = fs.statSync(abs).size;
    if (onDisk !== part.size) throw new Error(`${part.file}: ${onDisk} bytes on disk, index says ${part.size}`);
    return { ...part, sha256: part.sha256 ?? crypto.createHash("sha256").update(fs.readFileSync(abs)).digest("hex") };
  });
  const joined = parts.reduce((n, p) => n + p.size, 0);
  if (joined !== size) throw new Error(`${archive}: parts total ${joined} bytes, index says ${size}`);
} else {
  const whole = entries.filter((f) => /\.(tar\.gz|tgz)$/i.test(f)).sort();
  if (whole.length === 0) throw new Error(`${packsDir}: no pack archive and no parts index`);
  archive = whole[0];
  const bytes = fs.readFileSync(path.join(packsDir, archive));
  size = bytes.length;
  sha256 = crypto.createHash("sha256").update(bytes).digest("hex");
  const match = /^(.+?)-([0-9][^-]*)\.(?:tar\.gz|tgz)$/i.exec(archive);
  if (!match) throw new Error(`${archive}: cannot read the name and version from the file name`);
  [, name, version] = match;
}

const latest = {
  name,
  version,
  file: archive,
  size,
  sha256,
  partBytes,
  parts,
  sha: process.env.GITHUB_SHA ?? null,
  run: process.env.GITHUB_RUN_ID ?? null,
  built: new Date().toISOString().replace(/\.\d+Z$/, "Z"),
};
fs.writeFileSync(path.join(packsDir, "latest.json"), `${JSON.stringify(latest, null, 2)}\n`);
console.log(
  `latest.json: ${archive} ${size} bytes${parts ? ` in ${parts.length} parts of at most ${partBytes}` : " in one part"}`,
);
