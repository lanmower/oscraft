#!/usr/bin/env node
import { spawn } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { contentType } from "../dev/mime.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const [siteArg, outArg] = process.argv.slice(2);
if (!siteArg || !outArg) {
  console.error("usage: capture-duel-frames.mjs <site-dir> <out-dir>");
  process.exit(2);
}
const site = path.resolve(siteArg);
const out = path.resolve(outArg);
const replayText = fs.readFileSync(path.join(root, "scenarios", "duel-routes", "replays", "duel-routes.replay"), "utf8");
const ticks = fs.readFileSync(path.join(root, "scenarios", "duel-routes", "replays", "duel-routes.hashes"), "utf8")
  .split("\n").filter(Boolean).map((line) => Number(line.split(" ")[0])).filter((tick) => tick % 64 === 0);
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const WORLD_WAIT_MS = Number(process.env.CAPTURE_WORLD_WAIT_MS ?? 240000);

function serve() {
  const server = http.createServer((req, res) => {
    const rel = decodeURIComponent((req.url ?? "/").split("?")[0]).replace(/^\/+/, "") || "index.html";
    const file = path.join(site, rel);
    if (file !== site && !file.startsWith(site + path.sep)) {
      res.writeHead(403).end();
      return;
    }
    fs.readFile(file, (err, body) => {
      if (err) {
        res.writeHead(404).end();
        return;
      }
      res.writeHead(200, { "content-type": contentType(file), "cache-control": "no-store" });
      res.end(body);
    });
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

async function connect(port, events) {
  let pages = [];
  for (let i = 0; i < 80 && pages.length === 0; i += 1) {
    try {
      pages = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).filter((t) => t.type === "page");
    } catch {
      await sleep(250);
    }
  }
  if (pages.length === 0) throw new Error("no chromium page target");
  const ws = new WebSocket(pages[0].webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = () => reject(new Error("CDP socket failed"));
  });
  let nextId = 0;
  const pending = new Map();
  ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    if (msg.method === "Runtime.exceptionThrown") {
      const d = msg.params.exceptionDetails;
      events.push(`exception ${d.exception?.description ?? d.text}`);
      return;
    }
    if (msg.method === "Runtime.consoleAPICalled" && ["error", "warning", "assert"].includes(msg.params.type)) {
      events.push(`console.${msg.params.type} ${msg.params.args.map((a) => a.value ?? a.description ?? "").join(" ")}`.slice(0, 400));
      return;
    }
    const waiter = pending.get(msg.id);
    if (!waiter) return;
    pending.delete(msg.id);
    if (msg.error) waiter.reject(new Error(msg.error.message));
    else waiter.resolve(msg.result);
  };
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++nextId;
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });
  const evaluate = async (expression) => {
    const r = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (r.exceptionDetails) {
      const ex = r.exceptionDetails.exception;
      const detail = ex?.description ?? (ex?.value !== undefined ? String(ex.value) : r.exceptionDetails.text);
      throw new Error(`${detail} (${expression.slice(0, 80)})`);
    }
    return r.result.value;
  };
  return { send, evaluate, close: () => ws.close() };
}

async function waitFor(label, probe, ms) {
  const until = Date.now() + ms;
  for (;;) {
    if (await probe()) return;
    if (Date.now() > until) throw new Error(`timed out waiting for ${label}`);
    await sleep(1000);
  }
}

const events = [];
const server = await serve();
const port = 9500 + Math.floor(Math.random() * 1000);
const profile = fs.mkdtempSync(path.join(os.tmpdir(), "oscraft-capture-"));
const chromium = process.env.CHROMIUM_PATH ?? "/usr/bin/chromium";
const child = spawn(chromium, [
  `--user-data-dir=${profile}`, `--remote-debugging-port=${port}`, "--remote-debugging-address=127.0.0.1",
  "--no-first-run", "--no-default-browser-check", "--no-sandbox", "--disable-dev-shm-usage",
  "--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist",
  "about:blank",
], { stdio: "ignore", detached: true });
child.unref();

let exitCode = 1;
let cdp = null;
try {
  cdp = await connect(port, events);
  await cdp.send("Page.enable");
  await cdp.send("Runtime.enable");
  await cdp.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 720, deviceScaleFactor: 1, mobile: false });
  const { port: sitePort } = server.address();
  await cdp.send("Page.navigate", { url: `http://127.0.0.1:${sitePort}/js/client/play.html?ci=1&capture=1&pack=scenarios/duel-routes&nomods=1` });
  await waitFor("the duel world", () => cdp.evaluate("typeof window.__ciWorldReady === 'function' && window.__ciWorldReady() === true").catch(() => false), WORLD_WAIT_MS);
  const loaded = await cdp.evaluate(`window.__ciCaptureLoad(${JSON.stringify(replayText)})`);
  fs.mkdirSync(out, { recursive: true });
  const sums = [];
  for (const tick of ticks) {
    const dataUrl = await cdp.evaluate(`window.__ciCaptureFrame(${tick})`);
    const bytes = Buffer.from(String(dataUrl).replace(/^data:image\/png;base64,/, ""), "base64");
    const name = `tick-${String(tick).padStart(4, "0")}.png`;
    fs.writeFileSync(path.join(out, name), bytes);
    sums.push(`${sha256(bytes)}  ${name}`);
  }
  fs.writeFileSync(path.join(out, "SHA256SUMS"), sums.join("\n") + "\n");
  fs.writeFileSync(path.join(out, "ticks.txt"), ticks.join("\n") + "\n");
  console.log(`CAPTURE_OK frames=${ticks.length} replayLoadedAtTick=${loaded} out=${out}`);
  exitCode = 0;
} catch (err) {
  console.error(`capture-duel-frames: ${err.message}`);
  if (cdp) {
    const state = await cdp.evaluate("JSON.stringify({ hook: typeof window.__ciWorldReady, capture: typeof window.__ciCaptureState === 'function' ? window.__ciCaptureState() : null, body: document.body.dataset.phase ?? null, text: document.body.innerText.slice(0, 300) })").catch((e) => `state unavailable: ${e.message}`);
    console.error(`page state: ${state}`);
  }
  for (const e of events.slice(-25)) console.error(`page event: ${e}`);
} finally {
  try { if (cdp) cdp.close(); } catch {}
  try { process.kill(-child.pid, "SIGKILL"); } catch {}
  server.close();
  fs.rmSync(profile, { recursive: true, force: true });
}
process.exit(exitCode);
