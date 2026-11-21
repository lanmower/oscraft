import net from "node:net";
import { spawn } from "node:child_process";
import { sleep } from "./sleep.mjs";

export const freePort = () => new Promise((resolve, reject) => {
  const server = net.createServer();
  server.once("error", reject);
  server.listen(0, "127.0.0.1", () => { const port = server.address().port; server.close(() => resolve(port)); });
});

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
  process.on("exit", cleanup);
  process.on("SIGINT", () => { cleanup(); process.exit(130); });
  process.on("SIGTERM", () => { cleanup(); process.exit(143); });
  return group;
}
