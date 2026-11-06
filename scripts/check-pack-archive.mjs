import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import zlib from "node:zlib";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const packDir = resolve(root, "scenarios/moba");
const manifest = JSON.parse(fs.readFileSync(path.join(packDir, "manifest.json"), "utf8"));
const outDir = fs.mkdtempSync(path.join(os.tmpdir(), "pack-archive-check-"));

const fail = (what) => { throw new Error(what); };
const tarMembers = (buf) => {
  const out = new Map();
  for (let off = 0; off + 512 <= buf.length; off += 512) {
    const h = buf.subarray(off, off + 512);
    if (h.every((b) => b === 0)) break;
    let name = h.subarray(0, 100).toString("utf8").split("\0")[0];
    const prefix = h.subarray(345, 500).toString("utf8").split("\0")[0];
    if (prefix) name = prefix + "/" + name;
    const size = parseInt(h.subarray(124, 136).toString("utf8").split("\0")[0].trim(), 8) || 0;
    const start = off + 512;
    out.set(name, buf.subarray(start, start + size));
    off += Math.ceil(size / 512) * 512;
  }
  return out;
};

try {
  execFileSync(process.execPath, [path.join(root, "scripts/pack-package.mjs"), packDir, outDir], { encoding: "utf8", timeout: 180000 });
  const indexFile = fs.readdirSync(outDir).find((f) => f.endsWith(".parts.json"));
  let gz;
  if (indexFile) {
    const index = JSON.parse(fs.readFileSync(path.join(outDir, indexFile), "utf8"));
    gz = Buffer.concat(index.parts.map((p) => fs.readFileSync(path.join(outDir, p.file))));
  } else {
    gz = fs.readFileSync(path.join(outDir, `${manifest.name}-${manifest.version}.tar.gz`));
  }
  const members = tarMembers(zlib.gunzipSync(gz));
  const archived = JSON.parse(members.get("manifest.json").toString("utf8")) ?? fail("no manifest.json in the archive");
  const expected = new Map();
  for (const [stem, rel] of Object.entries(manifest.modules ?? {})) expected.set(rel, "modules:" + stem);
  for (const rel of manifest.tables ?? []) expected.set(rel, "tables");
  for (const rel of manifest.data ?? []) expected.set(rel, "data");
  for (const rel of manifest.mods ?? []) expected.set(rel, "mods");
  for (const key of ["map", "rules", "ui", "effects", "image"]) if (typeof manifest[key] === "string") expected.set(manifest[key], key);
  const missing = [...expected.keys()].filter((rel) => !members.has(rel));
  if (missing.length) fail("archive omits " + missing.length + " manifest files: " + missing.slice(0, 6).join(", "));
  if (Object.keys(archived.modules ?? {}).length !== Object.keys(manifest.modules ?? {}).length) fail("archived manifest lost module entries");
  if ((archived.tables ?? []).length !== (manifest.tables ?? []).length) fail("archived manifest lost table entries");
  console.log(`pack archive ok ${members.size} members, ${expected.size}/${expected.size} named files present`);
} finally {
  fs.rmSync(outDir, { recursive: true, force: true });
}
