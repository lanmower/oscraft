#!/usr/bin/env node

import { chromium } from "playwright";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { contentType } from "../dev/mime.mjs";

const args = process.argv.slice(2);
const siteDir = path.resolve(args.find((a) => !a.startsWith("--")) ?? "");
const timeoutMs = Number(args.find((a) => a.startsWith("--timeout-ms="))?.slice(13) ?? 90000);
const lobbyIdleMs = Number(args.find((a) => a.startsWith("--lobby-idle-s="))?.slice(15) ?? 30) * 1000;
if (!siteDir || !fs.existsSync(path.join(siteDir, "js/client/play.html"))) {
  console.error("usage: node scripts/perf/page-smoke.mjs <site-dir>; the directory must hold js/client/play.html");
  process.exit(2);
}

const host = "pages.oscraft.test";
const network = new RegExp("WebSocket|wss?://|ERR_NAME_NOT_RESOLVED|ERR_INTERNET_DISCONNECTED|ERR_CONNECTION|relay|stun:", "i");
const problems = [];
const ROSTER_HERO_ICONS = 127;
let heroIconsLoaded = 0;
let heroIconsPacked = 0;

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

const menuSnapshot = () => ({
  tick: window.oscraft.getState().tick,
  units: window.oscraft.getState().units.length,
  phase: document.body.dataset.phase,
  frames: window.__ciFrames.length,
  sent: window.__ciSent.size,
  drawSplat: document.body.dataset.drawSplat ?? "0",
  drawProp: document.body.dataset.drawProp ?? "0",
  drawUnit: document.body.dataset.drawUnit ?? "0",
});

async function assertMenuIdle(page) {
  const before = await page.evaluate(menuSnapshot);
  await page.waitForTimeout(lobbyIdleMs);
  const after = await page.evaluate(menuSnapshot);
  const idle = Math.round(lobbyIdleMs / 1000);
  if (before.phase !== "lobby" || after.phase !== "lobby") problems.push("the page is not in the lobby phase: " + before.phase + " then " + after.phase);
  if (after.tick !== 0) problems.push("sim tick is " + after.tick + " after " + idle + " s in the lobby, expected 0");
  if (after.units !== before.units) problems.push("unit count moved from " + before.units + " to " + after.units + " in the lobby");
  if (after.frames !== 0 || after.sent !== 0) problems.push("a STATE frame was sent in the lobby (frames " + after.frames + ", ticks logged " + after.sent + ")");
  if (after.drawSplat !== "0" || after.drawProp !== "0" || after.drawUnit !== "0") problems.push("the map was drawn in the lobby: splat " + after.drawSplat + " prop " + after.drawProp + " unit " + after.drawUnit);
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
      else if (url.includes("/icons/heroes/")) problems.push("hero icon fetched on its own, not from the hero icon pack: " + url);
      else if (url.includes("/packs/heroes.") && url.includes(".webp")) heroIconsLoaded += 1;
    });
    page.on("requestfailed", (req) => {
      if (isPageAsset(req.url())) problems.push("request failed " + (req.failure()?.errorText ?? "?") + " " + req.url());
    });
    await page.goto(base + "/js/client/play.html?host=1&ci=1#moba", { waitUntil: "domcontentloaded" });
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
    heroIconsPacked = Object.keys(JSON.parse(fs.readFileSync(path.join(siteDir, "scenarios/moba/assets/packs/index.json"), "utf8")).atlases.heroes.icons).length;
    if (heroIconsPacked !== ROSTER_HERO_ICONS) problems.push("the heroes atlas indexes " + heroIconsPacked + " icons, expected " + ROSTER_HERO_ICONS + " (the roster)");
    await assertMenuIdle(page);
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
console.log("built page smoke ok: lobby mounted, sim idle for " + Math.round(lobbyIdleMs / 1000) + " s (tick 0, no STATE, no map draw), no console errors, heroes atlas requests=" + heroIconsLoaded + ", icons indexed=" + heroIconsPacked);
