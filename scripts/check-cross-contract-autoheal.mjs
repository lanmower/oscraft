#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { comparableSha256, contractPaths } from "./cross-contract-inputs.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const paths = contractPaths(root);
const fail = (message) => { console.error(`check:cross-autoheal: ${message}`); process.exit(1); };
const sha = (file) => fs.existsSync(file) ? crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex") : "absent";
const run = (argv, label) => {
  const result = spawnSync(argv[0], argv.slice(1), { cwd: root, encoding: "utf8", maxBuffer: 8 * 1024 * 1024 });
  const text = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim();
  console.log(`--- ${label} exit=${result.status ?? 1} ---`);
  if (text) console.log(text);
  return { ...result, text };
};
const peerRoots = [paths.botaRoot, paths.hy3dRoot, paths.wscheppityRoot, paths.extraRoot];
const statusOf = (dir) => {
  const result = spawnSync("git", ["-C", dir, "status", "--short"], { cwd: root, encoding: "utf8", maxBuffer: 4 * 1024 * 1024 });
  return { dir, exit: result.status ?? 1, text: `${result.stdout ?? ""}${result.stderr ?? ""}`.trim() };
};
const beforeStatus = peerRoots.map(statusOf);
const beforeSnapshot = sha(paths.snapshot);
const autoheal = run([process.execPath, path.join(root, "scripts", "dev", "cross-contract-autoheal.mjs"), "--once"], "cross:autoheal --once");
if (autoheal.status !== 0) fail("autoheal --once did not exit 0");
const afterStatus = peerRoots.map(statusOf);
for (let i = 0; i < peerRoots.length; i += 1) {
  const before = beforeStatus[i], after = afterStatus[i];
  console.log(`peer status ${path.basename(before.dir)} exit=${after.exit} unchanged=${before.text === after.text}`);
  if (before.text !== after.text) fail(`peer status changed during autoheal: ${before.dir}`);
  if (before.exit !== after.exit) fail(`peer status command exit changed: ${before.dir} ${before.exit}->${after.exit}`);
}
const checker = run([process.execPath, path.join(root, "scripts", "check-cross-project-contracts.mjs")], "check:cross-contracts");
if (checker.status !== 0) fail("check:cross-contracts did not exit 0 after autoheal");
if (/DRIFT|STALE/.test(checker.text) && !/DRIFT\/STALE=0/.test(checker.text)) fail("unresolved DRIFT/STALE text remains after autoheal");
const refreshCheck = run([process.execPath, path.join(root, "scripts", "dev", "refresh-cross-contracts.mjs"), "--check"], "refresh --check");
if (refreshCheck.status !== 0) fail("refresh --check did not exit 0");
const saved = JSON.parse(fs.readFileSync(paths.snapshot, "utf8"));
const comparable = comparableSha256(saved);
const afterSnapshot = sha(paths.snapshot);
if (afterSnapshot !== beforeSnapshot && !/healed:/.test(autoheal.text)) fail("snapshot changed without a reported heal");
const secondRefresh = run([process.execPath, path.join(root, "scripts", "dev", "refresh-cross-contracts.mjs")], "determinism refresh");
if (secondRefresh.status !== 0) fail("determinism refresh did not exit 0");
if (sha(paths.snapshot) !== afterSnapshot) fail("snapshot bytes changed on an unchanged second refresh");
console.log(`check:cross-autoheal PASS: before=${beforeSnapshot} after=${afterSnapshot} comparableSha256=${comparable}`);
console.log(`check:cross-autoheal PASS: peer git status unchanged for bota, hy3d, wscheppity and extra; extra non-repo status is annotated`);
const healed = [...autoheal.text.matchAll(/healed STALE (\S+)/g)].map((m) => m[1]);
console.log(`stalePathsHealed=${healed.length}${healed.length ? ` ${healed.join(",")}` : ""}`);
for (const status of afterStatus) console.log(`read-only peer status ${path.basename(status.dir)} exit=${status.exit} evidenceLines=${status.text ? status.text.split("\n").length : 0}`);
