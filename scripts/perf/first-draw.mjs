#!/usr/bin/env node
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import zlib from "node:zlib";
import { fileURLToPath } from "node:url";
import { contentType, isCompressible } from "../dev/mime.mjs";

export function median(samples) {
  if (samples.length === 0) throw new RangeError("median needs at least one sample");
  const sorted = [...samples].sort((a, b) => a - b);
  const middle = sorted.length >> 1;
  return sorted.length % 2 === 1 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
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
  const gzipped = new Map();
  let traffic = freshTraffic();
  const note = (rel, bytes) => {
    traffic.bytes += bytes;
    traffic.paths.set(rel, (traffic.paths.get(rel) ?? 0) + bytes);
  };
  const server = http.createServer((req, res) => {
    traffic.requests += 1;
    const rel = decodeURIComponent(req.url.split("?")[0]).replace(/^\/+/, "") || "index.html";
    const file = path.join(siteDir, rel);
    if (!file.startsWith(siteDir + path.sep) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) { res.writeHead(404); res.end(); return; }
    const type = contentType(file);
    const headers = { "content-type": type, "cache-control": "no-store" };
    if (req.method === "HEAD") {
      headers["content-length"] = String(fs.statSync(file).size);
      headers["etag"] = `"${fs.statSync(file).size}"`;
      headers["last-modified"] = new Date(fs.statSync(file).mtimeMs).toUTCString();
      note(rel, 0);
      res.writeHead(200, headers);
      res.end();
      return;
    }
    if (isCompressible(type) && /\bgzip\b/.test(req.headers["accept-encoding"] ?? "")) {
      if (!gzipped.has(file)) gzipped.set(file, zlib.gzipSync(fs.readFileSync(file), { level: 6 }));
      const body = gzipped.get(file);
      note(rel, body.length);
      res.writeHead(200, { ...headers, "content-encoding": "gzip", "content-length": body.length });
      res.end(body);
      return;
    }
    note(rel, fs.statSync(file).size);
    res.writeHead(200, headers);
    fs.createReadStream(file).pipe(res);
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}`;

  const browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_PATH || undefined,
    args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"],
  });
  const worldBuiltMs = [];
  try {
    for (let i = 0; i < loads; i += 1) {
      const context = await browser.newContext({ serviceWorkers: "block" });
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
      await page.goto(`${base}/js/client/play.html?host=1&nocache=${started}#moba`, { waitUntil: "domcontentloaded" });
      const built = await page.waitForFunction(
        () => document.body.dataset.worldBuilt === "1",
        null,
        { timeout: 120000, polling: 100 },
      ).then(() => Date.now() - started, () => null);
      const startError = await page.evaluate(() => document.body.dataset.startError || null).catch(() => "page closed");
      if (built === null || startError) {
        console.error(`load ${i + 1}: worldBuilt not reached (startError=${startError}) errors=${errors.slice(0, 3).join(" | ")}`);
        process.exitCode = 1;
      } else {
        worldBuiltMs.push(built);
        console.log(`first-draw load=${i + 1} worldbuilt-ms=${built} transferred-bytes=${traffic.bytes} requests=${traffic.requests} distinct-paths=${traffic.paths.size} game=${phases.game ?? "?"} world=${phases.world ?? "?"} preview=${phases.preview ?? "?"}`);
        for (const [rel, bytes] of [...traffic.paths].sort((a, b) => b[1] - a[1]).slice(0, 6)) console.log(`  load=${i + 1} ${bytes} ${rel}`);
      }
      await context.close();
    }
  } finally {
    await browser.close();
    server.close();
  }

  if (worldBuiltMs.length === 0) {
    console.error("first-draw: no load reached worldBuilt");
    process.exitCode = 1;
    return;
  }
  console.log(`first-draw worldbuilt-median-ms=${median(worldBuiltMs)} loads=${worldBuiltMs.length}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}
