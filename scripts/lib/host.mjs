import fs from "node:fs";
import net from "node:net";
import { spawn, spawnSync } from "node:child_process";
import { sleep } from "./sleep.mjs";

export const freePort = () => new Promise((resolve, reject) => {
  const server = net.createServer();
  server.once("error", reject);
  server.listen(0, "127.0.0.1", () => { const port = server.address().port; server.close(() => resolve(port)); });
});

const groupLive = (pgid) => spawnSync("ps", ["-e", "-o", "pgid=,stat="], { encoding: "utf8" }).stdout.split("\n").map((line) => line.trim().split(/\s+/)).filter(([id, stat]) => Number(id) === pgid && stat && !stat.startsWith("Z")).length;

export function killGroupSync(pgid, timeoutMs = 5000) {
  try { process.kill(-pgid, "SIGKILL"); } catch { return; }
  const end = Date.now() + timeoutMs;
  while (groupLive(pgid) > 0 && Date.now() < end) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 50);
}

export async function waitUntil(fn, timeoutMs = 30000) {
  const end = Date.now() + timeoutMs;
  let last;
  while (Date.now() < end) {
    try { return await fn(); } catch (error) { last = error; await sleep(150); }
  }
  throw last ?? new Error("timeout");
}

export function startGroup({ command, args, env, cwd }) {
  const child = spawn(command, args, { cwd, env, stdio: ["ignore", "pipe", "pipe"], detached: true });
  const cleanup = () => { try { process.kill(-child.pid, "SIGTERM"); } catch {} };
  const group = { child, output: "", cleanup };
  const collect = (chunk) => { group.output += chunk.toString(); };
  child.stdout.on("data", collect);
  child.stderr.on("data", collect);
  process.on("exit", () => killGroupSync(child.pid));
  process.on("SIGINT", () => { cleanup(); process.exit(130); });
  process.on("SIGTERM", () => { cleanup(); process.exit(143); });
  return group;
}

export const removeOnExit = (dir) => process.on("exit", () => fs.rmSync(dir, { recursive: true, force: true }));
