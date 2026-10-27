#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const flag = (name) => args.find((arg) => arg.startsWith(`--${name}=`))?.slice(name.length + 3);
const [assetsArg, outArg] = args.filter((arg) => !arg.startsWith("--"));
if (!assetsArg || !outArg) {
  console.error("usage: node scripts/assets/build-packs.mjs <assets-dir> <out-dir> [--toktx=<path>] [--python=<path>]");
  process.exit(2);
}
const toktx = flag("toktx") ?? process.env.TOKTX ?? "toktx";
const python = flag("python") ?? process.env.PYTHON ?? "python3";
const atlasScript = path.join(here, "atlas.py");
const CELL = 64;
const WEBP_QUALITY = 80;
const KTX2_OPTIONS = ["--t2", "--genmipmap", "--assign_oetf", "srgb", "--encode", "etc1s", "--qlevel", "192", "--clevel", "2", "--threads", "1"];
const assets = path.resolve(assetsArg);
const out = path.resolve(outArg);
const MAGIC = Buffer.from("OSPK");
const VERSION = 1;
const FIXED_BYTES = 12;

const readAsset = (rel) => fs.readFileSync(path.join(assets, rel));
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const pngsIn = (dir) => fs.readdirSync(path.join(assets, dir)).filter((name) => name.endsWith(".png")).sort();

const voices = JSON.parse(readAsset("voices/manifest.json").toString("utf8"));
const heroKeys = Object.keys(voices).sort();

const heroPacks = new Map();
const entriesOf = (key) => {
  if (!heroPacks.has(key)) heroPacks.set(key, new Map());
  return heroPacks.get(key);
};

for (const key of heroKeys) {
  for (const value of Object.values(voices[key])) {
    if (typeof value !== "string" || !value.endsWith(".ogg")) continue;
    entriesOf(key).set(`voices/${value}`, readAsset(`voices/${value}`));
  }
}

const abilityRows = readAsset("icons/ABILITY-ICONS.tsv").toString("utf8").split("\n")
  .filter((line) => line && !line.startsWith("#"))
  .map((line) => line.split("\t"))
  .filter(([ability]) => ability !== "ability");
const skillDefs = new Map();
const skillIcons = new Map();
for (const [ability, osd, icon] of abilityRows) {
  const owner = heroKeys.filter((key) => ability.startsWith(`${key}_`)).sort((a, b) => b.length - a.length)[0];
  const shipped = icon && icon !== "GAP" ? `icons/${icon}` : null;
  if (shipped) skillIcons.set(shipped, path.join(assets, shipped));
  if (owner === undefined) continue;
  if (!skillDefs.has(owner)) skillDefs.set(owner, []);
  skillDefs.get(owner).push({ ability, osd, icon: shipped });
}
for (const [owner, rows] of skillDefs) {
  entriesOf(owner).set(`skills/${owner}.json`, Buffer.from(`${JSON.stringify(rows)}\n`));
}

const rosterModels = [...new Set([...fs.readFileSync(path.join(assets, "..", "units_heroes.osd"), "utf8").matchAll(/^unit\(.*?\bmodel="([^"]+)"/gm)].map((m) => m[1]))].sort();
const heroIcons = new Map(rosterModels.map((model) => [`icons/heroes/${model}.png`, path.join(assets, "icons/heroes", `${model}.png`)]));
const itemIcons = new Map(pngsIn("icons/items").map((file) => [`icons/items/${file}`, path.join(assets, "icons/items", file)]));

const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "oscraft-atlas-"));
const index = { schema: 2, atlases: {}, packs: {}, entries: {} };
const placed = [];

function hashedName(stem, ext, bytes) {
  return `${stem}.${sha256(bytes).slice(0, 16)}.${ext}`;
}

function writeOut(file, bytes) {
  fs.writeFileSync(path.join(out, file), bytes);
  index.packs[file] = { sha256: sha256(bytes), bytes: bytes.length };
  placed.push(file);
}

function buildAtlas(name, icons) {
  const spec = { name, cell: CELL, quality: WEBP_QUALITY, out_dir: scratch, icons: [...icons].map(([rel, file]) => ({ name: rel, path: file })) };
  const layout = JSON.parse(execFileSync(python, [atlasScript], { input: JSON.stringify(spec), maxBuffer: 256 * 1024 * 1024 }).toString("utf8"));
  const ktxScratch = path.join(scratch, `${name}.ktx2`);
  execFileSync(toktx, [...KTX2_OPTIONS, ktxScratch, path.join(scratch, `${name}.png`)], { stdio: ["ignore", "ignore", "inherit"] });
  const webpBytes = fs.readFileSync(path.join(scratch, `${name}.webp`));
  const ktxBytes = fs.readFileSync(ktxScratch);
  const webp = hashedName(name, "webp", webpBytes);
  const ktx2 = hashedName(name, "ktx2", ktxBytes);
  writeOut(webp, webpBytes);
  writeOut(ktx2, ktxBytes);
  index.atlases[name] = { webp, ktx2, cell: layout.cell, cols: layout.cols, rows: layout.rows, width: layout.width, height: layout.height, icons: layout.icons };
  return { icons: Object.keys(layout.icons).length, webp: webpBytes.length, ktx2: ktxBytes.length };
}

function buildPack(entries) {
  const sorted = [...entries.keys()].sort();
  let cursor = 0;
  const table = sorted.map((rel) => {
    const row = { path: rel, offset: cursor, length: entries.get(rel).length };
    cursor += row.length;
    return row;
  });
  const header = Buffer.from(JSON.stringify({ schema: 1, entries: table }));
  const fixed = Buffer.alloc(FIXED_BYTES);
  MAGIC.copy(fixed, 0);
  fixed.writeUInt32LE(VERSION, 4);
  fixed.writeUInt32LE(header.length, 8);
  const bytes = Buffer.concat([fixed, header, ...sorted.map((rel) => entries.get(rel))]);
  const payloadStart = FIXED_BYTES + header.length;
  return { bytes, table: table.map((row) => [row.path, payloadStart + row.offset, row.length]) };
}

function placeCharacter(key, entries) {
  const { bytes, table } = buildPack(entries);
  const file = hashedName(`character/${key}`, "bin", bytes);
  writeOut(file, bytes);
  index.packs[file].entries = table.length;
  for (const [rel, offset, length] of table) index.entries[rel] = [file, offset, length];
}

fs.mkdirSync(path.join(out, "character"), { recursive: true });
const atlases = {
  heroes: buildAtlas("heroes", heroIcons),
  items: buildAtlas("items", itemIcons),
  skills: buildAtlas("skills", skillIcons),
};
for (const key of [...heroPacks.keys()].sort()) {
  if (heroPacks.get(key).size > 0) placeCharacter(key, heroPacks.get(key));
}
fs.writeFileSync(path.join(out, "index.json"), `${JSON.stringify(index)}\n`);
fs.rmSync(scratch, { recursive: true, force: true });

const characters = placed.filter((file) => file.startsWith("character/")).length;
const bytes = placed.reduce((sum, file) => sum + index.packs[file].bytes, 0);
const atlasLine = Object.entries(atlases).map(([name, a]) => `${name} ${a.icons} icons (webp ${a.webp} B, ktx2 ${a.ktx2} B)`).join(", ");
console.log(`asset packs: atlases ${atlasLine}; ${characters} character packs; ${bytes} bytes -> ${out}`);
