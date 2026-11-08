#!/usr/bin/env node
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import zlib from "node:zlib";
import { fileURLToPath } from "node:url";

export const PASSES = ["cold", "warm"];

const packTypes = new Map([
  [".bin", "application/octet-stream"],
  [".css", "text/css"],
  [".glb", "model/gltf-binary"],
  [".html", "text/html; charset=utf-8"],
  [".js", "application/javascript"],
  [".json", "application/json"],
  [".ktx2", "image/ktx2"],
  [".lua", "text/plain"],
  [".mjs", "application/javascript"],
  [".ogg", "audio/ogg"],
  [".osd", "text/plain"],
  [".osdt", "application/octet-stream"],
  [".png", "image/png"],
  [".tsv", "text/plain"],
  [".txt", "text/plain"],
  [".wasm", "application/wasm"],
  [".webp", "image/webp"],
]);
const COMPRESSIBLE = /^(text\/|application\/(javascript|json|wasm)|image\/svg)/;
const PACK_PATH = /\/scenarios\/moba\/assets\/packs\//;
const IMMUTABLE = "public, max-age=31536000, immutable";
const REVALIDATE = "public, max-age=0, must-revalidate";

export function median(samples) {
  if (samples.length === 0) throw new RangeError("median needs at least one sample");
  const sorted = [...samples].sort((a, b) => a - b);
  const middle = sorted.length >> 1;
  return sorted.length % 2 === 1 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

export function contentTypeFor(rel) {
  return packTypes.get(path.extname(rel).toLowerCase()) ?? "application/octet-stream";
}

export function cacheControlFor(rel) {
  const urlPath = `/${rel}`;
  if (PACK_PATH.test(urlPath)) return IMMUTABLE;
  if (urlPath.endsWith("/packs/index.json")) return REVALIDATE;
  return "no-store";
}

export function headersFor(rel, stat) {
  return {
    "content-type": contentTypeFor(rel),
    "content-length": String(stat.size),
    "cache-control": cacheControlFor(rel),
    etag: `"${stat.size}-${Math.trunc(stat.mtimeMs)}"`,
    "access-control-allow-origin": "*",
  };
}

export function reportPasses(passes) {
  return PASSES.filter((pass) => passes[pass]?.length > 0).map(
    (pass) => `first-draw ${pass} worldbuilt-median-ms=${median(passes[pass])} loads=${passes[pass].length}`,
  );
}

function freshTraffic() {
  return { requests: 0, bytes: 0, paths: new Map() };
}

async function main(args) {
  const siteArg = args.find((a) => !a.startsWith("--"));
  const siteDir = path.resolve(siteArg ?? "");
  const loads = Number(args.find((a) => a.startsWith("--loads="))?.slice(8) ?? 3);
  if (!siteArg || !fs.existsSync(path.join(siteDir, "js/client/play.html"))) {
    console.error("usage: node scripts/perf/first-draw.mjs <site-dir>, and the directory must hold js/client/play.html");
    process.exitCode = 2;
    return;
  }

  const { chromium } = await import("playwright");
  const payloads = new Map();
  const payloadOf = (file, gzip) => {
    const key = `${gzip}:${file}`;
    if (!payloads.has(key)) {
      const raw = fs.readFileSync(file);
      payloads.set(key, gzip ? zlib.gzipSync(raw, { level: 6 }) : raw);
    }
    return payloads.get(key);
  };
  let traffic = freshTraffic();
  const note = (rel, bytes) => {
    traffic.bytes += bytes;
    traffic.paths.set(rel, (traffic.paths.get(rel) ?? 0) + bytes);
  };
  const server = http.createServer((req, res) => {
    traffic.requests += 1;
    const rel = decodeURIComponent(req.url.split("?")[0]).replace(/^\/+/, "") || "index.html";
    const file = path.join(siteDir, rel);
    if (!file.startsWith(siteDir + path.sep) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
      res.writeHead(404, { "content-type": "text/plain", "cache-control": "no-store" });
      res.end();
      return;
    }
    const stat = fs.statSync(file);
    const headers = headersFor(rel, stat);
    if (req.headers["if-none-match"] === headers.etag) {
      delete headers["content-length"];
      note(rel, 0);
      res.writeHead(304, headers);
      res.end();
      return;
    }
    if (req.method === "HEAD") {
      note(rel, 0);
      res.writeHead(200, headers);
      res.end();
      return;
    }
    const gzip = COMPRESSIBLE.test(headers["content-type"]) && /\bgzip\b/.test(req.headers["accept-encoding"] ?? "");
    const body = payloadOf(file, gzip);
    note(rel, body.length);
    res.writeHead(200, gzip
      ? { ...headers, "content-encoding": "gzip", "content-length": body.length }
      : { ...headers, "content-length": body.length });
    res.end(body);
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}`;

  const browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_PATH || undefined,
    args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"],
  });
  const passes = Object.fromEntries(PASSES.map((pass) => [pass, []]));
  const loadOnce = async (context, url, pass, index) => {
    const page = await context.newPage();
    const errors = [];
    page.on("pageerror", (err) => errors.push(err.message));
    const phases = { startedAt: 0, game: null, world: null, preview: null };
    page.on("console", (m) => {
      const at = phases.startedAt ? Date.now() - phases.startedAt : null;
      if (m.text() === "[boot] game") phases.game = at;
      if (m.text() === "[boot] world") phases.world = at;
      if (m.text() === "[play] local preview") phases.preview = at;
    });
    traffic = freshTraffic();
    const started = Date.now();
    phases.startedAt = started;
    await page.goto(url, { waitUntil: "domcontentloaded" });
    const built = await page.waitForFunction(
      () => document.body.dataset.worldBuilt === "1",
      null,
      { timeout: 120000, polling: 100 },
    ).then(() => Date.now() - started, () => null);
    const startError = await page.evaluate(() => document.body.dataset.startError || null).catch(() => "page closed");
    await page.close();
    if (built === null || startError) {
      console.error(`${pass} load ${index}: worldBuilt not reached (startError=${startError}) errors=${errors.slice(0, 3).join(" | ")}`);
      process.exitCode = 1;
      return null;
    }
    console.log(`first-draw ${pass} load=${index} worldbuilt-ms=${built} transferred-bytes=${traffic.bytes} requests=${traffic.requests} distinct-paths=${traffic.paths.size} game=${phases.game ?? "?"} world=${phases.world ?? "?"} preview=${phases.preview ?? "?"}`);
    for (const [rel, bytes] of [...traffic.paths].sort((a, b) => b[1] - a[1]).slice(0, 6)) console.log(`  ${pass} load=${index} ${bytes} ${rel}`);
    return built;
  };

  try {
    for (let i = 0; i < loads; i += 1) {
      const context = await browser.newContext({ serviceWorkers: "block" });
      try {
        const url = `${base}/js/client/play.html?host=1&nocache=${Date.now()}#moba`;
        for (const pass of PASSES) {
          const built = await loadOnce(context, url, pass, i + 1);
          if (built !== null) passes[pass].push(built);
        }
      } finally {
        await context.close();
      }
    }
  } finally {
    await browser.close();
    server.close();
  }

  for (const line of reportPasses(passes)) console.log(line);
  if (passes.cold.length === 0) {
    console.error("first-draw: no cold load reached worldBuilt");
    process.exitCode = 1;
    return;
  }
  if (passes.warm.length === 0) {
    console.error("first-draw: no warm load reached worldBuilt");
    process.exitCode = 1;
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}
