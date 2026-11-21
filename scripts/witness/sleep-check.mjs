import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { sleep } from "../lib/sleep.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const SHARED = "scripts/lib/sleep.mjs";
const OWN_DEFINITION = /const sleep = \(ms\) =>/;
const scripts = execFileSync("git", ["ls-files", "scripts"], { cwd: root, encoding: "utf8" })
  .split("\n")
  .filter((file) => /\.(mjs|cjs|js)$/.test(file) && file !== SHARED);
const copies = scripts.filter((file) => OWN_DEFINITION.test(fs.readFileSync(path.join(root, file), "utf8")));
const started = Date.now();
await sleep(200);
const waited = Date.now() - started;
const failures = [];
if (waited < 180) failures.push("sleep(200) returned after " + waited + " ms");
if (copies.length) failures.push("own sleep definitions in " + copies.join(", "));
if (failures.length) {
  console.error("sleep check: " + failures.join("; "));
  process.exitCode = 1;
} else {
  console.log("sleep check ok (sleep(200) waited " + waited + " ms; no own sleep definitions in " + scripts.length + " tracked scripts)");
}
