import { execFile } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
const run = promisify(execFile);
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const bin = resolve(root, "rust/target/debug/mod_check");
if (!existsSync(bin)) throw new Error("mod_check is missing; build it with cargo build -p mod-host --bin mod_check");
const mods = readdirSync(resolve(root, "mods")).filter((f) => f.endsWith(".lua"));
const results = await Promise.all(mods.map(async (mod) => {
  try {
    const r = await run(bin, ["scenarios/moba", "mods/" + mod, "1"], { cwd: root, timeout: 30000, maxBuffer: 8 * 1024 * 1024 });
    const out = (r.stdout || "") + (r.stderr || "");
    return out.includes("ev=mod_hook_failed") || out.includes("ev=mod_load_failed") ? mod + ": hook failed" : null;
  } catch (err) {
    return mod + ": " + String(err.message || err).slice(0, 80);
  }
}));
const failures = results.filter(Boolean);
if (failures.length) throw new Error("mod_check failed on " + failures.join(", "));
console.log("example mods ok " + mods.length + " mods load, tick and report no hook failures");
