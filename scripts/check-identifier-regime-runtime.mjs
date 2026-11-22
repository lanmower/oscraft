#!/usr/bin/env node
import { sleep } from "./lib/sleep.mjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { freePort, removeOnExit, startGroup } from "./lib/host.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const fail = (message) => { console.error(`identifier-runtime: ${message}`); process.exit(1); };
const READY_TIMEOUT_MS = 120000;
const port = await freePort();
const site = process.env.IDENTIFIER_RUNTIME_SITE ?? path.join(root, `.dev-site-identifier-runtime-${process.pid}`);
const command = path.join(root, "scripts", "dev", "serve-bota.sh");
const env = { ...process.env, BOTA_PORT: String(port), BOTA_SITE: site, WASM: process.env.WASM ?? "0" };
const group = startGroup({ command, args: [`--port=${port}`], env, cwd: root });
if (!process.env.IDENTIFIER_RUNTIME_SITE) removeOnExit(site);
async function fetchJson(url) {
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) throw new Error(`${url} HTTP ${response.status}`);
  return JSON.parse(await response.text());
}
async function waitFor(fn) {
  const end = Date.now() + READY_TIMEOUT_MS;
  for (;;) {
    try { return await fn(); } catch (error) {
      if (group.child.exitCode !== null) throw new Error(`launch wrapper exited with ${group.child.exitCode} before the pack was served: ${error.message}`);
      if (Date.now() >= end) throw new Error(`pack not served within ${READY_TIMEOUT_MS} ms: ${error.message}`);
      await sleep(150);
    }
  }
}
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
  };
  await waitFor(async () => {
    if (!group.output.includes("BOTA URL:")) throw new Error("launch wrapper has not printed a URL yet");
    const response = await fetch(urls.defaultManifest, { cache: "no-store" });
    if (!response.ok) throw new Error(`default manifest HTTP ${response.status}`);
  });
  for (const label of ["defaultHtml", "botaHtml", "historicalHtml"]) {
    const response = await fetch(urls[label], { cache: "no-store" });
    if (!response.ok || !(await response.text()).includes("play.js")) fail(`${label} did not serve the play page`);
  }
  const defaultManifest = await fetchJson(urls.defaultManifest);
  const defaultUi = await fetchJson(urls.defaultUi);
  const defaultIds = Object.keys(defaultUi.heroAcronyms ?? {});
  if (defaultManifest.name !== "moba-acronyms" || defaultUi.identifierMode !== "default-dota-acronyms") fail("default HTTP pack is not the acronym derived pack");
  if (defaultIds.length !== 127 || defaultIds.some((id) => !/^[a-z]{1,3}$/.test(id))) fail(`default HTTP active IDs are not 127 acronyms (${defaultIds.length})`);
  const botaManifest = await fetchJson(urls.botaManifest);
  const botaUi = await fetchJson(urls.botaUi);
  const botaIds = Object.keys(botaUi.heroes ?? {});
  if (botaManifest.name !== "bota" || botaUi.identifierMode !== "bota-fullname") fail("BOTA HTTP pack is not the fullname pack");
  if (botaIds.length !== 24 || botaIds.some((id) => /^[a-z]{1,3}$/.test(id) && id !== "npc")) fail(`BOTA HTTP active IDs are not 24 fullnames (${botaIds.length})`);
  const historicalManifest = await fetchJson(urls.historicalManifest);
  if (historicalManifest.name !== "moba" || historicalManifest.ui !== "moba_ui.json") fail("historical source pack is not available over HTTP");
  const loader = fs.readFileSync(path.join(root, "js", "client", "pack_load.js"), "utf8");
  if (!/PACK_PARAM[^\n]*"scenarios\/moba-acronyms"/.test(loader)) fail("PACK_PARAM default is not scenarios/moba-acronyms");
  console.log(`identifier-runtime PASS: default=${defaultManifest.name}/${defaultIds.length} acronyms; bota=${botaManifest.name}/${botaIds.length} fullnames; historical=${historicalManifest.name}`);
  console.log(`identifier-runtime PASS: HTTP 200 for default, BOTA and historical play pages; PACK_PARAM=scenarios/moba-acronyms`);
} catch (error) {
  console.error(group.output.trim());
  fail(error.message);
} finally {
  group.cleanup();
}
