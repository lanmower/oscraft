import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { freePort, removeOnExit, startGroup } from "../lib/host.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const self = fileURLToPath(import.meta.url);
const serve = process.env.GROUP_LEAK_SERVE ?? path.join(root, "scripts", "dev", "serve-bota.sh");
const integratedScript = path.join(root, "scripts", "serve-integrated.sh");
const scratch = path.join(root, "..", "tmp", "oscraft", "groupred");
const sitePath = (pid) => path.join(root, `.dev-site-group-leak-${pid}`);
const MODES = ["current", "prefix", "leader", "integrated", "unremoved"];

function owner(mode, port) {
  const site = sitePath(process.pid);
  const env = { ...process.env, BOTA_PORT: String(port), BOTA_SITE: site, WASM: "0" };
  const args = [`--port=${port}`];
  if (mode === "current" || mode === "unremoved") startGroup({ command: serve, args, env, cwd: root });
  if (mode === "prefix") {
    const group = spawn(serve, args, { cwd: root, env, stdio: "ignore", detached: true });
    process.on("exit", () => { try { process.kill(-group.pid, "SIGKILL"); } catch {} });
    process.on("SIGTERM", () => process.exit(143));
  }
  if (mode === "leader") {
    const leader = spawn(serve, args, { cwd: root, env, stdio: "ignore", detached: true });
    process.on("exit", () => { try { leader.kill("SIGTERM"); } catch {} });
    process.on("SIGTERM", () => process.exit(143));
  }
  if (mode === "integrated") {
    fs.mkdirSync(scratch, { recursive: true });
    const integratedEnv = { ...process.env, INTEGRATED_PORT: String(port), INTEGRATED_SITE: site, INTEGRATED_PID_FILE: path.join(scratch, `${process.pid}.pid`), INTEGRATED_LOG_FILE: path.join(scratch, `${process.pid}.log`), INTEGRATED_PUBLIC_BASE: `http://127.0.0.1:${port}` };
    const leader = spawn("bash", [integratedScript, `--port=${port}`, `--site=${site}`, `--pid-file=${path.join(scratch, `${process.pid}.pid`)}`], { cwd: root, env: integratedEnv, stdio: "ignore", detached: true });
    process.on("exit", () => { try { leader.kill("SIGTERM"); } catch {} });
    process.on("SIGTERM", () => process.exit(143));
  }
  if (mode !== "unremoved") removeOnExit(site);
  setInterval(() => {}, 1000);
}

async function serving(port) {
  try { return (await fetch(`http://127.0.0.1:${port}/js/client/play.html`)).ok; } catch { return false; }
}

function portOwners(port) {
  const out = spawnSync("ss", ["-H", "-ltnp", `sport = :${port}`], { encoding: "utf8" }).stdout;
  return [...out.matchAll(/pid=(\d+)/g)].map((m) => Number(m[1]));
}

async function drive(mode) {
  if (!MODES.includes(mode)) throw new Error(`mode must be one of ${MODES.join(", ")}`);
  const port = await freePort();
  const child = spawn(process.execPath, [self, "--owner", mode, String(port)], { cwd: root, stdio: "ignore" });
  const exited = new Promise((resolve) => child.on("exit", resolve));
  const deadline = Date.now() + 600000;
  let up = false;
  while (!up && child.exitCode === null && Date.now() < deadline) {
    up = await serving(port);
    if (!up) await new Promise((r) => setTimeout(r, 500));
  }
  if (!up) {
    child.kill("SIGKILL");
    await exited;
    console.log(`cross-runtime-group mode=${mode} NOT_SERVING`);
    process.exit(3);
  }
  child.kill("SIGTERM");
  await exited;
  await new Promise((r) => setTimeout(r, 3000));
  const siteLeft = fs.existsSync(sitePath(child.pid));
  const survivors = portOwners(port);
  const serverLeft = survivors.length > 0 || (await serving(port));
  for (const pid of survivors) process.kill(pid, "SIGKILL");
  if (siteLeft) fs.rmSync(sitePath(child.pid), { recursive: true, force: true });
  const verdict = siteLeft ? "LEAKED site" : serverLeft ? "LEAKED server" : "CLEAN";
  console.log(`cross-runtime-group mode=${mode} ${verdict}`);
  process.exit(verdict === "CLEAN" ? 0 : 1);
}

const args = process.argv.slice(2);
if (args[0] === "--owner") owner(args[1], Number(args[2]));
else await drive(args[0] ?? "current");
