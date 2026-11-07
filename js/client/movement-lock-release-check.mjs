import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const load = (name) => readFileSync(resolve(here, name), "utf8");
const fail = (what) => { throw new Error(what); };
const AUDITED = ["play.js", "modui.js", "debug_api.js"];

const isDefinition = (line) => /^\s*(export function |function )?setCamera\((opts = \{\}|renderer, opts)\)\s*\{/.test(line);
const isHelpText = (line) => /^\s*["']/.test(line);
const callsCamera = (line) => /setCamera\(/.test(line) && !isDefinition(line) && !isHelpText(line);

function writesTarget(lines, i) {
  const window = lines.slice(i, i + 3).join(" ");
  return /target|setCamera\((to|patch|flowCameraRestore\(\)|opts)\)/.test(window);
}

function guarded(lines, i) {
  return lines.slice(Math.max(0, i - 3), i + 1).some((l) => /heroViewLock\.release\(\)|onCameraTake\(\)/.test(l));
}

export function audit(sources) {
  const unclassified = [];
  const open = [];
  let sanctioned = 0;
  for (const name of AUDITED) {
    const lines = sources[name].split("\n");
    lines.forEach((line, i) => {
      if (!callsCamera(line) || !writesTarget(lines, i)) return;
      const where = `${name}:${i + 1}`;
      const callText = lines.slice(i, i + 3).join(" ");
      if (callText.includes("lockedFocus")) { sanctioned += 1; return; }
      if (callText.includes("from.target")) { sanctioned += 1; return; }
      if (callText.includes("orbitOrigin")) { sanctioned += 1; return; }
      if (guarded(lines, i)) { sanctioned += 1; return; }
      if (name === "debug_api.js" || /__ciFocus|oscraft\.setCamera|setCamera: \(opts/.test(line) || /setCamera\(opts\)/.test(line)) { open.push(where); return; }
      unclassified.push(where);
    });
  }
  return { unclassified, open, sanctioned };
}

export function staticViolations(sources) {
  const out = [];
  const modui = sources["modui.js"];
  const play = sources["play.js"];
  if (!modui.includes("onCameraTake = () => {}")) out.push("modui.js: onCameraTake option missing or no default");
  if (modui.split("onCameraTake();").length - 1 !== 2) out.push("modui.js: onCameraTake() must be called at the minimap drag and the camera drag branch");
  if (!play.includes("onCameraTake: () => heroViewLock.release(),")) out.push("play.js: createModUi does not pass onCameraTake releasing the hero lock");
  if (play.split("heroViewLock.release();\n      renderer3d.setCamera(flowCameraRestore());").length - 1 !== 2) out.push("play.js: both flowCameraRestore writes must release the lock first");
  if (!play.includes("heroViewLock.release();\n      renderer3d.setCamera({ target: cam.target")) out.push("play.js: podium camera write must release the lock first");
  return out;
}

const sources = Object.fromEntries(["play.js", "modui.js", "debug_api.js"].map((name) => [name, load(name)]));
const positive = audit(sources);
const positiveStatic = staticViolations(sources);
if (positive.unclassified.length > 0) fail(`unguarded camera target writes: ${positive.unclassified.join(", ")}`);
if (positiveStatic.length > 0) fail(positiveStatic.join("; "));

const lineHits = ([name, pattern]) => sources[name].split("\n").flatMap((line, i) => (pattern.test(line) ? [`${name}:${i + 1}`] : []));
const DEBUG_WRITERS = [
  ["play.js", /window\.__ciFocus = .*renderer3d\.setCamera\(/],
  ["play.js", /setCamera: \(opts = \{\}\) => \(renderer3d \? renderer3d\.setCamera\(opts\)/],
  ["play.js", /return renderer3d \? renderer3d\.setCamera\(\{ target: \[x, groundAt\(x, z\), z\] \}\)/],
  ["debug_api.js", /^\s*r\.setCamera\(patch\);$/],
  ["debug_api.js", /^\s*r\.setCamera\(\{ target: \[u\.x, 0, u\.z\] \}\);$/],
];
const PRODUCTION_WRITERS = [
  ["play.js", /renderer3d\.setCamera\(\{ target: cam\.target/],
  ["play.js", /renderer3d\.setCamera\(flowCameraRestore\(\)\);/],
  ["modui.js", /^\s*onCameraTake\(\);$/],
  ["modui.js", /if \(dragging\) \{ move = null; onCameraTake\(\);/],
];
const debugHits = DEBUG_WRITERS.map(lineHits);
debugHits.forEach((hits, i) => { if (hits.length !== 1) fail(`debug writer ${i} matches ${hits.length} lines, want 1: ${hits.join(", ")}`); });
const productionHits = PRODUCTION_WRITERS.flatMap(lineHits);
if (productionHits.length !== 5) fail(`production lock-release sites: ${productionHits.length}, want 5: ${productionHits.join(", ")}`);
const leaked = debugHits.flat().filter((hit) => productionHits.includes(hit));
if (leaked.length > 0) fail(`debug writers inside the five production sites: ${leaked.join(", ")}`);
const debugClaim = debugHits.flat().join(", ");

const mutate = (name, from, to) => ({ ...sources, [name]: sources[name].replace(from, to) });
const controls = {
  minimapDragNoRelease: mutate("modui.js", "    onCameraTake();\n    r.setCamera({ target: [e.detail.x", "    r.setCamera({ target: [e.detail.x"),
  cameraDragNoRelease: mutate("modui.js", "if (dragging) { move = null; onCameraTake(); r.setCamera(to); return; }", "if (dragging) { move = null; r.setCamera(to); return; }"),
  flowRestoreNoRelease: { ...sources, "play.js": sources["play.js"].split("heroViewLock.release();\n      renderer3d.setCamera(flowCameraRestore());").join("renderer3d.setCamera(flowCameraRestore());") },
  podiumNoRelease: mutate("play.js", "      heroViewLock.release();\n      renderer3d.setCamera({ target: cam.target", "      renderer3d.setCamera({ target: cam.target"),
  modUiNotWired: mutate("play.js", "    onCameraTake: () => heroViewLock.release(),\n", ""),
};
const controlReport = {};
for (const [label, mutated] of Object.entries(controls)) {
  const found = [...audit(mutated).unclassified, ...staticViolations(mutated)];
  if (found.length === 0) fail(`negative control ${label} did not fail the gate`);
  controlReport[label] = found[0];
}

console.log(`lock release ok: sanctioned writes ${positive.sanctioned}, open (not covered by the rule, reported) ${positive.open.join(", ") || "none"}`);
for (const [label, first] of Object.entries(controlReport)) console.log(`negative control ${label}: fails with "${first}"`);
console.log(`debug hooks outside the five production sites: ${debugClaim}`);
console.log(`movement lock release gate ok (5 explicit sites release the lock; ${Object.keys(controls).length} negative controls fail)`);
