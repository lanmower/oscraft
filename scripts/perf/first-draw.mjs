#!/usr/bin/env node
import { chromium } from "playwright";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import zlib from "node:zlib";
import { contentType, isCompressible } from "../dev/mime.mjs";

const args = process.argv.slice(2);
const siteDir = path.resolve(args.find((a) => !a.startsWith("--")) ?? "");
const loads = Number(args.find((a) => a.startsWith("--loads="))?.slice(8) ?? 3);
const ceilingMs = Number(args.find((a) => a.startsWith("--ceiling-ms="))?.slice(13) ?? 10000);
if (!siteDir || !fs.existsSync(path.join(siteDir, "js/client/play.html"))) {
  console.error("usage: node scripts/perf/first-draw.mjs <site-dir>, and the directory must hold js/client/play.html");
  process.exit(2);
}

const gzipped = new Map();
let sentBytes = 0;
let perFile = new Map();
const note = (rel, bytes) => { sentBytes += bytes; perFile.set(rel, (perFile.get(rel) ?? 0) + bytes); };
const server = http.createServer((req, res) => {
  const rel = decodeURIComponent(req.url.split("?")[0]).replace(/^\/+/, "") || "index.html";
  const file = path.join(siteDir, rel);
  if (!file.startsWith(siteDir + path.sep) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) { res.writeHead(404); res.end(); return; }
  const type = contentType(file);
  const headers = { "content-type": type, "cache-control": "no-store" };
  if (req.method === "HEAD") {
    // wasm watcher polls HEADs; answer headers-only so they cost no bytes
    headers["content-length"] = String(fs.statSync(file).size);
    headers["etag"] = `"${fs.statSync(file).size}"`;
    headers["last-modified"] = new Date(fs.statSync(file).mtimeMs).toUTCString();
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
const times = [];
try {
  for (let i = 0; i < loads; i += 1) {
    const context = await browser.newContext({ serviceWorkers: "block" });
    const page = await context.newPage();
    const errors = [];
    page.on("pageerror", (err) => errors.push(err.message));
    // Durable phase split: boot game (wasm+mods), boot world (asset build begins),
    // local preview (worldReady resolved; the first frame follows). Times are relative to the goto start.
    const phases = { startedAt: 0, game: null, world: null, preview: null, relaysAlive: null };
    page.on("console", (m) => {
      const at = phases.startedAt ? Date.now() - phases.startedAt : null;
      if (m.text() === "[boot] game") phases.game = at;
      if (m.text() === "[boot] world") phases.world = at;
      if (m.text() === "[play] local preview") phases.preview = at;
    });
    const sentBefore = sentBytes;
    perFile = new Map();
    const started = Date.now();
    phases.startedAt = started;
    await page.goto(`${base}/js/client/play.html?host=1&nocache=${started}#moba`, { waitUntil: "domcontentloaded" });
    const drawn = await page.waitForFunction(
      () => document.body.dataset.worldBuilt === "1" || (window.oscraft && window.oscraft.getState && window.oscraft.getState().frame && window.oscraft.getState().frame.drawCalls > 0),
      null,
      { timeout: 120000, polling: 100 },
    ).then(() => Date.now() - started, () => null);
    const startError = await page.evaluate(() => document.body.dataset.startError || null).catch(() => "page closed");
    if (drawn === null || startError) {
      console.error(`load ${i + 1}: no first frame (startError=${startError}) errors=${errors.slice(0, 3).join(" | ")}`);
      process.exitCode = 1;
    } else {
      times.push(drawn);
      console.log(`first-draw load=${i + 1} ms=${drawn} transferred-bytes=${sentBytes - sentBefore} requests=${perFile.size} game=${phases.game ?? "?"} world=${phases.world ?? "?"} preview=${phases.preview ?? "?"}`);
      for (const [rel, bytes] of [...perFile].sort((a, b) => b[1] - a[1]).slice(0, 6)) console.log(`  load=${i + 1} ${bytes} ${rel}`);
    }
    await context.close();
  }
} finally {
  await browser.close();
  server.close();
}

if (times.length === 0) { console.error("first-draw: no load drew a frame"); process.exit(1); }
const sorted = [...times].sort((a, b) => a - b);
const median = sorted[Math.floor(sorted.length / 2)];
console.log(`first-draw median-ms=${median} loads=${times.length} ceiling-ms=${ceilingMs} ${median <= ceilingMs ? "ok" : "over"}`);
if (median > ceilingMs || process.exitCode) process.exit(1);
