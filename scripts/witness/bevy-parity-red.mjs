import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const checker = path.join(root, "scripts", "check-bevy-parity.mjs");
const scratch = path.join(root, "..", "tmp", "oscraft", "parity");
fs.mkdirSync(scratch, { recursive: true });
const real = { caps: path.join(root, "docs", "CAPABILITIES.md"), doc: path.join(root, "docs", "OSCRAFT_ON_BEVY_PARITY.md"), targets: path.join(root, "scripts", "bevy-parity", "targets.json") };
const caps = fs.readFileSync(real.caps, "utf8");
const doc = fs.readFileSync(real.doc, "utf8");
const targets = JSON.parse(fs.readFileSync(real.targets, "utf8"));
const run = (options) => spawnSync(process.execPath, [checker, ...Object.entries(options).map(([key, value]) => `--${key}=${value}`)], { encoding: "utf8" });
const scratchFile = (name, text) => {
  const file = path.join(scratch, name);
  fs.writeFileSync(file, text);
  return file;
};
const failures = [];
const expect = (name, ok) => {
  console.log(`${ok ? "ok" : "FAIL"} ${name}`);
  if (!ok) failures.push(name);
};

let result = run(real);
expect("the real tree passes with every row targeted", result.status === 0 && result.stdout.includes("bevy parity ok"));

result = run({ ...real, caps: scratchFile("caps-no-row.md", caps.split("\n").filter((line) => !line.startsWith("| CAP-MAP-1 |")).join("\n")) });
expect("a removed row fails and names the id", result.status === 1 && result.stderr.includes("CAP-MAP-1"));

result = run({ ...real, caps: scratchFile("caps-new-row.md", caps + "\n| CAP-NEW-1 | A new capability | claimed | none | none |\n") });
expect("a row with no target fails and names the id", result.status === 1 && result.stderr.includes("CAP-NEW-1 has no Bevy target"));

result = run({ ...real, caps: scratchFile("caps-dup.md", caps + "\n| CAP-MAP-1 | Duplicate row | claimed | none | none |\n") });
expect("a duplicate id fails and names the id", result.status === 1 && result.stderr.includes("duplicate capability id CAP-MAP-1"));

const badStage = JSON.parse(JSON.stringify(targets));
badStage.categories.MAP.stage = "S9";
result = run({ ...real, targets: scratchFile("targets-bad-stage.json", JSON.stringify(badStage)) });
expect("an unknown stage fails and names the id", result.status === 1 && result.stderr.includes("CAP-MAP-1 names unknown stage S9"));

const badGame = JSON.parse(JSON.stringify(targets));
badGame.games.push({ scenario: "no-such-game", target: "missing", stage: "S1" });
result = run({ ...real, targets: scratchFile("targets-bad-game.json", JSON.stringify(badGame)) });
expect("a game with no scenario directory fails and names it", result.status === 1 && result.stderr.includes("no-such-game"));

const emptied = doc.split("\n").map((line) => (line.startsWith("| CAP-MAP-1 |") ? line.replace(/\| S1 \| planned \|$/, "| | planned |") : line)).join("\n");
expect("the red doc edit changed the row", emptied !== doc);
result = run({ ...real, doc: scratchFile("doc-emptied.md", emptied) });
expect("a doc row that drifts from the ledger fails and names the id", result.status === 1 && result.stderr.includes("drift") && result.stderr.includes("CAP-MAP-1"));

if (failures.length) {
  console.error(`bevy parity red tests failed: ${failures.join("; ")}`);
  process.exit(1);
}
console.log("bevy parity red tests ok (7 cases)");
