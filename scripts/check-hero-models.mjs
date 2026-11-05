import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const heroesFile = join(root, "scenarios", "moba", "units_heroes.osd");
const text = readFileSync(heroesFile, "utf8");

const unitsOf = (source) => [...source.matchAll(/^unit\("(\w+)".*?model="([^"]*)"/gm)].map((m) => ({ id: m[1], model: m[2] }));
const defectsOf = (source) => {
  const units = unitsOf(source);
  const ids = new Set();
  const duplicates = [];
  for (const unit of units) { if (ids.has(unit.id)) duplicates.push(unit.id); ids.add(unit.id); }
  return { count: units.length, empty: units.filter((u) => !u.model).map((u) => u.id), duplicates: duplicates };
};

if (process.argv.includes("--negative-fixture")) {
  const broken = text.replace(/model="[^"]*"/, "model=\"\"");
  const defects = defectsOf(broken);
  if (!defects.empty.length) { console.error("hero model negative fixture was not detected"); process.exit(1); }
  console.log("hero models negative fixture caught");
  process.exit(0);
}

const defects = defectsOf(text);
if (defects.count < 100 || defects.empty.length || defects.duplicates.length) {
  console.error("hero models: count=" + defects.count + " empty=" + defects.empty.join(",") + " duplicates=" + defects.duplicates.join(","));
  process.exit(1);
}
console.log("hero models ok: " + defects.count + " heroes, all with a model");
