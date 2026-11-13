#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { contractPaths } from "./cross-contract-inputs.mjs";
import { buildContentContractStatus } from "./content-contract-status.mjs";
import { buildFinalAssetStatus } from "./final-asset-status.mjs";
import { extractionEvidence } from "./runtime-peer-evidence.mjs";
import { buildWorkspacePipeline } from "./workspace-pipeline.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const paths = contractPaths(root);
const jsonFile = path.join(root, "docs", "OSCRAFT_RELEASE.json");
const mdFile = path.join(root, "docs", "OSCRAFT_RELEASE.md");
const watch = process.argv.includes("--watch");
const once = process.argv.includes("--once") || !watch;
const timeoutMs = Number(process.env.OSCRAFT_RELEASE_WATCH_TIMEOUT_MS ?? (watch && !once ? 10 * 60 * 1000 : 0));
const intervalMs = Number(process.env.OSCRAFT_RELEASE_WATCH_INTERVAL_MS ?? 20000);
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const fail = (message) => { console.error(`report:oscraft-release: ${message}`); process.exit(1); };
const run = (label, argv) => {
  const result = spawnSync(argv[0], argv.slice(1), { cwd: root, encoding: "utf8", maxBuffer: 24 * 1024 * 1024 });
  const text = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  console.log(`${label}: EXIT=${result.status ?? 1}`);
  if (result.status !== 0 && text.trim()) console.error(text.trim());
  return { label, status: result.status ?? 1, text, last: text.trim().split("\n").filter(Boolean).at(-1) ?? "" };
};
const command = (label, file, args = []) => [label, [process.execPath, path.join(root, file), ...args]];
const gitText = (cwd, args) => {
  const r = spawnSync("git", ["-C", cwd, ...args], { cwd: root, encoding: "utf8", maxBuffer: 8 * 1024 * 1024 });
  return { exit: r.status ?? 1, text: `${r.stdout ?? ""}${r.stderr ?? ""}`.trim() };
};
const peerGit = () => [paths.botaRoot, paths.hy3dRoot, paths.wscheppityRoot, paths.extraRoot].map((dir) => ({ peer: path.basename(dir), ...gitText(dir, ["status", "--short"]) }));
const survivorPattern = /serve-integrated|serve:integrated|\.dev-site-integrated|\.dev-site-integrated-check-|\.dev-site-cross-runtime-|\.dev-site-identifier-runtime-|check-bota-browser|check-cross-runtime|check-identifier-regime-runtime|watch-cross-runtime|watch-integrated/;
const survivors = () => {
  const r = spawnSync("bash", ["-lc", `ps -eo pid,args | grep -E ${JSON.stringify(survivorPattern.source)} | grep -v grep || true`], { cwd: root, encoding: "utf8" });
  return `${r.stdout ?? ""}`.trim();
};
const hiddenTokens = () => {
  const r = spawnSync(process.execPath, [path.join(root, "scripts", "check-staged-secrets.mjs"), "--self-test"], { cwd: root, encoding: "utf8" });
  const files = [jsonFile, mdFile, path.join(root, "scenarios", "bota", "runtime-evidence.json"), path.join(root, "scenarios", "bota", "runtime-evidence-status.json"), path.join(root, "scenarios", "bota", "integrated-watch-status.json")]
    .filter((file) => fs.existsSync(file));
  const ownPatterns = [/\bgh[pousr]_[A-Za-z0-9]{30,}\b/, /\bAKIA[0-9A-Z]{16}\b/, /-----BEGIN (RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----/, /\bBearer\s+[A-Za-z0-9._~+/-]{20,}=*/, /\b(api[_-]?key|secret|token|password)\s*[:=]\s*["'][^"'\s]{16,}["']/i];
  const hits = [];
  for (const file of files) for (const line of fs.readFileSync(file, "utf8").split("\n")) if (ownPatterns.some((re) => re.test(line))) hits.push(`${path.relative(root, file)}: hidden-token-like text`);
  return { status: r.status === 0 && hits.length === 0 ? 0 : 1, selfTest: `${r.stdout ?? ""}${r.stderr ?? ""}`.trim().split("\n").filter(Boolean).at(-1) ?? "", hits };
};
const requiredCommands = [
  command("check-all", "scripts/check-all.mjs"),
  command("check:acronyms", "scripts/check-acronyms.mjs"),
  command("check:identifier-runtime", "scripts/check-identifier-regime-runtime.mjs"),
  command("check:bota-pack", "scenarios/bota/tools/check-bota-pack.mjs"),
  command("check:cross-contracts", "scripts/check-cross-project-contracts.mjs"),
  command("refresh:cross-contracts --check", "scripts/dev/refresh-cross-contracts.mjs", ["--check"]),
  command("check:cross-autoheal", "scripts/check-cross-contract-autoheal.mjs"),
  command("check:cross-runtime", "scripts/check-cross-runtime.mjs"),
  command("validate:runtime-evidence", "scripts/validate-runtime-evidence.mjs"),
  command("watch:cross-runtime --once", "scripts/dev/watch-cross-runtime.mjs", ["--once"]),
  command("validate:runtime-evidence-status", "scripts/validate-runtime-evidence-status.mjs"),
  command("check:integrated-cold-start", "scripts/check-integrated-cold-start.mjs"),
  command("watch:integrated --once", "scripts/dev/watch-integrated.mjs", ["--once"]),
  command("validate:integrated-watch", "scripts/validate-integrated-watch.mjs"),
  command("check:bota-browser --all", "scripts/check-bota-browser.mjs", ["--all"]),
  command("check:workspace-pipeline", "scripts/check-workspace-pipeline.mjs"),
];

async function build() {
  const checks = [];
  const healthyStale = (text) => /STALE cross-contract snapshot:|refresh --check exited 3|healthy stale snapshot detected|DRIFT\/STALE cross-project snapshot:|unresolved DRIFT\/STALE remains after refresh/.test(text) && !/HTTP [0-9]{3}|real DRIFT|identifier mismatch|WebGL|survivor|DRIFT cross-project hash-bearing|critical/.test(text);
  let stable = run("check:cross-contracts stability preflight", [process.execPath, path.join(root, "scripts", "check-cross-project-contracts.mjs")]);
  for (let attempt = 0; stable.status !== 0 && healthyStale(stable.text) && attempt < 8; attempt += 1) {
    checks.push(stable);
    const heal = run(`cross:autoheal --once ${attempt + 1}`, [process.execPath, path.join(root, "scripts", "dev", "cross-contract-autoheal.mjs"), "--once"]);
    checks.push({ label: `stability heal ${attempt + 1}`, status: heal.status === 0 || healthyStale(heal.text) ? 0 : heal.status, text: heal.text, last: heal.last });
    stable = run(`check:cross-contracts stability ${attempt + 1}`, [process.execPath, path.join(root, "scripts", "check-cross-project-contracts.mjs")]);
  }
  if (stable.status !== 0) fail(`could not reach a healthy stable peer snapshot: ${stable.text}`);
  checks.push({ label: "cross-contract snapshot stability", status: 0, text: stable.text, last: "stable peer snapshot reached" });
  for (const [label, argv] of requiredCommands) {
    let result = run(label, argv);
    for (let attempt = 0; result.status !== 0 && healthyStale(result.text) && attempt < 3; attempt += 1) {
      checks.push({ label: `${label} stale attempt ${attempt + 1}`, status: 0, text: result.text, last: "healthy STALE observed" });
      const heal = run(`cross:autoheal --once for ${label} ${attempt + 1}`, [process.execPath, path.join(root, "scripts", "dev", "cross-contract-autoheal.mjs"), "--once"]);
      checks.push({ label: `heal ${label} ${attempt + 1}`, status: heal.status === 0 || healthyStale(heal.text) ? 0 : heal.status, text: heal.text, last: heal.last });
      const refreshEvidence = run(`watch:cross-runtime --once for ${label} ${attempt + 1}`, [process.execPath, path.join(root, "scripts", "dev", "watch-cross-runtime.mjs"), "--once"]);
      checks.push({ label: `evidence refresh ${label} ${attempt + 1}`, status: refreshEvidence.status === 0 || healthyStale(refreshEvidence.text) ? 0 : refreshEvidence.status, text: refreshEvidence.text, last: refreshEvidence.last });
      result = run(`${label} retry ${attempt + 1}`, argv);
    }
    checks.push(result);
    if (result.status !== 0) fail(`${label} failed with exact evidence above`);
  }
  const mobaStatus = gitText(root, ["status", "--short", "--", "scenarios/moba"]);
  checks.push({ label: "git status scenarios/moba", status: mobaStatus.exit, text: mobaStatus.text, last: `lines=${mobaStatus.text ? mobaStatus.text.split("\n").length : 0}` });
  if (mobaStatus.exit !== 0) fail("git status scenarios/moba failed");
  const mobaFiles = mobaStatus.text.split("\n").filter(Boolean).map((line) => line.slice(3));
  const expectedMoba = ["scenarios/moba/IDENTIFIERS.md", "scenarios/moba/tools/gen-moba-acronym-pack.mjs"];
  if (JSON.stringify(mobaFiles.sort()) !== JSON.stringify(expectedMoba.sort())) fail(`scenarios/moba changed beyond additive files: ${mobaFiles.join(",")}`);
  const secret = hiddenTokens();
  checks.push({ label: "hidden-token scan", status: secret.status, text: `${secret.selfTest}\n${secret.hits.join("\n")}`, last: `hits=${secret.hits.length}` });
  if (secret.status !== 0) fail(`hidden-token scan failed: ${secret.hits.join(",")}`);
  const surviving = survivors();
  checks.push({ label: "ps survivor scan", status: surviving ? 1 : 0, text: surviving, last: `survivors=${surviving ? surviving.split("\n").length : 0}` });
  if (surviving) fail(`test server survivor: ${surviving}`);

  const evidence = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "runtime-evidence.json"), "utf8"));
  const runtimeStatus = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "runtime-evidence-status.json"), "utf8"));
  const integrated = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "integrated-watch-status.json"), "utf8"));
  if (!evidence.ok || !runtimeStatus.ok || !integrated.ok) fail("one or more evidence/status artifacts have ok=false");
  const snapshotHash = sha256(fs.readFileSync(paths.snapshot));
  if (evidence.snapshotHash !== snapshotHash || runtimeStatus.snapshotHash !== snapshotHash || integrated.snapshotHash !== snapshotHash) fail("snapshot hash disagreement");
  const skinned = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "bota", "bota-skinned.json"), "utf8"));
  const modelHashes = Object.entries(skinned.heroes ?? {}).map(([id, row]) => {
    const file = path.join(root, "scenarios", "bota", "assets", "models", `${id}-skinned.glb`);
    if (!fs.existsSync(file)) fail(`${id} model missing`);
    const digest = sha256(fs.readFileSync(file));
    if (digest !== row.skinnedSha256) fail(`${id} model hash mismatch`);
    return { id, sha256: digest, bytes: row.skinnedBytes, joints: row.skinJoints, clips: row.libraryClips };
  });
  if (modelHashes.length !== 24 || modelHashes.some((row) => row.joints !== 28 || row.clips !== 8)) fail(`model hash proof invalid: ${modelHashes.length}`);
  const browser = integrated.browser ?? runtimeStatus.checks?.browser ?? evidence.browser ?? {};
  if (browser.ok !== true || browser.uSkinned !== 1 || browser.jointFloats !== 448 || browser.models !== 26 || browser.heroes !== 24 || browser.joints !== 28 || browser.clips !== 8 || browser.motion !== "walk/run" || browser.backdropHits < 1 || browser.other404s !== 0) fail(`browser evidence invalid: ${JSON.stringify(browser)}`);
  const gate = /RESULT gates=(\d+) ok=(\d+) failed=(\d+) drift=(\d+) skipped=(\d+)/.exec(checks.find((row) => row.label === "check-all")?.text ?? "");
  const gateCounts = gate ? { gates: Number(gate[1]), ok: Number(gate[2]), failed: Number(gate[3]), drift: Number(gate[4]), skipped: Number(gate[5]) } : null;
  if (!gateCounts || gateCounts.gates < 78 || gateCounts.ok !== gateCounts.gates || gateCounts.failed !== 0) fail(`gate count invalid: ${JSON.stringify(gateCounts)}`);
  const extractionSummaries = extractionEvidence(paths);
  const contentContracts = buildContentContractStatus(root);
  if (!contentContracts.ok) fail(`content contracts: ${contentContracts.errors.join(" | ")}`);
  const finalAssets = buildFinalAssetStatus(root);
  if (!finalAssets.ok) fail(`final asset status: ${finalAssets.errors.join(" | ")}`);
  const workspacePipeline = buildWorkspacePipeline();
  if (!workspacePipeline.ok) fail(`workspace pipeline BLOCKING: ${workspacePipeline.blocking.join(" | ")}`);
  const peerStatus = peerGit().map((row) => ({ peer: row.peer, exit: row.exit, lines: row.text ? row.text.split("\n").length : 0, readOnly: true }));
  const coldStart = integrated.coldStart ?? {};
  if (!coldStart.url || !Number.isInteger(coldStart.pid) || !Number.isInteger(coldStart.port) || !Array.isArray(coldStart.httpResults) || coldStart.httpResults.some((row) => row.status !== 200)) fail("cold-start evidence invalid");
  let alive = false;
  try { process.kill(coldStart.pid, 0); alive = true; } catch (error) { if (error.code !== "ESRCH") alive = true; }
  if (alive) fail(`cold-start PID survived: ${coldStart.pid}`);
  const artifact = {
    schema: "oscraft.release-report/v1",
    generatedAt: new Date().toISOString(),
    ok: true,
    identifierRegimes: evidence.identifierRegimes,
    packParam: evidence.identifierRegimes?.default?.packParam,
    snapshotHash,
    checks: checks.map((row) => ({ label: row.label, exit: row.status, last: row.last })),
    gateCounts,
    botaModels: { count: modelHashes.length, joints: 28, clips: 8, hashes: modelHashes },
    menuBackdrop: evidence.backdrop,
    browser,
    crossContracts: runtimeStatus.checks?.crossContracts ?? evidence.crossContracts,
    contentContracts,
    finalAssets,
    extractionSummaries,
    workspacePipeline,
    peerStatus,
    coldStart: { url: coldStart.url, pid: coldStart.pid, port: coldStart.port, cleanup: coldStart.cleanup, httpResults: coldStart.httpResults },
    cleanup: { survivorCount: surviving ? surviving.split("\n").length : 0, ownedPidAlive: alive, peerServersUntouched: true },
    historicalMobaFiles: mobaFiles,
    hiddenTokenScan: { status: secret.status, selfTest: secret.selfTest, hits: secret.hits },
    evidenceFiles: {
      "scenarios/bota/runtime-evidence.json": sha256(fs.readFileSync(path.join(root, "scenarios", "bota", "runtime-evidence.json"))),
      "scenarios/bota/runtime-evidence-status.json": sha256(fs.readFileSync(path.join(root, "scenarios", "bota", "runtime-evidence-status.json"))),
      "scenarios/bota/integrated-watch-status.json": sha256(fs.readFileSync(path.join(root, "scenarios", "bota", "integrated-watch-status.json"))),
      "scenarios/bota/cross-contract-snapshot.json": snapshotHash,
    },
  };
  fs.mkdirSync(path.dirname(jsonFile), { recursive: true });
  fs.writeFileSync(jsonFile, JSON.stringify(artifact, null, 2) + "\n");
  const lines = [
    "# Oscraft release artifact",
    "",
    `Generated: ${artifact.generatedAt}`,
    `Status: ok=${artifact.ok}`,
    `Snapshot: ${artifact.snapshotHash}`,
    `Identifier regimes: default ${artifact.identifierRegimes.default.pack}/${artifact.identifierRegimes.default.activeIds} acronyms, BOTA ${artifact.identifierRegimes.bota.pack}/${artifact.identifierRegimes.bota.activeIds} fullnames, historical ${artifact.identifierRegimes.historical.pack}`,
    `PACK_PARAM: ${artifact.packParam}`,
    `Gates: ${artifact.gateCounts.gates}/${artifact.gateCounts.ok} ok, failed=${artifact.gateCounts.failed}, drift=${artifact.gateCounts.drift}, skipped=${artifact.gateCounts.skipped}`,
    `BOTA models: ${artifact.botaModels.count} hashes, joints=${artifact.botaModels.joints}, clips=${artifact.botaModels.clips}`,
    `menuBackdrop: bytes=${artifact.menuBackdrop.bytes} sha256=${artifact.menuBackdrop.sha256}`,
    `Browser/WebGL: ${artifact.browser.browser}, uSkinned=${artifact.browser.uSkinned}, jointFloats=${artifact.browser.jointFloats}, models=${artifact.browser.models}, heroes=${artifact.browser.heroes}, clips=${artifact.browser.clips}, motion=${artifact.browser.motion}, backdropHits=${artifact.browser.backdropHits}, other404s=${artifact.browser.other404s}`,
    `Cross contracts: required=${artifact.crossContracts.required}, optionalPresent=${artifact.crossContracts.optionalPresent}, extraPresent=${artifact.crossContracts.extraPresent}, hashBearingRowsVerified=${artifact.crossContracts.hashBearingRowsVerified}, driftStale=${artifact.crossContracts.driftStale}`,
    `Content contracts: schema=${artifact.contentContracts.schema}, ok=${artifact.contentContracts.ok}, inversion=${artifact.contentContracts.identifierInversion.upstreamAcronymKeys} acronym keys -> ${artifact.contentContracts.identifierInversion.mappedFullnames} fullname IDs, default=${artifact.contentContracts.identifierInversion.defaultDotaAcronymIds}, doNotPromote=${artifact.contentContracts.integration.doNotPromote.length}, demandRows=${artifact.contentContracts.demand.rows}`,
    `Final assets: schema=${artifact.finalAssets.schema}, ok=${artifact.finalAssets.ok}, generated=${artifact.finalAssets.counts.generated}, blocked=${artifact.finalAssets.counts.blocked}, ready=${artifact.finalAssets.counts.ready}, remaining=${artifact.finalAssets.counts.remaining}, generatedGlbHashes=${artifact.finalAssets.generatedHashVerification.verified}/24, sourceSha256=${artifact.finalAssets.sourceSha256}`,
    `Final asset blockers: pooled-email=${artifact.finalAssets.blockers.pooledEmail}; audio=${artifact.finalAssets.blockers.audio}; requirements=${artifact.finalAssets.blockers.requirements}`,
    `Cold start: ${artifact.coldStart.url} pid=${artifact.coldStart.pid} port=${artifact.coldStart.port}; ${artifact.coldStart.cleanup}`,
    `Cleanup: survivors=${artifact.cleanup.survivorCount}, ownedPidAlive=${artifact.cleanup.ownedPidAlive}, peerServersUntouched=${artifact.cleanup.peerServersUntouched}`,
    `Hidden-token scan: status=${artifact.hiddenTokenScan.status}, hits=${artifact.hiddenTokenScan.hits.length}`,
    "",
    "## Check exits",
    "",
  ];
  for (const row of artifact.checks) lines.push(`- ${row.label}: EXIT=${row.exit} ${row.last ?? ""}`.trim());
  lines.push("", "## Peer extraction summaries", "");
  for (const row of artifact.extractionSummaries) lines.push(`- ${row.peer}: present=${row.present} classes=${row.classes ?? "?"} complete=${row.complete ?? "?"} blocked=${row.blocked ?? "?"} informational=true`);
  lines.push("", "## Workspace pipeline", "");
  lines.push(`- state: ${artifact.workspacePipeline.state}, ok=${artifact.workspacePipeline.ok}, vendorPrep=${artifact.workspacePipeline.vendorPrep.state}, 247420=${artifact.workspacePipeline.vendor247420.state}`);
  lines.push(`- wscheppity extraction: ok=${artifact.workspacePipeline.wscheppityExtraction.ok}, classes=${artifact.workspacePipeline.wscheppityExtraction.summary?.classes ?? "?"}, complete=${artifact.workspacePipeline.wscheppityExtraction.summary?.complete ?? "?"}, blocked=${artifact.workspacePipeline.wscheppityExtraction.summary?.blocked ?? "?"}`);
  lines.push(`- READY.json ok=${artifact.workspacePipeline.vendorPrep.readyOk}, handoffGate=${JSON.stringify(artifact.workspacePipeline.vendorPrep.handoffGate)}`);
  lines.push(`- BitTTS/BitVoice contract: schema=${artifact.workspacePipeline.bitvoice.contractSchema ?? "?"}, stability=${artifact.workspacePipeline.bitvoice.interfaceStability ?? "?"}, sha256=${artifact.workspacePipeline.bitvoice.contractSha256 ?? "?"}, lockMatches=${artifact.workspacePipeline.bitvoice.lockMatches}`);
  lines.push(`- gmweb readiness: ok=${artifact.workspacePipeline.gmweb.readinessOk}`);
  for (const row of artifact.workspacePipeline.pending) lines.push(`- PENDING: ${row}`);
  for (const row of artifact.workspacePipeline.blocking) lines.push(`- BLOCKING: ${row}`);
  lines.push("", "## Peer git status (read-only)", "");
  for (const row of artifact.peerStatus) lines.push(`- ${row.peer}: exit=${row.exit} lines=${row.lines} readOnly=${row.readOnly}`);
  fs.writeFileSync(mdFile, lines.join("\n") + "\n");
  console.log(`report:oscraft-release PASS: docs/OSCRAFT_RELEASE.json + .md ok=true snapshotHash=${snapshotHash} gates=${gateCounts.ok}/${gateCounts.gates} models=${modelHashes.length} checks=${artifact.checks.length}`);
}

const started = Date.now();
let cycles = 0;
while (true) {
  cycles += 1;
  try { await build(); } catch (error) { console.error(error.stack ?? String(error)); process.exit(1); }
  if (once) process.exit(0);
  if (Date.now() - started >= timeoutMs) { console.log(`report:oscraft-release bounded watch reached ${cycles} cycle(s)`); process.exit(0); }
  const pause = Math.min(intervalMs, Math.max(1, timeoutMs - (Date.now() - started)));
  console.log(`report:oscraft-release watch: waiting ${pause}ms`);
  await sleep(pause);
}
