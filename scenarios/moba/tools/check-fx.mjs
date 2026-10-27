import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const read = (p) => readFileSync(join(root, p), "utf8");
const UI = "scenarios/moba/moba_ui.json";
const EFFECTS = "scenarios/moba/effects.lua";
const SIGS = "scenarios/moba/ability_fx.lua";
const BUFFS = "scenarios/moba/buff_fx.lua";
const FX = "js/client/fx.js";

const problems = [];
const fail = (file, text, line, msg) => problems.push(`${file}:${text.slice(0, line).split("\n").length}: ${msg}`);
const each = (re, text, fn) => { for (const m of text.matchAll(re)) fn(m, m.index); };

const uiText = read(UI);
const ui = JSON.parse(uiText);
const colours = ui.effects?.colours ?? {};
const kinds = ui.effects?.kinds ?? {};
const uiLine = (needle) => Math.max(0, uiText.indexOf(`"${needle}"`));

const fxText = read(FX);
const drawn = new Set(["projectile"]);
each(/case "(\w+)":/g, fxText, (m) => drawn.add(m[1]));

for (const [name, kind] of Object.entries(kinds)) {
  const at = uiLine(name);
  if (!drawn.has(kind.renderer)) fail(UI, uiText, at, `kind ${name}: renderer "${kind.renderer}" is not drawn by ${FX}`);
  if (kind.colour !== undefined && colours[kind.colour] === undefined) fail(UI, uiText, at, `kind ${name}: colour "${kind.colour}" is not in effects.colours`);
  for (const next of kind.arrive ?? []) if (kinds[next] === undefined) fail(UI, uiText, at, `kind ${name}: arrive "${next}" is not a kind`);
}

const luaText = read(EFFECTS);
const schoolList = luaText.match(/local SCHOOLS = \{([^}]*)\}/);
const schools = new Set(schoolList ? [...schoolList[1].matchAll(/"(\w+)"/g)].map((m) => m[1]) : []);
if (schools.size === 0) fail(EFFECTS, luaText, 0, "no SCHOOLS list found");
for (const s of schools) if (colours[s] === undefined) fail(EFFECTS, luaText, luaText.indexOf("local SCHOOLS"), `school "${s}" is not in effects.colours`);

const emitted = new Map();
const emit = (name, at) => { if (!emitted.has(name)) emitted.set(name, at); };
each(/game\.effect\(\s*"(\w+)"/g, luaText, (m, at) => emit(m[1], at));
each(/"(fx_\w*[a-z0-9])"/g, luaText, (m, at) => emit(m[1], at));
each(/\b[A-Z]+\[school\] = "(\w+_)" \.\. school/g, luaText, (m, at) => { for (const s of schools) emit(m[1] + s, at); });
for (const [name, at] of emitted) if (kinds[name] === undefined) fail(EFFECTS, luaText, at, `emits kind "${name}" that moba_ui.json effects.kinds lacks`);
each(/game\.effect\([^\n]*colour = "(\w+)"/g, luaText, (m, at) => {
  if (colours[m[1]] === undefined) fail(EFFECTS, luaText, at, `colour "${m[1]}" is not in effects.colours`);
});

const forms = new Set();
each(/form == "(\w+)"/g, luaText, (m) => forms.add(m[1]));
for (const table of ["CAST_KIND", "GROUND_KIND"]) {
  const body = luaText.match(new RegExp(`local ${table} = \\{([^}]*)\\}`));
  if (body) each(/(\w+) = "/g, body[1], (m) => forms.add(m[1]));
}

const sigText = read(SIGS);
each(/\["(\w+)"\] = \{ ([^}]*)\}/g, sigText, (m, at) => {
  const school = m[2].match(/school = "(\w+)"/)?.[1];
  const form = m[2].match(/form = "(\w+)"/)?.[1];
  if (!schools.has(school)) fail(SIGS, sigText, at, `${m[1]}: school "${school}" is not in effects.lua SCHOOLS`);
  if (!forms.has(form)) fail(SIGS, sigText, at, `${m[1]}: form "${form}" is not handled by effects.lua on_cast`);
});

const buffText = read(BUFFS);
each(/\["(\w+)"\] = \{ school = "(\w+)"/g, buffText, (m, at) => {
  if (colours[m[2]] === undefined) fail(BUFFS, buffText, at, `buff kind ${m[1]}: school "${m[2]}" is not in effects.colours`);
  else if (!schools.has(m[2])) fail(BUFFS, buffText, at, `buff kind ${m[1]}: school "${m[2]}" is not in effects.lua SCHOOLS`);
});
const buffKinds = new Set([...buffText.matchAll(/^    \["(\w+)"\] = \{ school/gm)].map((m) => m[1]));
each(/\["(\w+)"\] = \{ kind = "(\w+)"/g, buffText, (m, at) => {
  if (!buffKinds.has(m[2])) fail(BUFFS, buffText, at, `buff ability ${m[1]}: kind "${m[2]}" is not in buff_fx.kinds`);
});

const MANIFEST = "scenarios/moba/manifest.json";
const manifestText = read(MANIFEST);
const listed = new Set(Object.values(JSON.parse(manifestText).modules ?? {}));
for (const file of readdirSync(join(root, "scenarios/moba"), { recursive: true })) {
  const rel = String(file).replace(/\\/g, "/");
  if (rel.endsWith(".lua") && !rel.startsWith("tools/") && !rel.startsWith("fixtures/") && !listed.has(rel)) fail(MANIFEST, manifestText, 0, `scenarios/moba/${rel} is not in manifest modules, so the page cannot load it`);
}

if (problems.length > 0) {
  console.error(problems.join("\n"));
  console.error(`check-fx: ${problems.length} problem(s)`);
  process.exit(1);
}
console.log(`check-fx: ok (${Object.keys(kinds).length} kinds, ${emitted.size} emitted, ${schools.size} schools)`);
