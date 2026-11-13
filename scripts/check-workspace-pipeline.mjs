#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { buildWorkspacePipeline, pipelinePaths } from "./workspace-pipeline.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const once = process.argv.includes("--once") || !process.argv.includes("--watch");
const watch = process.argv.includes("--watch");
const timeoutMs = Number(process.env.WORKSPACE_PIPELINE_TIMEOUT_MS ?? (watch && !once ? 10 * 60 * 1000 : 0));
const intervalMs = Number(process.env.WORKSPACE_PIPELINE_INTERVAL_MS ?? 20000);
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const p = pipelinePaths();
const started = Date.now();
let cycles = 0;

function attempt() {
  cycles += 1;
  const state = buildWorkspacePipeline();
  console.log(`check:workspace-pipeline state=${state.state} ok=${state.ok} vendorPrep=${state.vendorPrep.state} vendor247420=${state.vendor247420.state}`);
  console.log(`wscheppity extraction: ok=${state.wscheppityExtraction.ok} classes=${state.wscheppityExtraction.summary?.classes ?? "?"} complete=${state.wscheppityExtraction.summary?.complete ?? "?"} blocked=${state.wscheppityExtraction.summary?.blocked ?? "?"}`);
  for (const row of state.vendorPrep.required) console.log(`prep ${row.present ? "PRESENT" : "MISSING"} ${row.path}${row.sha256 ? ` sha256=${row.sha256}` : ""}`);
  console.log(`READY.json ok=${state.vendorPrep.readyOk} handoffGate=${JSON.stringify(state.vendorPrep.handoffGate)}`);
  if (state.vendor247420.vendored.length) for (const row of state.vendor247420.vendored) console.log(`247420 vendored ${row.path}`);
  else console.log(`247420 vendored content: none`);
  console.log(`bitvoice contract schema=${state.bitvoice.contractSchema ?? "?"} stability=${state.bitvoice.interfaceStability ?? "?"} hash=${state.bitvoice.contractSha256 ?? "?"} lockMatches=${state.bitvoice.lockMatches}`);
  console.log(`gmweb readiness ok=${state.gmweb.readinessOk} path=${state.gmweb.path}`);
  console.log(`extra status path=${state.extra.path} present=${state.evidence.extraExtractionStatus.present}`);
  console.log(`247420 prep checker path=${state.prepChecker.path} present=${state.prepChecker.present}; docs=${state.docs247420.length}`);
  for (const row of state.pending) console.log(`PENDING ${row}`);
  for (const row of state.blocking) console.error(`BLOCKING ${row}`);
  return state;
}

while (true) {
  const state = attempt();
  if (!state.ok) process.exit(3);
  console.log(`check:workspace-pipeline PASS: state=${state.state} pending=${state.pending.length} blocking=0 cycles=${cycles}`);
  if (once) process.exit(0);
  if (Date.now() - started >= timeoutMs) {
    console.log(`check:workspace-pipeline bounded timeout reached after ${cycles} cycle(s)`);
    process.exit(0);
  }
  const pause = Math.min(intervalMs, Math.max(1, timeoutMs - (Date.now() - started)));
  console.log(`check:workspace-pipeline: waiting ${pause}ms`);
  await sleep(pause);
}
