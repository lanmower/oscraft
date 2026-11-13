#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { comparableSha256, contractPaths } from "./cross-contract-inputs.mjs";
import { extractionEvidence } from "./runtime-peer-evidence.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const paths = contractPaths(root);
const evidenceFile = path.join(root, "scenarios", "bota", "runtime-evidence.json");
const fail = (message) => { console.error(`validate:runtime-evidence: ${message}`); process.exit(1); };
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
if (!fs.existsSync(evidenceFile)) fail(`${evidenceFile} is missing; run npm run check:cross-runtime`);
const evidence = JSON.parse(fs.readFileSync(evidenceFile, "utf8"));
if (evidence.schema !== "oscraft.cross-runtime-evidence/v1" || evidence.ok !== true) fail("evidence schema or ok flag invalid");
if (!fs.existsSync(paths.snapshot)) fail("cross-contract snapshot is missing");
const snapshotBytes = fs.readFileSync(paths.snapshot);
const snapshot = JSON.parse(snapshotBytes.toString("utf8"));
if (evidence.snapshotHash !== sha256(snapshotBytes)) fail(`snapshotHash is stale: expected ${sha256(snapshotBytes)}, got ${evidence.snapshotHash}`);
if (evidence.snapshotComparableSha256 !== comparableSha256(snapshot)) fail("snapshotComparableSha256 is stale");
const refresh = spawnSync(process.execPath, [path.join(root, "scripts", "dev", "refresh-cross-contracts.mjs"), "--check"], { cwd: root, encoding: "utf8" });
if (refresh.status !== 0) { console.error(`${refresh.stdout ?? ""}${refresh.stderr ?? ""}`.trim()); fail(`refresh --check exited ${refresh.status}`); }
const regimes = evidence.identifierRegimes ?? {};
if (regimes.default?.packParam !== "scenarios/moba-acronyms" || regimes.default?.pack !== "moba-acronyms" || regimes.default?.activeIds !== 127 || regimes.default?.acronymOnly !== true) fail("default identifier regime evidence invalid");
if (regimes.bota?.pack !== "bota" || regimes.bota?.mode !== "bota-fullname" || regimes.bota?.activeIds !== 26 || regimes.bota?.fullnameOnly !== true) fail("BOTA identifier regime evidence invalid");
if (regimes.historical?.pack !== "moba" || regimes.historical?.ui !== "moba_ui.json") fail("historical pack evidence invalid");
if (Object.values(evidence.httpStatuses ?? {}).some((status) => status !== 200)) fail("one or more HTTP statuses are not 200");
if (evidence.backdrop?.bytes !== 3654481 || evidence.backdrop?.sha256 !== "431b1f861938c26edb7ce226e75d1c66a433c701d70a59463e064c402a077549" || evidence.backdrop?.key !== "textures.menu_backdrop") fail("backdrop evidence invalid");
const backdropBytes = fs.readFileSync(path.join(root, "scenarios", "bota", "assets", "textures", "menu-backdrop.jpg"));
if (sha256(backdropBytes) !== evidence.backdrop.sha256) fail("live backdrop hash differs from evidence");
if (evidence.heroModel?.id !== "neckbeard" || evidence.heroModel?.magic !== "glTF" || evidence.heroModel?.version !== 2) fail("hero model identity evidence invalid");
const heroBytes = fs.readFileSync(path.join(root, "scenarios", "bota", "assets", "models", `${evidence.heroModel.id}-skinned.glb`));
if (sha256(heroBytes) !== evidence.heroModel.sha256) fail("live hero model hash differs from evidence");
const skinned = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "bota-skinned.json"), "utf8"));
if (evidence.botaSkinnedHashRows?.expected !== 26 || evidence.botaSkinnedHashRows?.matched !== 26 || evidence.botaSkinnedHashRows?.clips !== skinned.clips) fail("bota-skinned evidence counts invalid");
let matched = 0;
for (const [id, row] of Object.entries(skinned.heroes ?? {})) {
  const file = path.join(root, "scenarios", "bota", "assets", "models", `${id}-skinned.glb`);
  if (!fs.existsSync(file) || sha256(fs.readFileSync(file)) !== row.skinnedSha256) fail(`${id}: live skinned hash differs`);
  matched += 1;
}
if (matched !== 26) fail(`only ${matched}/26 live skinned hashes match`);
const browser = evidence.browser ?? {};
if (browser.ok !== true || browser.uSkinned !== 1 || browser.jointFloats !== 448 || browser.models !== 28 || browser.heroes !== 26 || browser.joints !== 28 || browser.clips !== 8 || browser.motion !== "walk/run" || browser.backdropHits < 1 || browser.other404s !== 0) fail(`browser/WebGL evidence invalid: ${JSON.stringify(browser)}`);
const cross = evidence.crossContracts ?? {};
if (cross.required !== 8 || cross.driftStale !== 0 || cross.hashBearingRowsVerified < 1) fail(`cross-contract counts invalid: ${JSON.stringify(cross)}`);
if (!Array.isArray(evidence.optionalPeerStatus?.optionalPresent) || !Array.isArray(evidence.optionalPeerStatus?.optionalMissing) || !Array.isArray(evidence.optionalPeerStatus?.extraPresent) || !Array.isArray(evidence.optionalPeerStatus?.extraMissing)) fail("optional peer inputs are not annotated");
const liveExtraction = extractionEvidence(paths);
if (!Array.isArray(evidence.extractionSummaries) || evidence.extractionSummaries.length !== liveExtraction.length) fail("extraction summaries are not annotated");
for (let i = 0; i < liveExtraction.length; i += 1) {
  const saved = evidence.extractionSummaries[i], live = liveExtraction[i];
  if (saved?.peer !== live.peer || saved?.path !== live.path || saved?.present !== live.present || saved?.informational !== true) fail(`extraction summary ${live.peer} is stale or not informational`);
  if (live.present && (saved.classes ?? null) !== (live.classes ?? null)) fail(`extraction summary ${live.peer} classes mismatch`);
}
console.log(`validate:runtime-evidence PASS: path=${path.relative(root, evidenceFile)} snapshotHash=${evidence.snapshotHash} generatedAt=${evidence.generatedAt}`);
console.log(`validate:runtime-evidence PASS: default 127 acronyms, BOTA 26 fullnames, historical moba, backdrop/hero hashes, 26/26 skinned hashes, WebGL summary, cross-contract counts`);
