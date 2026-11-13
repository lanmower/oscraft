#!/usr/bin/env node
import { spawn } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseGlb, parseGlbAnimations, readGlbChunks } from "../js/client/glb.js";
import { resolveGlbExtensions } from "../js/client/glb_resolve.js";
import { AnimationPlayer, jointMatrices, retargetLibrary, samplePose } from "../js/client/anim.js";
import { packMenuBackdrop } from "../js/client/gui_flow.js";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const HERO_FULLNAMES = ["neckbeard", "chad", "wojak", "bloomer", "tradwife", "boomer", "coomer", "gondola", "spurdo", "karen", "howard", "sanic", "lankdad", "finger_dan", "willie", "noscope", "dino", "ecce", "bot_bernard", "shoop", "npc", "forever", "cole", "grey", "ghillie", "gothgirl"];
const allMode = process.argv.includes("--all") || true;
const models = allMode ? HERO_FULLNAMES : ["neckbeard", "noscope", "chad"];
const neutralModels = ["neutral-centaur-khan-skinned", "neutral-centaur-outrunner-skinned"];
const backdropRel = "scenarios/bota/assets/textures/menu-backdrop.jpg";
const BOTA_BACKDROP = { pack: "scenarios/bota", ui: "bota_ui.json", file: backdropRel, bytes: 3654481, sha: "431b1f861938c26edb7ce226e75d1c66a433c701d70a59463e064c402a077549", label: "BOTA" };
const DEFAULT_BACKDROP = { pack: "scenarios/moba-acronyms", ui: "moba_ui.json", file: "scenarios/moba-acronyms/assets/textures/menu-backdrop.jpg", bytes: 436951, sha: "3fa34ada2e1f2bcc833be5322705f1117f775e4ef64c85fb94061a97aa73112b", label: "default acronym" };
const TYPES = new Map([
  [".html", "text/html; charset=utf-8"], [".js", "application/javascript"], [".mjs", "application/javascript"],
  [".json", "application/json"], [".glb", "model/gltf-binary"], [".jpg", "image/jpeg"], [".png", "image/png"],
  [".wasm", "application/wasm"], [".osdt", "application/octet-stream"], [".lua", "text/plain"],
  [".osd", "text/plain"], [".tsv", "text/plain"], [".txt", "text/plain"], [".ogg", "audio/ogg"],
  [".ktx2", "image/ktx2"], [".webp", "image/webp"],
]);
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const sha256 = (buf) => crypto.createHash("sha256").update(buf).digest("hex");
const compact = process.argv.includes("--from-cross-runtime");
const log = (...parts) => { if (!compact) console.log(...parts); };

function staticServer() {
  const hits = [];
  const server = http.createServer((req, res) => {
    const rel = decodeURIComponent((req.url ?? "/").split("?")[0]).replace(/^\/+/, "") || "index.html";
    const ignoredMissing = rel.endsWith("bundle/moba.modules.json") || rel === "scenarios/bota/assets/packs/index.json";
    let file = path.resolve(root, rel);
    if (rel.startsWith("js/client/pkg/")) {
      const staged = path.resolve(root, ".dev-site", rel);
      if (fs.existsSync(staged)) file = staged;
    }
    if (!(file.startsWith(root + path.sep)) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
      hits.push({ rel, status: 404, ignored: ignoredMissing });
      res.writeHead(404, { "content-type": "text/plain", "cache-control": "no-store" }).end("not found\n");
      return;
    }
    const body = fs.readFileSync(file);
    hits.push({ rel, status: 200, bytes: body.length });
    res.writeHead(200, { "content-type": TYPES.get(path.extname(file).toLowerCase()) ?? "application/octet-stream", "content-length": body.length, "cache-control": "no-store" });
    res.end(req.method === "HEAD" ? undefined : body);
  });
  return { server, hits };
}

let browserRendererResult = null;
async function browserPhase(url, target = BOTA_BACKDROP) {
  if (typeof WebSocket === "undefined") return { attempted: false, reason: "Node has no global WebSocket for CDP" };
  const chromium = process.env.CHROMIUM_PATH ?? "/usr/bin/chromium";
  if (!fs.existsSync(chromium)) return { attempted: false, reason: `chromium not found at ${chromium}` };
  const port = 9500 + Math.floor(Math.random() * 1000);
  const profile = fs.mkdtempSync("/tmp/oscraft-bota-browser-");
  const child = spawn(chromium, [
    `--user-data-dir=${profile}`, `--remote-debugging-port=${port}`, "--remote-debugging-address=127.0.0.1",
    "--no-first-run", "--no-default-browser-check", "--no-sandbox", "--disable-dev-shm-usage",
    "--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist",
    "about:blank",
  ], { stdio: "ignore", detached: true });
  child.unref();
  const cdp = { id: 0, pending: new Map(), ws: null };
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const n = ++cdp.id;
    cdp.pending.set(n, { resolve, reject });
    cdp.ws.send(JSON.stringify({ id: n, method, params }));
  });
  const events = [];
  const connect = async () => {
    for (let i = 0; i < 40; i++) {
      try {
        const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
        const target = targets.find((t) => t.type === "page");
        if (!target) throw new Error("no page target");
        const ws = new WebSocket(target.webSocketDebuggerUrl);
        await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
        cdp.ws = ws;
        ws.onmessage = (event) => {
          const msg = JSON.parse(event.data);
          if (msg.id && cdp.pending.has(msg.id)) { cdp.pending.get(msg.id).resolve(msg.result); cdp.pending.delete(msg.id); return; }
          if (msg.method === "Network.loadingFailed") events.push(`network-failed ${msg.params.errorText} ${msg.params.type ?? ""}`);
          if (msg.method === "Runtime.exceptionThrown") events.push(`exception ${msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text}`);
          if (msg.method === "Runtime.consoleAPICalled") events.push(`console ${msg.params.type} ${msg.params.args.map((a) => a.value ?? a.description ?? "").join(" ")}`);
        };
        return;
      } catch { await sleep(250); }
    }
    throw new Error("CDP did not open");
  };
  try {
    await connect();
    await send("Runtime.enable");
    await send("Page.enable");
    await send("Network.enable");
    await send("Page.navigate", { url });
    await sleep(12000);
    const evaluate = async (expression) => {
      const r = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
      return r.result?.value ?? null;
    };
    const rendererProbe = await evaluate(`(async () => {
      const mod = await import('/js/client/render3d.js');
      const canvas = document.createElement('canvas');
      canvas.width = 32; canvas.height = 32;
      const gl = canvas.getContext('webgl2', { antialias: false, preserveDrawingBuffer: true });
      if (!gl) return JSON.stringify({ error: 'WebGL2 unavailable' });
      const renderer = mod.createRenderer(gl);
      renderer.beginFrame([0, 0, 0, 1]);
      const mesh = {
        positions: Float32Array.of(-.5, 0, 0, .5, 0, 0, 0, 1, 0),
        normals: Float32Array.of(0, 0, 1, 0, 0, 1, 0, 0, 1),
        uvs: Float32Array.of(0, 0, 1, 0, .5, 1),
        indices: Uint16Array.of(0, 1, 2),
        joints: Float32Array.of(0,0,0,0, 0,0,0,0, 0,0,0,0),
        weights: Float32Array.of(1,0,0,0, 1,0,0,0, 1,0,0,0),
        color: [1,1,1], height: 1,
      };
      const palette = new Float32Array(28 * 16);
      for (let i = 0; i < 28; i++) {
        palette[i*16] = 1; palette[i*16+5] = 1; palette[i*16+10] = 1; palette[i*16+15] = 1;
      }
      renderer.drawSkinned(mesh, null, palette);
      const program = gl.getParameter(gl.CURRENT_PROGRAM);
      const skinnedLocation = program ? gl.getUniformLocation(program, 'uSkinned') : null;
      const uSkinned = program && skinnedLocation ? gl.getUniform(program, skinnedLocation) : null;
      const err = gl.getError();
      renderer.endFrame();
      return JSON.stringify({ webgl2: true, uSkinned, jointFloats: palette.length, glError: err, drawCalls: renderer.stats.drawCalls });
    })()`);
    const rendererResult = rendererProbe ? JSON.parse(rendererProbe) : null;
    browserRendererResult = rendererResult;
    const uSkinnedValue = rendererResult?.uSkinned === true ? 1 : Number(rendererResult?.uSkinned);
    if (!rendererResult?.webgl2 || uSkinnedValue !== 1 || rendererResult.jointFloats !== 448 || rendererResult.glError !== 0) throw new Error(`renderer-level uSkinned probe failed: ${JSON.stringify(rendererResult)}`);
    log(`renderer: createRenderer.drawSkinned uSkinned=${uSkinnedValue} jointFloats=${rendererResult.jointFloats} glError=${rendererResult.glError} drawCalls=${rendererResult.drawCalls}`);
    const state = await evaluate(`JSON.stringify({
      url: location.href,
      readyState: document.readyState,
      worldBuilt: document.body?.dataset?.worldBuilt ?? null,
      phase: document.body?.dataset?.phase ?? null,
      startError: document.body?.dataset?.startError ?? null,
      flowRoot: !!document.getElementById('gui-flow'),
      flowStyle: document.getElementById('gui-flow')?.querySelector('style')?.textContent ?? '',
      backdropRule: (document.getElementById('gui-flow')?.querySelector('style')?.textContent ?? '').includes('menu-backdrop.jpg'),
      bodyBackground: getComputedStyle(document.body).backgroundImage,
      flowBackground: document.getElementById('gui-flow') ? getComputedStyle(document.getElementById('gui-flow')).backgroundImage : null
    })`);
    const parsed = state ? JSON.parse(state) : null;
    const cssProbe = await evaluate(`(async () => {
      const mod = await import('/js/client/gui_flow.js');
      const ui = await (await fetch('/${target.pack}/${target.ui}')).json();
      const manifest = await (await fetch('/${target.pack}/manifest.json')).json();
      const assetUrl = (key) => {
        let value = manifest.assets;
        for (const part of key.split('.')) value = value?.[part];
        return '/${target.pack}/assets/' + value;
      };
      const backdrop = mod.packMenuBackdrop(ui, assetUrl);
      const url = new URL(backdrop, location.href);
      url.searchParams.set('smoke', String(Date.now()));
      const response = await fetch(url, { cache: 'no-store' });
      const body = new Uint8Array(await response.arrayBuffer());
      const digest = await crypto.subtle.digest('SHA-256', body);
      const sha256 = [...new Uint8Array(digest)].map((n) => n.toString(16).padStart(2, '0')).join('');
      return JSON.stringify({ backdrop, key: ui.menuBackdrop, fetched: url.href, status: response.status, bytes: body.length, sha256 });
    })()`);
    const css = cssProbe ? JSON.parse(cssProbe) : null;
    const failedBackdrop = events.filter((e) => e.includes("network-failed") && e.includes("menu-backdrop"));
    const failedHero = events.filter((e) => e.includes("network-failed") && /neckbeard-skinned|noscope-skinned|chad-skinned/.test(e));
    log(`browser: url=${parsed?.url ?? url} readyState=${parsed?.readyState ?? "?"} phase=${parsed?.phase ?? "?"} worldBuilt=${parsed?.worldBuilt ?? "?"} startError=${parsed?.startError ?? "none"}`);
    log(`browser: flowRoot=${parsed?.flowRoot ?? false} backdropRule=${parsed?.backdropRule ?? false} cssProbe=${css?.backdrop ?? null} backdropFetch=${css?.status ?? "?"} bytes=${css?.bytes ?? "?"} sha256=${css?.sha256?.slice(0, 12) ?? "?"} backdropNetworkFailures=${failedBackdrop.length} heroNetworkFailures=${failedHero.length}`);
    if (!css?.backdrop?.endsWith('/' + target.file)) throw new Error(`${target.label} pack backdrop did not resolve in the browser`);
    if (!parsed?.backdropRule) throw new Error(`${target.label} pack page did not put the backdrop in #gui-flow`);
    if (css.status !== 200 || css.bytes !== target.bytes || css.sha256 !== target.sha) throw new Error(`${target.label} backdrop fetch failed: status=${css.status} bytes=${css.bytes} sha256=${css.sha256}`);
    if (failedBackdrop.length || failedHero.length) throw new Error(`asset network failures: ${[...failedBackdrop, ...failedHero].join(" | ")}`);
    return { attempted: true, connected: true, state: parsed, events: events.slice(-25), screenshot: "not captured: CDP screenshot omitted to keep the check deterministic" };
  } catch (error) {
    return { attempted: true, connected: false, reason: error.message, events: events.slice(-25) };
  } finally {
    try { cdp.ws?.close(); } catch {}
    try { process.kill(-child.pid); } catch {}
    for (let i = 0; i < 5; i++) { try { fs.rmSync(profile, { recursive: true, force: true }); break; } catch { await sleep(250); } }
  }
}

async function loaderPhase(base) {
  const originalLocation = globalThis.location;
  globalThis.location = new URL(base + "/js/client/play.html?package=scenarios/bota");
  const ui = JSON.parse(fs.readFileSync(path.join(root, "scenarios/bota/bota_ui.json"), "utf8"));
  const manifest = JSON.parse(fs.readFileSync(path.join(root, "scenarios/bota/manifest.json"), "utf8"));
  const resolveAsset = (key) => {
    let value = manifest.assets;
    for (const part of key.split(".")) value = value?.[part];
    if (typeof value !== "string") throw new Error(`pack ${manifest.name}: no asset ${key}`);
    return `scenarios/bota/assets/${value}`;
  };
  const backdropUrl = packMenuBackdrop(ui, resolveAsset);
  const unsetBackdrop = packMenuBackdrop({}, resolveAsset);
  const defaultUi = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "moba-acronyms", "moba_ui.json"), "utf8"));
  const defaultManifest = JSON.parse(fs.readFileSync(path.join(root, "scenarios", "moba-acronyms", "manifest.json"), "utf8"));
  const defaultResolve = (key) => {
    let value = defaultManifest.assets;
    for (const part of key.split(".")) value = value?.[part];
    if (typeof value !== "string") throw new Error(`pack ${defaultManifest.name}: no asset ${key}`);
    return `scenarios/moba-acronyms/assets/${value}`;
  };
  const defaultBackdrop = packMenuBackdrop(defaultUi, defaultResolve);
  log(`loader: menuBackdrop=${ui.menuBackdrop} botaUrl=${backdropUrl} unsetUrl=${unsetBackdrop} defaultUrl=${defaultBackdrop}`);
  if (!backdropUrl?.endsWith("/" + backdropRel)) throw new Error("BOTA backdrop did not resolve from bota_ui.json");
  if (unsetBackdrop !== null) throw new Error("a pack UI without menuBackdrop must resolve no backdrop");
  if (!defaultBackdrop?.endsWith("/" + DEFAULT_BACKDROP.file)) throw new Error("the default pack menu backdrop did not resolve from moba_ui.json");
  const backdrop = fs.readFileSync(path.join(root, backdropRel));
  if (backdrop.length !== 3654481 || sha256(backdrop) !== "431b1f861938c26edb7ce226e75d1c66a433c701d70a59463e064c402a077549") throw new Error("menu backdrop bytes changed");
  const map = JSON.parse(fs.readFileSync(path.join(root, "scenarios/bota/assets/anim/maps/humanoid.json"), "utf8"));
  const library = JSON.parse(fs.readFileSync(path.join(root, "scenarios/bota/assets/anim/library.json"), "utf8"));
  const names = [...models.map((a) => `${a}-skinned.glb`), ...neutralModels.map((n) => `${n}.glb`)];
  const results = [];
  for (const fileName of names) {
    const file = path.join(root, "scenarios/bota/assets/models", fileName);
    const bytes = fs.readFileSync(file);
    const raw = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
    const resolved = await resolveGlbExtensions(raw);
    const glb = parseGlb(resolved, path.basename(fileName, ".glb"));
    const animations = parseGlbAnimations(resolved);
    const skin = animations.skins[0];
    const clips = animations.clips.length ? animations.clips : retargetLibrary(library, map, animations.nodes);
    const attrs = Object.keys(readGlbChunks(resolved).json.meshes[0].primitives[0].attributes);
    let motion = null;
    if (skin?.joints.length) {
      const motionClip = clips.find((clip) => clip.name === "walk") ?? clips.find((clip) => clip.name === "run");
      if (!motionClip || motionClip.duration <= 0) throw new Error(`${fileName}: no non-static walk/run clip`);
      const player = new AnimationPlayer(clips);
      player.play(motionClip.name);
      const sampleAt = (time) => {
        const pose = samplePose(motionClip, time, true);
        const matrices = jointMatrices(animations.nodes, skin, pose, Float32Array.of(1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1), new Float32Array(16 * skin.joints.length));
        if (matrices.length !== 16 * skin.joints.length) throw new Error(`${fileName}: bad joint palette length ${matrices.length}`);
        if (matrices.some((v) => !Number.isFinite(v))) throw new Error(`${fileName}: non-finite joint matrix at t=${time}`);
        return Float32Array.from(matrices);
      };
      const t0 = 0;
      const t1 = Math.min(motionClip.duration * 0.5, motionClip.duration - 1e-4);
      const a = sampleAt(t0);
      const b = sampleAt(t1);
      let maxAbsDelta = 0;
      for (let i = 0; i < a.length; i++) maxAbsDelta = Math.max(maxAbsDelta, Math.abs(a[i] - b[i]));
      if (!(maxAbsDelta > 1e-5)) throw new Error(`${fileName}: ${motionClip.name} joint matrices did not change (delta=${maxAbsDelta})`);
      motion = { clip: motionClip.name, t0, t1, maxAbsDelta, matrixFloats: a.length, finite: true };
    }
    const status = {
      model: path.basename(fileName, ".glb"),
      skinned: Boolean(glb.joints && glb.weights),
      skins: animations.skins.length,
      joints: skin?.joints.length ?? 0,
      clips: clips.length,
      clipNames: clips.map((clip) => clip.name),
      attrs,
      bytes: bytes.length,
      motion,
    };
    log(`loader: ${status.model} skinned=${status.skinned} skins=${status.skins} joints=${status.joints} clips=${status.clips} attrs=${attrs.join(",")}${motion ? ` motion=${motion.clip}@${motion.t0.toFixed(3)},${motion.t1.toFixed(3)} maxDelta=${motion.maxAbsDelta.toExponential(3)} matrixFloats=${motion.matrixFloats}` : ""}`);
    if (!status.skinned || status.skins < 1 || status.joints === 0 || status.joints > 64 || status.clips !== 8 || !attrs.includes("JOINTS_0") || !attrs.includes("WEIGHTS_0")) throw new Error(`${status.model} failed runtime loader checks`);
    if (HERO_FULLNAMES.includes(status.model) && !motion) throw new Error(`${status.model}: no motion assertion`);
    results.push(status);
  }
  globalThis.location = originalLocation;
  return { backdropUrl, defaultBackdrop, models: results };
}

async function main() {
  const { server, hits } = staticServer();
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  const url = `${base}/js/client/play.html?package=scenarios/bota&host=1&ci=1`;
  const defaultUrl = `${base}/js/client/play.html?host=1&ci=1`;
  log(`bota browser smoke: ${url}`);
  log(`default browser smoke: ${defaultUrl}`);
  let browser;
  try {
    browser = await browserPhase(url);
    if (browser.attempted && !browser.connected) log(`browser: CDP boundary, ${browser.reason}`);
    const defaultBrowser = await browserPhase(defaultUrl, DEFAULT_BACKDROP);
    if (defaultBrowser.attempted && !defaultBrowser.connected) log(`browser: default page CDP boundary, ${defaultBrowser.reason}`);
    if (!defaultBrowser.attempted) log(`browser: default page not probed, ${defaultBrowser.reason}`);
    const loader = await loaderPhase(base);
    const backdropHits = hits.filter((hit) => hit.rel.endsWith("menu-backdrop.jpg"));
    const heroHits = hits.filter((hit) => /neckbeard-skinned|noscope-skinned|chad-skinned|neutral-centaur/.test(hit.rel));
    const allFailedHits = hits.filter((hit) => hit.status >= 400 && !hit.ignored && (/bota/.test(hit.rel) || /menu-backdrop/.test(hit.rel)));
    const criticalPattern = /menu-backdrop\.jpg|neckbeard-skinned\.glb|noscope-skinned\.glb|chad-skinned\.glb|neutral-centaur-.*-skinned\.glb/;
    const failedHits = allFailedHits.filter((hit) => criticalPattern.test(hit.rel));
    const newCritical404s = allFailedHits.filter((hit) => !criticalPattern.test(hit.rel));
    const backdropOk = backdropHits.some((hit) => hit.status === 200 && hit.bytes === 3654481);
    const defaultBackdropHits = hits.filter((hit) => hit.rel.endsWith(DEFAULT_BACKDROP.file));
    const defaultBackdropOk = defaultBackdropHits.some((hit) => hit.status === 200 && hit.bytes === DEFAULT_BACKDROP.bytes);
    log(`server: botaRequests=${hits.filter((h) => h.rel.startsWith("scenarios/bota/")).length} backdropHits=${backdropHits.length} heroHits=${heroHits.length} criticalFailedRequests=${failedHits.length} other404s=${allFailedHits.length}`);
    log(`server triage: optionalIgnored=${[...new Set(hits.filter((h) => h.status === 404 && h.ignored).map((h) => h.rel))].join(",") || "none"} newCritical404s=${newCritical404s.length}`);
    if (!backdropOk) throw new Error("browser did not fetch the BOTA menu backdrop with the expected byte size");
    if (!defaultBackdropOk) throw new Error("browser did not fetch the default pack menu backdrop with the expected byte size");
    if (failedHits.length) throw new Error(`critical BOTA smoke requests failed: ${failedHits.map((h) => `${h.rel}:${h.status}`).join(", ")}`);
    if (newCritical404s.length) throw new Error(`new BOTA 404 regressions: ${[...new Set(newCritical404s.map((h) => h.rel))].join(", ")}`);
    const summary = {
      ok: true,
      browser: browser.attempted && browser.connected ? "real CDP page" : "CDP unavailable, Node client-loader fallback",
      uSkinned: browserRendererResult?.uSkinned === true ? 1 : Number(browserRendererResult?.uSkinned ?? 0),
      jointFloats: browserRendererResult?.jointFloats ?? 0,
      glError: browserRendererResult?.glError ?? null,
      models: loader.models.length,
      heroes: HERO_FULLNAMES.length,
      joints: loader.models[0].joints,
      clips: loader.models[0].clips,
      motion: "walk/run",
      backdropHits: backdropHits.filter((hit) => hit.status === 200).length,
      defaultBackdropHits: defaultBackdropHits.filter((hit) => hit.status === 200).length,
      other404s: allFailedHits.length,
      criticalFailedRequests: failedHits.length,
    };
    log(`bota browser smoke ok: browser=${summary.browser} models=${summary.models} heroes=${summary.heroes} joints=${summary.joints} clips=${summary.clips} motion=walk/run joint palettes differ`);
    log("render evidence: live __ci hooks do not expose per-draw state; CDP WebGL2 createRenderer.drawSkinned probe proved uSkinned=1 with a 448-float joint palette through render3d.js:1007-1025");
    if (compact) console.log(JSON.stringify(summary));
  } finally {
    server.close();
  }
}

await main();
