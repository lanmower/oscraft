#!/usr/bin/env node
import { spawn, spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { contractPaths, comparableSha256 } from "./cross-contract-inputs.mjs";
import { extractionEvidence } from "./runtime-peer-evidence.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const paths = contractPaths(root);
const evidenceFile = path.join(root, "scenarios", "bota", "runtime-evidence.json");
const fail = (message) => { console.error(`check:cross-runtime: ${message}`); process.exit(1); };
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const freePort = () => new Promise((resolve, reject) => {
  const server = net.createServer();
  server.once("error", reject);
  server.listen(0, "127.0.0.1", () => { const port = server.address().port; server.close(() => resolve(port)); });
});
async function waitFor(fn, timeoutMs = 30000) {
  const end = Date.now() + timeoutMs;
  let last;
  while (Date.now() < end) {
    try { return await fn(); } catch (error) { last = error; await sleep(150); }
  }
  throw last ?? new Error("timeout");
}
const runNode = (file, args = []) => spawnSync(process.execPath, [file, ...args], { cwd: root, encoding: "utf8", maxBuffer: 8 * 1024 * 1024 });
let snapshotBytes = fs.existsSync(paths.snapshot) ? fs.readFileSync(paths.snapshot) : null;
if (!snapshotBytes) fail("cross-contract snapshot is missing; run npm run cross:autoheal -- --once");
let snapshot = JSON.parse(snapshotBytes.toString("utf8"));
let snapshotHash = sha256(snapshotBytes);
let refreshCheck = runNode(path.join(root, "scripts", "dev", "refresh-cross-contracts.mjs"), ["--check"]);
if (refreshCheck.status === 3) {
  console.log("cross-runtime: healthy stale snapshot detected; running npm run cross:autoheal -- --once");
  const heal = runNode(path.join(root, "scripts", "dev", "cross-contract-autoheal.mjs"), ["--once"]);
  const healText = `${heal.stdout ?? ""}${heal.stderr ?? ""}`.trim();
  if (healText) console.log(healText);
  if (heal.status !== 0) fail(`cross:autoheal --once exited ${heal.status}`);
  snapshotBytes = fs.readFileSync(paths.snapshot);
  snapshot = JSON.parse(snapshotBytes.toString("utf8"));
  snapshotHash = sha256(snapshotBytes);
  refreshCheck = runNode(path.join(root, "scripts", "dev", "refresh-cross-contracts.mjs"), ["--check"]);
}
if (refreshCheck.status !== 0) {
  console.error(`${refreshCheck.stdout ?? ""}${refreshCheck.stderr ?? ""}`.trim());
  fail(`snapshot is stale: refresh --check exited ${refreshCheck.status}`);
}
const contractCheck = runNode(path.join(root, "scripts", "check-cross-project-contracts.mjs"));
if (contractCheck.status !== 0) {
  console.error(`${contractCheck.stdout ?? ""}${contractCheck.stderr ?? ""}`.trim());
  fail(`check:cross-contracts exited ${contractCheck.status}`);
}
const contractText = `${contractCheck.stdout ?? ""}${contractCheck.stderr ?? ""}`;
const optionalPresent = /optionalPresent=(\d+)/.exec(contractText)?.[1] ?? String(snapshot.counts.optionalPresent ?? 0);
const optionalMissing = /optionalMissing=(\d+)/.exec(contractText)?.[1] ?? String(snapshot.counts.optionalMissing ?? 0);
const extraPresent = /extraPresent=(\d+)/.exec(contractText)?.[1] ?? String(snapshot.counts.extraPresent ?? 0);
const extraMissing = /extraMissing=(\d+)/.exec(contractText)?.[1] ?? String(snapshot.counts.extraMissing ?? 0);
const hashRows = /(\d+) hash-bearing available\/ready content rows/.exec(contractText)?.[1] ?? "0";
console.log(`snapshot not stale: hash=${snapshotHash} comparableSha256=${comparableSha256(snapshot)}`);
console.log(`optional peer inputs annotated: optionalPresent=${optionalPresent} optionalMissing=${optionalMissing} extraPresent=${extraPresent} extraMissing=${extraMissing}`);

const port = await freePort();
const site = process.env.CROSS_RUNTIME_SITE ?? path.join(root, `.dev-site-cross-runtime-${process.pid}`);
const command = path.join(root, "scripts", "dev", "serve-bota.sh");
const env = { ...process.env, BOTA_PORT: String(port), BOTA_SITE: site, WASM: process.env.WASM ?? "0" };
const child = spawn(command, [`--port=${port}`], { cwd: root, env, stdio: ["ignore", "pipe", "pipe"], detached: true });
let output = "";
child.stdout.on("data", (chunk) => { output += chunk.toString(); });
child.stderr.on("data", (chunk) => { output += chunk.toString(); });
const cleanup = () => { try { process.kill(-child.pid, "SIGTERM"); } catch {} };
process.on("exit", cleanup);
process.on("SIGINT", () => { cleanup(); process.exit(130); });
process.on("SIGTERM", () => { cleanup(); process.exit(143); });
async function fetchOk(url) {
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) throw new Error(`${url} HTTP ${response.status}`);
  const bytes = Buffer.from(await response.arrayBuffer());
  return { status: response.status, contentType: response.headers.get("content-type") ?? "", bytes };
}
const fetchJsonResult = async (url) => {
  const response = await fetchOk(url);
  return JSON.parse(response.bytes.toString("utf8"));
};
try {
  const base = `http://127.0.0.1:${port}`;
  const urls = {
    defaultHtml: `${base}/js/client/play.html`,
    botaHtml: `${base}/js/client/play.html?package=scenarios/bota`,
    historicalHtml: `${base}/js/client/play.html?package=scenarios/moba`,
    defaultManifest: `${base}/scenarios/moba-acronyms/manifest.json`,
    defaultUi: `${base}/scenarios/moba-acronyms/moba_ui.json`,
    botaManifest: `${base}/scenarios/bota/manifest.json`,
    botaUi: `${base}/scenarios/bota/bota_ui.json`,
    historicalManifest: `${base}/scenarios/moba/manifest.json`,
    backdrop: `${base}/scenarios/bota/assets/textures/menu-backdrop.jpg`,
    heroGlb: `${base}/scenarios/bota/assets/models/neckbeard-skinned.glb`,
  };
  await waitFor(async () => {
    if (!output.includes("BOTA URL:")) throw new Error("launch wrapper has not printed a URL");
    const response = await fetch(urls.defaultManifest, { cache: "no-store" });
    if (!response.ok) throw new Error(`default manifest HTTP ${response.status}`);
  });
  const html = {};
  for (const label of ["defaultHtml", "botaHtml", "historicalHtml"]) {
    const result = await fetchOk(urls[label]);
    if (!result.bytes.toString("utf8").includes("play.js")) fail(`${label} did not serve the play page`);
    html[label] = { url: urls[label], status: result.status, contentType: result.contentType };
  }
  const defaultManifest = await fetchJsonResult(urls.defaultManifest);
  const defaultUi = await fetchJsonResult(urls.defaultUi);
  const defaultIds = Object.keys(defaultUi.heroAcronyms ?? {});
  const botaManifest = await fetchJsonResult(urls.botaManifest);
  const botaUi = await fetchJsonResult(urls.botaUi);
  const botaIds = Object.keys(botaUi.heroes ?? {});
  const historicalManifest = await fetchJsonResult(urls.historicalManifest);
  const loader = fs.readFileSync(path.join(root, "js", "client", "pack_load.js"), "utf8");
  if (!/PACK_PARAM[^\n]*"scenarios\/moba-acronyms"/.test(loader)) fail("PACK_PARAM default is not scenarios/moba-acronyms");
  if (defaultManifest.name !== "moba-acronyms" || defaultUi.identifierMode !== "default-dota-acronyms" || defaultIds.length !== 127 || defaultIds.some((id) => !/^[a-z]{1,3}$/.test(id))) fail("default runtime pack is not 127 acronym-only IDs");
  if (botaManifest.name !== "bota" || botaUi.identifierMode !== "bota-fullname" || botaUi.menuBackdrop !== "textures.menu_backdrop" || botaIds.length !== 26 || botaIds.some((id) => /^[a-z]{1,3}$/.test(id) && id !== "npc")) fail("BOTA runtime pack is not 26 fullname IDs with menuBackdrop");
  if (historicalManifest.name !== "moba" || historicalManifest.ui !== "moba_ui.json") fail("historical source pack manifest mismatch");
  const backdrop = await fetchOk(urls.backdrop);
  const backdropHash = sha256(backdrop.bytes);
  if (backdrop.bytes.length !== 3654481 || backdropHash !== "431b1f861938c26edb7ce226e75d1c66a433c701d70a59463e064c402a077549") fail("menuBackdrop bytes or SHA-256 mismatch");
  const heroGlb = await fetchOk(urls.heroGlb);
  const heroMagic = heroGlb.bytes.readUInt32LE(0), heroVersion = heroGlb.bytes.readUInt32LE(4);
  if (heroMagic !== 0x46546c67 || heroVersion !== 2) fail("fullname hero GLB is not glTF/v2");
  const heroHash = sha256(heroGlb.bytes);
  const skinned = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "bota-skinned.json"), "utf8"));
  const localHero = fs.readFileSync(path.join(root, "scenarios", "bota", "assets", "models", "neckbeard-skinned.glb"));
  if (skinned.heroes.neckbeard?.skinnedSha256 !== heroHash || sha256(localHero) !== heroHash || skinned.heroes.neckbeard?.skinnedBytes !== localHero.length) fail("neckbeard GLB hash does not match bota-skinned.json and local bytes");
  let localHashMatches = 0;
  for (const [id, row] of Object.entries(skinned.heroes ?? {})) {
    const file = path.join(root, "scenarios", "bota", "assets", "models", `${id}-skinned.glb`);
    if (!fs.existsSync(file) || sha256(fs.readFileSync(file)) !== row.skinnedSha256) fail(`${id}: bota-skinned.json hash mismatch`);
    localHashMatches += 1;
  }
  if (localHashMatches !== 26) fail(`only ${localHashMatches}/26 BOTA skinned hashes match local GLBs`);
  const browserSummary = runNode(path.join(root, "scripts", "check-bota-browser.mjs"), ["--all", "--from-cross-runtime"]);
  const browserText = `${browserSummary.stdout ?? ""}${browserSummary.stderr ?? ""}`.trim();
  if (browserSummary.status !== 0) { console.error(browserText); fail(`browser summary exited ${browserSummary.status}`); }
  let browser;
  try { browser = JSON.parse(browserText.split("\n").filter(Boolean).at(-1)); } catch { fail(`browser summary is not compact JSON: ${browserText.slice(-300)}`); }
  if (!browser.ok || browser.uSkinned !== 1 || browser.jointFloats !== 448 || browser.models !== 28 || browser.heroes !== 26 || browser.joints !== 28 || browser.clips !== 8 || browser.motion !== "walk/run" || browser.backdropHits < 1 || browser.other404s !== 0) fail(`browser/WebGL summary failed: ${JSON.stringify(browser)}`);
  const evidence = {
    schema: "oscraft.cross-runtime-evidence/v1",
    generatedAt: new Date().toISOString(),
    ok: true,
    snapshotHash,
    snapshotComparableSha256: comparableSha256(snapshot),
    identifierRegimes: {
      default: { packParam: "scenarios/moba-acronyms", pack: defaultManifest.name, mode: defaultUi.identifierMode, activeIds: defaultIds.length, acronymOnly: true },
      bota: { pack: botaManifest.name, mode: botaUi.identifierMode, activeIds: botaIds.length, fullnameOnly: true },
      historical: { pack: historicalManifest.name, ui: historicalManifest.ui },
    },
    packUrls: html,
    httpStatuses: {
      defaultManifest: 200, defaultUi: 200, botaManifest: 200, botaUi: 200, historicalManifest: 200,
      backdrop: backdrop.status, heroGlb: heroGlb.status,
    },
    backdrop: { status: backdrop.status, bytes: backdrop.bytes.length, sha256: backdropHash, key: botaUi.menuBackdrop },
    heroModel: { id: "neckbeard", status: heroGlb.status, bytes: heroGlb.bytes.length, sha256: heroHash, magic: "glTF", version: 2 },
    botaSkinnedHashRows: { expected: 26, matched: localHashMatches, clips: skinned.clips },
    browser,
    crossContracts: { required: snapshot.counts.required, optionalPresent: Number(optionalPresent), optionalMissing: Number(optionalMissing), extraPresent: Number(extraPresent), extraMissing: Number(extraMissing), hashBearingRowsVerified: Number(hashRows), driftStale: 0 },
    optionalPeerStatus: { optionalPresent: snapshot.optionalPresent, optionalMissing: snapshot.optionalMissing, extraPresent: snapshot.extraPresent, extraMissing: snapshot.extraMissing, extraSummary: snapshot.extraSummary },
    extractionSummaries: extractionEvidence(paths),
  };
  fs.mkdirSync(path.dirname(evidenceFile), { recursive: true });
  fs.writeFileSync(evidenceFile, JSON.stringify(evidence, null, 2) + "\n");
  console.log(`runtime evidence: path=scenarios/bota/runtime-evidence.json snapshotHash=${snapshotHash} ok=true`);
  console.log(`cross-runtime PASS: default=${defaultManifest.name}/${defaultIds.length} acronyms; bota=${botaManifest.name}/${botaIds.length} fullnames; historical=${historicalManifest.name}`);
  console.log(`cross-runtime PASS: backdrop sha256=${backdropHash} heroModel=${heroHash} skinnedHashRows=${localHashMatches}/26`);
  console.log(`cross-runtime PASS: browser=${browser.browser} uSkinned=${browser.uSkinned} jointFloats=${browser.jointFloats} models=${browser.models} heroes=${browser.heroes} clips=${browser.clips} motion=${browser.motion} backdropHits=${browser.backdropHits} other404s=${browser.other404s}`);
  console.log(`cross-runtime PASS: snapshot not stale, optionalPresent=${optionalPresent} optionalMissing=${optionalMissing} extraPresent=${extraPresent} extraMissing=${extraMissing} hash-bearing rows=${hashRows}`);
  for (const row of evidence.extractionSummaries) console.log(`extraction evidence ${row.peer}: present=${row.present} classes=${row.classes ?? "?"} complete=${row.complete ?? "?"} blocked=${row.blocked ?? "?"} informational=true`);
} catch (error) {
  console.error(output.trim());
  fail(error.message);
} finally {
  cleanup();
}
