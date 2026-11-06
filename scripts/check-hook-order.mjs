import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const moba = resolve(root, "scenarios/moba");
const init = readFileSync(resolve(moba, "init.lua"), "utf8");
const manifest = JSON.parse(readFileSync(resolve(moba, "manifest.json"), "utf8"));
const requires = [...init.matchAll(/^require\("([^"]+)"\)/gm)].map((m) => m[1]);
const modules = manifest.modules ?? {};
if (requires.length < 60) throw new Error("init.lua lost its hook order");
if (requires[0] !== "aegis") throw new Error("aegis must load first");
if (requires.at(-1) !== "unit_rules") throw new Error("unit_rules must load last");
const unknown = requires.filter((name) => !modules[name]);
if (unknown.length) throw new Error("init.lua requires unlisted modules: " + unknown.join(", "));
const duplicates = requires.filter((name, i) => requires.indexOf(name) !== i);
if (duplicates.length) throw new Error("init.lua requires a module twice: " + duplicates.join(", "));
if (!(requires.indexOf("heroes") < requires.indexOf("kill_gold")) || !(requires.indexOf("kill_gold") < requires.indexOf("unit_rules"))) throw new Error("death hooks are out of order");
const EXTRA = ["bots", "bottle", "pings", "runes", "spell_kit", "chat_lobby"];
const witnesses = [];
for (const name of EXTRA) {
  const rel = modules[name];
  if (!rel) throw new Error(name + " is not a pack module");
  if (requires.includes(name)) throw new Error(name + " must not be required by init.lua");
  const src = readFileSync(resolve(moba, rel), "utf8");
  const hooks = [...new Set([...src.matchAll(/\bon_[a-z_]+/g)].map((m) => m[0]))];
  if (!hooks.length) throw new Error(name + " references no hooks");
  witnesses.push(name + "=" + hooks.length + "hooks");
}
console.log("hook order ok " + requires.length + " requires, aegis first, unit_rules last, extra " + witnesses.join(" "));
