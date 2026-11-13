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
const evidenceFile = path.join(root, "scenarios", "bota", "runtime-evidence.json");
const statusFile = path.join(root, "scenarios", "bota", "runtime-evidence-status.json");
const fail = (message) => { console.error(`validate:runtime-evidence-status: ${message}`); process.exit(1); };
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
if (!fs.existsSync(statusFile)) fail("runtime-evidence-status.json is missing; run npm run watch:cross-runtime -- --once");
if (!fs.existsSync(evidenceFile)) fail("runtime-evidence.json is missing; run npm run check:cross-runtime");
const status = JSON.parse(fs.readFileSync(statusFile, "utf8"));
const evidence = JSON.parse(fs.readFileSync(evidenceFile, "utf8"));
if (status.schema !== "oscraft.cross-runtime-status/v1" || status.ok !== true) fail("status schema or ok flag invalid");
if (!fs.existsSync(paths.snapshot)) fail("cross-contract snapshot is missing");
const snapshotHash = sha256(fs.readFileSync(paths.snapshot));
if (status.snapshotHash !== snapshotHash || evidence.snapshotHash !== snapshotHash) fail(`snapshotHash is stale: status=${status.snapshotHash} evidence=${evidence.snapshotHash} live=${snapshotHash}`);
if (status.evidenceGeneratedAt !== evidence.generatedAt) fail("status evidenceGeneratedAt does not match runtime-evidence.json");
const validate = spawnSync(process.execPath, [path.join(root, "scripts", "validate-runtime-evidence.mjs")], { cwd: root, encoding: "utf8" });
if (validate.status !== 0) { console.error(`${validate.stdout ?? ""}${validate.stderr ?? ""}`.trim()); fail(`validate:runtime-evidence exited ${validate.status}`); }
const checks = status.checks ?? {};
if (checks.checkCrossRuntime !== 0 || checks.validateRuntimeEvidence !== 0 || checks.requiredPeerManifests !== paths.required.length) fail(`status checks are contradictory: ${JSON.stringify(checks)}`);
const missing = paths.required.filter((file) => !fs.existsSync(file));
if (missing.length) fail(`required peer manifest disappeared: ${missing.join(",")}`);
const regimes = checks.identifierRegimes ?? {};
if (regimes.default?.packParam !== "scenarios/moba-acronyms" || regimes.default?.activeIds !== 127 || regimes.default?.acronymOnly !== true) fail("default identifier regime in status is invalid");
if (regimes.bota?.pack !== "bota" || regimes.bota?.activeIds !== 26 || regimes.bota?.fullnameOnly !== true) fail("BOTA identifier regime in status is invalid");
if (regimes.historical?.pack !== "moba") fail("historical pack in status is invalid");
const browser = checks.browser ?? {};
if (browser.ok !== true || browser.uSkinned !== 1 || browser.jointFloats !== 448 || browser.models !== 28 || browser.heroes !== 26 || browser.joints !== 28 || browser.clips !== 8 || browser.motion !== "walk/run" || browser.backdropHits < 1 || browser.other404s !== 0) fail(`status browser/WebGL evidence invalid: ${JSON.stringify(browser)}`);
const cross = checks.crossContracts ?? {};
if (cross.required !== 8 || cross.driftStale !== 0 || cross.hashBearingRowsVerified < 1) fail(`status cross-contract counts invalid: ${JSON.stringify(cross)}`);
const liveExtraction = extractionEvidence(paths);
if (!Array.isArray(status.extractionSummaries) || status.extractionSummaries.length !== liveExtraction.length) fail("status extraction summaries are missing");
for (let i = 0; i < liveExtraction.length; i += 1) {
  const saved = status.extractionSummaries[i], live = liveExtraction[i];
  if (saved?.peer !== live.peer || saved?.path !== live.path || saved?.present !== live.present || saved?.informational !== true || (saved.classes ?? null) !== (live.classes ?? null) || (saved.complete ?? null) !== (live.complete ?? null) || (saved.blocked ?? null) !== (live.blocked ?? null)) fail(`status extraction summary ${live.peer} is stale or contradictory`);
}
if (!Array.isArray(status.optionalPeerStatus?.optionalPresent) || !Array.isArray(status.optionalPeerStatus?.extraPresent)) fail("status optional peer inputs are not annotated");
if (!Array.isArray(status.peerStatus) || status.peerStatus.length !== 4 || status.peerStatus.some((row) => row.readOnly !== true)) fail("peer status evidence is not read-only annotated");
if (!Array.isArray(status.peerChangesHealed)) fail("peerChangesHealed is not an array");
if (!status.bounded || typeof status.bounded.cycles !== "number" || typeof status.bounded.timeoutMs !== "number") fail("bounded watch metadata is missing");
console.log(`validate:runtime-evidence-status PASS: path=scenarios/bota/runtime-evidence-status.json snapshotHash=${snapshotHash} generatedAt=${status.generatedAt}`);
console.log(`validate:runtime-evidence-status PASS: checks match runtime evidence, live snapshot, identifier regimes, WebGL summary, cross-contract counts, extraction summaries and peer status`);
