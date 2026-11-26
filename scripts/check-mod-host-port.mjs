import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const option = (name, fallback) => process.argv.slice(2).find((arg) => arg.startsWith(`--${name}=`))?.slice(name.length + 3) ?? fallback;
const crate = option("crate", path.join(root, "rust", "crates", "mod-host"));
const read = (file) => fs.readFileSync(file, "utf8");
const runtime = read(path.join(crate, "src", "runtime.rs"));
const lib = read(path.join(crate, "src", "lib.rs"));
const piccolo = read(path.join(crate, "src", "piccolo_host.rs"));
const testsDir = path.join(crate, "tests");
const errors = [];

const body = (text, header) => {
  const match = new RegExp(`${header}\\s*\\{([\\s\\S]*?)\\n\\}`).exec(text);
  if (!match) throw new Error(`no block for ${header}`);
  return match[1];
};
const methodNames = (text) => [...text.matchAll(/fn (\w+)\(/g)].map((match) => match[1]);

const traitNames = methodNames(body(runtime, "pub trait ModRuntime"));
const implNames = methodNames(body(runtime, "impl ModRuntime for ModHost"));
for (const name of traitNames) {
  if (!implNames.includes(name)) errors.push(`ModRuntime method ${name} is not implemented for ModHost`);
}
for (const name of implNames) {
  if (!traitNames.includes(name)) errors.push(`ModHost implements ${name}, which ModRuntime does not declare`);
}

let testFiles = 0;
for (const file of fs.readdirSync(testsDir).filter((name) => name.endsWith(".rs"))) {
  const text = read(path.join(testsDir, file));
  const calls = [...new Set([...text.matchAll(/\bhost\w*\.(\w+)\(/g)].map((match) => match[1]))];
  if (calls.length === 0) continue;
  testFiles += 1;
  if (!/\bModRuntime\b/.test(text)) errors.push(`${file} calls host methods but does not import ModRuntime`);
  for (const call of calls) {
    if (!traitNames.includes(call)) errors.push(`${file} calls host.${call}, which ModRuntime does not declare`);
  }
}

const globals = [...lib.match(/HOST_GLOBALS: &\[&str\] = &\[([\s\S]*?)\];/)[1].matchAll(/"(\w+)"/g)].map((match) => match[1]);
const events = [...lib.match(/EVENTS: \[\(&str, &str\); \d+\] = \[([\s\S]*?)\];/)[1].matchAll(/\("(\w+)",/g)].map((match) => match[1]);
for (const name of [...globals, ...events]) {
  const registered = piccolo.includes(`"${name}"`) || new RegExp(`env\\.${name}\\b`).test(piccolo);
  if (!registered) errors.push(`script API ${name} is not registered in piccolo_host.rs`);
}

if (errors.length) {
  for (const error of errors) console.error(`FAIL ${error}`);
  process.exit(1);
}
console.log(`mod-host port ok: ${traitNames.length} ModRuntime methods implemented for ModHost, ${testFiles} test files on the port, ${globals.length} globals and ${events.length} events registered`);
