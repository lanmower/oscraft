import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

export const CONTENT_CONTRACT_SCHEMA = "oscraft.content-contract-status/v1";
export const CONTENT_CONTRACT_RULES = Object.freeze({
  upstreamAcronymKeys: 26,
  oscraftFullnameIds: 26,
  defaultDotaAcronymIds: 127,
  skinnedModels: 26,
});

const sha256Text = (text) => crypto.createHash("sha256").update(text).digest("hex");
const readJson = (file) => JSON.parse(fs.readFileSync(file, "utf8"));
const unique = (values) => [...new Set(values)].sort();
const isAcronym = (id) => /^[a-z]{1,3}$/.test(id);

export function contentContractPaths(root) {
  const botaRoot = path.resolve(root, "..", "bota");
  const hy3dRoot = path.resolve(root, "..", "hy3d");
  return {
    integration: path.join(botaRoot, "export", "integration-contract.json"),
    progress: path.join(botaRoot, "export", "progress-dashboard.json"),
    coordination: path.join(botaRoot, "export", "coordination-status.json"),
    conformance: path.join(botaRoot, "export", "conformance.json"),
    demand: path.join(hy3dRoot, "output", "content-demand-v2.json"),
    ui: path.join(root, "scenarios", "bota", "bota_ui.json"),
    identifiers: path.join(root, "scenarios", "bota", "IDENTIFIERS.md"),
    skinned: path.join(root, "scenarios", "bota", "bota-skinned.json"),
    map: path.join(root, "scenarios", "moba-acronyms", "identifier-map.tsv"),
  };
}

export function contentContractErrors(inputs) {
  const { integration, progress, coordination, demand, conformance, ui, skinned, identifiers, mapText } = inputs;
  const rules = integration.identifier_rules ?? {};
  const inversion = rules.downstream_inversion ?? {};
  const acronymKeys = unique(rules.active_hero_keys ?? []);
  const fullnameIds = unique(Object.keys(ui.heroes ?? {}));
  const defaultIds = unique(mapText.trim().split("\n").slice(1).filter(Boolean).map((line) => line.split("\t")[2]));
  const modelIds = unique(Object.keys(skinned.heroes ?? {}));
  const slotToFullname = new Map([...identifiers.matchAll(/^\|\s*(\d{3})\s*\|\s*`([^`]+)`/gm)].map((m) => [m[1], m[2]]));
  const acronymToSlot = conformance.acronym_to_slot ?? {};
  const mappedFullnames = unique(acronymKeys.map((acro) => slotToFullname.get(acronymToSlot[acro])).filter(Boolean));
  const doNotPromote = (integration.integration_rules ?? []).filter((row) => row.do_not_promote === true).map((row) => row.id).sort();
  const errors = [];
  const check = (condition, message) => { if (!condition) errors.push(message); };
  check(integration.schema === "bota.engine-integration-contract/v1", `integration schema=${integration.schema}`);
  check(demand.schema === "hy3d.content-demand-v2/v1", `demand schema=${demand.schema}`);
  check(progress.schema === "bota.progress-dashboard/v1", `progress schema=${progress.schema}`);
  check(coordination.schema === "bota.coordination-status/v1", `coordination schema=${coordination.schema}`);
  check(acronymKeys.length === CONTENT_CONTRACT_RULES.upstreamAcronymKeys, `upstream acronym keys=${acronymKeys.length}`);
  check(acronymKeys.every(isAcronym), "upstream active_hero_keys contain a non-acronym ID");
  check(fullnameIds.length === CONTENT_CONTRACT_RULES.oscraftFullnameIds, `oscraft BOTA fullname IDs=${fullnameIds.length}`);
  check(fullnameIds.some((id) => id.length > 3), "oscraft BOTA IDs are not fullname IDs");
  check(defaultIds.length === CONTENT_CONTRACT_RULES.defaultDotaAcronymIds && defaultIds.every(isAcronym), `default Dota acronym IDs=${defaultIds.length}`);
  check(modelIds.length === CONTENT_CONTRACT_RULES.skinnedModels, `skinned models=${modelIds.length}`);
  check(mappedFullnames.length === fullnameIds.length && mappedFullnames.every((id) => fullnameIds.includes(id)), "upstream acronym order does not invert to oscraft fullname IDs");
  check(inversion.canonicalExport?.includes("acronym") && inversion.oscraftBotaArtifacts?.includes("full"), "downstream_inversion does not name both regimes");
  check(inversion.defaultDotaHeroes?.includes("acronym"), "downstream_inversion does not name default Dota acronym IDs");
  check(inversion.validatorModes?.includes("acronym") && inversion.validatorModes?.includes("bota-fullname"), "downstream_inversion validator modes are incomplete");
  check(Number(integration.counts?.active_heroes ?? -1) === 26, `integration active_heroes=${integration.counts?.active_heroes}`);
  check(Number(demand.counts?.byKind?.["bota-hero"] ?? -1) === 26, `demand bota-hero rows=${demand.counts?.byKind?.["bota-hero"]}`);
  check(Number(demand.counts?.blocked ?? -1) === 0, `demand blocked=${demand.counts?.blocked}`);
  check(Number(demand.counts?.generated ?? -1) === 0, `demand generated=${demand.counts?.generated}`);
  check(doNotPromote.length > 0 && progress.gated?.do_not_promote === true, "do_not_promote rules are not represented in the progress dashboard");
  check(coordination.readOnly === true && progress.readOnly === true, "peer contracts do not report readOnly=true");
  check(Number(coordination.counts?.UNAVAILABLE ?? -1) === 0, `coordination UNAVAILABLE=${coordination.counts?.UNAVAILABLE}`);
  check((coordination.counts?.unresolved ?? 0) >= (coordination.counts?.["owner-decision"] ?? 0) + (coordination.counts?.["intentionally-open"] ?? 0), "coordination unresolved count excludes gated rows");
  return { errors, acronymKeys, fullnameIds, defaultIds, modelIds, mappedFullnames, doNotPromote, inversion };
}

export function buildContentContractStatus(root) {
  const files = contentContractPaths(root);
  const missing = Object.entries(files).filter(([, file]) => !fs.existsSync(file)).map(([key, file]) => ({ key, file }));
  if (missing.length) return { schema: CONTENT_CONTRACT_SCHEMA, ok: false, missing, errors: missing.map((row) => `${row.key}: missing ${row.file}`) };
  const inputs = {
    integration: readJson(files.integration),
    progress: readJson(files.progress),
    coordination: readJson(files.coordination),
    demand: readJson(files.demand),
    conformance: readJson(files.conformance),
    ui: readJson(files.ui),
    skinned: readJson(files.skinned),
    identifiers: fs.readFileSync(files.identifiers, "utf8"),
    mapText: fs.readFileSync(files.map, "utf8"),
  };
  const { integration, progress, coordination, demand } = inputs;
  const { errors, acronymKeys, fullnameIds, defaultIds, modelIds, mappedFullnames, doNotPromote, inversion } = contentContractErrors(inputs);
  const sourceText = [files.integration, files.progress, files.coordination, files.conformance, files.demand].map((file) => fs.readFileSync(file, "utf8")).join("\n");
  return {
    schema: CONTENT_CONTRACT_SCHEMA,
    ok: errors.length === 0,
    errors,
    sourceSha256: sha256Text(sourceText),
    integration: {
      schema: integration.schema,
      status: integration.status,
      activeHeroKeys: acronymKeys,
      integrationRules: (integration.integration_rules ?? []).length,
      doNotPromote,
      confidencePolicy: integration.confidence_policy ?? {},
    },
    demand: {
      schema: demand.schema,
      rows: demand.counts?.rows ?? null,
      available: demand.counts?.available ?? null,
      blocked: demand.counts?.blocked ?? null,
      generated: demand.counts?.generated ?? null,
      ownerDecision: demand.counts?.ownerDecision ?? null,
      intentionallyOpen: demand.counts?.intentionallyOpen ?? null,
    },
    progress: { schema: progress.schema, status: progress.status, doNotPromote: progress.gated?.do_not_promote === true, unavailable: (progress.unavailable ?? []).length },
    coordination: { schema: coordination.schema, rows: coordination.counts?.rows ?? null, resolved: coordination.counts?.resolved ?? null, unresolved: coordination.counts?.unresolved ?? null, unavailable: coordination.counts?.UNAVAILABLE ?? null, readOnly: coordination.readOnly === true },
    identifierInversion: {
      canonicalExport: "bota active_hero_keys",
      oscraftBotaArtifacts: "full BOTA character names/slugs",
      defaultDotaHeroes: "three-letter acronym legacy IDs",
      upstreamAcronymKeys: acronymKeys.length,
      oscraftFullnameIds: fullnameIds.length,
      defaultDotaAcronymIds: defaultIds.length,
      skinnedModels: modelIds.length,
      mappedFullnames: mappedFullnames.length,
      validatorModes: inversion.validatorModes ?? [],
    },
  };
}

export function assertContentContracts(root) {
  const status = buildContentContractStatus(root);
  if (!status.ok) {
    const error = new Error(`content-contracts: ${status.errors.join(" | ")}`);
    error.status = status;
    throw error;
  }
  return status;
}
