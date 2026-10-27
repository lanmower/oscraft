#!/usr/bin/env node
import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const url = process.argv[2] || "https://lanmower.github.io/oscraft/js/client/play.html";
const seconds = Number(process.argv[3] ?? 20);
const chromium = process.env.CHROMIUM_PATH ?? "/usr/bin/chromium";
const ports = [9333, 9334];
const root = fs.mkdtempSync(path.join(os.tmpdir(), "oscraft-peers-"));

const launched = ports.map((port, i) => {
  const profile = path.join(root, `peer${i}`);
  const child = spawn(chromium, [
    `--user-data-dir=${profile}`, `--remote-debugging-port=${port}`, "--remote-debugging-address=127.0.0.1",
    "--no-first-run", "--no-default-browser-check", "--new-window", url,
  ], { stdio: "ignore", detached: true });
  child.unref();
  return { port, child, profile };
});

async function pageTarget(port) {
  for (let t = 0; t < 40; t++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      const page = list.find((x) => x.type === "page" && x.url.startsWith("https://lanmower.github.io/oscraft"));
      if (page) return page;
    } catch {}
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`no game page on port ${port}`);
}

function attach(target, label) {
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  let id = 0;
  const pending = new Map();
  const logs = [];
  const send = (method, params = {}) => new Promise((resolve) => {
    const n = ++id;
    pending.set(n, resolve);
    ws.send(JSON.stringify({ id: n, method, params }));
  });
  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg.result); pending.delete(msg.id); return; }
    if (msg.method === "Runtime.consoleAPICalled") {
      logs.push(`[${msg.params.type}] ${msg.params.args.map((a) => a.value ?? a.description ?? "").join(" ")}`);
    }
    if (msg.method === "Runtime.exceptionThrown") {
      logs.push(`[exception] ${msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text}`);
    }
  };
  return new Promise((resolve) => ws.onopen = async () => {
    await send("Runtime.enable");
    resolve({ label, send, logs, close: () => ws.close() });
  });
}

const sessions = [];
for (const { port } of launched) {
  const target = await pageTarget(port);
  sessions.push(await attach(target, `peer@${port}`));
}

await new Promise((r) => setTimeout(r, seconds * 1000));

for (const s of sessions) {
  const { result } = await s.send("Runtime.evaluate", {
    expression: `JSON.stringify({ status: document.getElementById("status")?.textContent, hud: document.getElementById("hud")?.textContent, url: location.href })`,
    returnByValue: true,
  });
  console.log(`== ${s.label}`);
  console.log(result?.value ?? "(no state)");
  console.log(s.logs.slice(-40).join("\n"));
  s.close();
}
for (const { child } of launched) { try { process.kill(-child.pid); } catch {} }
