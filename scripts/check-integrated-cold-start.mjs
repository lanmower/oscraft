#!/usr/bin/env node
import { spawn, spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const fail = (message) => { console.error(`check:integrated-cold-start: ${message}`); process.exit(1); };
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const freePort = () => new Promise((resolve, reject) => {
  const server = net.createServer();
  server.once("error", reject);
  server.listen(0, "127.0.0.1", () => { const port = server.address().port; server.close(() => resolve(port)); });
});
const port = await freePort();
const site = path.join(root, `.dev-site-integrated-check-${process.pid}`);
const pidFile = path.join(root, `.dev-site-integrated-check-${process.pid}.pid`);
const logFile = path.join(root, `.dev-site-integrated-check-${process.pid}.log`);
const script = path.join(root, "scripts", "serve-integrated.sh");
const env = { ...process.env, INTEGRATED_PORT: String(port), INTEGRATED_SITE: site, INTEGRATED_PID_FILE: pidFile, INTEGRATED_LOG_FILE: logFile, INTEGRATED_PUBLIC_BASE: `http://127.0.0.1:${port}`, WASM: process.env.WASM ?? "0" };
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const stopOwned = () => spawnSync("bash", [script, "--stop", `--port=${port}`, `--site=${site}`, `--pid-file=${pidFile}`], { cwd: root, env, encoding: "utf8" });
let output = "";
let child = null;
let ownedPid = null;
const cleanup = () => {
  if (ownedPid) {
    try { process.kill(ownedPid, "SIGTERM"); } catch {}
  }
  if (child?.pid) {
    try { process.kill(child.pid, "SIGTERM"); } catch {}
  }
};
process.on("exit", cleanup);
process.on("SIGINT", () => { cleanup(); process.exit(130); });
process.on("SIGTERM", () => { cleanup(); process.exit(143); });
async function fetchOk(url, expected = null) {
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) fail(`${url} HTTP ${response.status}`);
  const bytes = Buffer.from(await response.arrayBuffer());
  if (expected?.includes && !bytes.toString("utf8").includes(expected.includes)) fail(`${url} missing ${expected.includes}`);
  if (expected?.json) JSON.parse(bytes.toString("utf8"));
  if (expected?.jpeg && bytes.readUInt16BE(0) !== 0xffd8) fail(`${url} is not JPEG`);
  if (expected?.glb && (bytes.readUInt32LE(0) !== 0x46546c67 || bytes.readUInt32LE(4) !== 2)) fail(`${url} is not glTF/v2`);
  return { status: response.status, contentType: response.headers.get("content-type") ?? "", bytes };
}
async function waitFor(fn, timeoutMs = 90000) {
  const end = Date.now() + timeoutMs;
  let last;
  while (Date.now() < end) {
    try { return await fn(); } catch (error) { last = error; await sleep(150); }
  }
  throw last ?? new Error("timeout");
}
try {
  const pre = stopOwned();
  if (pre.status !== 0) fail(`pre-stop failed: ${pre.stderr || pre.stdout}`);
  child = spawn("bash", [script, `--port=${port}`, `--site=${site}`, `--pid-file=${pidFile}`], { cwd: root, env, stdio: ["ignore", "pipe", "pipe"], detached: true });
  child.stdout.on("data", (chunk) => { output += chunk.toString(); });
  child.stderr.on("data", (chunk) => { output += chunk.toString(); });
  const base = `http://127.0.0.1:${port}`;
  const urls = [
    ["root", `${base}/`, { includes: "js/client/play.html" }],
    ["default play", `${base}/js/client/play.html`, { includes: "play.js" }],
    ["bota play", `${base}/js/client/play.html?package=scenarios/bota`, { includes: "play.js" }],
    ["historical play", `${base}/js/client/play.html?package=scenarios/moba`, { includes: "play.js" }],
    ["default manifest", `${base}/scenarios/moba-acronyms/manifest.json`, { json: true }],
    ["bota manifest", `${base}/scenarios/bota/manifest.json`, { json: true }],
    ["historical manifest", `${base}/scenarios/moba/manifest.json`, { json: true }],
    ["backdrop", `${base}/scenarios/bota/assets/textures/menu-backdrop.jpg`, { jpeg: true }],
    ["hero GLB", `${base}/scenarios/bota/assets/models/neckbeard-skinned.glb`, { glb: true }],
    ["health", `${base}/`, { includes: "js/client/play.html" }],
  ];
  await waitFor(async () => {
    if (!output.includes("INTEGRATED PID:")) throw new Error("integrated banner not printed yet");
    await fetchOk(`${base}/scenarios/moba-acronyms/manifest.json`, { json: true });
  });
  if (!fs.existsSync(pidFile)) fail("integrated PID file is missing");
  ownedPid = Number(fs.readFileSync(pidFile, "utf8").trim());
  if (!Number.isInteger(ownedPid) || ownedPid <= 0) fail(`invalid owned PID ${ownedPid}`);
  const results = new Map();
  for (const [label, url, expected] of urls) {
    const result = await fetchOk(url, expected);
    results.set(label, result);
    console.log(`cold-start fetch ${label}: HTTP ${result.status} ${result.contentType} ${result.bytes.length} bytes`);
  }
  const defaultManifest = JSON.parse(results.get("default manifest").bytes.toString("utf8"));
  const botaManifest = JSON.parse(results.get("bota manifest").bytes.toString("utf8"));
  const historicalManifest = JSON.parse(results.get("historical manifest").bytes.toString("utf8"));
  if (defaultManifest.name !== "moba-acronyms" || botaManifest.name !== "bota" || historicalManifest.name !== "moba") fail("pack manifest identities are wrong");
  const backdropHash = sha256(results.get("backdrop").bytes);
  if (results.get("backdrop").bytes.length !== 3654481 || backdropHash !== "431b1f861938c26edb7ce226e75d1c66a433c701d70a59463e064c402a077549") fail("cold-start backdrop hash mismatch");
  let evidence = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "runtime-evidence.json"), "utf8"));
  let status = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "runtime-evidence-status.json"), "utf8"));
  if (!evidence.ok || !status.ok || evidence.snapshotHash !== status.snapshotHash) fail("runtime evidence/status are not ok or disagree");
  let validate = spawnSync(process.execPath, [path.join(root, "scripts", "validate-runtime-evidence-status.mjs")], { cwd: root, env, encoding: "utf8" });
  const validateText = () => `${validate.stdout ?? ""}${validate.stderr ?? ""}`;
  if (validate.status !== 0 && /STALE cross-contract snapshot:|refresh --check exited 3/.test(validateText())) {
    const heal = spawnSync(process.execPath, [path.join(root, "scripts", "dev", "cross-contract-autoheal.mjs"), "--once"], { cwd: root, env, encoding: "utf8" });
    if (heal.status !== 0) fail(`cold-start validation autoheal failed: ${heal.stdout ?? ""}${heal.stderr ?? ""}`);
    const watch = spawnSync(process.execPath, [path.join(root, "scripts", "dev", "watch-cross-runtime.mjs"), "--once"], { cwd: root, env, encoding: "utf8" });
    if (watch.status !== 0) fail(`cold-start watch refresh failed: ${watch.stdout ?? ""}${watch.stderr ?? ""}`);
    evidence = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "runtime-evidence.json"), "utf8"));
    status = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "runtime-evidence-status.json"), "utf8"));
    validate = spawnSync(process.execPath, [path.join(root, "scripts", "validate-runtime-evidence-status.mjs")], { cwd: root, env, encoding: "utf8" });
  }
  if (validate.status !== 0 && /status evidenceGeneratedAt does not match|snapshotHash is stale|refresh --check exited 3|STALE cross-contract snapshot/.test(validateText())) {
    const watch = spawnSync(process.execPath, [path.join(root, "scripts", "dev", "watch-cross-runtime.mjs"), "--once"], { cwd: root, env, encoding: "utf8" });
    if (watch.status !== 0) fail(`cold-start status refresh failed: ${watch.stdout ?? ""}${watch.stderr ?? ""}`);
    evidence = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "runtime-evidence.json"), "utf8"));
    status = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "runtime-evidence-status.json"), "utf8"));
    validate = spawnSync(process.execPath, [path.join(root, "scripts", "validate-runtime-evidence-status.mjs")], { cwd: root, env, encoding: "utf8" });
  }
  if (validate.status !== 0) fail(`status validator failed: ${validate.stdout ?? ""}${validate.stderr ?? ""}`);
  console.log(`cold-start evidence: snapshotHash=${status.snapshotHash} default=127 acronyms bota=26 fullnames uSkinned=${status.checks.browser.uSkinned} jointFloats=${status.checks.browser.jointFloats}`);
  const stopped = stopOwned();
  if (stopped.status !== 0) fail(`--stop failed: ${stopped.stdout ?? ""}${stopped.stderr ?? ""}`);
  await waitFor(async () => {
    if (fs.existsSync(pidFile)) throw new Error("PID file still present");
    try { process.kill(ownedPid, 0); throw new Error(`owned PID ${ownedPid} still alive`); } catch (error) { if (error.code === "ESRCH") return true; throw error; }
  }, 5000);
  if (child.pid) { try { process.kill(child.pid, "SIGTERM"); } catch {} }
  console.log(`cold-start cleanup: pid=${ownedPid} stopped; pidFile removed; peer servers untouched`);
  console.log(`check:integrated-cold-start PASS: url=${base}/js/client/play.html pid=${ownedPid} port=${port} ok=true`);
} catch (error) {
  console.error(output.trim());
  stopOwned();
  fail(error.message);
} finally {
  cleanup();
}
