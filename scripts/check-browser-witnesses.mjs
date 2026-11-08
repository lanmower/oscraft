import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const doc = readFileSync(resolve(root, "docs/BROWSER_WITNESSES.md"), "utf8");
for (const witness of ["HMR", "finished match", "two-tab dispatch", "strike while walking"]) {
  if (!doc.includes(witness)) throw new Error("the witness spec is missing " + witness);
}
for (const rule of ["own context", "at most two", "finally", "name every page and server"]) {
  if (!doc.includes(rule)) throw new Error("the bounded run rule is missing: " + rule);
}
console.log("browser witness spec ok (4 witnesses, 4 bounded-run rules)");
