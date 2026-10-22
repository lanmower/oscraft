#!/usr/bin/env node
// First-draw witness: loads the built site in headless Chromium and times how long the page takes to draw its first
// frame, from navigation to the first frame that reports draw calls. Runs N loads with the cache disabled and prints
// each time and the median, then fails when the median is above the ceiling.
//
// Usage: node scripts/perf/first-draw.mjs <site-dir> [--loads=3] [--ceiling-ms=10000]
// The site directory is the tree the Pages workflow builds (the `_site` artifact). The script serves it on
// 127.0.0.1 with no caching, so each load reads the files it is measured against.
import { chromium } from "playwright";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";

const args = process.argv.slice(2);
const siteDir = path.resolve(args.find((a) => !a.startsWith("--")) ?? "");
const loads = Number(args.find((a) => a.startsWith("--loads="))?.slice(8) ?? 3);
const ceilingMs = Number(args.find((a) => a.startsWith("--ceiling-ms="))?.slice(13) ?? 10000);
if (!siteDir || !fs.existsSync(path.join(siteDir, "js/client/play.html"))) {
  console.error("usage: node scripts/perf/first-draw.mjs <site-dir>, and the directory must hold js/client/play.html");
  process.exit(2);
}

const TYPES = { ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".json": "application/json", ".wasm": "application/wasm", ".css": "text/css", ".glb": "model/gltf-binary", ".osdt": "application/octet-stream", ".ktx2": "image/ktx2", ".png": "image/png", ".jpg": "image/jpeg", ".txt": "text/plain", ".osd": "text/plain", ".lua": "text/plain", ".tsv": "text/plain", ".tar": "application/x-tar", ".gz": "application/gzip" };
const server = http.createServer((req, res) => {
  const rel = decodeURIComponent(req.url.split("?")[0]).replace(/^\/+/, "") || "index.html";
  const file = path.join(siteDir, rel);
  if (!file.startsWith(siteDir) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { "content-type": TYPES[path.extname(file)] ?? "application/octet-stream", "cache-control": "no-store" });
  fs.createReadStream(file).pipe(res);
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const base = `http://127.0.0.1:${server.address().port}`;

// CHROMIUM_PATH runs the timing against a browser already on the machine instead of the one Playwright
// downloads, so the script works without that download. CI leaves it unset and uses the pinned browser.
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
    const started = Date.now();
    await page.goto(`${base}/js/client/play.html?host=1&nocache=${started}#moba`, { waitUntil: "domcontentloaded" });
    const drawn = await page.waitForFunction(
      () => window.oscraft && window.oscraft.getState && window.oscraft.getState().frame && window.oscraft.getState().frame.drawCalls > 0,
      null,
      { timeout: 120000, polling: 100 },
    ).then(() => Date.now() - started, () => null);
    // A load that never drew its first frame may also have torn the page down, so read the start error
    // through the same guard: one bad load reports once and the run continues to the next.
    const startError = await page.evaluate(() => document.body.dataset.startError || null).catch(() => "page closed");
    if (drawn === null || startError) {
      console.error(`load ${i + 1}: no first frame (startError=${startError}) errors=${errors.slice(0, 3).join(" | ")}`);
      process.exitCode = 1;
    } else {
      times.push(drawn);
      console.log(`first-draw load=${i + 1} ms=${drawn}`);
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
