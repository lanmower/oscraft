#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { contractPaths } from "./cross-contract-inputs.mjs";
import { extractionEvidence } from "./runtime-peer-evidence.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const paths = contractPaths(root);
const statusFile = path.join(root, "scenarios", "bota", "integrated-watch-status.json");
const evidenceFile = path.join(root, "scenarios", "bota", "runtime-evidence.json");
const runtimeStatusFile = path.join(root, "scenarios", "bota", "runtime-evidence-status.json");
const fail = (message) => { console.error(`validate:integrated-watch: ${message}`); process.exit(1); };
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
if (!fs.existsSync(statusFile)) fail("integrated-watch-status.json is missing; run npm run watch:integrated -- --once");
const status = JSON.parse(fs.readFileSync(statusFile, "utf8"));
if (status.schema !== "oscraft.integrated-watch-status/v1" || status.ok !== true) fail("schema or ok flag invalid");
if (!fs.existsSync(paths.snapshot) || !fs.existsSync(evidenceFile) || !fs.existsSync(runtimeStatusFile)) fail("one or more generated evidence files are missing");
const snapshotHash = sha256(fs.readFileSync(paths.snapshot));
const evidence = JSON.parse(fs.readFileSync(evidenceFile, "utf8"));
const runtimeStatus = JSON.parse(fs.readFileSync(runtimeStatusFile, "utf8"));
if (status.snapshotHash !== snapshotHash || evidence.snapshotHash !== snapshotHash || runtimeStatus.snapshotHash !== snapshotHash) fail(`snapshot hash is stale: status=${status.snapshotHash} evidence=${evidence.snapshotHash} runtimeStatus=${runtimeStatus.snapshotHash} live=${snapshotHash}`);
if (status.generatedFiles?.["scenarios/bota/runtime-evidence.json"] !== sha256(fs.readFileSync(evidenceFile))) fail("runtime-evidence.json hash differs from watch status");
if (status.generatedFiles?.["scenarios/bota/runtime-evidence-status.json"] !== sha256(fs.readFileSync(runtimeStatusFile))) fail("runtime-evidence-status.json hash differs from watch status");
if (status.generatedFiles?.["scenarios/bota/cross-contract-snapshot.json"] !== snapshotHash) fail("cross-contract-snapshot.json hash differs from watch status");
const checks = [
  ["validate:runtime-evidence", "scripts/validate-runtime-evidence.mjs"],
  ["validate:runtime-evidence-status", "scripts/validate-runtime-evidence-status.mjs"],
  ["refresh --check", "scripts/dev/refresh-cross-contracts.mjs"],
];
for (const [label, file] of checks) {
  const args = file.includes("refresh-") ? ["--check"] : [];
  const result = spawnSync(process.execPath, [path.join(root, file), ...args], { cwd: root, encoding: "utf8" });
  if (result.status !== 0) { console.error(`${result.stdout ?? ""}${result.stderr ?? ""}`.trim()); fail(`${label} exited ${result.status}`); }
}
const cold = status.coldStart ?? {};
if (!cold.url || !/^http:\/\/127\.0\.0\.1:\d+\/js\/client\/play\.html$/.test(cold.url) || !Number.isInteger(cold.pid) || !Number.isInteger(cold.port)) fail(`coldStart identity invalid: ${JSON.stringify(cold)}`);
if (!Array.isArray(cold.httpResults) || cold.httpResults.length < 10 || cold.httpResults.some((row) => row.status !== 200)) fail(`coldStart HTTP results invalid: ${JSON.stringify(cold.httpResults ?? [])}`);
if (!/stopped/.test(cold.cleanup ?? "")) fail("coldStart cleanup evidence missing");
let alive = false;
try { process.kill(cold.pid, 0); alive = true; } catch (error) { if (error.code !== "ESRCH") alive = true; }
if (alive) fail(`integrated PID ${cold.pid} survived`);
const survivorPattern = /serve-integrated|serve:integrated|\.dev-site-integrated|\.dev-site-integrated-check-|\.dev-site-cross-runtime-|\.dev-site-identifier-runtime-|check-bota-browser|check-cross-runtime|check-identifier-regime-runtime|watch-cross-runtime/;
const orphan = spawnSync("bash", ["-lc", "ps -eo pid,ppid,args | awk '$2 == 1 && /watch-cross-runtime\\.mjs --once/ {print $1}'"], { cwd: root, encoding: "utf8" });
if (`${orphan.stdout ?? ""}`.trim()) fail(`orphan watch-cross-runtime survivor found: ${orphan.stdout.trim()}`);
const ps = spawnSync("bash", ["-lc", `ps -eo pid,args | grep -E ${JSON.stringify(survivorPattern.source)} | grep -v grep || true`], { cwd: root, encoding: "utf8" });
if (`${ps.stdout ?? ""}`.trim()) fail(`integrated/test survivor process found: ${ps.stdout.trim()}`);
const browser = status.browser ?? {};
if (browser.ok !== true || browser.uSkinned !== 1 || browser.jointFloats !== 448 || browser.models !== 28 || browser.heroes !== 26 || browser.joints !== 28 || browser.clips !== 8 || browser.motion !== "walk/run" || browser.backdropHits < 1 || browser.other404s !== 0) fail(`browser/WebGL evidence invalid: ${JSON.stringify(browser)}`);
const regimes = status.identifierRegimes ?? {};
if (regimes.default?.packParam !== "scenarios/moba-acronyms" || regimes.default?.activeIds !== 127 || regimes.default?.acronymOnly !== true) fail("default identifier regime invalid");
if (regimes.bota?.pack !== "bota" || regimes.bota?.activeIds !== 26 || regimes.bota?.fullnameOnly !== true) fail("BOTA identifier regime invalid");
if (regimes.historical?.pack !== "moba") fail("historical pack invalid");
const cross = status.crossContracts ?? {};
if (cross.required !== 8 || cross.driftStale !== 0 || cross.hashBearingRowsVerified < 1) fail(`cross-contract counts invalid: ${JSON.stringify(cross)}`);
const liveExtraction = extractionEvidence(paths);
if (!Array.isArray(status.extractionSummaries) || status.extractionSummaries.length !== liveExtraction.length) fail("extraction summaries missing or stale");
for (let i = 0; i < liveExtraction.length; i += 1) {
  const saved = status.extractionSummaries[i], live = liveExtraction[i];
  if (saved?.peer !== live.peer || saved?.path !== live.path || saved?.present !== live.present || saved?.informational !== true || (saved.classes ?? null) !== (live.classes ?? null) || (saved.complete ?? null) !== (live.complete ?? null) || (saved.blocked ?? null) !== (live.blocked ?? null)) fail(`extraction summary ${live.peer} is stale or contradictory`);
}
if (!Array.isArray(status.peerStatus) || status.peerStatus.length !== 4 || status.peerStatus.some((row) => row.readOnly !== true)) fail("peer status evidence invalid");
if (!Array.isArray(status.healedPaths) || !Number.isInteger(status.attempts) || status.attempts < 1 || !status.bounded || typeof status.bounded.cycles !== "number") fail("attempt/heal/bounded metadata invalid");
console.log(`validate:integrated-watch PASS: path=scenarios/bota/integrated-watch-status.json snapshotHash=${snapshotHash} generatedAt=${status.generatedAt}`);
console.log(`validate:integrated-watch PASS: coldStart pid=${cold.pid} port=${cold.port} httpResults=${cold.httpResults.length}; no survivors; evidence, snapshot, browser, extraction and peer state agree`);
