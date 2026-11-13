import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { buildContentContractStatus } from "./content-contract-status.mjs";
import { buildFinalAssetStatus } from "./final-asset-status.mjs";

export const rowPath = (row) => row.destinationPath ?? row.optimizedPath ?? row.model ?? row.sourcePath ?? row.path ?? row.file ?? row.destination ?? null;
export const rowHash = (row) => row.destinationSha256 ?? row.destinationHash ?? row.sha256 ?? row.sourceHash ?? row.hash ?? null;
export const rowsOf = (json) => [json.rows, json.items, json.characters, json.destinationHashes, json.probes, json.requests, json.gaps].find(Array.isArray) ?? [];
export const isAvailable = (row) => ["available", "ready", "prepared-only", "preparedOnly"].includes(String(row.status ?? row.delivery ?? row.availability ?? "").toLowerCase());
export const sha256File = (file) => crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
export const sha256Text = (text) => crypto.createHash("sha256").update(text).digest("hex");

export function contractPaths(root) {
  const botaRoot = path.resolve(root, "..", "bota");
  const hy3dRoot = path.resolve(root, "..", "hy3d");
  const wscheppityRoot = path.resolve(root, "..", "wscheppity");
  const extraRoot = path.resolve(root, "..", "extra");
  return {
    root, botaRoot, hy3dRoot, wscheppityRoot, extraRoot,
    required: [
      path.join(botaRoot, "export", "rule-demand.json"),
      path.join(botaRoot, "export", "conformance.json"),
      path.join(botaRoot, "export", "export-status.json"),
      path.join(botaRoot, "export", "spawn-compatibility.json"),
      path.join(hy3dRoot, "output", "content-for-bota.json"),
      path.join(hy3dRoot, "output", "final-asset-inventory.json"),
      path.join(hy3dRoot, "output", "bota-ready.json"),
      path.join(hy3dRoot, "output", "asset-delivery-report.json"),
    ],
    optional: [
      path.join(botaRoot, "export", "content-request-queue.json"),
      path.join(botaRoot, "export", "spawn-extraction-gaps.json"),
      path.join(botaRoot, "export", "integration-contract.json"),
      path.join(hy3dRoot, "output", "content-demand-v2.json"),
      path.join(botaRoot, "export", "progress-dashboard.json"),
      path.join(botaRoot, "export", "coordination-status.json"),
      path.join(hy3dRoot, "output", "spawn-cdn-probes.json"),
      path.join(wscheppityRoot, ".local", "extraction-manifest.json"),
    ],
    extraOptional: [
      path.join(extraRoot, ".local", "extraction-manifest.json"),
      path.join(extraRoot, ".local", "cdn", "manifest-all.json"),
    ],
    own: [
      path.join(root, "scenarios", "bota", "IDENTIFIERS.md"),
      path.join(root, "scenarios", "bota", "bota_ui.json"),
      path.join(root, "scenarios", "bota", "bota-skinned.json"),
      path.join(root, "scenarios", "moba-acronyms", "identifier-map.tsv"),
      path.join(root, "js", "client", "pack_load.js"),
    ],
    snapshot: path.join(root, "scenarios", "bota", "cross-contract-snapshot.json"),
  };
}

export function manifestLabel(file) {
  const rel = file.replace(/^\/workspace\//, "");
  return rel.split(path.sep).join("/");
}

export function inputRecord(file) {
  if (!fs.existsSync(file)) return null;
  const bytes = fs.readFileSync(file);
  const stat = fs.statSync(file);
  let generatedAt = null;
  if (file.endsWith(".json")) {
    try { generatedAt = JSON.parse(bytes.toString("utf8")).generatedAt ?? null; } catch {}
  }
  return {
    path: manifestLabel(file),
    bytes: bytes.length,
    sha256: sha256Text(bytes.toString("utf8")),
    mtimeMs: Math.floor(stat.mtimeMs),
    generatedAt,
  };
}

export function buildSnapshot(paths) {
  const inputs = {};
  const required = {};
  const optional = {};
  const optionalPresent = [];
  const optionalMissing = [];
  for (const file of paths.required) {
    const rec = inputRecord(file);
    if (!rec) throw new Error(`missing required manifest ${file}`);
    required[rec.path] = rec;
    inputs[rec.path] = rec;
  }
  const extraPresent = [];
  const extraMissing = [];
  const extraSummary = [];
  const allOptional = [...paths.optional, ...(paths.extraOptional ?? [])];
  for (const file of allOptional) {
    const rec = inputRecord(file);
    if (rec) {
      optional[rec.path] = rec;
      inputs[rec.path] = rec;
      optionalPresent.push(rec.path);
      if (file.startsWith(paths.extraRoot)) {
        extraPresent.push(rec.path);
        let json = null;
        try { json = JSON.parse(fs.readFileSync(file, "utf8")); } catch {}
        const summary = json?.summary ?? json?.counts ?? {};
        extraSummary.push({ path: rec.path, classes: summary.classes ?? summary.total ?? null, complete: summary.complete ?? null, blocked: summary.blocked ?? null, informational: true });
      }
    } else {
      optionalMissing.push(manifestLabel(file));
      if (file.startsWith(paths.extraRoot)) extraMissing.push(manifestLabel(file));
    }
  }
  const identifiers = fs.readFileSync(paths.own[0], "utf8");
  const ui = JSON.parse(fs.readFileSync(paths.own[1], "utf8"));
  const skinned = JSON.parse(fs.readFileSync(paths.own[2], "utf8"));
  const mapText = fs.readFileSync(paths.own[3], "utf8");
  const loader = fs.readFileSync(paths.own[4], "utf8");
  const contentContracts = buildContentContractStatus(paths.root);
  const finalAssets = buildFinalAssetStatus(paths.root);
  return {
    schema: "oscraft.cross-contract-snapshot/v1",
    generatedAt: new Date().toISOString(),
    identifierMode: { defaultDota: "default-dota-acronyms", bota: "bota-fullname", packParam: /PACK_PARAM[^\n]*"scenarios\/moba-acronyms"/.test(loader) ? "scenarios/moba-acronyms" : null },
    contentContracts: { schema: contentContracts.schema, ok: contentContracts.ok, sourceSha256: contentContracts.sourceSha256 ?? null, errors: contentContracts.errors ?? [], integrationRules: contentContracts.integration?.integrationRules ?? null, doNotPromote: contentContracts.integration?.doNotPromote?.length ?? null, inversion: contentContracts.identifierInversion ?? null },
    finalAssets: { schema: finalAssets.schema, ok: finalAssets.ok, sourceSha256: finalAssets.sourceSha256 ?? null, errors: finalAssets.errors ?? [], counts: finalAssets.counts ?? null, generatedHashVerification: finalAssets.generatedHashVerification ?? null, blockers: finalAssets.blockers ?? null },
    counts: {
      required: Object.keys(required).length,
      optionalPresent: optionalPresent.length,
      optionalMissing: optionalMissing.length,
      extraPresent: extraPresent.length,
      extraMissing: extraMissing.length,
      botaFullnameIds: Object.keys(ui.heroes ?? {}).length,
      defaultAcronymIds: mapText.trim().split("\n").slice(1).length,
      skinnedModels: Object.keys(skinned.heroes ?? {}).length,
    },
    identifierDocSha256: sha256Text(identifiers),
    identifiers: { bota: Object.keys(ui.heroes ?? {}).sort(), defaultDota: mapText.trim().split("\n").slice(1).filter(Boolean).map((line) => line.split("\t")[2]).sort() },
    required,
    optional,
    optionalPresent: optionalPresent.sort(),
    optionalMissing: optionalMissing.sort(),
    extraPresent: extraPresent.sort(),
    extraMissing: extraMissing.sort(),
    extraSummary,
    inputs,
  };
}

export function snapshotComparable(snapshot) {
  const copy = JSON.parse(JSON.stringify(snapshot));
  delete copy.generatedAt;
  return copy;
}

export function comparableSha256(snapshot) {
  return sha256Text(JSON.stringify(snapshotComparable(snapshot)));
}

export function classifyDrift(paths, snapshot) {
  if (!snapshot) return [{ kind: "STALE", path: manifestLabel(paths.snapshot), detail: "cross-contract-snapshot.json is absent; run npm run refresh:cross-contracts" }];
  const current = buildSnapshot(paths);
  const drift = [];
  for (const [label, wanted] of Object.entries(snapshot.required ?? {})) {
    const got = current.required[label];
    if (!got) drift.push({ kind: "STALE", path: label, detail: "required manifest vanished" });
    else if (got.sha256 !== wanted.sha256 || got.mtimeMs !== wanted.mtimeMs || (got.generatedAt ?? null) !== (wanted.generatedAt ?? null)) drift.push({ kind: "STALE", path: label, detail: `snapshot expected sha256=${wanted.sha256} mtimeMs=${wanted.mtimeMs} generatedAt=${wanted.generatedAt ?? ""}; current sha256=${got.sha256} mtimeMs=${got.mtimeMs} generatedAt=${got.generatedAt ?? ""}` });
  }
  for (const label of current.optionalPresent) {
    if (!snapshot.optional?.[label]) drift.push({ kind: "STALE", path: label, detail: "optional manifest appeared after snapshot; run refresh:cross-contracts" });
    else if (current.optional[label].sha256 !== snapshot.optional[label].sha256 || current.optional[label].mtimeMs !== snapshot.optional[label].mtimeMs) drift.push({ kind: "STALE", path: label, detail: "optional manifest changed after snapshot" });
  }
  for (const label of snapshot.optionalPresent ?? []) if (!current.optional[label]) drift.push({ kind: "STALE", path: label, detail: "optional manifest present in snapshot is now absent" });
  return drift;
}
