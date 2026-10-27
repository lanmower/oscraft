import { createHash } from "node:crypto";
import { setDefaultResultOrder } from "node:dns";

setDefaultResultOrder?.("ipv4first");

export const FIXED = ["version.json", "js/client/version.json"];

export const sha256 = (buf) => createHash("sha256").update(buf).digest("hex");

const resolveFrom = (fromRel, spec) => new URL(spec, `http://site/${fromRel}`).pathname.slice(1);

export function readJson(readText) {
  return async (rel) => {
    try {
      const text = await readText(rel);
      if (text == null) return null;
      return JSON.parse(text);
    } catch {
      return null;
    }
  };
}

// Rows a scenario manifest adds: everything the page fetches from the
// scenario directory at runtime (data tables, image, mods, ui json, rules
// and the asset catalogue paths). Paths resolve against the manifest's dir.
async function discoverScenario(dir, manifest, found, readJsonAt) {
  const add = (rel) => {
    if (typeof rel === "string" && rel) found.add(resolveFrom(`${dir}/x`, rel));
  };
  add("manifest.json");
  if (manifest == null || typeof manifest !== "object") return;
  for (const k of ["map", "image", "rules", "ui", "effects"]) add(manifest[k]);
  for (const list of ["data", "mods", "tables", "fixtures"]) {
    if (Array.isArray(manifest[list])) for (const v of manifest[list]) add(v);
  }
  if (manifest.modules && typeof manifest.modules === "object") {
    for (const v of Object.values(manifest.modules)) add(v);
  }
  const assets = manifest.assets ?? {};
  const root = assets.root ?? ".";
  for (const v of Object.values(assets.textures ?? {})) add(`${root}/${v}`);
  for (const k of ["catalogue", "ktx2", "props", "propSizes", "anim"]) add(`${root}/${assets[k]}`);
  // The catalogue lists every shipped asset path (models, textures, icons,
  // sounds); ktx2.json lists the compressed textures the page fetches.
  const catalogue = await readJsonAt(`${dir}/${root}/${assets.catalogue ?? ""}`);
  if (catalogue && Array.isArray(catalogue.items)) {
    for (const item of catalogue.items) if (item?.path) add(`${root}/${item.path}`);
  }
  const ktx2 = await readJsonAt(`${dir}/${root}/${assets.ktx2 ?? ""}`);
  if (Array.isArray(ktx2)) for (const v of ktx2) add(`${root}/${v}`);
}

export async function discover(readText, page = "js/client/play.html") {
  const found = new Set([...FIXED, page]);
  const packDirs = new Set();
  const readJsonAt = readJson(readText);
  const texts = new Map([[page, await readText(page)]]);
  if (texts.get(page) == null) return [...found].sort();
  const queue = [];
  for (const v of texts.get(page).matchAll(/\b(src|data-map|data-pkg)="([^"]+)"/g)) {
    if (/^(https?:|data:)/.test(v[2])) continue;
    queue.push(resolveFrom(page, v[2]));
  }
  while (queue.length) {
    const rel = queue.shift();
    if (found.has(rel)) continue;
    found.add(rel);
    if (!/\.m?js$/.test(rel)) continue;
    const text = await readText(rel);
    if (text == null) continue;
    texts.set(rel, text);
    for (const m of text.matchAll(/(?:from\s+|import\s*\(\s*)["'](\.{1,2}\/[^"']+)["']/g)) queue.push(resolveFrom(rel, m[1]));
    for (const m of text.matchAll(/["'`]([^"'`\s]+\.wasm)["'`]/g)) queue.push(resolveFrom(rel, m[1]));
  }
  // Runtime surfaces. The pack loader takes its base dir from the `pack`
  // query param (default literal in pack_load.js) and fetches
  // <dir>/manifest.json from it, so every pack dir names a scenario whose
  // manifest declares the rest of the content. Any literal naming
  // <something/manifest.json> counts too.
  for (const text of texts.values()) {
    const packDefault = text.match(/PACK_PARAMS\.get\("pack"\)\s*\|\|\s*PACK_PARAMS\.get\("package"\)\s*\|\|\s*"([^"]+)"/);
    if (packDefault) packDirs.add(packDefault[1]);
    for (const m of text.matchAll(/["']((?:scenarios|packs)\/[\w.@/-]+)["']/g)) {
      if (/manifest\.json$/.test(m[1])) found.add(m[1]);
      else packDirs.add(m[1]);
    }
  }
  for (const dir of packDirs) found.add(resolveFrom("x", `${dir}/manifest.json`));
  // The site ships packs/latest.json unconditionally (layout-site.sh);
  // its index names the served archive and every split part. The index
  // itself is always compared, even when the local copy fails to parse.
  found.add("packs/latest.json");
  const latest = {};
  try {
    Object.assign(latest, JSON.parse(await readText("packs/latest.json")));
  } catch {
    // unparseable local index: the fetch above already makes it mismatch
  }
  // The parts are what the site serves (pack-package.mjs splits past the
  // size limit and drops the joined archive), so when parts[] exists the
  // `file` name is only the label of what joining would produce.
  const parts = latest.parts ?? [];
  if (latest.file && parts.length === 0) found.add(resolveFrom("packs/x", latest.file));
  for (const part of parts) if (part?.file) found.add(resolveFrom("packs/x", part.file));
  const manifests = [...found].filter((rel) => /manifest\.json$/.test(rel));
  for (const rel of manifests) {
    const dir = rel.replace(/\/manifest\.json$/, "");
    await discoverScenario(dir, await readJsonAt(rel), found, readJsonAt);
  }
  return [...found].sort();
}

export function table(headers, rows) {
  const all = [headers, ...rows].map((r) => r.map((c) => String(c ?? "")));
  const widths = headers.map((_, i) => Math.max(...all.map((r) => r[i].length)));
  const line = (r) => r.map((c, i) => c.padEnd(widths[i])).join("  ").trimEnd();
  return [line(all[0]), widths.map((w) => "-".repeat(w)).join("  "), ...all.slice(1).map(line)].join("\n");
}

export const normBase = (base) => (base.endsWith("/") ? base : `${base}/`);

export function liveGet(base, cacheBust = Date.now()) {
  return async (rel) => {
    try {
      const r = await fetch(`${base}${rel}?cb=${cacheBust}`, { cache: "no-store", redirect: "follow" });
      return { status: r.status, buf: Buffer.from(await r.arrayBuffer()) };
    } catch (e) {
      return { status: 0, buf: null, error: e.cause?.code || e.message };
    }
  };
}

export function claimOf(rel, buf, mapStats) {
  if (!buf) return "-";
  if (rel.endsWith("version.json")) {
    try {
      const j = JSON.parse(buf.toString("utf8"));
      return `sha=${j.sha ?? "?"} built=${j.built ?? "?"}`;
    } catch {
      return "unparseable";
    }
  }
  if (rel.endsWith(".bin")) {
    try {
      const s = mapStats(buf);
      return `OSDT v${s.format.version} ${s.grid.width}x${s.grid.height}`;
    } catch (e) {
      return `not OSDT: ${e.message}`;
    }
  }
  return "-";
}
