#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { contentContractErrors, contentContractPaths } from "./content-contract-status.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const files = contentContractPaths(root);
const readJson = (file) => JSON.parse(fs.readFileSync(file, "utf8"));
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
const positive = contentContractErrors(inputs);
if (positive.errors.length) throw new Error(`positive live contracts failed: ${positive.errors.join(" | ")}`);
const badInversion = structuredClone(inputs);
badInversion.integration.identifier_rules.active_hero_keys = inputs.integration.identifier_rules.active_hero_keys.slice(1);
const inversionErrors = contentContractErrors(badInversion).errors;
if (!inversionErrors.some((row) => row.includes("upstream acronym keys=25") || row.includes("invert to oscraft fullname IDs"))) throw new Error(`negative inversion control passed: ${JSON.stringify(inversionErrors)}`);
const badGate = structuredClone(inputs);
badGate.progress.gated.do_not_promote = false;
const gateErrors = contentContractErrors(badGate).errors;
if (!gateErrors.some((row) => row.includes("do_not_promote"))) throw new Error(`negative gate control passed: ${JSON.stringify(gateErrors)}`);
const badDemand = structuredClone(inputs);
badDemand.demand.counts.blocked = 1;
const demandErrors = contentContractErrors(badDemand).errors;
if (!demandErrors.some((row) => row.includes("demand blocked=1"))) throw new Error(`negative demand control passed: ${JSON.stringify(demandErrors)}`);
console.log(`check:content-contract-status PASS: live ${positive.acronymKeys.length}->${positive.fullnameIds.length} inversion, 127 default acronyms, doNotPromote=${positive.doNotPromote.length}; negative inversion/gate/demand controls rejected`);
