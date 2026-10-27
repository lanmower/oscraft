import { liveRenderers } from "./render3d.js";

const SOURCE = "/__hmr";
const SELF = "hmr.js";
let queue = Promise.resolve();
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
    if (source.readyState === EventSource.CLOSED) console.log("[hmr] no dev server channel here; hot reload is off");
  };
  console.log("[hmr] listening for changes from the dev server (pushed, not polled)");
}

if (new URLSearchParams(location.search).has("hmr") || ["localhost", "127.0.0.1"].includes(location.hostname)) start();
