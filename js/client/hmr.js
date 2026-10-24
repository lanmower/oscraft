// Hot reload for the client, on localhost and on any page opened with ?hmr. The dev server (scripts/dev/serve.mjs) watches
// the served files and pushes one message per changed file over /__hmr (server-sent events). This page only listens: nothing
// is polled, and a change arrives once, in the order the server saw it. Each message is { kind, path } and is announced as a
// window "oscraft:hmr" CustomEvent with detail { url, kind, value }, url the absolute URL of the file. kind is one of:
//   shader  js/client/shaders/*.glsl. Recompiled in place on each live renderer; value is undefined.
//   module  js/client/**/*.js (not vendor or pkg). Re-imported with ?v=; value is what the module's hmrReload(mod) hook returns.
//           A module without a hook reloads the page.
//   asset   the pack's json (scenarios/moba/*.json, scenarios/moba/assets/*.json). The consumer re-reads it; value is undefined.
//   lua     the pack's mod files (scenarios/moba/**.lua). The consumer re-reads it; value is undefined.
//   map     scenarios/moba/map.osd or moba.osdt. The consumer re-reads it; value is undefined.
//   model   scenarios/moba/assets/models/*.glb. The consumer refetches by url; value is undefined.
//   wasm    js/client/pkg/*. A page reload is needed, so the consumer decides; value is undefined.
// The messages do not depend on the transport: a relay can deliver the same { kind, path } later, and handle() applies it.
//
// Hook contract (hot-accept). A module exports hmrReload(mod). It runs on the NEW module instance, so it cannot read the old
// instance's state; instead each module stashes the inputs of its builders on globalThis.__oscraftHot (a Map shared by every
// module instance) when the builder runs, and the hook re-runs the builder from that stash. The hook's return value is
// dispatched as a window "oscraft:hmr" event: detail = { url, kind: "module", value }. For data modules (terrain, tiles, anim)
// value is the rebuilt data for the owner to rebuild its mesh or clips; for panels (hud, ui_*) value is the new panel object,
// and the old root element (same id) is replaced in place.
import { liveRenderers } from "./render3d.js";

const SOURCE = "/__hmr";
const SELF = "hmr.js";
let queue = Promise.resolve(); // messages are applied one at a time, in arrival order, so reloads never overlap
let reloadTimer = null;

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

function scheduleReload() {
  clearTimeout(reloadTimer);
  reloadTimer = setTimeout(() => location.reload(), 300);
}

function announce(url, kind) {
  console.log("[hmr] " + url + " changed (" + kind + ")");
  window.dispatchEvent(new CustomEvent("oscraft:hmr", { detail: { url, kind, value: undefined } }));
}

// Applies one change message from the dev server (or, later, a relay).
async function handle({ kind, path }) {
  const url = new URL("/" + path, location.origin).href;
  if (kind === "module") {
    if (url.endsWith("/" + SELF)) return;
    await reloadModule(url);
    return;
  }
  if (kind === "shader") {
    await reloadShaders();
    announce(url, kind);
    return;
  }
  if (kind === "wasm") console.log("[hmr] " + url + " changed (wasm): a page reload is needed; announced for the consumer to decide");
  announce(url, kind);
}

function start() {
  const source = new EventSource(SOURCE);
  source.addEventListener("change", (e) => {
    let msg;
    try {
      msg = JSON.parse(e.data);
    } catch (err) {
      console.error("[hmr] unreadable message: " + err.message);
      return;
    }
    queue = queue
      .then(() => handle(msg))
      .catch((err) => console.error("[hmr] " + msg.path + " failed: " + err.message));
  });
  source.onerror = () => {
    // The browser retries a dropped channel itself. A channel that closed for good (no dev server here) is reported once.
    if (source.readyState === EventSource.CLOSED) console.log("[hmr] no dev server channel here; hot reload is off");
  };
  console.log("[hmr] listening for changes from the dev server (pushed, not polled)");
}

// On by default on localhost (the dev preview); the ?hmr query turns it on anywhere else.
if (new URLSearchParams(location.search).has("hmr") || ["localhost", "127.0.0.1"].includes(location.hostname)) start();
