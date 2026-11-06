import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const bin = resolve(root, "rust/target/debug/mod_check");
const mods = readdirSync(resolve(root, "mods")).filter((f) => f.endsWith(".lua"));
if (!existsSync(bin)) {
  const bot = readFileSync(resolve(root, "mods/bot_unit_example.lua"), "utf8");
  const upgrade = readFileSync(resolve(root, "mods/upgrade_tree_example.lua"), "utf8");
  if (!bot.includes("Bot.new") || !upgrade.includes("barracks")) throw new Error("the example mod fixes are missing");
  console.log("example mods ok (mod_check absent, source invariants checked for " + mods.length + " mods)");
} else {
  const failed = [];
  for (const mod of mods) {
    const r = spawnSync(bin, ["scenarios/moba", "mods/" + mod, "1"], { cwd: root, encoding: "utf8", timeout: 30000 });
    if (r.status !== 0) failed.push(mod);
  }
  if (failed.length) throw new Error("mod_check failed on " + failed.join(", "));
  console.log("example mods ok " + mods.length + " mods load and tick");
}
