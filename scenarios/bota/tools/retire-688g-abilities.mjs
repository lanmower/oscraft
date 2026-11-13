import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { RETIRED_ABILITIES, RETIRED_ICON_STEMS } from "./retired-abilities.mjs";

const moba = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const root = path.resolve(moba, "..", "..");
const check = process.argv.includes("--check");
const retired = (name) => RETIRED_ABILITIES.has(name);
const retiredIcon = (relPath) => RETIRED_ICON_STEMS.has(path.basename(relPath, ".png"));

const ABILITY_DEFINITION = /^(ability|skill|ability_ext|ability_scaled)\("([^"]+)"/;
const ABILITY_LIST = /, abilities=\[([^\]]*)\]/;

const dropDefinitionLines = (text) =>
  text
    .split("\n")
    .filter((line) => {
      const m = ABILITY_DEFINITION.exec(line);
      return !(m && retired(m[2]));
    })
    .join("\n");

const dropListEntries = (text) =>
  text
    .split("\n")
    .map((line) => {
      const m = ABILITY_LIST.exec(line);
      if (!m) return line;
      const names = m[1].split(",").map((s) => s.trim().replace(/^"|"$/g, "")).filter(Boolean);
      if (!names.some(retired)) return line;
      const kept = names.filter((name) => !retired(name));
      const list = kept.length ? `, abilities=[${kept.map((name) => `"${name}"`).join(", ")}]` : "";
      return line.replace(ABILITY_LIST, list);
    })
    .join("\n");

const dropRowsByColumn = (column) => (text) =>
  text
    .split("\n")
    .filter((line) => !retired(line.split("\t")[column]))
    .join("\n");

const dropManifestEntries = (text) => {
  const manifest = JSON.parse(text);
  for (const key of Object.keys(manifest.files)) if (retiredIcon(key)) delete manifest.files[key];
  return `${JSON.stringify(manifest, null, 2)}\n`;
};

const TARGETS = [
  [path.join(moba, "abilities_688g_b05.osd"), dropDefinitionLines],
  [path.join(moba, "abilities_688g_b07.osd"), dropDefinitionLines],
  [path.join(moba, "units_heroes.osd"), dropListEntries],
  [path.join(moba, "ability-688g-ids.tsv"), dropRowsByColumn(1)],
  [path.join(moba, "assets/icons/ABILITY-ICONS.tsv"), dropRowsByColumn(0)],
  [path.join(moba, "abilities_688g_reference.tsv"), dropRowsByColumn(0)],
  [path.join(root, "scripts/assets/prep-manifest-icons.json"), dropManifestEntries],
];
const ICON_FILES = [...RETIRED_ICON_STEMS].map((stem) => path.join(moba, "assets/icons/abilities", `${stem}.png`));

const stale = [];
for (const [file, rewrite] of TARGETS) {
  const before = fs.readFileSync(file, "utf8");
  const after = rewrite(before);
  const name = path.relative(root, file);
  if (check) {
    if (after !== before) stale.push(name);
    continue;
  }
  if (after !== before) fs.writeFileSync(file, after);
  console.log(`${name}: ${after === before ? "unchanged" : "retired entries removed"}`);
}
for (const file of ICON_FILES) {
  if (!fs.existsSync(file)) continue;
  const name = path.relative(root, file);
  if (check) {
    stale.push(name);
    continue;
  }
  fs.unlinkSync(file);
  console.log(`${name}: removed`);
}
if (check) {
  if (stale.length) {
    console.error(`retired ability entries still present in: ${stale.join(", ")}`);
    process.exit(1);
  }
  console.log(`no retired ability entries in ${TARGETS.length} files or ${ICON_FILES.length} icon paths`);
}
