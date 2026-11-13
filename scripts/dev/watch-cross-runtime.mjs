#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { contractPaths } from "../cross-contract-inputs.mjs";
import { extractionEvidence } from "../runtime-peer-evidence.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const paths = contractPaths(root);
const evidenceFile = path.join(root, "scenarios", "bota", "runtime-evidence.json");
const statusFile = path.join(root, "scenarios", "bota", "runtime-evidence-status.json");
const once = process.argv.includes("--once");
const timeoutMs = Number(process.env.WATCH_CROSS_RUNTIME_TIMEOUT_MS ?? (once ? 0 : 10 * 60 * 1000));
const intervalMs = Number(process.env.WATCH_CROSS_RUNTIME_INTERVAL_MS ?? 15000);
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const run = (label, file, args = []) => {
  const result = spawnSync(process.execPath, [file, ...args], { cwd: root, encoding: "utf8", maxBuffer: 12 * 1024 * 1024 });
  const text = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  if (text.trim()) console.log(`--- ${label} exit=${result.status ?? 1} ---\n${text.trim()}`);
  else console.log(`--- ${label} exit=${result.status ?? 1} ---`);
  return { ...result, text };
};
const fail = (message, result) => { if (result) console.error((result.text ?? "").trim()); console.error(`watch:cross-runtime: ${message}`); process.exit(1); };
const stalePaths = (text) => [...new Set([...text.matchAll(/^\s*STALE\s+(\S+):/gm)].map((m) => m[1]))];
const staleOnly = (text) => /STALE cross-contract snapshot:|healthy stale snapshot detected|DRIFT\/STALE cross-project snapshot:/.test(text) && !/real DRIFT|UNAVAILABLE required peer manifests|HTTP |WebGL|identifier|browser summary|critical|DRIFT cross-project hash-bearing/.test(text);
const peerRoots = [paths.botaRoot, paths.hy3dRoot, paths.wscheppityRoot, paths.extraRoot];
const statusOf = (dir) => {
  const result = spawnSync("git", ["-C", dir, "status", "--short"], { cwd: root, encoding: "utf8", maxBuffer: 4 * 1024 * 1024 });
  return { peer: path.basename(dir), exit: result.status ?? 1, text: `${result.stdout ?? ""}${result.stderr ?? ""}`.trim() };
};
const statusLines = () => peerRoots.map(statusOf);
const requiredPresent = () => paths.required.filter((file) => fs.existsSync(file)).length;
const started = Date.now();
let cycles = 0;
let peerChangesHealed = [];

async function attempt() {
  cycles += 1;
  const requiredBefore = requiredPresent();
  if (requiredBefore !== paths.required.length) fail(`required peer manifests missing: ${paths.required.filter((file) => !fs.existsSync(file)).join(",")}`);
  const beforeStatus = statusLines();
  let runtime = run("check:cross-runtime", path.join(root, "scripts", "check-cross-runtime.mjs"));
  let healed = [];
  const healthyStaleText = (text) => staleOnly(text) || /^STALE cross-contract snapshot:/m.test(text) && /refresh --check exited 3/.test(text);
  if (runtime.status === 3 || (runtime.status !== 0 && healthyStaleText(runtime.text))) {
    healed = stalePaths(runtime.text);
    const heal = run("cross:autoheal --once", path.join(root, "scripts", "dev", "cross-contract-autoheal.mjs"), ["--once"]);
    if (heal.status !== 0) fail("cross:autoheal --once failed", heal);
    healed = [...new Set([...healed, ...stalePaths(heal.text)])];
    peerChangesHealed = healed;
    runtime = run("check:cross-runtime retry", path.join(root, "scripts", "check-cross-runtime.mjs"));
  }
  if (runtime.status !== 0) fail(`check:cross-runtime exited ${runtime.status}; real failure evidence follows`, runtime);
  let validate = run("validate:runtime-evidence", path.join(root, "scripts", "validate-runtime-evidence.mjs"));
  if (validate.status === 3 || (validate.status !== 0 && healthyStaleText(validate.text))) {
    const heal = run("cross:autoheal after validation", path.join(root, "scripts", "dev", "cross-contract-autoheal.mjs"), ["--once"]);
    if (heal.status !== 0) fail("cross:autoheal after validation failed", heal);
    healed = [...new Set([...healed, ...stalePaths(heal.text)])];
    peerChangesHealed = healed;
    runtime = run("check:cross-runtime after validation heal", path.join(root, "scripts", "check-cross-runtime.mjs"));
    if (runtime.status !== 0) fail(`check:cross-runtime exited ${runtime.status} after validation heal; real failure evidence follows`, runtime);
    validate = run("validate:runtime-evidence retry", path.join(root, "scripts", "validate-runtime-evidence.mjs"));
  }
  if (validate.status !== 0) fail("runtime evidence validation failed", validate);
  const afterStatus = statusLines();
  for (let i = 0; i < beforeStatus.length; i += 1) {
    if (beforeStatus[i].text !== afterStatus[i].text) fail(`peer status changed during watch: ${beforeStatus[i].peer}`);
  }
  if (!fs.existsSync(evidenceFile)) fail("runtime-evidence.json is missing after check:cross-runtime");
  const evidence = JSON.parse(fs.readFileSync(evidenceFile, "utf8"));
  const snapshotBytes = fs.readFileSync(paths.snapshot);
  const snapshotHash = sha256(snapshotBytes);
  if (evidence.snapshotHash !== snapshotHash) fail(`evidence snapshotHash mismatch: evidence=${evidence.snapshotHash} live=${snapshotHash}`);
  const status = {
    schema: "oscraft.cross-runtime-status/v1",
    generatedAt: new Date().toISOString(),
    ok: true,
    snapshotHash,
    evidenceFile: "scenarios/bota/runtime-evidence.json",
    evidenceGeneratedAt: evidence.generatedAt,
    checks: {
      checkCrossRuntime: 0,
      validateRuntimeEvidence: 0,
      identifierRegimes: evidence.identifierRegimes,
      browser: evidence.browser,
      crossContracts: evidence.crossContracts,
      requiredPeerManifests: requiredBefore,
    },
    peerChangesHealed: healed,
    peerStatus: afterStatus.map((row) => ({ peer: row.peer, exit: row.exit, lines: row.text ? row.text.split("\n").length : 0, readOnly: true })),
    extractionSummaries: extractionEvidence(paths),
    optionalPeerStatus: evidence.optionalPeerStatus,
    bounded: { once, timeoutMs, intervalMs, cycles },
  };
  fs.mkdirSync(path.dirname(statusFile), { recursive: true });
  fs.writeFileSync(statusFile, JSON.stringify(status, null, 2) + "\n");
  console.log(`watch:cross-runtime PASS: status=scenarios/bota/runtime-evidence-status.json snapshotHash=${snapshotHash} peerChangesHealed=${healed.length} cycles=${cycles}`);
  for (const item of status.extractionSummaries) console.log(`extraction evidence ${item.peer}: present=${item.present} classes=${item.classes ?? "?"} complete=${item.complete ?? "?"} blocked=${item.blocked ?? "?"} informational=true`);
  return true;
}

while (true) {
  try {
    await attempt();
    if (once) process.exit(0);
    if (Date.now() - started >= timeoutMs) {
      console.log(`watch:cross-runtime bounded timeout reached after ${cycles} cycle(s)`);
      process.exit(0);
    }
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exit(1);
  }
  const remaining = Math.max(1, timeoutMs - (Date.now() - started));
  const pause = Math.min(intervalMs, remaining);
  console.log(`watch:cross-runtime: waiting ${pause}ms before next bounded cycle`);
  await sleep(pause);
}
