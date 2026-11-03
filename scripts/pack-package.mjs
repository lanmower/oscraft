#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import zlib from "node:zlib";

const DEFAULT_PART_MB = 25;
const sha256 = (buf) => crypto.createHash("sha256").update(buf).digest("hex");

const args = process.argv.slice(2);
const build = args.includes("--build");
const partFlag = args.find((a) => a.startsWith("--part-size="));
const partMb = Number(partFlag ? partFlag.slice("--part-size=".length) : DEFAULT_PART_MB);
if (!Number.isFinite(partMb) || partMb < 1) throw new Error(`--part-size must be a number of MiB, got ${partFlag}`);
const PART_BYTES = Math.floor(partMb * 1024 * 1024);
const positional = args.filter((a) => a !== "--build" && !a.startsWith("--part-size="));
const [pkgArg, outArg] = positional;
if (!pkgArg || !outArg) {
  console.error("usage: node scripts/pack-package.mjs <package-dir> <out-dir> [--build] [--part-size=<MiB>]");
  process.exit(2);
}
const pkgDir = path.resolve(pkgArg);
const outDir = path.resolve(outArg);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(fs.readFileSync(path.join(pkgDir, "manifest.json"), "utf8"));
if (manifest.schema !== 1) throw new Error(`manifest schema ${manifest.schema} is not 1`);

const ALLOWED = new Set([
  ".lua", ".osd", ".json", ".txt", ".tsv", ".osdt",
  ".png", ".jpg", ".jpeg", ".webp", ".glb", ".gltf", ".bin", ".ogg", ".wav", ".ktx2",
]);
const checkKind = (rel) => {
  const ext = path.extname(rel).toLowerCase();
  if (!ALLOWED.has(ext)) throw new Error(`${rel}: a pack holds Lua, data and assets only (extension ${ext || "none"})`);
};

if (build) {
  const image = path.join(pkgDir, manifest.image);
  execFileSync("cargo", ["run", "-q", "-p", "mapexport", "--", pkgDir, image], { cwd: path.join(root, "rust"), stdio: "inherit" });
}

const stage = fs.mkdtempSync(path.join(os.tmpdir(), "pack-"));
const files = new Map();
const copyIn = (absSrc, archRel) => {
  if (fs.statSync(absSrc).isDirectory()) {
    for (const entry of fs.readdirSync(absSrc)) copyIn(path.join(absSrc, entry), path.posix.join(archRel, entry));
  } else {
    checkKind(archRel);
    files.set(archRel, absSrc);
  }
};
const inside = (abs) => abs === pkgDir || abs.startsWith(pkgDir + path.sep);
const place = (rel, archPrefix) => {
  const abs = path.resolve(pkgDir, rel);
  if (!fs.existsSync(abs)) throw new Error(`manifest names ${rel}, which is missing`);
  if (inside(abs)) {
    copyIn(abs, path.relative(pkgDir, abs).split(path.sep).join("/"));
    return path.relative(pkgDir, abs).split(path.sep).join("/");
  }
  const archRel = `${archPrefix}/${path.basename(abs)}`;
  copyIn(abs, archRel);
  return archRel;
};

const out = JSON.parse(JSON.stringify(manifest));
for (const key of ["map", "rules", "ui", "effects"]) if (typeof manifest[key] === "string") out[key] = place(manifest[key], "external");
out.data = manifest.data.map((rel) => place(rel, "external"));
out.mods = manifest.mods.map((rel) => place(rel, "external"));
out.fixtures = (manifest.fixtures ?? []).map((rel) => place(rel, "external"));
out.image = place(manifest.image, "external");
out.tables = [];
if (manifest.assets) {
  const assetRoot = path.resolve(pkgDir, manifest.assets.root ?? ".");
  const assetOut = { ...manifest.assets, root: "assets" };
  const copyAsset = (value) => {
    const abs = path.join(assetRoot, value);
    if (!fs.existsSync(abs)) throw new Error(`asset ${value} is missing under ${assetRoot}`);
    copyIn(abs, `assets/${value}`);
  };
  for (const [key, value] of Object.entries(manifest.assets)) {
    if (key === "root") continue;
    if (typeof value === "string") { copyAsset(value); assetOut[key] = value; }
    else for (const [k2, v2] of Object.entries(value)) { copyAsset(v2); assetOut[key][k2] = v2; }
  }
  out.assets = assetOut;
}

const manifestText = JSON.stringify(out, null, 2) + "\n";
const entries = [["manifest.json", Buffer.from(manifestText)], ...[...files].map(([rel, abs]) => [rel, fs.readFileSync(abs)])];
const header = (name, size) => {
  const h = Buffer.alloc(512, 0);
  let prefix = "";
  let base = name;
  if (Buffer.byteLength(name) > 100) {
    const cut = name.lastIndexOf("/", name.length - 2);
    prefix = name.slice(0, cut);
    base = name.slice(cut + 1);
    if (Buffer.byteLength(base) > 100 || Buffer.byteLength(prefix) > 155) throw new Error(`path too long for ustar: ${name}`);
  }
  h.write(base, 0, 100);
  h.write("0000644\0", 100);
  h.write("0000000\0", 108);
  h.write("0000000\0", 116);
  h.write(size.toString(8).padStart(11, "0") + "\0", 124);
  h.write("00000000000\0", 136);
  h.write("        ", 148);
  h.write("0", 156);
  h.write("ustar\0", 257);
  h.write("00", 263);
  h.write(prefix, 345, 155);
  let sum = 0;
  for (const b of h) sum += b;
  h.write(sum.toString(8).padStart(6, "0") + "\0 ", 148);
  return h;
};
const blocks = [];
for (const [name, data] of entries) {
  blocks.push(header(name, data.length), data, Buffer.alloc((512 - (data.length % 512)) % 512));
}
blocks.push(Buffer.alloc(1024));
const tar = Buffer.concat(blocks);
fs.mkdirSync(outDir, { recursive: true });
const gz = zlib.gzipSync(tar, { level: 9 });
const archive = `${manifest.name}-${manifest.version}.tar.gz`;

if (gz.length <= PART_BYTES) {
  fs.writeFileSync(path.join(outDir, archive), gz);
  console.log(`packed ${entries.length} files into ${archive} (${gz.length} bytes, one part)`);
} else {
  const count = Math.ceil(gz.length / PART_BYTES);
  const width = Math.max(2, String(count - 1).length);
  const parts = [];
  for (let i = 0; i < count; i += 1) {
    const slice = gz.subarray(i * PART_BYTES, Math.min((i + 1) * PART_BYTES, gz.length));
    const file = `${archive}.part${String(i).padStart(width, "0")}`;
    fs.writeFileSync(path.join(outDir, file), slice);
    parts.push({ file, size: slice.length, sha256: sha256(slice) });
  }
  const index = {
    schema: 1,
    name: manifest.name,
    version: manifest.version,
    archive,
    partBytes: PART_BYTES,
    size: gz.length,
    sha256: sha256(gz),
    parts,
  };
  fs.writeFileSync(path.join(outDir, `${archive}.parts.json`), `${JSON.stringify(index, null, 2)}\n`);
  const biggest = Math.max(...parts.map((p) => p.size));
  console.log(
    `packed ${entries.length} files into ${archive} (${gz.length} bytes) as ${count} parts of at most ${biggest} bytes`,
  );
}
fs.rmSync(stage, { recursive: true, force: true });
