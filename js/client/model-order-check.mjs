import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const settle = () => new Promise((done) => setTimeout(done, 0));

globalThis.location = new URL("http://localhost/js/client/play.html");
globalThis.createImageBitmap = async () => ({ width: 1, height: 1 });

const here = new URL("./", import.meta.url);
const packRoot = new URL("../../scenarios/moba/", import.meta.url);
const readPack = (rel) => {
  try {
    const bytes = readFileSync(new URL(rel, packRoot));
    return Promise.resolve({ ok: true, status: 200, arrayBuffer: async () => bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) });
  } catch {
    return Promise.resolve({ ok: false, status: 404 });
  }
};
const beforeDir = process.argv[2] ? pathToFileURL(`${resolve(process.argv[2])}/`) : null;
const glb = "http://localhost/models/hero.glb";
const ready = Promise.resolve();

async function scenario(dir, gated) {
  const inFlight = new Set();
  const started = [];
  let heroBuildOpen = 0;
  let violations = 0;
  let outstanding = 0;
  let resolved = 0;
  let expected = 0;

  const note = () => {
    if (heroBuildOpen > 0 && [...inFlight].some((entry) => entry.kind === "speculative")) violations += 1;
  };

  globalThis.fetch = (url, init = {}) => {
    const href = String(url);
    const packAt = href.indexOf("/scenarios/moba/");
    if (packAt >= 0) return readPack(href.slice(packAt + "/scenarios/moba/".length));
    if (!href.includes("/models/")) return Promise.resolve({ ok: false, status: 404 });
    if (init.signal?.aborted) return Promise.reject(new DOMException("aborted", "AbortError"));
    return new Promise((resolveFetch, rejectFetch) => {
      const entry = { name: href.slice(href.lastIndexOf("/") + 1), kind: href.includes("hero") ? "selected" : "speculative" };
      const drop = () => inFlight.delete(entry);
      entry.release = () => {
        drop();
        resolveFetch({ ok: true, status: 200, blob: async () => new Blob([new Uint8Array(4)], { type: "image/png" }) });
      };
      init.signal?.addEventListener("abort", () => { drop(); rejectFetch(new DOMException("aborted", "AbortError")); }, { once: true });
      inFlight.add(entry);
      started.push(entry.name);
      note();
    });
  };

  const { createModelOrder } = await import(new URL("model_order.js", dir).href);
  const { loadBaseColorImage } = await import(new URL("texture.js", dir).href);
  const order = createModelOrder();
  const gate = (isSelected, load) => (gated ? order.ordered(isSelected, ready, load) : load(null));
  const track = (promise) => {
    expected += 1;
    outstanding += 1;
    promise.then(() => { resolved += 1; }, () => {}).finally(() => { outstanding -= 1; });
    return promise;
  };
  const heroOpenWhile = (build) => async () => {
    heroBuildOpen += 1;
    try { return await build(); } finally { heroBuildOpen -= 1; }
  };
  const modelLoad = (signal) => globalThis.fetch(new URL("hero.glb", glb), signal ? { signal } : undefined);
  const heroTexture = (signal) => loadBaseColorImage({ uri: "hero_skin.png" }, glb, signal);
  const heroBuild = gated
    ? async () => {
      await order.ordered(true, ready, modelLoad);
      return order.ordered(true, ready, heroTexture);
    }
    : async () => {
      await order.selected(modelLoad);
      return heroTexture(null);
    };
  const rosterTexture = (name) => track(gate(false, (signal) => loadBaseColorImage({ uri: `${name}.png` }, glb, signal)));
  const startHero = () => track(heroOpenWhile(heroBuild)());
  const releaseNamed = (name) => { for (const entry of [...inFlight]) if (entry.name === name) entry.release(); };

  rosterTexture("roster_a");
  await settle();
  startHero();
  await settle();
  rosterTexture("roster_b");
  await settle();
  releaseNamed("hero.glb");
  await settle();
  releaseNamed("hero_skin.png");
  await settle();
  for (let round = 0; round < 20 && (inFlight.size > 0 || outstanding > 0); round += 1) {
    for (const entry of [...inFlight]) entry.release();
    await settle();
  }

  return { violations, resolved, expected, started: started.join(" ") };
}

function verdict(label, result) {
  const ok = result.violations === 0 && result.resolved === result.expected;
  const line = `${label}: ${ok ? "PASS" : "FAIL"} speculative fetch in flight while a selected hero build is open: ${result.violations} events; fetch order ${result.started}; resolved ${result.resolved} of ${result.expected}`;
  return { ok, line };
}

const before = beforeDir ? verdict("texture before (HEAD, direct texture load)", await scenario(beforeDir, false)) : null;
const after = verdict("texture after (through model order)", await scenario(here, true));
if (before) console.log(before.line);
console.log(after.line);
process.exitCode = after.ok ? 0 : 1;
