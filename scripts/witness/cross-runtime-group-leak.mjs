import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { freePort, killGroupSync, removeOnExit, startGroup } from "../lib/host.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const self = fileURLToPath(import.meta.url);
const serve = process.env.GROUP_LEAK_SERVE ?? path.join(root, "scripts", "dev", "serve-bota.sh");
const integratedScript = path.join(root, "scripts", "serve-integrated.sh");
const checkScript = path.join(root, "scripts", "check-integrated-cold-start.mjs");
const redCheckScript = path.join(root, "scripts", ".red-integrated-check.mjs");
const scratch = path.join(root, "..", "tmp", "oscraft", "groupred");
const sitePath = (pid) => path.join(root, `.dev-site-group-leak-${pid}`);
const MODES = ["current", "prefix", "leader", "unremoved", "integrated", "integrated-current", "check", "check-leader", "check-stage", "check-stage-leader"];

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
  if (mode === "integrated" || mode === "integrated-current") {
    fs.mkdirSync(scratch, { recursive: true });
    const pidFile = path.join(scratch, `${process.pid}.pid`);
    const integratedEnv = { ...process.env, INTEGRATED_PORT: String(port), INTEGRATED_SITE: site, INTEGRATED_PID_FILE: pidFile, INTEGRATED_LOG_FILE: path.join(scratch, `${process.pid}.log`), INTEGRATED_PUBLIC_BASE: `http://127.0.0.1:${port}` };
    const wrapper = spawn("bash", [integratedScript, `--port=${port}`, `--site=${site}`, `--pid-file=${pidFile}`], { cwd: root, env: integratedEnv, stdio: "ignore", detached: true });
    if (mode === "integrated") process.on("exit", () => { try { wrapper.kill("SIGTERM"); } catch {} });
    else process.on("exit", () => killGroupSync(wrapper.pid));
    process.on("SIGTERM", () => process.exit(143));
  }
  if (mode !== "unremoved") removeOnExit(site);
  setInterval(() => {}, 1000);
}

function psLines() {
  return spawnSync("ps", ["-eo", "pid=,args="], { encoding: "utf8" }).stdout.split("\n").filter(Boolean);
}

function processesOnPort(port) {
  const needle = new RegExp(`--port=${port}(\\s|$)`);
  return psLines().filter((line) => needle.test(line)).map((line) => Number(line.trim().split(/\s+/)[0]));
}

function processesNamed(name) {
  const needle = new RegExp(`${name.replace(/[.]/g, "\\.")}(\\D|$)`);
  return psLines().filter((line) => needle.test(line)).map((line) => Number(line.trim().split(/\s+/)[0]));
}

async function serving(port) {
  try { return (await fetch(`http://127.0.0.1:${port}/js/client/play.html`)).ok; } catch { return false; }
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function report(mode, leaks) {
  const verdict = leaks.length ? `LEAKED ${leaks.join(" ")}` : "CLEAN";
  console.log(`cross-runtime-group mode=${mode} ${verdict}`);
  process.exit(leaks.length ? 1 : 0);
}

async function driveCheck(mode) {
  const script = mode.endsWith("-leader") ? redCheckScript : checkScript;
  const stage = mode.includes("stage");
  const child = spawn(process.execPath, [script], { cwd: root, stdio: "ignore" });
  const exited = new Promise((resolve) => child.on("exit", resolve));
  const tag = `.dev-site-integrated-check-${child.pid}`;
  const names = [tag, `${tag}.pid`, `${tag}.log`];
  const siteDir = path.join(root, tag);
  const pidFile = path.join(root, `${tag}.pid`);
  const isServing = () => psLines().some((line) => line.includes(tag) && line.includes("serve.mjs"));
  const ready = () => (stage ? fs.existsSync(siteDir) && !fs.existsSync(pidFile) : fs.existsSync(siteDir) && isServing());
  const deadline = Date.now() + 1200000;
  while (child.exitCode === null && Date.now() < deadline && !ready()) await sleep(200);
  if (child.exitCode !== null || !ready()) {
    child.kill("SIGKILL");
    await exited;
    console.log(`cross-runtime-group mode=${mode} NOT_READY`);
    process.exit(3);
  }
  child.kill("SIGTERM");
  await exited;
  await sleep(3000);
  const survivors = processesNamed(tag);
  for (const pid of survivors) { try { process.kill(pid, "SIGKILL"); } catch {} }
  const left = names.filter((name) => fs.existsSync(path.join(root, name)));
  for (const name of left) fs.rmSync(path.join(root, name), { recursive: true, force: true });
  const leaks = [...left.map((name) => `file(${name})`)];
  if (survivors.length) leaks.push(`process(${survivors.join(",")})`);
  report(mode, leaks);
}

async function drive(mode) {
  if (!MODES.includes(mode)) throw new Error(`mode must be one of ${MODES.join(", ")}`);
  if (mode.startsWith("check")) return driveCheck(mode);
  const port = await freePort();
  const child = spawn(process.execPath, [self, "--owner", mode, String(port)], { cwd: root, stdio: "ignore" });
  const exited = new Promise((resolve) => child.on("exit", resolve));
  const deadline = Date.now() + 900000;
  while (!fs.existsSync(sitePath(child.pid)) && child.exitCode === null && Date.now() < deadline) await sleep(200);
  if (!fs.existsSync(sitePath(child.pid))) {
    child.kill("SIGKILL");
    await exited;
    console.log(`cross-runtime-group mode=${mode} NOT_READY`);
    process.exit(3);
  }
  child.kill("SIGTERM");
  await exited;
  await sleep(3000);
  const siteLeft = fs.existsSync(sitePath(child.pid));
  const survivors = processesOnPort(port);
  for (const pid of survivors) { try { process.kill(pid, "SIGKILL"); } catch {} }
  if (siteLeft) fs.rmSync(sitePath(child.pid), { recursive: true, force: true });
  const leaks = [];
  if (siteLeft) leaks.push("site");
  if (survivors.length) leaks.push(`process(${survivors.join(",")})`);
  report(mode, leaks);
}

const args = process.argv.slice(2);
if (args[0] === "--owner") owner(args[1], Number(args[2]));
else await drive(args[0] ?? "current");
