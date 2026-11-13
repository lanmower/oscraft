import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const moba = path.join(here, "..");
const args = process.argv.slice(2);
const check = args.includes("--check");
const [iniPath, outPath = path.join(moba, "ability_text_688g.lua")] = args.filter((arg) => arg !== "--check");
if (!iniPath) {
  console.error("usage: gen-ability-text-688g.mjs [--check] <6.88g ability.ini> [out.lua]");
  process.exit(2);
}

const sections = new Map();
let current = null;
let inUbertip = false;
for (const line of fs.readFileSync(iniPath, "utf8").split("\n")) {
  const header = /^\[([A-Za-z0-9]+)\]\s*$/.exec(line);
  if (header) {
    current = sections.has(header[1]) ? null : { name: null, ubertip: [] };
    if (current) sections.set(header[1], current);
    inUbertip = false;
    continue;
  }
  if (current === null) continue;
  const name = /^Name = "(.*)"\s*$/.exec(line);
  if (name) current.name = name[1];
  const single = /^Ubertip = "(.*)"\s*$/.exec(line);
  if (single) current.ubertip = [single[1]];
  if (/^Ubertip = \{\s*$/.test(line)) {
    inUbertip = true;
    continue;
  }
  if (inUbertip) {
    if (/^\}/.test(line)) inUbertip = false;
    else {
      const entry = /^"(.*)",?\s*$/.exec(line);
      if (entry) current.ubertip.push(entry[1]);
    }
  }
}

const escapeRegExp = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const acronymByName = new Map([...fs.readFileSync(path.join(moba, "hero_acronyms.lua"), "utf8").matchAll(/^\s*\["([^"]+)"\] = "([^"]+)",/gm)].map((m) => [m[1], m[2]]));
const displayNames = [...acronymByName.keys()].filter((name) => /[A-Z \-]/.test(name)).sort((a, b) => b.length - a.length);
const displayNamePattern = new RegExp(`\\b(${displayNames.map(escapeRegExp).join("|")})\\b`, "g");

const TAG = /\|c[0-9a-fA-F]{8}|\|r/g;
const TAIL = /\s*(Cooldown|Mana per second|Mana cost):\s*[\d.]+(\s+seconds?)?/g;
function plain(raw) {
  return raw
    .replace(TAG, "")
    .replace(/\|n/g, " ")
    .replace(TAIL, "")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/[^\x20-\x7e]/g, "?")
    .replace(/"/g, "'")
    .replace(/\\/g, "/")
    .replace(displayNamePattern, (name) => acronymByName.get(name));
}

const rows = fs.readFileSync(path.join(moba, "abilities_688g_reference.tsv"), "utf8").trim().split("\n");
const header = rows.shift().split("\t");
const [nameCol, codeCol, rangeCol] = ["oscraft_ability", "code", "range"].map((col) => header.indexOf(col));
const overrides = fs.readFileSync(path.join(moba, "ability_text_688g_overrides.tsv"), "utf8").trim().split("\n").slice(1).map((line) => {
  const [ability, , name, range = "", ...tips] = line.split("\t");
  return { ability, name, range, tips: tips.filter(Boolean) };
});

const osdNames = new Set();
for (const file of fs.readdirSync(moba)) {
  if (!/^abilities.*\.osd$/.test(file)) continue;
  for (const m of fs.readFileSync(path.join(moba, file), "utf8").matchAll(/^ability\("([^"]+)"/gm)) osdNames.add(m[1]);
}

const quoted = (text) => `"${text}"`;
const long = (text) => {
  if (text.includes("]==]")) throw new Error(`description contains a long-bracket close: ${text}`);
  return `[==[${text}]==]`;
};
const list = (items) => `{${items.join(", ")}}`;
const NUMBER = /\d+(?:\.\d+)?/g;

function descriptionFields(tips) {
  if (tips.every((tip) => tip === tips[0])) return [`tips = ${list([long(tips[0])])}`];
  const parts = tips.map((tip) => tip.split(NUMBER));
  const numbers = tips.map((tip) => tip.match(NUMBER) ?? []);
  const sameText = parts.every((p) => p.length === parts[0].length && p.every((text, i) => text === parts[0][i]));
  const sameCount = numbers.every((n) => n.length === numbers[0].length);
  if (sameText && sameCount) {
    return [
      `tpl = ${list(parts[0].map(long))}`,
      `nums = ${list(numbers.map((n) => list(n.map(quoted))))}`,
    ];
  }
  return [`tips = ${list(tips.map(long))}`];
}

const records = rows.map((row) => {
  const cells = row.split("\t");
  return { name: cells[nameCol], section: sections.get(cells[codeCol]), range: cells[rangeCol] ?? "" };
});
const referenced = new Set(records.map((record) => record.name));
for (const override of overrides) {
  if (!referenced.has(override.ability)) records.push({ name: override.ability, section: { name: override.name, ubertip: override.tips }, range: override.range });
}

const entries = [];
const unmatched = [];
for (const { name, section, range } of records.sort((a, b) => a.name.localeCompare(b.name))) {
  const fields = [];
  if (!osdNames.has(name)) unmatched.push(name);
  if (section && section.name) fields.push(`name = "${plain(section.name)}"`);
  if (/^\d+(\|\d+)*$/.test(range)) fields.push(`range = {${range.split("|").join(", ")}}`);
  if (section && section.ubertip.length > 0) fields.push(...descriptionFields(section.ubertip.map(plain)));
  if (fields.length > 0) entries.push(`  ["${name}"] = { ${fields.join(", ")} },`);
}

const out = `return {\n${entries.join("\n")}\n}\n`;
const previous = fs.existsSync(outPath) ? fs.readFileSync(outPath, "utf8") : null;
if (check) {
  if (previous !== out) {
    console.error(`${path.basename(outPath)} differs from the 6.88g ability.ini`);
    process.exit(1);
  }
  console.log(`${path.basename(outPath)} matches`);
} else {
  fs.writeFileSync(outPath, out);
}
console.log(`${entries.length} abilities, ${out.length} bytes; ${unmatched.length} reference names not in an osd: ${unmatched.join(" ")}`);
