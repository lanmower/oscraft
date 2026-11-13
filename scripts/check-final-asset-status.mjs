#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { assertFinalAssetStatus, finalAssetErrors, finalAssetPaths, FINAL_ASSET_COUNTS } from "./final-asset-status.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const files = finalAssetPaths(root);
const readText = (file) => fs.readFileSync(file, "utf8");
const readJson = (file) => JSON.parse(readText(file));
const live = assertFinalAssetStatus(root);
const input = {
  status: readJson(files.status),
  evidence: readJson(files.evidence),
  tsv: readText(files.tsv),
  markdown: readText(files.markdown),
  reconciliation: readText(files.reconciliation),
  slugs: readText(files.slugs),
  generatedFiles: live.generatedHashVerification,
};
const badCounts = finalAssetErrors({ ...input, status: structuredClone(input.status), evidence: structuredClone(input.evidence) });
const mutated = structuredClone(input.status);
mutated.generatedCount = 23;
const countErrors = finalAssetErrors({ ...input, status: mutated });
if (!countErrors.some((row) => row.includes("generated=23"))) throw new Error("negative generated-count control passed");
const badAudio = structuredClone(input.status);
badAudio.audio.wavCount = 1;
const audioErrors = finalAssetErrors({ ...input, status: badAudio });
if (!audioErrors.some((row) => row.includes("audio blocker evidence changed"))) throw new Error("negative audio-blocker control passed");
const badHash = { ...input, generatedFiles: { verified: 23, failures: ["negative-control"] } };
const hashErrors = finalAssetErrors(badHash);
if (!hashErrors.some((row) => row.includes("live GLB hashes verified=23"))) throw new Error("negative generated-hash control passed");
if (badCounts.length) throw new Error(`positive status unexpectedly failed: ${badCounts.join(" | ")}`);
if (live.counts.generated !== FINAL_ASSET_COUNTS.generated || live.counts.blocked !== FINAL_ASSET_COUNTS.blocked || live.counts.ready !== FINAL_ASSET_COUNTS.ready) throw new Error(`unexpected live counts: ${JSON.stringify(live.counts)}`);
console.log(`check:final-asset-status PASS: generated=${live.counts.generated} blocked=${live.counts.blocked} ready=${live.counts.ready} remaining=${live.counts.remaining}; ${live.generatedHashVerification.verified}/24 GLB hashes verified; audio blocker preserved; negative count/audio/hash controls rejected`);
