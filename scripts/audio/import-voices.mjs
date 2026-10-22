#!/usr/bin/env node
// Imports a generated voice pack into the pack catalogue and writes the manifest the page reads.
//
// Usage: node scripts/audio/import-voices.mjs <scenario-dir> [voices-dir] [pipeline-manifest]
//
// The pipeline manifest is written by the voice generator and is the source of truth for which file
// plays for which hero event. Two shapes are read: a hero-keyed map
//   { "<hero>": { "spawn": "path", "attack": "path", "ability": "path", "death": "path", "voice_ref": "id" } }
// and the generator's own wrapper, { "events": [...], "voices": { "<hero>": { ... } } }.
// Paths are resolved against the directory holding the manifest and, failing that, each of its parents
// up to the filesystem root: a generator that writes "voices/out/<hero>/000.wav" next to a manifest one
// directory below that tree is read without being rearranged first.
function resolveFile(base, rel) {
  for (let at = path.resolve(base); ; ) {
    const candidate = path.resolve(at, rel);
    if (fs.existsSync(candidate)) return candidate;
    const parent = path.dirname(at);
    if (parent === at) return null;
    at = parent;
  }
}
//
// Each line is copied into <voices-dir>/<hero>/<file> and becomes one catalogue item of kind "audio",
// carrying its hero and event so the client can find a line without reading the manifest, and its
// voice_ref so a replaced reference can be traced to exactly the heroes that need regenerating.
// A file already present with the same bytes is not copied again.
//
// The runtime manifest written at <voices-dir>/manifest.json maps hero -> event -> path relative to
// <voices-dir>, which is what js/client/audio.js reads.
//
// Existing audio entries are replaced, and entries whose file is gone are dropped, so an import after
// a regeneration leaves no stale rows. Nothing outside <voices-dir> and assets.json is written.
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const scenario = path.resolve(process.argv[2] ?? "scenarios/moba");
const voicesDir = path.resolve(process.argv[3] ?? path.join(scenario, "assets", "voices"));
const manifestArg = process.argv[4] ? path.resolve(process.argv[4]) : path.join(voicesDir, "manifest.json");
const catalogPath = path.join(scenario, "assets", "assets.json");

if (!fs.existsSync(manifestArg)) {
  console.error(`no pipeline manifest at ${manifestArg}; the voice generator writes it`);
  process.exit(2);
}
if (!fs.existsSync(catalogPath)) {
  console.error(`no catalogue at ${catalogPath}`);
  process.exit(2);
}

const sha256 = (buf) => crypto.createHash("sha256").update(buf).digest("hex");
const pipeline = JSON.parse(fs.readFileSync(manifestArg, "utf8"));
const heroes = pipeline.voices && typeof pipeline.voices === "object" && !Array.isArray(pipeline.voices) ? pipeline.voices : pipeline;
const base = path.dirname(manifestArg);
const catalog = JSON.parse(fs.readFileSync(catalogPath, "utf8"));
const items = Array.isArray(catalog.items) ? catalog.items.filter((i) => i && i.kind !== "audio") : [];

fs.mkdirSync(voicesDir, { recursive: true });
const runtime = {};
let added = 0;
let copied = 0;
const wanted = new Set(["manifest.json"]);
const missing = [];
for (const [hero, entry] of Object.entries(heroes)) {
  if (!entry || typeof entry !== "object") continue;
  const events = {};
  for (const [event, rel] of Object.entries(entry)) {
    if (event === "voice_ref" || typeof rel !== "string") continue;
    const source = resolveFile(base, rel);
    if (!source) { missing.push(`${hero}/${event}: ${rel}`); continue; }
    const file = path.basename(rel);
    const target = path.join(voicesDir, hero, file);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    const bytes = fs.readFileSync(source);
    if (!fs.existsSync(target) || sha256(fs.readFileSync(target)) !== sha256(bytes)) {
      fs.writeFileSync(target, bytes);
      copied += 1;
    }
    const packRel = path.posix.join(hero, file);
    wanted.add(path.relative(voicesDir, target).split(path.sep).join("/"));
    events[event] = packRel;
    items.push({
      id: `voice.${hero}.${event}`,
      kind: "audio",
      path: path.posix.join("voices", packRel),
      bytes: bytes.length,
      role: "voice",
      hero,
      event,
      voice_ref: entry.voice_ref ?? null,
      placed_by: "voices/manifest.json",
    });
    added += 1;
  }
  if (Object.keys(events).length) runtime[hero] = { ...events, ...(entry.voice_ref ? { voice_ref: entry.voice_ref } : {}) };
}

fs.writeFileSync(path.join(voicesDir, "manifest.json"), `${JSON.stringify(runtime, null, 2)}\n`);

// A re-import that swaps format (WAV to Opus, say) leaves the old files beside the new ones, where
// nothing names them and the site still ships them. Every file under the voices directory the new
// manifest does not name is dropped, and the directories it empties go with them.
const prune = (dir) => {
  let removed = 0;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      removed += prune(abs);
      if (fs.readdirSync(abs).length === 0) fs.rmdirSync(abs);
      continue;
    }
    const rel = path.relative(voicesDir, abs).split(path.sep).join("/");
    if (wanted.has(rel)) continue;
    fs.unlinkSync(abs);
    removed += 1;
  }
  return removed;
};
const pruned = prune(voicesDir);

catalog.items = items;
catalog.count = items.length;
catalog.total_bytes = items.reduce((n, i) => n + (Number(i?.bytes) || 0), 0);
fs.writeFileSync(catalogPath, `${JSON.stringify(catalog, null, 2)}\n`);

const audioBytes = items.reduce((n, i) => n + (i.kind === "audio" ? Number(i.bytes) || 0 : 0), 0);
console.log(
  `imported ${added} voice lines for ${Object.keys(runtime).length} heroes into ${path.relative(process.cwd(), voicesDir)}` +
    ` (${copied} copied, ${pruned} dropped, ${(audioBytes / 1048576).toFixed(1)} MiB of audio)`,
);
if (missing.length) {
  console.error(`${missing.length} manifest entries point at a missing file:`);
  for (const m of missing.slice(0, 10)) console.error(`  ${m}`);
  process.exit(1);
}
