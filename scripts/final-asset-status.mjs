import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

export const FINAL_ASSET_SCHEMA = "oscraft.hy3d-final-asset-status/v1";
export const FINAL_ASSET_COUNTS = Object.freeze({ generated: 24, blocked: 30, ready: 24, remaining: 30, rows: 56 });
export const FINAL_ASSET_BLOCKERS = Object.freeze({
  pooledEmail: "stale local rig request shape fixed; generic service probe returned HTTP 200 with remainingTimes; stale witch_doctor chain still returns HTTP 500/no worksId",
  audio: "no studio/relay audio action; local BitVoice service absent and production bridge missing",
  requirements: "12 kbps Opus and Brotli requirements were not relaxed; no uncompressed or fabricated audio was published",
});

const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const readText = (file) => fs.readFileSync(file, "utf8");
const readJson = (file) => JSON.parse(readText(file));

export function finalAssetPaths(root) {
  const hy3dRoot = path.resolve(root, "..", "hy3d", "output");
  return {
    status: path.join(hy3dRoot, "bota-final-asset-status.json"),
    tsv: path.join(hy3dRoot, "bota-final-asset-status.tsv"),
    markdown: path.join(hy3dRoot, "BOTA-FINAL-ASSET-STATUS.md"),
    evidence: path.join(hy3dRoot, "bota-final-asset-evidence.json"),
    reconciliation: path.join(hy3dRoot, "bota-remaining-reconciliation.txt"),
    slugs: path.join(hy3dRoot, "bota-remaining-slugs.txt"),
  };
}

export function finalAssetErrors(input) {
  const { status, evidence, tsv, markdown, reconciliation, slugs, generatedFiles } = input;
  const errors = [];
  const check = (condition, message) => { if (!condition) errors.push(message); };
  check(status.schema === "hy3d.bota-final-asset-status/v1", `final asset schema=${status.schema}`);
  check(status.generatedCount === FINAL_ASSET_COUNTS.generated, `generated=${status.generatedCount}`);
  check(status.blockedCount === FINAL_ASSET_COUNTS.blocked, `blocked=${status.blockedCount}`);
  check(status.counts?.ready === FINAL_ASSET_COUNTS.ready, `ready=${status.counts?.ready}`);
  check(status.remainingAssetCount === FINAL_ASSET_COUNTS.remaining, `remaining=${status.remainingAssetCount}`);
  check(status.rows?.length === FINAL_ASSET_COUNTS.rows, `rows=${status.rows?.length}`);
  check(status.generated?.length === FINAL_ASSET_COUNTS.generated, `generated rows=${status.generated?.length}`);
  check(status.rows?.filter((row) => row.status === "blocked").length === FINAL_ASSET_COUNTS.blocked, "blocked row count differs");
  check(status.rows?.filter((row) => row.status === "ready").length === FINAL_ASSET_COUNTS.ready, "ready row count differs");
  check(status.rows?.filter((row) => row.status === "blocked").every((row) => ["hero-voice", "clip"].includes(row.kind)), "blocked rows are not audio/clip rows");
  check(status.rows?.filter((row) => row.status === "blocked").every((row) => row.path === null && row.bytes === null && row.sha256 === null), "a blocked row claims generated asset bytes");
  check(status.generated?.every((row) => row.validation?.status === "valid" && row.validation?.complete === true && row.validation?.magic === "glTF"), "a generated GLB is not complete/valid");
  check(generatedFiles?.verified === FINAL_ASSET_COUNTS.generated, `live GLB hashes verified=${generatedFiles?.verified}`);
  check(status.audio?.wavCount === 0 && status.audio?.healthStatus === 0 && status.audio?.bridgeScriptExists === false && status.audio?.opusEncoder === false && status.audio?.brotliEncoder === false, "audio blocker evidence changed");
  check(String(status.audio?.requirementsPreserved ?? "") === FINAL_ASSET_BLOCKERS.requirements, "Opus/Brotli requirement preservation differs");
  check(status.pooledEmail?.attempted === true && status.pooledEmail?.quotaBefore === 30 && status.pooledEmail?.quotaAfter === 30 && status.pooledEmail?.recovery === true, "pooled-email quota/recovery evidence changed");
  check(String(status.pooledEmail?.diagnosis ?? "") === FINAL_ASSET_BLOCKERS.pooledEmail, "pooled-email diagnosis differs");
  check(status.quotaProbe?.total === 30 && status.quotaProbe?.perAccount === 5 && status.quotaProbe?.exitCode === 0, "quota probe evidence changed");
  check(status.relay?.queued === 24 && status.relay?.completeGlbs === 24 && status.relay?.incomplete === 0, "relay GLB evidence changed");
  check(status.rows?.filter((row) => row.status === "blocked").every((row) => row.reason === FINAL_ASSET_BLOCKERS.audio), "audio/clip blocked reason differs");
  check(evidence.generated?.length === FINAL_ASSET_COUNTS.generated, `evidence generated rows=${evidence.generated?.length}`);
  check(JSON.stringify(evidence.generated) === JSON.stringify(status.generated), "evidence generated rows differ from status");
  check(JSON.stringify(evidence.manifestHashes) === JSON.stringify(status.manifestHashes), "evidence manifest hashes differ from status");
  check(JSON.stringify(evidence.audio) === JSON.stringify(status.audio), "evidence audio blocker differs from status");
  check(tsv.startsWith("slug\tkind\tpriorStatus\tstatus\tmethod\treason\tpath\tbytes\tsha256\n"), "status TSV header differs");
  check(tsv.trim().split("\n").length === FINAL_ASSET_COUNTS.rows + 1, `status TSV rows=${tsv.trim().split("\n").length - 1}`);
  check(markdown.includes("Remaining assets: **30**. Generated in this run: **24**. Blocked: **30**."), "markdown counts differ");
  check(markdown.includes("HTTP 200 with a real worksId") && markdown.includes("quota moved 30 -> 29"), "markdown pooled-email recovery evidence differs");
  check(markdown.includes("no BOTA row mapping from chatnano WAVs") && markdown.includes("scripts/execute-bittts-audio-bridge.mjs") && markdown.includes("no 12 kbps Opus/Brotli delivery manifest"), "markdown audio blocker differs");
  check(markdown.includes("12 kbps Opus/Brotli requirements are preserved; no audio bytes were fabricated."), "markdown audio requirements differ");
  check(reconciliation.includes("externalBlocker") && reconciliation.includes("HTTP 500 kind ok no worksId"), "reconciliation blocker evidence differs");
  check(reconciliation.includes('"incompleteGlbs":0,"parts":0'), "reconciliation relay evidence differs");
  check(slugs.startsWith("SLUGS\n") && slugs.includes("\thero-voice/nec\t"), "remaining slug evidence differs");
  check(status.evidencePaths?.length === 17, `evidence paths=${status.evidencePaths?.length}`);
  return errors;
}

export function buildFinalAssetStatus(root) {
  const files = finalAssetPaths(root);
  const missing = Object.entries(files).filter(([, file]) => !fs.existsSync(file)).map(([key, file]) => ({ key, file }));
  if (missing.length) return { schema: FINAL_ASSET_SCHEMA, ok: false, missing, errors: missing.map((row) => `${row.key}: missing ${row.file}`) };
  const bytes = Object.fromEntries(Object.entries(files).map(([key, file]) => [key, fs.readFileSync(file)]));
  const status = JSON.parse(bytes.status.toString("utf8"));
  const evidence = JSON.parse(bytes.evidence.toString("utf8"));
  const generatedFiles = { verified: 0, failures: [] };
  for (const row of status.generated ?? []) {
    const file = row.path;
    if (!file || !fs.existsSync(file) || fs.statSync(file).size !== row.bytes || sha256(fs.readFileSync(file)) !== row.sha256) generatedFiles.failures.push(row.slug ?? file ?? "?");
    else generatedFiles.verified += 1;
  }
  const input = {
    status,
    evidence,
    tsv: bytes.tsv.toString("utf8"),
    markdown: bytes.markdown.toString("utf8"),
    reconciliation: bytes.reconciliation.toString("utf8"),
    slugs: bytes.slugs.toString("utf8"),
    generatedFiles,
  };
  const errors = finalAssetErrors(input);
  return {
    schema: FINAL_ASSET_SCHEMA,
    ok: errors.length === 0,
    errors,
    generatedAt: status.generatedAt,
    sourceSha256: sha256(Object.entries(bytes).map(([key, value]) => `${key}\0${value.toString("utf8").length}\0${value.toString("utf8")}`).join("\n")),
    fileHashes: Object.fromEntries(Object.entries(bytes).map(([key, value]) => [key, sha256(value)])),
    counts: {
      generated: status.generatedCount,
      blocked: status.blockedCount,
      ready: status.counts?.ready,
      remaining: status.remainingAssetCount,
      rows: status.rows?.length,
      ownerDecision: status.counts?.["owner-decision"] ?? null,
      intentionallyOpen: status.counts?.["intentionally-open"] ?? null,
    },
    generatedKinds: [...new Set((status.generated ?? []).map((row) => row.kind))].sort(),
    blockedKinds: [...new Set((status.rows ?? []).filter((row) => row.status === "blocked").map((row) => row.kind))].sort(),
    generatedHashVerification: generatedFiles,
    quotaProbe: status.quotaProbe,
    pooledEmail: status.pooledEmail,
    relay: status.relay,
    audio: status.audio,
    blockers: { ...FINAL_ASSET_BLOCKERS },
    policy: status.policy,
    authority: status.authority,
    evidencePaths: status.evidencePaths,
  };
}

export function assertFinalAssetStatus(root) {
  const status = buildFinalAssetStatus(root);
  if (!status.ok) {
    const error = new Error(`final asset status: ${status.errors.join(" | ")}`);
    error.status = status;
    throw error;
  }
  return status;
}
