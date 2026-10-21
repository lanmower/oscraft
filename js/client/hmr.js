// Hot reload for the graphics code, on only when the page was opened with ?hmr. Every 500 ms it checks the shader files
// and the js/client modules that are loaded. A changed shader is recompiled in place on each live renderer, so the
// match keeps running. A changed module is re-imported with a ?v= query; if its exports include hmrReload(mod), that
// hook is called, otherwise the log says a page reload is needed.
import { liveRenderers, SHADER_FILES } from "./render3d.js";

const POLL_MS = 500;
const SELF = "hmr.js";
const seen = new Map(); // url -> signature

// Cheap change key from the response headers (HEAD). When the server sends no validators, fall back to a text hash.
async function signature(url) {
  const head = await fetch(url, { method: "HEAD", cache: "no-store" });
  if (!head.ok) return null;
  const key = [head.headers.get("etag"), head.headers.get("last-modified"), head.headers.get("content-length")].join("|");
  if (key !== "null|null|null") return key;
  const get = await fetch(url, { cache: "no-store" });
  if (!get.ok) return null;
  return hash(await get.text());
}

function hash(text) {
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) h = Math.imul(h ^ text.charCodeAt(i), 0x01000193);
  return `fnv:${h >>> 0}:${text.length}`;
}

function shaderUrls() {
  return SHADER_FILES.map((name) => new URL(`./shaders/${name}`, import.meta.url).href);
}

// The loaded js/client modules, from the browser's resource timing (no directory listing is available to the page).
function moduleUrls() {
  const out = new Set();
  for (const entry of performance.getEntriesByType("resource")) {
    const u = new URL(entry.name);
    if (!u.pathname.includes("/js/client/") || !u.pathname.endsWith(".js")) continue;
    if (u.pathname.endsWith("/" + SELF) || /\/(vendor|pkg)\//.test(u.pathname)) continue;
    u.search = "";
    out.add(u.href);
  }
  return [...out];
}

async function reloadShaders() {
  for (const r of liveRenderers) {
    const swapped = await r.recompile();
    console.log(`[hmr] shaders recompiled: ${swapped.length ? swapped.join(", ") : "none (kept the old programs)"}`);
  }
}

async function reloadModule(url) {
  const bumped = new URL(url);
  bumped.search = `?v=${Date.now()}`;
  let mod;
  try {
    mod = await import(bumped.href);
  } catch (err) {
    console.error(`[hmr] ${url} failed to re-import: ${err.message}`);
    return;
  }
  if (typeof mod.hmrReload === "function") {
    try {
      mod.hmrReload(mod);
      console.log(`[hmr] ${url} reloaded through its hook`);
    } catch (err) {
      console.error(`[hmr] ${url} hook failed: ${err.message}`);
    }
  } else {
    console.log(`[hmr] ${url} changed: no hook, a page reload is needed to apply it`);
  }
}

async function poll() {
  const shaders = shaderUrls();
  const modules = moduleUrls();
  let shadersChanged = false;
  const changedModules = [];
  for (const url of [...shaders, ...modules]) {
    let sig;
    try {
      sig = await signature(url);
    } catch {
      continue; // a transient network error is retried on the next tick
    }
    if (sig === null) continue;
    const prev = seen.get(url);
    seen.set(url, sig);
    if (prev === undefined || prev === sig) continue;
    if (shaders.includes(url)) shadersChanged = true;
    else changedModules.push(url);
  }
  if (shadersChanged) await reloadShaders();
  for (const url of changedModules) await reloadModule(url);
}

function start() {
  const tick = async () => {
    try {
      await poll();
    } catch (err) {
      console.error(`[hmr] poll failed: ${err.message}`);
    }
    setTimeout(tick, POLL_MS);
  };
  tick();
  console.log(`[hmr] watching ${SHADER_FILES.length} shader file(s) and the js/client modules every ${POLL_MS} ms`);
}

if (new URLSearchParams(location.search).has("hmr")) start();
