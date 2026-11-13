#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { buildSnapshot, comparableSha256, contractPaths, snapshotComparable } from "../cross-contract-inputs.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const paths = contractPaths(root);
const check = process.argv.includes("--check");
const fail = (message) => { console.error(`refresh:cross-contracts: ${message}`); process.exit(1); };
const missing = paths.required.filter((file) => !fs.existsSync(file));
if (missing.length) {
  console.error("UNAVAILABLE required peer manifests:");
  for (const file of missing) console.error(`  ${file}`);
  process.exit(2);
}
const current = buildSnapshot(paths);
if (check) {
  if (!fs.existsSync(paths.snapshot)) fail("snapshot is absent; run npm run refresh:cross-contracts");
  const saved = JSON.parse(fs.readFileSync(paths.snapshot, "utf8"));
  const expectedHash = comparableSha256(current);
  const actualHash = comparableSha256(saved);
  if (expectedHash !== actualHash) {
    console.error("STALE cross-contract snapshot:");
    console.error(`  expected comparableSha256=${expectedHash}`);
    console.error(`  actual comparableSha256=${actualHash}`);
    process.exit(3);
  }
  console.log(`cross-contract snapshot current: comparableSha256=${actualHash} generatedAt=${saved.generatedAt ?? "?"}`);
} else {
  if (fs.existsSync(paths.snapshot)) {
    const saved = JSON.parse(fs.readFileSync(paths.snapshot, "utf8"));
    const contentEqual = comparableSha256(saved) === comparableSha256(current);
    if (contentEqual) {
      console.log(`cross-contract snapshot unchanged: comparableSha256=${comparableSha256(current)} generatedAt kept=${saved.generatedAt ?? "?"}`);
    } else {
      fs.mkdirSync(path.dirname(paths.snapshot), { recursive: true });
      fs.writeFileSync(paths.snapshot, JSON.stringify(current, null, 2) + "\n");
      console.log(`cross-contract snapshot refreshed: optionalPresent=${current.counts.optionalPresent} optionalMissing=${current.counts.optionalMissing}`);
    }
  } else {
    fs.mkdirSync(path.dirname(paths.snapshot), { recursive: true });
    fs.writeFileSync(paths.snapshot, JSON.stringify(current, null, 2) + "\n");
    console.log(`cross-contract snapshot created: optionalPresent=${current.counts.optionalPresent} optionalMissing=${current.counts.optionalMissing}`);
  }
}
const checker = path.join(root, "scripts", "check-cross-project-contracts.mjs");
let result = spawnSync(process.execPath, [checker], { cwd: root, stdio: "inherit" });
if (check && result.status === 0) process.exit(0);
if (!check) {
  for (let attempt = 0; attempt < 3 && result.status === 3; attempt += 1) {
    console.log(`refresh:cross-contracts: peer changed during check; refreshing stable snapshot attempt=${attempt + 1}`);
    fs.writeFileSync(paths.snapshot, JSON.stringify(buildSnapshot(paths), null, 2) + "\n");
    result = spawnSync(process.execPath, [checker], { cwd: root, stdio: "inherit" });
  }
}
process.exit(result.status ?? 1);
