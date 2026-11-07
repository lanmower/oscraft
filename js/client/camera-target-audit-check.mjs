import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const fail = (what) => { throw new Error(what); };
const SOURCES = readdirSync(here).filter((n) => /\.(js|mjs)$/.test(n) && !/-check\.mjs$/.test(n));
const load = (name) => readFileSync(resolve(here, name), "utf8");

const CLASSIFIED = [
  { file: "play.js", label: "production: podium camera, releases the lock first", re: /renderer3d\.setCamera\(\{ target: cam\.target/ },
  { file: "play.js", label: "production: flow camera restore, releases the lock first", re: /renderer3d\.setCamera\(flowCameraRestore\(\)\);/ },
  { file: "modui.js", label: "production: minimap drag, releases the lock first", re: /^\s*r\.setCamera\(\{ target: \[e\.detail\.x/ },
  { file: "modui.js", label: "production: mod camera drag, releases the lock first", re: /if \(dragging\) \{ move = null; onCameraTake\(\); r\.setCamera\(to\)/ },
  { file: "play.js", label: "locked path: driveLockedHero hero-follow target", re: /renderer3d\.setCamera\(\{ target: lockedFocus \}\)/ },
  { file: "modui.js", label: "glide step, dropped while locked (step locked argument)", re: /^\s*r\.setCamera\(\{$/ },
  { file: "play.js", label: "boot: world camera before the lock can engage", re: /renderer3d\.setCamera\(\{ target: orbitOrigin/ },
  { file: "play.js", label: "zoom: distance only", re: /set: \(distance\) => renderer3d\?\.setCamera\(\{ distance:/ },
  { file: "modui.js", label: "camera lock: yawLocked only", re: /setCamera\(\{ yawLocked:/ },
  { file: "zoom_keys.js", label: "zoom keys: distance or yawLocked only", re: /r\.setCamera\(\{ (distance|yawLocked):/ },
  { file: "render3d.js", label: "setCamera internal target clamp", re: /cam\.target\[[02]\] = Math\.min/ },
  { file: "render3d.js", label: "setCamera wrapper export", re: /return renderer\.setCamera\(opts\);/ },
  { file: "render3d.js", label: "model preview: its own renderer, not the game camera", re: /^\s*renderer\.setCamera\(\{$/ },
  { file: "render3d.js", label: "figure scene: its own renderer per canvas", re: /renderer\.setCamera\(\{ target: cam\.target, yaw: 0, pitch: SCENE_VIEW_PITCH/ },
  { file: "editor.js", label: "editor page: its own renderer, separate HTML", re: /renderer\.setCamera\(\{ \.\.\.cam, aspect/ },
  { file: "play.js", label: "debug: window.__ciFocus", re: /window\.__ciFocus = / },
  { file: "play.js", label: "debug: oscraft.setCamera", re: /setCamera: \(opts = \{\}\) => \(renderer3d \? renderer3d\.setCamera\(opts\)/ },
  { file: "play.js", label: "debug: oscraft.centreOn", re: /return renderer3d \? renderer3d\.setCamera\(\{ target: \[x, groundAt/ },
  { file: "debug_api.js", label: "debug: oscraft.setCamera patch", re: /^\s*r\.setCamera\(patch\);$/ },
  { file: "debug_api.js", label: "debug: oscraft centre on unit", re: /^\s*r\.setCamera\(\{ target: \[u\.x, 0, u\.z\] \}\);$/ },
];

const SKIPPED = [
  /^\s*(export\s+)?(function\s+)?setCamera\((opts|renderer)/,
  /^\s*"/,
];

const WRITE = /setCamera\(|\.target\[[02]\] =/;

export function audit(sources) {
  const unclassified = [];
  const hits = new Map(CLASSIFIED.map((c) => [c, 0]));
  let writes = 0;
  for (const name of Object.keys(sources)) {
    sources[name].split("\n").forEach((line, i) => {
      if (!WRITE.test(line) || SKIPPED.some((re) => re.test(line))) return;
      writes += 1;
      const match = CLASSIFIED.find((c) => c.file === name && c.re.test(line));
      if (match) hits.set(match, hits.get(match) + 1);
      else unclassified.push(`${name}:${i + 1}`);
    });
  }
  const stale = CLASSIFIED.filter((c) => hits.get(c) === 0 && sources[c.file] !== undefined).map((c) => `${c.file} ${c.label}`);
  return { unclassified, stale, writes, hits };
}

const sources = Object.fromEntries(SOURCES.map((name) => [name, load(name)]));
const positive = audit(sources);
if (positive.unclassified.length > 0) fail(`unclassified camera-target writes: ${positive.unclassified.join(", ")}`);
if (positive.stale.length > 0) fail(`classified entries with no matching line: ${positive.stale.join("; ")}`);

const injected = { ...sources, "play.js": `${sources["play.js"]}\nrenderer3d.setCamera({ target: [1, 0, 1] });\n` };
const control = audit(injected);
if (control.unclassified.length === 0) fail("negative control: a newly introduced unclassified write did not fail the gate");

const production = CLASSIFIED.filter((c) => c.label.startsWith("production")).reduce((n, c) => n + positive.hits.get(c), 0);
const debug = CLASSIFIED.filter((c) => c.label.startsWith("debug")).reduce((n, c) => n + positive.hits.get(c), 0);
console.log(`camera target audit ok: ${positive.writes} writes in ${SOURCES.length} files, all classified (production ${production}, debug ${debug}, other ${positive.writes - production - debug}); ${CLASSIFIED.length} classified entries, none stale`);
console.log(`negative control: an injected play.js write fails with "${control.unclassified[0]}"`);
