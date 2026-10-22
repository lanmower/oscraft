#!/usr/bin/env node
// Imports a Chatterbox-Nano voice manifest into the pack catalog.
//
// Usage: node scripts/audio/import-voices.mjs <scenario-dir> [voices-dir]
//
// The manifest is written by the voice pipeline and is the source of truth for which file plays for
// which hero event:
//   { "<hero_key>": { "spawn": "path", "attack": "path", "ability": "path", "death": "path",
//                     "voice_ref": "<ref id>" } }
// Paths are relative to the voices directory. Each becomes one catalog item of kind "audio", carrying
// its hero and event so the client can find a line without reading the manifest at runtime, and its
// voice_ref so a replaced reference can be traced to exactly the heroes that need regenerating.
//
// Existing audio entries are replaced, and entries whose file is gone are dropped, so an import after
// a regeneration leaves no stale rows. Nothing but assets.json is written.
import fs from "node:fs";
import path from "node:path";

const scenario = path.resolve(process.argv[2] ?? "scenarios/moba");
const voicesDir = path.resolve(process.argv[3] ?? path.join(scenario, "assets/voices"));
const manifestPath = path.join(voicesDir, "manifest.json");
const catalogPath = path.join(scenario, "assets/assets.json");

if (!fs.existsSync(manifestPath)) {
  console.error(`no manifest at ${manifestPath}; the voice pipeline writes it`);
  process.exit(2);
}
if (!fs.existsSync(catalogPath)) {
  console.error(`no catalog at ${catalogPath}`);
  process.exit(2);
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const catalog = JSON.parse(fs.readFileSync(catalogPath, "utf8"));
const items = Array.isArray(catalog.items) ? catalog.items.filter((i) => i && i.kind !== "audio") : [];

let added = 0;
const missing = [];
for (const [hero, entry] of Object.entries(manifest)) {
  for (const [event, rel] of Object.entries(entry ?? {})) {
    if (event === "voice_ref") continue;
    const file = path.join(voicesDir, rel);
    if (!fs.existsSync(file)) { missing.push(`${hero}/${event}: ${rel}`); continue; }
    items.push({
      id: `voice.${hero}.${event}`,
      kind: "audio",
      path: path.posix.join("voices", rel.split(path.sep).join("/")),
      bytes: fs.statSync(file).size,
      role: "voice",
      hero,
      event,
      voice_ref: entry.voice_ref ?? null,
      placed_by: "voices/manifest.json",
    });
    added += 1;
  }
}

catalog.items = items;
catalog.count = items.length;
catalog.total_bytes = items.reduce((n, i) => n + (Number(i?.bytes) || 0), 0);
fs.writeFileSync(catalogPath, `${JSON.stringify(catalog, null, 2)}\n`);

console.log(`imported ${added} voice lines from ${Object.keys(manifest).length} heroes`);
if (missing.length) {
  console.error(`${missing.length} manifest entries point at a missing file:`);
  for (const m of missing.slice(0, 10)) console.error(`  ${m}`);
  process.exit(1);
}
