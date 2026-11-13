#!/usr/bin/env node
import { spawn } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const fail = (message) => { console.error(`bota launch check: ${message}`); process.exit(1); };

function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const port = server.address().port;
      server.close(() => resolve(port));
    });
  });
}

async function waitFor(fn, timeoutMs = 240000) {
  const end = Date.now() + timeoutMs;
  let last;
  while (Date.now() < end) {
    try { return await fn(); } catch (error) { last = error; await sleep(150); }
  }
  throw last ?? new Error("timeout");
}

async function fetchOk(url, { binary = false } = {}) {
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) throw new Error(`${url} HTTP ${response.status}`);
  const body = Buffer.from(await response.arrayBuffer());
  return { status: response.status, contentType: response.headers.get("content-type") ?? "", body };
}

const port = await freePort();
const site = process.env.BOTA_LAUNCH_SITE ?? path.join(root, ".dev-site-bota-check");
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

try {
  const base = `http://127.0.0.1:${port}`;
  const playUrl = `${base}/js/client/play.html?package=scenarios/bota`;
  await waitFor(async () => {
    if (!output.includes("BOTA URL:")) throw new Error("launch wrapper has not printed a URL");
    for (const url of [playUrl, `${base}/js/client/play.html`, `${base}/js/client/play.html?package=scenarios/moba`, `${base}/scenarios/moba-acronyms/manifest.json`]) {
      const res = await fetch(url, { cache: "no-store" });
      if (!res.ok) throw new Error(`launch URL HTTP ${res.status}: ${url}`);
    }
  });
  console.log(`launch: ${playUrl}`);

  const checks = [
    ["play", playUrl, "text/html"],
    ["default play", `${base}/js/client/play.html`, "text/html"],
    ["historical play", `${base}/js/client/play.html?package=scenarios/moba`, "text/html"],
    ["manifest", `${base}/scenarios/bota/manifest.json`, "application/json"],
    ["default manifest", `${base}/scenarios/moba-acronyms/manifest.json`, "application/json"],
    ["historical manifest", `${base}/scenarios/moba/manifest.json`, "application/json"],
    ["default ui", `${base}/scenarios/moba-acronyms/moba_ui.json`, "application/json"],
    ["bota ui", `${base}/scenarios/bota/bota_ui.json`, "application/json"],
    ["backdrop", `${base}/scenarios/bota/assets/textures/menu-backdrop.jpg`, "image/jpeg"],
    ["hero glb", `${base}/scenarios/bota/assets/models/neckbeard-skinned.glb`, "model/gltf-binary"],
  ];
  const results = new Map();
  for (const [label, url, expectedType] of checks) {
    const result = await fetchOk(url, { binary: true });
    if (expectedType && !result.contentType.includes(expectedType)) fail(`${label} content-type ${result.contentType}, expected ${expectedType}`);
    results.set(label, result);
    console.log(`fetch: ${label} HTTP ${result.status} ${result.contentType} ${result.body.length} bytes`);
  }
  const backdrop = results.get("backdrop").body;
  if (backdrop.length !== 3654481 || sha256(backdrop) !== "431b1f861938c26edb7ce226e75d1c66a433c701d70a59463e064c402a077549") fail("backdrop bytes or SHA-256 mismatch");
  const hero = results.get("hero glb").body;
  if (hero.readUInt32LE(0) !== 0x46546c67 || hero.readUInt32LE(4) !== 2) fail("hero GLB magic/version mismatch");
  const manifest = JSON.parse(results.get("manifest").body.toString("utf8"));
  if (manifest.name !== "bota" || manifest.image !== "bota.osdt" || manifest.ui !== "bota_ui.json") fail("BOTA manifest identity mismatch");
  const ui = JSON.parse(results.get("bota ui").body.toString("utf8"));
  const botaIds = Object.keys(ui.heroes ?? {});
  if (ui.menuBackdrop !== "textures.menu_backdrop" || botaIds.length !== 24 || botaIds.some((id) => /^[a-z]{1,3}$/.test(id) && id !== "npc")) fail("BOTA UI backdrop or 24 fullname hero roster mismatch");
  const defaultManifest = JSON.parse(results.get("default manifest").body.toString("utf8"));
  const defaultUi = JSON.parse(results.get("default ui").body.toString("utf8"));
  const defaultIds = Object.keys(defaultUi.heroAcronyms ?? {});
  if (defaultManifest.name !== "moba-acronyms" || defaultUi.identifierMode !== "default-dota-acronyms" || defaultIds.length !== 127 || defaultIds.some((id) => !/^[a-z]{1,3}$/.test(id))) fail("default URL does not resolve to the acronym-only derived pack");
  const historicalManifest = JSON.parse(results.get("historical manifest").body.toString("utf8"));
  if (historicalManifest.name !== "moba" || historicalManifest.ui !== "moba_ui.json") fail("historical scenarios/moba source pack is not available");
  const loader = fs.readFileSync(path.join(root, "js", "client", "pack_load.js"), "utf8");
  if (!/PACK_PARAM[^\n]*"scenarios\/moba-acronyms"/.test(loader)) fail("PACK_PARAM default is not scenarios/moba-acronyms");
  for (const label of ["default play", "historical play"]) if (!results.get(label).body.toString("utf8").includes("play.js")) fail(`${label} page did not resolve`);
  console.log(`packs: default=${defaultManifest.name} acronymIds=${defaultIds.length}; bota=${manifest.name} fullnameIds=${botaIds.length}; historical=${historicalManifest.name}`);
  console.log(`bota launch check ok: port=${port} site=${site} pid=${child.pid} backdropSha256=${sha256(backdrop)} heroMagic=glTF/v2`);
} catch (error) {
  console.error(output.trim());
  fail(error.message);
} finally {
  cleanup();
}
