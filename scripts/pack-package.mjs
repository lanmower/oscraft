#!/usr/bin/env node
// Packs one game package (a directory with manifest.json at its root) into a single <name>-<version>.tar.gz.
// Usage: node scripts/pack-package.mjs <package-dir> <out-dir> [--build]
//   --build  runs mapexport first to write the manifest's image (the compiled map) into the package
// The pack holds only what the manifest names: Lua (.lua), data (.osd, .json, .txt), the compiled image and assets.
// Files the manifest names outside the package are copied in: assets under assets/ (the asset root is rewritten to
// "assets"), other files under external/. The archive's manifest.json is the rewritten one, so one loader reads both.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import zlib from "node:zlib";

const args = process.argv.slice(2);
const build = args.includes("--build");
const [pkgArg, outArg] = args.filter((a) => a !== "--build");
if (!pkgArg || !outArg) {
  console.error("usage: node scripts/pack-package.mjs <package-dir> <out-dir> [--build]");
  process.exit(2);
}
const pkgDir = path.resolve(pkgArg);
const outDir = path.resolve(outArg);
const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const manifest = JSON.parse(fs.readFileSync(path.join(pkgDir, "manifest.json"), "utf8"));
if (manifest.schema !== 1) throw new Error(`manifest schema ${manifest.schema} is not 1`);

const ALLOWED = new Set([".lua", ".osd", ".json", ".txt", ".osdt", ".glb", ".png", ".jpg", ".jpeg", ".ktx2"]);
const checkKind = (rel) => {
  const ext = path.extname(rel).toLowerCase();
  if (!ALLOWED.has(ext)) throw new Error(`${rel}: a pack holds Lua, data and assets only (extension ${ext || "none"})`);
};

if (build) {
  const image = path.join(pkgDir, manifest.image);
  execFileSync("cargo", ["run", "-q", "-p", "mapexport", "--", pkgDir, image], { cwd: path.join(root, "rust"), stdio: "inherit" });
}

// Staging directory outside the repo; removed at the end.
const stage = fs.mkdtempSync(path.join(os.tmpdir(), "pack-"));
const files = new Map(); // archive path -> absolute source path
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

// Pack the rewritten manifest and every file, as ustar.
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
const dest = path.join(outDir, `${manifest.name}-${manifest.version}.tar.gz`);
fs.writeFileSync(dest, zlib.gzipSync(tar, { level: 9 }));
fs.rmSync(stage, { recursive: true, force: true });
console.log(`packed ${entries.length} files into ${dest}`);
