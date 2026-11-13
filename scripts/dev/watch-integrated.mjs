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
const watchStatusFile = path.join(root, "scenarios", "bota", "integrated-watch-status.json");
const evidenceFile = path.join(root, "scenarios", "bota", "runtime-evidence.json");
const runtimeStatusFile = path.join(root, "scenarios", "bota", "runtime-evidence-status.json");
const once = process.argv.includes("--once");
const timeoutMs = Number(process.env.WATCH_INTEGRATED_TIMEOUT_MS ?? (once ? 0 : 10 * 60 * 1000));
const intervalMs = Number(process.env.WATCH_INTEGRATED_INTERVAL_MS ?? 20000);
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const fail = (message, result) => { if (result?.text?.trim()) console.error(result.text.trim()); console.error(`watch:integrated: ${message}`); process.exit(1); };
const run = (label, file, args = []) => {
  const result = spawnSync(process.execPath, [file, ...args], { cwd: root, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 });
  const text = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  console.log(`--- ${label} exit=${result.status ?? 1} ---`);
  if (text.trim()) console.log(text.trim());
  return { ...result, text };
};
const stalePaths = (text) => [...new Set([...text.matchAll(/^\s*STALE\s+(\S+):/gm)].map((m) => m[1]))];
const healthyStale = (text) => /STALE cross-contract snapshot:|healthy stale snapshot detected|DRIFT\/STALE cross-project snapshot:|refresh --check exited 3|unresolved DRIFT\/STALE remains after refresh/.test(text) && !/real DRIFT|UNAVAILABLE required peer manifests|HTTP [0-9]{3}|WebGL|identifier mismatch|browser summary failed|critical|DRIFT cross-project hash-bearing|survivor|survived/.test(text);
const survivorPattern = /serve-integrated|serve:integrated|\.dev-site-integrated|\.dev-site-integrated-check-|\.dev-site-cross-runtime-|\.dev-site-identifier-runtime-|check-bota-browser|check-cross-runtime|check-identifier-regime-runtime|watch-cross-runtime/;
const orphanHeals = () => {
  const result = spawnSync("bash", ["-lc", "ps -eo pid,ppid,args | awk '$2 == 1 && /watch-cross-runtime\\.mjs --once/ {print $1}'"], { cwd: root, encoding: "utf8" });
  return `${result.stdout ?? ""}`.trim().split("\n").filter(Boolean);
};
const cleanupOrphanHeals = () => {
  for (const pid of orphanHeals()) { try { process.kill(Number(pid), "SIGTERM"); } catch {} }
};
const survivors = () => {
  cleanupOrphanHeals();
  const result = spawnSync("bash", ["-lc", `ps -eo pid,args | grep -E ${JSON.stringify(survivorPattern.source)} | grep -v grep || true`], { cwd: root, encoding: "utf8" });
  return `${result.stdout ?? ""}`.trim();
};
const peerRoots = [paths.botaRoot, paths.hy3dRoot, paths.wscheppityRoot, paths.extraRoot];
const peerStatus = () => peerRoots.map((dir) => {
  const r = spawnSync("git", ["-C", dir, "status", "--short"], { cwd: root, encoding: "utf8", maxBuffer: 4 * 1024 * 1024 });
  return { peer: path.basename(dir), exit: r.status ?? 1, lines: `${r.stdout ?? ""}${r.stderr ?? ""}`.trim() ? `${r.stdout ?? ""}${r.stderr ?? ""}`.trim().split("\n").length : 0, readOnly: true };
});
const requiredPresent = () => paths.required.every((file) => fs.existsSync(file));
const started = Date.now();
let cycles = 0;

async function attempt() {
  cycles += 1;
  if (!requiredPresent()) fail(`required peer manifest missing: ${paths.required.filter((file) => !fs.existsSync(file)).join(",")}`);
  const peersBefore = peerStatus();
  let attempts = 0;
  let healedPaths = [];
  let cold = run("check:integrated-cold-start", path.join(root, "scripts", "check-integrated-cold-start.mjs"));
  attempts += 1;
  if (cold.status === 3 || (cold.status !== 0 && healthyStale(cold.text))) {
    healedPaths = [...new Set([...healedPaths, ...stalePaths(cold.text)])];
    for (let healAttempt = 0; healAttempt < 4; healAttempt += 1) {
      const heal = run(`cross:autoheal --once ${healAttempt + 1}`, path.join(root, "scripts", "dev", "cross-contract-autoheal.mjs"), ["--once"]);
      healedPaths = [...new Set([...healedPaths, ...stalePaths(heal.text)])];
      const watch = run(`watch:cross-runtime --once ${healAttempt + 1}`, path.join(root, "scripts", "dev", "watch-cross-runtime.mjs"), ["--once"]);
      if (watch.status === 0) break;
      if (!healthyStale(watch.text)) fail("watch:cross-runtime --once failed with non-STALE evidence", watch);
    }
    cold = run("check:integrated-cold-start retry", path.join(root, "scripts", "check-integrated-cold-start.mjs"));
    attempts += 1;
  }
  if (cold.status !== 0) fail(`check:integrated-cold-start exited ${cold.status}; real failure evidence follows`, cold);
  const url = /check:integrated-cold-start PASS: url=(\S+)/.exec(cold.text)?.[1] ?? null;
  const pid = /check:integrated-cold-start PASS: url=\S+ pid=(\d+)/.exec(cold.text)?.[1] ?? null;
  const port = /check:integrated-cold-start PASS: url=\S+ pid=\d+ port=(\d+)/.exec(cold.text)?.[1] ?? null;
  const cleanupLine = /cold-start cleanup: (.+)/.exec(cold.text)?.[1] ?? null;
  const evidenceLine = /cold-start evidence: (.+)/.exec(cold.text)?.[1] ?? null;
  const httpMap = new Map();
  for (const m of cold.text.matchAll(/^cold-start fetch (.+?): HTTP (\d+) (.+?) (\d+) bytes$/gm)) httpMap.set(m[1], { label: m[1], status: Number(m[2]), contentType: m[3], bytes: Number(m[4]) });
  const httpResults = [...httpMap.values()];
  if (!url || !pid || !port || !cleanupLine || !evidenceLine || httpResults.length < 10) fail(`cold-start proof output is incomplete: ${JSON.stringify({ url, pid, port, cleanupLine, evidenceLine, httpResults: httpResults.length })}`, cold);
  if (httpResults.some((row) => row.status !== 200)) fail(`HTTP failure in cold-start proof: ${JSON.stringify(httpResults)}`, cold);
  const survivorText = survivors();
  if (survivorText) fail(`integrated/test survivor process found: ${survivorText}`);
  const peersAfter = peerStatus();
  if (JSON.stringify(peersBefore) !== JSON.stringify(peersAfter)) fail("peer status changed during integrated watch");
  const validations = [
    ["validate:runtime-evidence", run("validate:runtime-evidence", path.join(root, "scripts", "validate-runtime-evidence.mjs"))],
    ["validate:runtime-evidence-status", run("validate:runtime-evidence-status", path.join(root, "scripts", "validate-runtime-evidence-status.mjs"))],
    ["refresh --check", run("refresh --check", path.join(root, "scripts", "dev", "refresh-cross-contracts.mjs"), ["--check"])],
  ];
  for (const [label, result] of validations) if (result.status !== 0) fail(`${label} exited ${result.status}`, result);
  const snapshotBytes = fs.readFileSync(paths.snapshot);
  const snapshotHash = sha256(snapshotBytes);
  const evidence = JSON.parse(fs.readFileSync(evidenceFile, "utf8"));
  const runtimeStatus = JSON.parse(fs.readFileSync(runtimeStatusFile, "utf8"));
  if (!evidence.ok || !runtimeStatus.ok || evidence.snapshotHash !== snapshotHash || runtimeStatus.snapshotHash !== snapshotHash) fail(`generated files disagree with snapshot: evidence=${evidence.snapshotHash} status=${runtimeStatus.snapshotHash} live=${snapshotHash}`);
  const browser = runtimeStatus.checks?.browser ?? evidence.browser ?? {};
  if (browser.ok !== true || browser.uSkinned !== 1 || browser.jointFloats !== 448 || browser.models !== 28 || browser.heroes !== 26 || browser.joints !== 28 || browser.clips !== 8 || browser.motion !== "walk/run" || browser.backdropHits < 1 || browser.other404s !== 0) fail(`browser/WebGL summary invalid: ${JSON.stringify(browser)}`);
  const regimes = runtimeStatus.checks?.identifierRegimes ?? evidence.identifierRegimes ?? {};
  if (regimes.default?.activeIds !== 127 || regimes.default?.acronymOnly !== true || regimes.bota?.activeIds !== 26 || regimes.bota?.fullnameOnly !== true || regimes.historical?.pack !== "moba") fail(`identifier regimes invalid: ${JSON.stringify(regimes)}`);
  const status = {
    schema: "oscraft.integrated-watch-status/v1",
    generatedAt: new Date().toISOString(),
    ok: true,
    snapshotHash,
    attempts,
    healedPaths,
    coldStart: { url, pid: Number(pid), port: Number(port), cleanup: cleanupLine, evidenceLine, httpResults },
    browser,
    identifierRegimes: regimes,
    crossContracts: runtimeStatus.checks?.crossContracts ?? evidence.crossContracts,
    extractionSummaries: extractionEvidence(paths),
    peerStatus: peersAfter,
    generatedFiles: {
      "scenarios/bota/runtime-evidence.json": sha256(fs.readFileSync(evidenceFile)),
      "scenarios/bota/runtime-evidence-status.json": sha256(fs.readFileSync(runtimeStatusFile)),
      "scenarios/bota/cross-contract-snapshot.json": snapshotHash,
      "scenarios/bota/integrated-watch-status.json": "written-by-this-run",
    },
    bounded: { once, timeoutMs, intervalMs, cycles },
  };
  fs.mkdirSync(path.dirname(watchStatusFile), { recursive: true });
  fs.writeFileSync(watchStatusFile, JSON.stringify(status, null, 2) + "\n");
  console.log(`watch:integrated PASS: status=scenarios/bota/integrated-watch-status.json snapshotHash=${snapshotHash} attempts=${attempts} healedPaths=${healedPaths.length} httpResults=${httpResults.length} url=${url} pid=${pid} port=${port}`);
  console.log(`watch:integrated PASS: owned PID cleaned; survivors=0; browser uSkinned=${browser.uSkinned} jointFloats=${browser.jointFloats} models=${browser.models} heroes=${browser.heroes} clips=${browser.clips}`);
  return true;
}

while (true) {
  try {
    await attempt();
    if (once) process.exit(0);
    if (Date.now() - started >= timeoutMs) {
      console.log(`watch:integrated bounded timeout reached after ${cycles} cycle(s)`);
      process.exit(0);
    }
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exit(1);
  }
  const pause = Math.min(intervalMs, Math.max(1, timeoutMs - (Date.now() - started)));
  console.log(`watch:integrated: waiting ${pause}ms before next bounded cycle`);
  await sleep(pause);
}
