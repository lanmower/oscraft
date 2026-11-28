import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const pack = path.join(root, "scenarios", "duel-routes");
const manifest = JSON.parse(fs.readFileSync(path.join(pack, "manifest.json"), "utf8"));
const assets = manifest.assets;
const modelsDir = path.join(pack, assets.root, assets.models);
const failures = [];
const fail = (message) => failures.push(message);

const modelNames = new Set();
for (const file of fs.readdirSync(path.join(pack))) {
  if (!file.endsWith(".osd")) continue;
  for (const m of fs.readFileSync(path.join(pack, file), "utf8").matchAll(/model="([^"]+)"/g)) modelNames.add(m[1]);
}

for (const name of modelNames) {
  const glb = path.join(modelsDir, `${name}.glb`);
  if (!fs.existsSync(glb)) {
    fail(`model ${name}: ${path.relative(root, glb)} is missing`);
    continue;
  }
  const buf = fs.readFileSync(glb);
  const jsonLen = buf.readUInt32LE(12);
  const doc = JSON.parse(buf.subarray(20, 20 + jsonLen).toString("utf8"));
  for (const img of doc.images ?? []) {
    if (!img.uri) continue;
    const file = path.join(modelsDir, decodeURIComponent(img.uri));
    if (!fs.existsSync(file)) fail(`model ${name}: texture ${img.uri} is missing from ${path.relative(root, modelsDir)}`);
  }
  for (const b of doc.buffers ?? []) {
    if (b.uri && !b.uri.startsWith("data:") && !fs.existsSync(path.join(modelsDir, decodeURIComponent(b.uri)))) fail(`model ${name}: buffer ${b.uri} is missing`);
  }
  if ((doc.skins ?? []).length > 0) {
    for (const key of ["anim", "animMaps"]) {
      if (!assets[key]) fail(`model ${name} is skinned and the manifest has no assets.${key}`);
      else if (!fs.existsSync(path.join(pack, assets.root, assets[key]))) fail(`model ${name} is skinned and assets.${key} (${assets[key]}) is missing`);
    }
  }
}
for (const [key, value] of Object.entries(assets)) {
  if (typeof value !== "string" || key === "root" || key === "catalogue") continue;
  if (key === "textures" || typeof value !== "string") continue;
  if (!fs.existsSync(path.join(pack, assets.root, value))) fail(`assets.${key} (${value}) is missing`);
}
for (const [key, value] of Object.entries(assets.textures ?? {})) {
  if (!fs.existsSync(path.join(pack, assets.root, value))) fail(`assets.textures.${key} (${value}) is missing`);
}
if (failures.length) {
  for (const f of failures) console.error(`check-duel-assets: ${f}`);
  process.exit(1);
}
console.log(`check-duel-assets PASS: ${modelNames.size} unit models, every referenced texture and buffer present, skinned models have anim and animMaps`);
