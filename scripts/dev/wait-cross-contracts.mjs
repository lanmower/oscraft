#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const botaRoot = path.resolve(root, "..", "bota");
const hy3dRoot = path.resolve(root, "..", "hy3d");
const required = [
  path.join(botaRoot, "export", "rule-demand.json"),
  path.join(botaRoot, "export", "conformance.json"),
  path.join(botaRoot, "export", "export-status.json"),
  path.join(botaRoot, "export", "spawn-compatibility.json"),
  path.join(hy3dRoot, "output", "content-for-bota.json"),
  path.join(hy3dRoot, "output", "final-asset-inventory.json"),
  path.join(hy3dRoot, "output", "bota-ready.json"),
  path.join(hy3dRoot, "output", "asset-delivery-report.json"),
];
const watchOnce = process.argv.includes("--watch-once");
const timeoutMs = Number(process.env.CROSS_CONTRACT_TIMEOUT_MS ?? (watchOnce ? 0 : 10 * 60 * 1000));
const intervalMs = Number(process.env.CROSS_CONTRACT_INTERVAL_MS ?? 10000);
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const missing = () => required.filter((file) => !fs.existsSync(file));
const checker = path.join(root, "scripts", "check-cross-project-contracts.mjs");
const started = Date.now();
let waited = 0;
let absences = missing();
if (watchOnce) {
  console.log(`watch-once: contracts already ready=${absences.length === 0} missing=${absences.length}`);
} else {
  console.log(`watch: bounded timeout=${timeoutMs}ms interval=${intervalMs}ms initialMissing=${absences.length}`);
  while (absences.length && Date.now() - started < timeoutMs) {
    console.log(`watch: waiting for ${absences.length} manifests\n${absences.join("\n")}`);
    await sleep(Math.min(intervalMs, Math.max(1, timeoutMs - (Date.now() - started))));
    waited += 1;
    absences = missing();
  }
}
if (absences.length) {
  console.error(`UNAVAILABLE cross-project contracts after ${waited} poll(s):`);
  for (const file of absences) console.error(`  ${file}`);
  process.exit(2);
}
console.log(`watch: all ${required.length} upstream manifests present after ${waited} poll(s); running npm run check:cross-contracts`);
const result = spawnSync(process.execPath, [checker], { cwd: root, stdio: "inherit" });
process.exit(result.status ?? 1);
