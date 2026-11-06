import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const wit = readFileSync(join(root, "rust/crates/wasip3/wit/sim-component.wit"), "utf8");
const lib = readFileSync(join(root, "rust/crates/wasip3/src/lib.rs"), "utf8");
const exports = [...wit.matchAll(/^\s*export\s+([a-z0-9-]+)\s*:/gm)].map((m) => m[1]).sort();
const block = /pub const COMMANDS: &\[&str\] = &\[([\s\S]*?)\];/.exec(lib);
if (!block) throw new Error("wasip3 COMMANDS block is missing");
const commands = [...block[1].matchAll(/"([a-z0-9-]+)"/g)].map((m) => m[1]).sort();
const missing = exports.filter((name) => !commands.includes(name));
const extra = commands.filter((name) => !exports.includes(name));
if (missing.length || extra.length) throw new Error("WIT/COMMANDS drift missing=" + missing.join() + " extra=" + extra.join());
if (exports.length !== 20) throw new Error("expected 20 WIT exports, found " + exports.length);
console.log("wasip3 parity ok " + exports.length + " exports = " + commands.length + " commands");
