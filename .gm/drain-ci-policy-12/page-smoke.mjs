#!/usr/bin/env node
// Headless smoke of the built page, the browser step of ci.yml. It serves the site the Pages build produces under a
// name that is not localhost, as the live page is served, so the dev hot reload (hmr.js) stays off as it is on the
// live page. It checks that the lobby mounts (#ui-modal has content), that no page response is 4xx or 5xx (each one is
// named by its URL), that the page raises no uncaught error and no console error, and that the hero icons the lobby
// requests load. A failed connection to an external relay or STUN server is the network, not the page, and
// /favicon.ico is not part of the page, so neither is counted. Exits 1 with one line per problem.
//
// Usage: node scripts/perf/page-smoke.mjs <site-dir> [--timeout-ms=90000]

import { chromium } from "playwright";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { contentType } from "../dev/mime.mjs";

const args = process.argv.slice(2);
const siteDir = path.resolve(args.find((a) => !a.startsWith("--")) ?? "");
const timeoutMs = Number(args.find((a) => a.startsWith("--timeout-ms="))?.slice(13) ?? 90000);
if (!siteDir || !fs.existsSync(path.join(siteDir, "js/client/play.html"))) {
  console.error("usage: node scripts/perf/page-smoke.mjs <site-dir>; the directory must hold js/client/play.html");
  process.exit(2);
}

const host = "pages.oscraft.test";
const network = new RegExp("WebSocket|wss?://|ERR_NAME_NOT_RESOLVED|ERR_INTERNET_DISCONNECTED|ERR_CONNECTION|relay|stun:", "i");
const problems = [];
let heroIconsLoaded = 0;

const server = http.createServer((req, res) => {
  const rel = decodeURIComponent(req.url.split("?")[0]).replace(new RegExp("^/+"), "") || "index.html";
  const file = path.join(siteDir, rel);
  if (!file.startsWith(siteDir + path.sep) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
    res.writeHead(404);
    res.end();
    return;
  }
  res.writeHead(200, { "content-type": contentType(file), "cache-control": "no-store" });
  fs.createReadStream(file).pipe(res);
});

function isPageAsset(url) {
  try {
    const u = new URL(url);
    return u.hostname === host && u.pathname !== "/favicon.ico";
  } catch {
    return false;
  }
}

async function smoke(base) {
  const browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_PATH || undefined,
    args: [
      "--host-resolver-rules=MAP " + host + " 127.0.0.1",
      "--use-gl=angle",
      "--use-angle=swiftshader",
      "--enable-unsafe-swiftshader",
      "--ignore-gpu-blocklist",
    ],
  });
  try {
    const context = await browser.newContext({ serviceWorkers: "block" });
    const page = await context.newPage();
    page.on("pageerror", (err) => problems.push("uncaught error: " + err.message));
    page.on("console", (msg) => {
      const text = msg.text();
      if (msg.type() !== "error" || network.test(text) || text.startsWith("Failed to load resource")) return;
      problems.push("console error at " + (msg.location().url || "?") + ": " + text);
    });
    page.on("response", (res) => {
      const url = res.url();
      if (!isPageAsset(url)) return;
      if (res.status() >= 400) problems.push("HTTP " + res.status() + " " + url);
      else if (url.includes("/icons/heroes/")) heroIconsLoaded += 1;
    });
    page.on("requestfailed", (req) => {
      if (isPageAsset(req.url())) problems.push("request failed " + (req.failure()?.errorText ?? "?") + " " + req.url());
    });
    await page.goto(base + "/js/client/play.html?host=1#moba", { waitUntil: "domcontentloaded" });
    try {
      await page.waitForFunction(() => {
        const lobby = document.getElementById("ui-modal");
        return !!lobby && lobby.children.length > 0;
      }, null, { timeout: timeoutMs });
    } catch {
      problems.push("the lobby did not mount: #ui-modal has no content");
    }
    const deadline = Date.now() + 60000;
    while (heroIconsLoaded === 0 && Date.now() < deadline) await page.waitForTimeout(250);
    if (heroIconsLoaded === 0) problems.push("no hero icon loaded in the lobby");
  } finally {
    await browser.close();
  }
}

await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
try {
  await smoke("http://" + host + ":" + server.address().port);
} catch (err) {
  problems.push("smoke error: " + String(err.message).split("\n")[0]);
} finally {
  server.close();
}

const unique = [...new Set(problems)];
if (unique.length) {
  for (const line of unique.slice(0, 20)) console.error(line);
  console.error("RESULT: FAIL built page smoke, " + unique.length + " problem(s)");
  process.exit(1);
}
console.log("built page smoke ok: lobby mounted, no console errors, hero icons loaded=" + heroIconsLoaded);
