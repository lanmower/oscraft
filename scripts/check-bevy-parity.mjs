import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { gameErrors, OWNER_GAP, parseCapabilities, planRows, renderDoc } from "./lib/bevy-parity.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const option = (name, fallback) => args.find((arg) => arg.startsWith(`--${name}=`))?.slice(name.length + 3) ?? fallback;
const capsPath = option("caps", path.join(root, "docs", "CAPABILITIES.md"));
const docPath = option("doc", path.join(root, "docs", "OSCRAFT_ON_BEVY_PARITY.md"));
const targetsPath = option("targets", path.join(root, "scripts", "bevy-parity", "targets.json"));

const targets = JSON.parse(fs.readFileSync(targetsPath, "utf8"));
const { plan, errors } = planRows(parseCapabilities(fs.readFileSync(capsPath, "utf8")), targets);
errors.push(...gameErrors(targets.games, root));
if (errors.length) {
  for (const error of errors) console.error(`FAIL ${error}`);
  process.exit(1);
}

const expected = renderDoc(plan, targets);
if (args.includes("--write")) {
  fs.writeFileSync(docPath, expected);
  console.log(`WROTE ${docPath} rows=${plan.length}`);
  process.exit(0);
}

const current = fs.existsSync(docPath) ? fs.readFileSync(docPath, "utf8") : "";
const rowIds = (text) => text.split("\n").filter((line) => line.startsWith("| CAP-")).map((line) => line.split("|")[1].trim());
const ledgerIds = new Set(plan.map((row) => row.id));
const docIds = new Set(rowIds(current));
const drifted = [
  ...[...docIds].filter((id) => !ledgerIds.has(id)).map((id) => `FAIL drift: ${id} is in ${docPath} but has no row in the capability ledger`),
  ...[...ledgerIds].filter((id) => !docIds.has(id)).map((id) => `FAIL drift: ${id} is in the capability ledger but has no row in ${docPath}`),
];
if (drifted.length === 0 && current !== expected) {
  const got = current.split("\n");
  const want = expected.split("\n");
  const line = want.findIndex((text, index) => got[index] !== text);
  drifted.push(`FAIL drift: ${docPath} differs from the capability ledger at line ${line + 1}: doc=[${got[line] ?? "missing"}] expected=[${want[line] ?? "missing"}]`);
}
if (drifted.length) {
  for (const message of drifted) console.error(message);
  process.exit(1);
}
console.log(`bevy parity ok: ${plan.length} capability rows each with a Bevy target, ${plan.filter((row) => row.stage === OWNER_GAP).length} owner-held gaps, ${targets.games.length} games`);
