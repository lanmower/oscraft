import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const moba = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const check = process.argv.includes("--check");
const TOWER_BODY_MILLI = { tower: 4500, tower_t2: 4500, tower_t3: 4500 };
const file = path.join(moba, "units.osd");

const before = fs.readFileSync(file, "utf8");
const seen = new Set();
const after = before
  .split("\n")
  .map((line) => {
    const m = /^unit\("(\w+)", \d+, \d+, \d+/.exec(line);
    if (!m || !Object.hasOwn(TOWER_BODY_MILLI, m[1])) return line;
    seen.add(m[1]);
    return line.replace(/^(unit\("\w+", \d+, \d+, )\d+/, `$1${TOWER_BODY_MILLI[m[1]]}`);
  })
  .join("\n");

const missing = Object.keys(TOWER_BODY_MILLI).filter((name) => !seen.has(name));
if (missing.length > 0) {
  console.error(`units.osd has no unit line for: ${missing.join(", ")}`);
  process.exit(1);
}
if (check) {
  if (after !== before) {
    console.error("units.osd tower bodies differ from TOWER_BODY_MILLI");
    process.exit(1);
  }
  console.log(`${seen.size} tower bodies: units.osd is current`);
  process.exit(0);
}
fs.writeFileSync(file, after);
console.log(`${seen.size} tower bodies written`);
