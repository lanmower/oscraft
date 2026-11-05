import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(join(root, path), "utf8");

const FORBIDDEN = [
  ["README.md", /committed wasm|python3 -m http\.server|play\.html#moba/, "staged dev loop"],
  ["docs/HOST.md", /map images are not in git|compare\.mjs` fetches the live page/, "wasm is built, maps are rebuilt"],
  ["docs/ARCHITECTURE.md", /checks the live page against the build|ci\.yml` has two jobs|on_spawned.*on_pointer/s, "no live compare, no ci.yml, no hand-copied hook list"],
  ["docs/CAPABILITIES.md", /GATE_ONLY|runs the gate, so/, "historical gate labels"],
  ["docs/GAMEPLAY_PARITY.md", /tests\.yml` \(workflow_dispatch\) runs it/, "mapping check only"],
  ["docs/WIRE.md", /js\/wire\/test\.js` runs|js\/wire\/test\.js` round-trips/, "round-trip test retired"],
  ["docs/PACKAGING.md", /bash scripts\/package\/install-check\.sh|oscraftd|proves it installs/, "archive layout follows build.sh"],
  ["docs/LOG_SCHEMA.md", /CI job `log-schema` reads/, "no server log schema"],
  ["docs/BOT_AI.md", /API has no mana query/, "game.mana and game.max_mana exist"],
  ["docs/SHOP.md", /within 64 units of the hero/, "PICKUP_REACH_MILLI = 20312"],
  ["docs/GAME.md", /not yet applied by the damage code/, "magic_keep is applied"],
  ["docs/ABILITIES.md", /cargo run -p oscraft-gate/, "rust/gate is not a workspace member"],
  ["docs/ASSET_PIPELINE.md", /`ci\.yml` install/, "pages.yml installs the pinned tools"],
];

const REQUIRED = [
  ["docs/ARCHITECTURE.md", "HOOK_KEYS", "hook list is code"],
  ["docs/ARCHITECTURE.md", "IMPROVEMENTS.md", "one improvement list"],
  ["docs/DEVELOPMENT.md", "scripts/check-all.mjs", "one validation entry point"],
  ["README.md", "scripts/dev/stage.sh", "staged dev loop"],
];

const failures = [];
for (const [path, pattern, reason] of FORBIDDEN) {
  if (pattern.test(read(path))) failures.push(path + " still claims " + reason);
}
for (const [path, needle, reason] of REQUIRED) {
  if (!read(path).includes(needle)) failures.push(path + " is missing " + reason);
}
if (failures.length) {
  for (const line of failures) console.error("doc claim: " + line);
  process.exit(1);
}
console.log("doc claims ok " + (FORBIDDEN.length + REQUIRED.length) + "/" + (FORBIDDEN.length + REQUIRED.length));
