import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const play = readFileSync(join(root, "js/client/play.js"), "utf8");
const block = play.slice(play.indexOf("installDebugApi({"), play.indexOf("installDebugApi({") + 4000);
const GATED_HOOKS = ["run", "order", "attack", "setClassOverride", "setAttackHold"];

const missing = GATED_HOOKS.filter((name) => {
  const start = block.indexOf("  " + name + ": ");
  if (start < 0) return true;
  const end = block.indexOf("\n  ", start + 4);
  const body = block.slice(start, end < 0 ? start + 400 : end + 300);
  return !body.includes("gate.live");
});

if (missing.length > 0) {
  console.error("debug api hooks that act before the start gate opens: " + missing.join(", "));
  process.exit(1);
}
console.log("debug api hooks are gated on gate.live: " + GATED_HOOKS.join(", "));
