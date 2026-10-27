import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const pack = join(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(readFileSync(join(pack, "manifest.json"), "utf8"));
const modules = manifest.modules ?? {};
const problems = [];

const luaFiles = [];
const walk = (dir) => {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      if (name !== "tools" && name !== "assets" && name !== "fixtures") walk(path);
    } else if (name.endsWith(".lua")) luaFiles.push(relative(pack, path));
  }
};
walk(pack);

for (const [stem, file] of Object.entries(modules)) {
  if (!existsSync(join(pack, file))) problems.push(`manifest.json: module ${stem} names ${file}, which does not exist`);
}
for (const root of manifest.mods ?? []) {
  if (!Object.values(modules).includes(root)) problems.push(`manifest.json: root module ${root} is not in modules`);
}

const mapped = new Set(Object.values(modules));
for (const file of luaFiles.sort()) {
  if (!mapped.has(file)) problems.push(`${file}: not in manifest.json modules, so the page cannot ship it`);
  const source = readFileSync(join(pack, file), "utf8").replace(/--[^\n]*/g, (c) => " ".repeat(c.length));
  const line = (index) => source.slice(0, index).split("\n").length;
  for (const m of source.matchAll(/\brequire\b\s*\(?\s*(["'])?([A-Za-z0-9_./-]*)(["'])?/g)) {
    const before = source.slice(Math.max(0, m.index - 12), m.index);
    if (/function\s*$/.test(before) || /\.\s*$/.test(before) || m[2] === "") continue;
    const literal = source.slice(m.index).match(/^require\("([^"]+)"\)/);
    if (!literal) problems.push(`${file}:${line(m.index)}: require is not the literal require("<stem>") form the page scans for`);
    else if (modules[literal[1]] === undefined) problems.push(`${file}:${line(m.index)}: require("${literal[1]}") names a stem manifest.json modules does not map`);
  }
}

for (const problem of problems) console.error(problem);
if (problems.length > 0) {
  console.error(`check-modules: ${problems.length} problem(s)`);
  process.exit(1);
}
console.log(`check-modules: ${Object.keys(modules).length} modules, ${luaFiles.length} pack files, every require resolves`);
