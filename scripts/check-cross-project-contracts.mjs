#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { classifyDrift, contractPaths, inputRecord, isAvailable, manifestLabel, rowHash, rowPath, rowsOf, sha256File } from "./cross-contract-inputs.mjs";
import { assertContentContracts } from "./content-contract-status.mjs";
import { assertFinalAssetStatus, buildFinalAssetStatus } from "./final-asset-status.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const paths = contractPaths(root);
if (process.argv.includes("--watch-once")) {
  const watcher = path.join(root, "scripts", "dev", "wait-cross-contracts.mjs");
  const result = spawnSync(process.execPath, [watcher, "--watch-once"], { cwd: root, stdio: "inherit" });
  process.exit(result.status ?? 1);
}
const fail = (message) => { console.error(`cross-contracts: ${message}`); process.exit(1); };
const readJson = (file) => JSON.parse(fs.readFileSync(file, "utf8"));
const unavailable = [...paths.required, ...paths.own].filter((file) => !fs.existsSync(file));
if (unavailable.length) {
  console.error("UNAVAILABLE cross-project contracts:");
  for (const file of unavailable) console.error(`  ${file}`);
  process.exit(2);
}
const optionalInputs = [...paths.optional, ...paths.extraOptional];
const optionalPresent = [];
const optionalMissing = [];
const extraPresent = [];
const extraMissing = [];
for (const file of optionalInputs) {
  const label = manifestLabel(file);
  const present = fs.existsSync(file);
  (present ? optionalPresent : optionalMissing).push(label);
  if (file.startsWith(paths.extraRoot)) (present ? extraPresent : extraMissing).push(label);
}
console.log(`optionalPresent=${optionalPresent.length}${optionalPresent.length ? ` ${optionalPresent.join(",")}` : ""}`);
console.log(`optionalMissing=${optionalMissing.length}${optionalMissing.length ? ` ${optionalMissing.join(",")}` : ""}`);
console.log(`extraPresent=${extraPresent.length}${extraPresent.length ? ` ${extraPresent.join(",")}` : ""}`);
console.log(`extraMissing=${extraMissing.length}${extraMissing.length ? ` ${extraMissing.join(",")}` : ""}`);

const snapshot = fs.existsSync(paths.snapshot) ? readJson(paths.snapshot) : null;
const drift = classifyDrift(paths, snapshot);
if (drift.length) {
  console.error("DRIFT/STALE cross-project snapshot:");
  for (const row of drift) console.error(`  ${row.kind} ${row.path}: ${row.detail}`);
  process.exit(3);
}
let contentContracts;
let finalAssets;
try {
  contentContracts = assertContentContracts(root);
  finalAssets = assertFinalAssetStatus(root);
} catch (error) { fail(`peer status: ${error.message}`); }
const staleSummary = (label, expected, actual, source) => {
  console.error("DRIFT/STALE cross-project snapshot:");
  console.error(`  STALE ${source}: snapshot expected ${label} sourceSha256=${expected}; current sourceSha256=${actual}; run npm run refresh:cross-contracts`);
  process.exit(3);
};
if (snapshot?.contentContracts?.ok !== true || snapshot?.finalAssets?.ok !== true) fail("snapshot contentContracts/finalAssets summary has ok=false; real peer status failure");
if (snapshot?.contentContracts?.sourceSha256 !== contentContracts.sourceSha256) staleSummary("contentContracts", snapshot?.contentContracts?.sourceSha256 ?? "absent", contentContracts.sourceSha256, "bota/export/integration-contract.json");
if (snapshot?.finalAssets?.sourceSha256 !== finalAssets.sourceSha256) staleSummary("finalAssets", snapshot?.finalAssets?.sourceSha256 ?? "absent", finalAssets.sourceSha256, "hy3d/output/bota-final-asset-status.json");

const ruleDemand = readJson(paths.required[0]);
const conformance = readJson(paths.required[1]);
const exportStatus = readJson(paths.required[2]);
const spawnCompat = readJson(paths.required[3]);
const content = readJson(paths.required[4]);
const inventory = readJson(paths.required[5]);
const botaReady = readJson(paths.required[6]);
const delivery = readJson(paths.required[7]);
const identifiers = fs.readFileSync(paths.own[0], "utf8");
const ui = readJson(paths.own[1]);
const skinned = readJson(paths.own[2]);
const mapText = fs.readFileSync(paths.own[3], "utf8");
const fullnames = Object.keys(ui.heroes ?? {}).sort();
const fullnameSet = new Set(fullnames);
const legacyToFullname = new Map([...identifiers.matchAll(/^\|\s*\d{3}\s*\|\s*`([^`]+)`\s*\|\s*`([^`]+)`/gm)].map((m) => [m[2], m[1]]));
if (fullnames.length !== 26 || fullnames.some((id) => /^[a-z]{1,3}$/.test(id) && id !== "npc")) fail(`BOTA UI fullname set is not exactly 26 (${fullnames.length})`);
const acronymToSlot = conformance.acronym_to_slot ?? {};
const slotToFullname = new Map([...identifiers.matchAll(/^\|\s*(\d{3})\s*\|\s*`([^`]+)`/gm)].map((m) => [m[1], m[2]]));
const acronymToFullname = new Map(Object.entries(acronymToSlot).map(([acro, slot]) => [acro, slotToFullname.get(slot)]));
if (acronymToFullname.size !== 26 || [...acronymToFullname.values()].some((id) => !fullnameSet.has(id))) fail("bota conformance acronym map does not map through IDENTIFIERS.md to 26 fullnames");
const botaActive = new Set((conformance.active_acronyms ?? []).map((acro) => acronymToFullname.get(acro) ?? acro));
if (botaActive.size !== 26 || [...botaActive].some((id) => !fullnameSet.has(id))) fail(`bota active hero set mismatch: ${botaActive.size} mapped IDs`);
const demandActive = new Set(rowsOf(ruleDemand).filter((row) => row.kind === "bota-hero").map((row) => row.slug?.split("/").at(-1) ?? row.slug).filter(Boolean));
if (demandActive.size && (demandActive.size !== 26 || [...demandActive].some((id) => !fullnameSet.has(id)))) fail(`bota rule-demand active hero set mismatch: ${demandActive.size} IDs`);
if (Number(spawnCompat.bota_counts?.active_heroes ?? 26) !== 26) fail("spawn-compatibility does not report 26 active BOTA heroes");
if (exportStatus.status && !["PASS", "READY", "OK"].includes(String(exportStatus.status).toUpperCase())) console.log(`note: bota export-status status=${exportStatus.status}`);

const defaultRows = mapText.trim().split("\n").slice(1).map((line) => line.split("\t"));
const defaultIds = defaultRows.map((row) => row[2]);
if (defaultIds.length !== 127 || defaultIds.some((id) => !/^[a-z]{1,3}$/.test(id)) || new Set(defaultIds).size !== 127) fail(`default Dota IDs are not 127 unique acronyms (${defaultIds.length})`);
const loader = fs.readFileSync(paths.own[4], "utf8");
if (!/PACK_PARAM[^\n]*"scenarios\/moba-acronyms"/.test(loader)) fail("default PACK_PARAM is not scenarios/moba-acronyms");
if (Object.keys(skinned.heroes ?? {}).length !== 26) fail(`bota-skinned.json has ${Object.keys(skinned.heroes ?? {}).length} heroes`);
for (const id of fullnames) {
  const provenance = skinned.heroes[id];
  if (!provenance) fail(`${id}: missing bota-skinned.json provenance`);
  const file = path.join(root, "scenarios", "bota", "assets", "models", `${id}-skinned.glb`);
  if (!fs.existsSync(file)) fail(`${id}: skinned model is missing`);
  const hash = sha256File(file);
  if (hash !== provenance.skinnedSha256 || fs.statSync(file).size !== provenance.skinnedBytes) fail(`${id}: skinned model hash/bytes mismatch`);
}

const inside = (file, base) => file === base || file.startsWith(base + path.sep);
const allowedRoots = [paths.botaRoot, paths.hy3dRoot, paths.wscheppityRoot, root];
const resolveAny = (value, ...roots) => {
  if (!value || typeof value !== "string") return null;
  const candidates = [];
  for (const base of roots.filter(Boolean)) {
    candidates.push(value, path.join(base, value.replace(/^\/?/, "")), path.join(base, value.replace(new RegExp(`^/?${path.basename(base)}/`), "")));
  }
  for (const candidate of candidates) {
    const abs = path.resolve(candidate);
    if (allowedRoots.some((base) => inside(abs, base)) && fs.existsSync(abs) && fs.statSync(abs).isFile()) return abs;
  }
  return null;
};
const migratedCandidates = (row) => {
  const out = [];
  for (const value of [row.legacyDestinationPath, row.destinationPath, row.legacyPath, row.oldPath]) {
    if (!value || typeof value !== "string") continue;
    const base = path.basename(value, ".glb");
    const stem = base.replace(/^bota-/, "").replace(/-skinned$/, "").replace(/_skinned$/, "");
    const active = legacyToFullname.get(stem) ?? (fullnameSet.has(stem) ? stem : acronymToFullname.get(stem));
    if (active) out.push(path.join(root, "scenarios", "bota", "assets", "models", `${active}-skinned.glb`));
    out.push(value);
  }
  return out;
};
const allManifests = [];
for (const file of paths.required) allManifests.push([manifestLabel(file), readJson(fs.existsSync(file) ? file : paths.required[0]), file]);
for (const file of optionalInputs) if (fs.existsSync(file)) allManifests.push([`optional ${manifestLabel(file)}`, readJson(file), file]);
const contentRows = rowsOf(content);
let availableRows = 0, verifiedRows = 0, abstractRows = 0, resolvedMigrationRows = 0;
const driftRows = [];
for (const [label, json, file] of allManifests) {
  const baseRoot = file.startsWith(paths.botaRoot) ? paths.botaRoot : file.startsWith(paths.hy3dRoot) ? paths.hy3dRoot : file.startsWith(paths.wscheppityRoot) ? paths.wscheppityRoot : root;
  for (const row of rowsOf(json)) {
    if (!isAvailable(row)) continue;
    availableRows += 1;
    const rawPath = rowPath(row);
    const expected = rowHash(row);
    if (!rawPath && !expected) { abstractRows += 1; continue; }
    let fileResolved = rawPath && expected ? resolveAny(rawPath, baseRoot, paths.hy3dRoot, paths.botaRoot, paths.wscheppityRoot, root) : null;
    let hash = expected;
    if (!fileResolved && rawPath && !expected) {
      const parts = String(rawPath).split("/");
      const category = parts[0] === "content" ? parts[1] : null;
      const slug = parts[0] === "content" ? parts.slice(2).join("/") : null;
      const linked = contentRows.find((item) => item.category === category && item.slug === slug);
      if (linked) {
        hash = rowHash(linked);
        fileResolved = resolveAny(rowPath(linked), paths.hy3dRoot, paths.botaRoot, root);
      }
      if (!fileResolved) { abstractRows += 1; continue; }
    }
    if (!fileResolved && expected) {
      for (const candidate of migratedCandidates(row)) {
        const hit = resolveAny(candidate, root, paths.hy3dRoot, paths.botaRoot);
        if (hit) { fileResolved = hit; resolvedMigrationRows += 1; break; }
      }
    }
    if (!fileResolved) { driftRows.push(`${label}: ${row.slug ?? row.id ?? row.demandSlug ?? "?"}: DRIFT missing path ${rawPath ?? "?"} hash ${hash ?? "?"}`); continue; }
    const actual = sha256File(fileResolved);
    if (actual !== hash) driftRows.push(`${label}: ${row.slug ?? row.id ?? row.demandSlug ?? path.basename(fileResolved)}: DRIFT SHA-256 expected=${hash} actual=${actual} file=${fileResolved}`);
    else verifiedRows += 1;
  }
}
if (driftRows.length) {
  console.error("DRIFT cross-project hash-bearing content rows:");
  for (const row of driftRows.slice(0, 30)) console.error(`  ${row}`);
  if (driftRows.length > 30) console.error(`  ... ${driftRows.length - 30} more`);
  process.exit(3);
}
const snapshotRec = inputRecord(paths.snapshot);
console.log(`snapshot: path=${manifestLabel(paths.snapshot)} sha256=${snapshotRec?.sha256 ?? "?"} generatedAt=${snapshot?.generatedAt ?? "?"}; DRIFT/STALE=0`);
console.log(`cross-contracts PASS: BOTA 26 fullname IDs match bota active set via acronym-to-slot map; default Dota 127 acronym-only IDs; PACK_PARAM=scenarios/moba-acronyms`);
console.log(`cross-contracts PASS: 26 bota-skinned.json hashes match local fullname GLBs`);
console.log(`cross-contracts PASS: content contracts schema=${contentContracts.schema} inversion=${contentContracts.identifierInversion.upstreamAcronymKeys} acronym keys -> ${contentContracts.identifierInversion.mappedFullnames} fullname IDs, default ${contentContracts.identifierInversion.defaultDotaAcronymIds} acronyms, doNotPromote=${contentContracts.integration.doNotPromote.length}`);
console.log(`cross-contracts PASS: final assets generated=${finalAssets.counts.generated} blocked=${finalAssets.counts.blocked} ready=${finalAssets.counts.ready}, ${finalAssets.generatedHashVerification.verified}/24 GLB hashes verified, audio blocker preserved`);
console.log(`cross-contracts PASS: ${verifiedRows} hash-bearing available/ready content rows resolve to real files with matching SHA-256; ${resolvedMigrationRows} resolved through fullname migration; ${abstractRows} abstract rows; ${availableRows} total available/ready rows`);
console.log(`optional manifests: present=${optionalPresent.length} missing=${optionalMissing.length}; extra present=${extraPresent.length} missing=${extraMissing.length}`);
for (const row of snapshot?.extraSummary ?? []) console.log(`extra evidence ${row.path}: classes=${row.classes ?? "?"} complete=${row.complete ?? "?"} blocked=${row.blocked ?? "?"} informational=true`);
console.log(`read-only sources: required ${paths.required.length}, optional ${optionalPresent.length}, oscraft ${paths.own.length} contract files`);
