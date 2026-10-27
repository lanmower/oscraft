import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

// mapexport feeds every manifest data[] entry to the Starlark data evaluator
// and resolves it relative to the scenario dir, so data[] must name only OSDT
// data files that exist there. The .lua and .mjs pollution regressions
// (forms.lua, hero_spells.lua, projectiles.lua, toggle_spells.lua, then
// scripts/gamesparks-bootstrap.mjs) all came from entries that violated this.
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const problems = [];
let manifests = 0;

const walk = (dir) => {
  if (!statSync(dir).isDirectory()) return;
  for (const name of readdirSync(dir)) {
    if (name === "tools" || name === "assets" || name === "fixtures" || name.startsWith(".")) continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) walk(path);
    else if (name === "manifest.json") check(dir);
  }
};

function check(dir) {
  const label = relative(root, join(dir, "manifest.json"));
  manifests += 1;
  const data = JSON.parse(readFileSync(join(dir, "manifest.json"), "utf8")).data ?? [];
  for (const entry of data) {
    const target = join(dir, entry);
    const rel = relative(dir, target);
    if (rel.startsWith("..") || rel.length === 0) {
      problems.push(`${label}: data entry ${entry} does not name a file inside the scenario dir`);
      continue;
    }
    if (!/\.(osd|tsv)$/.test(entry)) {
      problems.push(`${label}: data entry ${entry} is not an .osd or .tsv data file (mapexport sends data entries to the Starlark evaluator)`);
      continue;
    }
    if (!existsSync(target)) {
      problems.push(`${label}: data entry ${entry} does not exist in the scenario dir`);
    }
  }
}

walk(root);

for (const problem of problems) console.error(problem);
if (problems.length > 0) {
  console.error(`check-manifest-data: ${problems.length} problem(s)`);
  process.exit(1);
}
console.log(`check-manifest-data: ${manifests} manifest(s), every data entry is an .osd/.tsv in its scenario dir`);
