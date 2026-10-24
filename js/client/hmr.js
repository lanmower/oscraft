// Hot reload for the client: on for localhost and any page opened with ?hmr. Every 500 ms it checks the URLs it knows and
// announces each change as a window "oscraft:hmr" CustomEvent with detail { url, kind, value }. kind is one of:
//   shader  the shader files. Recompiled in place on each live renderer; value is undefined.
//   module  the js/client modules the page loaded. Re-imported with ?v=; value is what the module's hmrReload(mod) hook returns.
//           A module without a hook reloads the page.
//   asset   the pack's catalogue, ktx2, doodads, doodad sizes and UI json. value is the parsed JSON.
//   lua     the pack's mod files (manifest.mods and manifest.modules). value is the new text.
//   map     the pack's map (manifest.map). value is the new text.
//   model   each .glb the page has fetched. value is undefined; the consumer refetches by url.
//   wasm    each js/client/pkg file the page has fetched. value is undefined; a page reload is needed, so the consumer decides.
// Pack files are resolved through js/client/pack_load.js (manifest, packUrl, assetUrl, modulePaths), the way the pack reads them.
// Shader and module changes apply at once. The other kinds are batched: a burst of saves is announced once a poll passes with
// no new change, or BURST_MS after the burst began, whichever comes first.
//
// Hook contract (hot-accept). A module exports hmrReload(mod). It runs on the NEW module instance, so it cannot read the old
// instance's state; instead each module stashes the inputs of its builders on globalThis.__oscraftHot (a Map shared by every
// module instance) when the builder runs, and the hook re-runs the builder from that stash. The hook's return value is
// dispatched as a window "oscraft:hmr" event: detail = { url, kind: "module", value }. For data modules (terrain, tiles, anim)
// value is the rebuilt data for the owner to rebuild its mesh or clips; for panels (hud, ui_*) value is the new panel object,
// and the old root element (same id) is replaced in place.
import { liveRenderers, SHADER_FILES } from "./render3d.js";

const POLL_MS = 500;
const BURST_MS = 2000;
const MAX_BACKOFF_MS = 30000;
const SELF = "hmr.js";
const seen = new Map(); // url -> change key
const failures = new Map(); // url -> { count, retryAt }: a failing URL backs off while the rest of the poll goes on
const unwatchable = new Set(); // binary URLs that send no validators: logged once, not polled
const pending = new Map(); // url -> hit, the batched changes of the burst being collected
let batchStartedAt = 0;
let packTargets = []; // { url, kind, textual } of the pack's asset, lua and map files, set once the pack has loaded

// Cheap change key from the response headers (HEAD). When the server sends no validators, a text file falls back to a text
// hash; a binary file with no validators cannot be watched and yields null.
async function signature(url, textual) {
  const head = await fetch(url, { method: "HEAD", cache: "no-store" });
  if (!head.ok) throw new Error("HTTP " + head.status);
  const key = [head.headers.get("etag"), head.headers.get("last-modified"), head.headers.get("content-length")].join("|");
  if (key !== "null|null|null") return key;
  if (!textual) return null;
  const get = await fetch(url, { cache: "no-store" });
  if (!get.ok) throw new Error("HTTP " + get.status);
  return hash(await get.text());
}

function hash(text) {
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) h = Math.imul(h ^ text.charCodeAt(i), 0x01000193);
  return "fnv:" + (h >>> 0) + ":" + text.length;
}

function shaderUrls() {
  return SHADER_FILES.map((name) => new URL("./shaders/" + name, import.meta.url).href);
}

// The files the page has already fetched, from the browser's resource timing (the page has no directory listing). Query and
// fragment are dropped, so a versioned fetch is watched at its file. Only http(s) URLs are kept.
function fetchedFiles() {
  const out = new Map();
  for (const entry of performance.getEntriesByType("resource")) {
    const u = new URL(entry.name);
    if (u.protocol !== "http:" && u.protocol !== "https:") continue;
    u.search = "";
    u.hash = "";
    out.set(u.href, u);
  }
  return [...out.values()];
}

// The loaded js/client modules: not the vendor or pkg code, and not this file.
function moduleUrls(files) {
  return files
    .filter((u) => u.pathname.includes("/js/client/") && u.pathname.endsWith(".js")
      && !u.pathname.endsWith("/" + SELF) && !/\/(vendor|pkg)\//.test(u.pathname))
    .map((u) => u.href);
}

// The pack's own files, resolved through pack_load.js the way the pack reads them. An archive pack reads blob URLs, which do
// not change while the page is open, so only http(s) URLs are watched. A manifest without an entry is skipped.
async function resolvePackTargets() {
  const pack = await import("./pack_load.js");
  const { manifest } = pack;
  const lua = [...new Set([...(manifest.mods ?? []), ...pack.modulePaths(manifest)])];
  const files = [
    ...["catalogue", "ktx2", "doodads", "doodadSizes"].map((key) => ({ kind: "asset", make: () => pack.assetUrl(key) })),
    { kind: "asset", make: () => manifest.ui && pack.packUrl(manifest.ui) },
    { kind: "map", make: () => manifest.map && pack.packUrl(manifest.map) },
    ...lua.map((file) => ({ kind: "lua", make: () => pack.packUrl(file) })),
  ];
  const out = [];
  for (const { kind, make } of files) {
    let url;
    try {
      url = make();
    } catch {
      continue;
    }
    if (typeof url === "string" && /^https?:/.test(url)) out.push({ url, kind, textual: true });
  }
  return out;
}

async function reloadShaders() {
  for (const r of liveRenderers) {
    const swapped = await r.recompile();
    console.log("[hmr] shaders recompiled: " + (swapped.length ? swapped.join(", ") : "none (kept the old programs)"));
  }
}

async function reloadModule(url) {
  const bumped = new URL(url);
  bumped.search = "?v=" + Date.now();
  let mod;
  try {
    mod = await import(bumped.href);
  } catch (err) {
    console.error("[hmr] " + url + " failed to re-import: " + err.message);
    return;
  }
  if (typeof mod.hmrReload === "function") {
    try {
      const value = mod.hmrReload(mod);
      window.dispatchEvent(new CustomEvent("oscraft:hmr", { detail: { url, kind: "module", value } }));
      console.log("[hmr] " + url + " reloaded through its hook");
    } catch (err) {
      console.error("[hmr] " + url + " hook failed: " + err.message);
    }
  } else {
    // Without a hook the change cannot be bound in place: the modules that import this one keep the old instance. The page
    // reloads to apply it (the match restarts). Changes that land together share one reload.
    console.log("[hmr] " + url + " changed: no hook, reloading the page to apply it");
    scheduleReload();
  }
}

let reloadTimer = null;
function scheduleReload() {
  clearTimeout(reloadTimer);
  reloadTimer = setTimeout(() => location.reload(), 300);
}

// The change key of a target against its key at the last sighting. Returns the key it replaced when the target changed,
// else null. The first sighting only records the key. A failed request backs its URL off.
async function check(target, now) {
  const { url, textual } = target;
  const retry = failures.get(url);
  if (retry && now < retry.retryAt) return null;
  let sig;
  try {
    sig = await signature(url, textual);
  } catch (err) {
    backOff(url, err, now);
    return null;
  }
  if (retry) console.log("[hmr] " + url + " reachable again");
  failures.delete(url);
  if (sig === null) {
    if (!unwatchable.has(url)) {
      unwatchable.add(url);
      console.log("[hmr] " + url + " sends no validators, so it is not watched");
    }
    return null;
  }
  const prev = seen.get(url);
  seen.set(url, sig);
  return prev !== undefined && prev !== sig ? prev : null;
}

// A failing URL waits 1 s, 2 s, 4 s and so on (capped) before its next request. Only the first failure is logged.
function backOff(url, err, now) {
  const count = (failures.get(url)?.count ?? 0) + 1;
  failures.set(url, { count, retryAt: now + Math.min(MAX_BACKOFF_MS, POLL_MS * 2 ** count) });
  if (count === 1) console.warn("[hmr] " + url + " unreachable (" + err.message + "); retrying with backoff");
}

// The data a consumer applies: parsed JSON for an asset, the text for lua and map. A model or wasm is refetched by its url by
// the consumer, so nothing is read here.
async function valueOf({ url, kind }) {
  if (kind === "model" || kind === "wasm") return undefined;
  const res = await fetch(url, { cache: "no-store" });
  if (!res.ok) throw new Error("HTTP " + res.status);
  const text = await res.text();
  return kind === "asset" ? JSON.parse(text) : text;
}

// Announces the batched changes of one settled burst, one event per URL. A URL whose data cannot be read is put back at its
// old key, so the next poll sees the change again once its backoff ends.
async function flushBatch() {
  const hits = [...pending.values()];
  pending.clear();
  batchStartedAt = 0;
  const counts = {};
  for (const hit of hits) counts[hit.kind] = (counts[hit.kind] ?? 0) + 1;
  const summary = Object.entries(counts).map(([kind, n]) => n + " " + kind).join(", ");
  console.log("[hmr] burst settled: " + hits.length + " change(s) (" + summary + ")");
  for (const hit of hits) {
    let value;
    try {
      value = await valueOf(hit);
    } catch (err) {
      seen.set(hit.url, hit.prev);
      backOff(hit.url, err, Date.now());
      console.error("[hmr] " + hit.url + " could not be read: " + err.message);
      continue;
    }
    if (hit.kind === "wasm") console.log("[hmr] " + hit.url + " changed (wasm): a page reload is needed; announced for the consumer to decide");
    else console.log("[hmr] " + hit.url + " changed (" + hit.kind + "): announced");
    window.dispatchEvent(new CustomEvent("oscraft:hmr", { detail: { url: hit.url, kind: hit.kind, value } }));
  }
}

async function poll() {
  const now = Date.now();
  const files = fetchedFiles();
  const targets = [
    ...shaderUrls().map((url) => ({ url, kind: "shader", textual: true })),
    ...moduleUrls(files).map((url) => ({ url, kind: "module", textual: true })),
    ...packTargets,
    ...files.filter((u) => u.pathname.endsWith(".glb")).map((u) => ({ url: u.href, kind: "model", textual: false })),
    ...files.filter((u) => u.pathname.includes("/js/client/pkg/")).map((u) => ({ url: u.href, kind: "wasm", textual: false })),
  ];
  const shaderHits = [];
  const moduleHits = [];
  let batched = 0;
  for (const target of targets) {
    const prev = await check(target, now);
    if (prev === null) continue;
    const hit = { ...target, prev };
    if (hit.kind === "shader") shaderHits.push(hit);
    else if (hit.kind === "module") moduleHits.push(hit);
    else {
      batched++;
      if (!pending.has(hit.url)) pending.set(hit.url, hit);
      if (!batchStartedAt) batchStartedAt = now;
    }
  }
  if (shaderHits.length) {
    await reloadShaders();
    for (const hit of shaderHits) {
      console.log("[hmr] " + hit.url + " changed (shader)");
      window.dispatchEvent(new CustomEvent("oscraft:hmr", { detail: { url: hit.url, kind: "shader", value: undefined } }));
    }
  }
  for (const hit of moduleHits) await reloadModule(hit.url);
  // A poll with no new batched change ends the burst; a burst still running is cut off after BURST_MS.
  if (pending.size && (batched === 0 || now - batchStartedAt >= BURST_MS)) await flushBatch();
}

function start() {
  resolvePackTargets().then(
    (targets) => {
      packTargets = targets;
      console.log("[hmr] watching " + targets.length + " pack file(s): assets, lua and map");
    },
    (err) => console.error("[hmr] pack files not watched: " + err.message),
  );
  const tick = async () => {
    try {
      await poll();
    } catch (err) {
      console.error("[hmr] poll failed: " + err.message);
    }
    setTimeout(tick, POLL_MS);
  };
  tick();
  console.log("[hmr] watching " + SHADER_FILES.length + " shader file(s), the js/client modules, fetched models and pkg wasm every " + POLL_MS + " ms");
}

// On by default on localhost (the dev preview); the ?hmr query turns it on anywhere else.
if (new URLSearchParams(location.search).has("hmr") || ["localhost", "127.0.0.1"].includes(location.hostname)) start();
