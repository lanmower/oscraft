#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { contractPaths } from "../cross-contract-inputs.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const paths = contractPaths(root);
const once = process.argv.includes("--once");
const timeoutMs = Number(process.env.CROSS_AUTOHEAL_TIMEOUT_MS ?? (once ? 0 : 10 * 60 * 1000));
const intervalMs = Number(process.env.CROSS_AUTOHEAL_INTERVAL_MS ?? 10000);
const checker = path.join(root, "scripts", "check-cross-project-contracts.mjs");
const refresh = path.join(root, "scripts", "dev", "refresh-cross-contracts.mjs");
const snapshotFile = paths.snapshot;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const sha = () => fs.existsSync(snapshotFile) ? crypto.createHash("sha256").update(fs.readFileSync(snapshotFile)).digest("hex") : "absent";
const runNode = (file, args = []) => spawnSync(process.execPath, [file, ...args], { cwd: root, encoding: "utf8", maxBuffer: 8 * 1024 * 1024 });
const emit = (label, result) => {
  const text = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim();
  if (text) console.log(`--- ${label} exit=${result.status ?? 1} ---\n${text}`);
  else console.log(`--- ${label} exit=${result.status ?? 1} ---`);
};
const stalePaths = (text) => [...text.matchAll(/^\s*STALE\s+(\S+):/gm)].map((m) => m[1]);
const realDrift = (text) => /^\s*DRIFT\s+\S+:/m.test(text) || /DRIFT cross-project hash-bearing content rows:/m.test(text) || /cross-contracts: /.test(text);
const fail = (message, result) => { if (result) emit("failure evidence", result); console.error(`cross:autoheal: ${message}`); process.exit(1); };

async function attempt() {
  const beforeHash = sha();
  const initial = runNode(checker);
  emit("check:cross-contracts before", initial);
  if (initial.status === 0) {
    console.log(`cross:autoheal clean: before=${beforeHash} after=${beforeHash} stalePaths=0`);
    return { healed: false, beforeHash, afterHash: beforeHash, stale: [] };
  }
  const initialText = `${initial.stdout ?? ""}${initial.stderr ?? ""}`;
  if (initial.status !== 3) fail(`check:cross-contracts exited ${initial.status}, not a healable STALE signal`, null);
  const stale = [...new Set(stalePaths(initialText))];
  if (realDrift(initialText)) fail("real DRIFT is present alongside STALE; refusing to auto-heal silently", initial);
  if (!stale.length) fail("exit 3 had no parseable STALE paths; refusing to auto-heal", initial);
  console.log(`cross:autoheal STALE: ${stale.join(",")}`);
  const refreshed = runNode(refresh);
  emit("refresh:cross-contracts", refreshed);
  if (refreshed.status !== 0) fail("refresh:cross-contracts failed", refreshed);
  const afterHash = sha();
  const final = runNode(checker);
  emit("check:cross-contracts after", final);
  if (final.status !== 0) {
    const finalText = `${final.stdout ?? ""}${final.stderr ?? ""}`;
    if (realDrift(finalText) || final.status === 3) fail("unresolved DRIFT/STALE remains after refresh", final);
    fail(`check:cross-contracts exited ${final.status} after refresh`, final);
  }
  console.log(`cross:autoheal healed: before=${beforeHash} after=${afterHash} stalePaths=${stale.length}`);
  for (const stalePath of stale) console.log(`healed STALE ${stalePath}`);
  return { healed: true, beforeHash, afterHash, stale };
}

const started = Date.now();
let cycles = 0;
while (true) {
  cycles += 1;
  const ok = await attempt();
  if (ok || once) process.exit(0);
  if (Date.now() - started >= timeoutMs) {
    console.error(`cross:autoheal bounded timeout reached after ${cycles} attempt(s), ${timeoutMs}ms`);
    process.exit(1);
  }
  const remaining = Math.max(1, timeoutMs - (Date.now() - started));
  console.log(`cross:autoheal: waiting ${Math.min(intervalMs, remaining)}ms before next bounded attempt`);
  await sleep(Math.min(intervalMs, remaining));
}
