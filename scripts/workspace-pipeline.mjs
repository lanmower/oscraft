import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const ROOT = "/workspace";
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const rel = (file) => file.replace(/^\/workspace\//, "");

export function pipelinePaths() {
  const wscheppity = path.join(ROOT, "wscheppity");
  const vendorPrep = path.join(wscheppity, ".local", "vendor-prep");
  const bitvoice = path.join(ROOT, "bitvoice");
  const gmweb = path.join(ROOT, "gmweb");
  const extra = path.join(ROOT, "extra");
  return {
    root: ROOT,
    wscheppity,
    vendorPrep,
    bitvoice,
    gmweb,
    extra,
    files: {
      wscheppityExtraction: path.join(wscheppity, ".local", "extraction-manifest.json"),
      vendorPrepManifest: path.join(vendorPrep, "manifest.json"),
      vendorPrepReady: path.join(vendorPrep, "READY.json"),
      vendorPrepHandoff: path.join(vendorPrep, "handoff.json"),
      vendorPrepNotes: path.join(vendorPrep, "247420-HANDOFF.md"),
      bitvoiceContract: path.join(bitvoice, "service-contract.json"),
      bitvoiceLock: path.join(bitvoice, "service-contract.lock.json"),
      bitvoiceRelease: path.join(bitvoice, "docs", "BITVOICE_RELEASE.json"),
      gmwebReadiness: path.join(gmweb, "docs", "RELEASE_READINESS.json"),
      bitvoiceReleaseDuplicate: path.join(bitvoice, "docs", "BITVOICE_RELEASE.json"),
      extraExtractionStatus: path.join(extra, ".local", "extraction-status.json"),
      prepChecker: path.join(ROOT, "247420", "scripts", "check-wonschweppidytime-prep.mjs"),
      docs247420: path.join(ROOT, "247420", "docs"),
    },
  };
}

function readJson(file) {
  try { return JSON.parse(fs.readFileSync(file, "utf8")); } catch (error) { return { __error: error.message }; }
}

function fileEvidence(file) {
  if (!fs.existsSync(file)) return { path: rel(file), present: false };
  const bytes = fs.readFileSync(file);
  const out = { path: rel(file), present: true, bytes: bytes.length, sha256: sha256(bytes) };
  if (file.endsWith(".json")) {
    const json = readJson(file);
    if (json && json.__error) out.error = json.__error;
    else out.json = json;
  }
  return out;
}

function countFrom(obj, keys) {
  if (!obj || typeof obj !== "object") return null;
  for (const key of keys) {
    if (Object.prototype.hasOwnProperty.call(obj, key)) return obj[key];
  }
  return null;
}

function collectCounts(...objects) {
  const out = {};
  for (const obj of objects.filter(Boolean)) {
    if (typeof obj !== "object") continue;
    for (const [key, value] of Object.entries(obj)) {
      if (/opus|12kbps|brotli|rerun|optimal|compression|package|content/i.test(key)) out[key] = value;
    }
  }
  return Object.keys(out).length ? out : null;
}

function list247420Docs(dir) {
  if (!fs.existsSync(dir)) return [];
  const out = [];
  const walk = (base, relDir = "") => {
    for (const entry of fs.readdirSync(base, { withFileTypes: true })) {
      const abs = path.join(base, entry.name);
      const next = path.posix.join(relDir, entry.name);
      if (entry.isDirectory()) walk(abs, next);
      else out.push({ path: `247420/docs/${next}`, bytes: fs.statSync(abs).size, sha256: sha256(fs.readFileSync(abs)) });
    }
  };
  walk(dir);
  return out.sort((a, b) => a.path.localeCompare(b.path));
}

function vendored247420(p) {
  const candidates = [
    path.join(ROOT, "247420", "vendor"),
    path.join(ROOT, "247420", "vendored"),
    path.join(ROOT, "247420", "assets", "wonschweppidytime"),
    path.join(ROOT, "247420", "docs", "VENDORED.json"),
    path.join(ROOT, "247420", "docs", "VENDORED.md"),
  ];
  const found = candidates.filter((file) => fs.existsSync(file));
  return found.map((file) => {
    const stat = fs.statSync(file);
    return { path: rel(file), kind: stat.isDirectory() ? "directory" : "file", bytes: stat.isFile() ? stat.size : null, sha256: stat.isFile() ? sha256(fs.readFileSync(file)) : null };
  });
}

export function buildWorkspacePipeline() {
  const p = pipelinePaths();
  const evidence = {};
  for (const [key, file] of Object.entries(p.files)) {
    if (key === "docs247420" || key === "prepChecker") continue;
    evidence[key] = fileEvidence(file);
  }
  const prepChecker = fileEvidence(p.files.prepChecker);
  const docs247420 = list247420Docs(p.files.docs247420);
  const blocking = [];
  const pending = [];
  const extraction = evidence.wscheppityExtraction?.json;
  const extractionSummary = extraction?.summary ?? null;
  const extractionOk = Boolean(extractionSummary && extractionSummary.complete === extractionSummary.classes && extractionSummary.blocked === 0);
  if (!evidence.wscheppityExtraction.present) pending.push(`${rel(p.files.wscheppityExtraction)}: wscheppity extraction manifest absent`);
  else if (!extractionOk) blocking.push(`${rel(p.files.wscheppityExtraction)}: extraction summary incomplete or blocked`);

  const requiredPrep = [evidence.vendorPrepManifest, evidence.vendorPrepReady, evidence.vendorPrepHandoff, evidence.vendorPrepNotes];
  const prepMissing = requiredPrep.filter((row) => !row.present).map((row) => row.path);
  const readyJson = evidence.vendorPrepReady?.json;
  const handoff = evidence.vendorPrepHandoff?.json;
  const manifest = evidence.vendorPrepManifest?.json;
  let vendorPrepState = "UNAVAILABLE";
  if (!prepMissing.length) {
    const badJson = requiredPrep.some((row) => row.error);
    const readyOk = readyJson?.ok === true;
    const handoffGate = handoff?.DO_NOT_VENDOR_UNTIL === "READY.json ok=true";
    const handoffArray = Array.isArray(handoff?.files) || Array.isArray(handoff?.members);
    vendorPrepState = badJson ? "NOT_READY" : readyOk && handoffGate && handoffArray && manifest && typeof manifest === "object" ? "READY" : "NOT_READY";
    if (vendorPrepState === "NOT_READY") {
      if (!readyOk) blocking.push(`${rel(p.files.vendorPrepReady)}: ok is not true`);
      if (!handoffGate) blocking.push(`${rel(p.files.vendorPrepHandoff)}: DO_NOT_VENDOR_UNTIL is not 'READY.json ok=true'`);
      if (!handoffArray) blocking.push(`${rel(p.files.vendorPrepHandoff)}: no files/members array`);
      for (const row of requiredPrep) if (row.error) blocking.push(`${row.path}: invalid JSON ${row.error}`);
    }
  } else {
    pending.push(`vendor-prep missing: ${prepMissing.join(",")}`);
  }

  const vendored = vendored247420(p);
  let vendor247420State = "WAITING";
  if (vendored.length && vendorPrepState !== "READY") {
    vendor247420State = "BLOCKED";
    blocking.push(`247420 vendored content present before READY.json ok=true: ${vendored.map((row) => row.path).join(",")}`);
  } else if (vendored.length) vendor247420State = "VENDORED";

  const contract = evidence.bitvoiceContract?.json;
  const lock = evidence.bitvoiceLock?.json;
  const contractHash = evidence.bitvoiceContract?.sha256 ?? null;
  if (evidence.bitvoiceContract.present && evidence.bitvoiceLock.present && lock?.sha256 && lock.sha256 !== contractHash) blocking.push(`${rel(p.files.bitvoiceLock)}: sha256 does not match ${rel(p.files.bitvoiceContract)}`);
  if (contract?.interfaceStability && contract.interfaceStability !== "stable") blocking.push(`${rel(p.files.bitvoiceContract)}: interfaceStability is not stable`);
  const bitvoiceRelease = evidence.bitvoiceRelease?.json;
  const gmwebReadiness = evidence.gmwebReadiness?.json;
  const extraStatus = evidence.extraExtractionStatus?.json;
  if (evidence.gmwebReadiness.present && gmwebReadiness?.ok !== true) blocking.push(`${rel(p.files.gmwebReadiness)}: ok is not true`);
  if (evidence.extraExtractionStatus.present && extraStatus?.ok === false) blocking.push(`${rel(p.files.extraExtractionStatus)}: ok is false`);
  if (evidence.bitvoiceRelease.present && bitvoiceRelease?.ok === false) blocking.push(`${rel(p.files.bitvoiceRelease)}: ok is false`);

  const counts = collectCounts(readyJson, handoff, manifest, bitvoiceRelease, extraStatus);
  const state = blocking.length ? "BLOCKED" : pending.length ? "PENDING" : "READY";
  return {
    schema: "oscraft.workspace-pipeline/v1",
    state,
    ok: blocking.length === 0,
    wscheppityExtraction: { ok: extractionOk, summary: extractionSummary, path: rel(p.files.wscheppityExtraction) },
    vendorPrep: {
      state: vendorPrepState,
      readyOk: readyJson?.ok === true,
      required: requiredPrep.map((row) => ({ path: row.path, present: row.present, sha256: row.sha256 ?? null })),
      handoffGate: handoff?.DO_NOT_VENDOR_UNTIL ?? null,
      counts,
    },
    vendor247420: { state: vendor247420State, vendored, prepRequiredBeforeVendor: true },
    bitvoice: {
      contractSchema: contract?.schemaVersion ?? null,
      contractName: contract?.contractName ?? null,
      interfaceStability: contract?.interfaceStability ?? null,
      contractSha256: contractHash,
      lockSha256: lock?.sha256 ?? null,
      lockMatches: Boolean(contractHash && lock?.sha256 === contractHash),
      release: bitvoiceRelease ? { ok: bitvoiceRelease.ok ?? null, path: rel(p.files.bitvoiceRelease) } : null,
    },
    gmweb: { readinessOk: gmwebReadiness?.ok === true, path: rel(p.files.gmwebReadiness) },
    extra: { status: extraStatus ?? null, path: rel(p.files.extraExtractionStatus) },
    docs247420,
    prepChecker: { path: prepChecker.path, present: prepChecker.present, sha256: prepChecker.sha256 ?? null },
    botaHy3d: "existing oscraft cross-contract manifests remain authoritative",
    pending,
    blocking,
    evidence,
  };
}

export function workspacePipelineState() {
  return buildWorkspacePipeline();
}
