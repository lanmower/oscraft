#!/usr/bin/env node
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const pack = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const root = path.resolve(pack, "..", "..");
const moba = path.join(root, "scenarios", "moba");
const botaRoot = process.env.BOTA_ROOT ?? path.resolve(root, "..", "bota");
const hy3dRoot = process.env.HY3D_ROOT ?? path.resolve(root, "..", "hy3d");
const check = process.argv.includes("--check");
const apply = process.argv.includes("--apply");
if (check && apply) throw new Error("--check and --apply are exclusive");

const LEGACY_ACRONYMS = {
  "001": "nec", "002": "cha", "004": "woj", "005": "blo", "006": "tra", "007": "boo", "008": "coo",
  "009": "gon", "010": "spu", "011": "kar", "012": "how", "013": "san", "014": "lan", "015": "fin",
  "016": "wil", "017": "nos", "018": "din", "019": "ecc", "020": "bot", "021": "sho", "022": "npc",
  "023": "for", "024": "col", "025": "gre", "026": "ghi", "027": "got",
};
const SLUGS = {
  "001": "neckbeard", "002": "chad", "004": "wojak", "005": "bloomer", "006": "tradwife", "007": "boomer", "008": "coomer",
  "009": "gondola", "010": "spurdo", "011": "karen", "012": "howard", "013": "sanic", "014": "lankdad", "015": "finger_dan",
  "016": "willie", "017": "noscope", "018": "dino", "019": "ecce", "020": "bot_bernard", "021": "shoop", "022": "npc",
  "023": "forever", "024": "cole", "025": "grey", "026": "ghillie", "027": "gothgirl",
};
const reserveNames = ["Gigachad", "Wanderer", "Raker", "Seeder", "Wartrucker", "Bouncer"];
const rosterText = fs.readFileSync(path.join(botaRoot, "ROSTER.md"), "utf8");
const retiredSlots = (rosterText.match(/slots? ([\d,\s]+and [\d\s]+) are retired/i)?.[1] ?? "")
  .split(/[\s,]+/).map((token) => token.trim()).filter((token) => /^\d{3}$/.test(token)).sort();
if (retiredSlots.length === 0) throw new Error(`${botaRoot}/ROSTER.md states no retired slots`);
const active = rosterText.split("\n")
  .filter((line) => /^\| 0\d\d \|/.test(line))
  .map((line) => {
    const [slot, name, promptFile] = line.split("|").slice(1, 4).map((s) => s.trim());
    const acro = LEGACY_ACRONYMS[slot];
    const slug = SLUGS[slot];
    if (!acro || !/^[a-z]{1,3}$/.test(acro) || !slug || !/^[a-z][a-z0-9_]*$/.test(slug)) throw new Error(`missing or invalid fullname identifier for slot ${slot}`);
    return { slot, name, promptFile, acro, slug, fileSlug: slug.replace(/_/g, "-"), label: slug.split("_").map((w) => w[0].toUpperCase() + w.slice(1)).join(" ") };
  });
const expectedSlots = Object.keys(SLUGS).filter((slot) => !retiredSlots.includes(slot)).sort();
const rosterSlots = active.map((a) => a.slot).sort();
if (rosterSlots.join(",") !== expectedSlots.join(",")) {
  const missing = expectedSlots.filter((slot) => !rosterSlots.includes(slot));
  const extra = rosterSlots.filter((slot) => !expectedSlots.includes(slot));
  throw new Error(`expected ${expectedSlots.length} active heroes (retired ${retiredSlots.join(", ")}), got ${active.length}; ${botaRoot}/ROSTER.md lists no slot(s) ${missing.map((slot) => `${slot} ${SLUGS[slot]}`).join(", ") || "none"} and unexpected slot(s) ${extra.join(", ") || "none"}`);
}
if (new Set(active.map((a) => a.slug)).size !== active.length) throw new Error("fullname identifier collision");

const baselineHeroText = fs.readFileSync(path.join(moba, "units_heroes.osd"), "utf8");
const originals = [...new Set([...baselineHeroText.matchAll(/^unit\("(\w+)"/gm)].map((m) => m[1]))].sort();
const mapping = new Map(originals.map((id, index) => [id, active[index % active.length]]));
const byActiveId = new Map(active.map((a) => [a.slug, []]));
for (const [id, a] of mapping) byActiveId.get(a.slug).push(id);

const ready = JSON.parse(fs.readFileSync(path.join(hy3dRoot, "output", "bota-ready.json"), "utf8"));
if (ready.missing !== 0 || !(ready.relayed >= active.length)) throw new Error(`bota-ready.json reports missing=${ready.missing} relayed=${ready.relayed} for ${active.length} active heroes`);
const readyBySlug = new Map(ready.characters.map((row) => [row.slug, row]));
const uncovered = active.filter((a) => !readyBySlug.has(a.slug));
if (uncovered.length > 0) throw new Error(`bota-ready.json has no row for ${uncovered.map((a) => a.slug).join(", ")}`);
const modelFor = (a) => `${a.slug}-skinned`;
const portraitFor = (a) => `icons/heroes/${a.slug}.jpg`;
const modelPathFor = (a) => `assets/models/${modelFor(a)}.glb`;
const sourceModelFor = (a) => path.join(hy3dRoot, readyBySlug.get(a.slug).model);
const destModelFor = (a) => path.join(pack, modelPathFor(a));

const identifierTokens = new Map();
for (const a of active) {
  identifierTokens.set(a.acro, a.slug);
  identifierTokens.set(a.name, a.slug);
  identifierTokens.set(`bota_${a.slot}_${a.slug}`, a.slug);
}
for (const [id, a] of mapping) identifierTokens.set(id, a.slug);
const orderedTokens = [...identifierTokens].sort((a, b) => b[0].length - a[0].length || a[0].localeCompare(b[0]));
const tokenPattern = new RegExp(`(?<![A-Za-z0-9_-])(?:${orderedTokens.map(([token]) => token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})(?![A-Za-z0-9_-])`, "g");
const rewriteText = (text, file) => text.replace(tokenPattern, (token) => identifierTokens.get(token) ?? token);

const textExt = new Set([".lua", ".osd", ".json", ".txt", ".tsv", ".md"]);
const generatedNames = new Set([
  "character-mapping.tsv", "bota_roster.lua", "units_heroes.osd", "draft_data.lua", "hero_acronyms.lua",
  "CHARACTERS.md", "bota_ui.json", "assets/assets.json", "assets/props.json", "IDENTIFIERS.md",
]);
const keepRaw = new Set(["character-mapping.tsv", "CHARACTERS.md"]);
const outputs = new Map();
const write = (rel, text) => outputs.set(path.join(pack, rel), text);
const stamp = "# Generated by scenarios/bota/tools/build-bota-pack.mjs. Do not edit by hand.\n";

write("character-mapping.tsv", [
  ["original_id", "legacy_acronym", "active_id", "slot", "model", "portrait", "optimized_3d_model", "source_model"],
  ...originals.map((id) => {
    const a = mapping.get(id);
    return [id, a.acro, a.slug, a.slot, modelFor(a), portraitFor(a), modelPathFor(a), readyBySlug.get(a.slug).model];
  }).map((row) => row.join("\t")),
].join("\n") + "\n");

write("bota_roster.lua", stamp + "return {\n" + active.map((a) => `  ["${a.slug}"] = { slot = "${a.slot}", label = "${a.label}", legacy_acronym = "${a.acro}", model = "${modelFor(a)}", portrait = "${portraitFor(a)}" },`).join("\n") + "\n}\n");

write("units_heroes.osd", stamp + active.map((a) => {
  const source = baselineHeroText.split("\n").find((line) => line.startsWith(`unit("${byActiveId.get(a.slug)[0]}"`));
  return source.replace(/^unit\("\w+"/, `unit("${a.slug}"`).replace(/model="[^"]*"/, `model="${modelFor(a)}"`);
}).join("\n") + "\n");

write("draft_data.lua", `local data = {}\n\ndata.heroes = {\n${active.map((a) => `    "${a.slug}",`).join("\n")}\n}\n\ndata.groups = "${"1".repeat(active.length)}"\n\ndata.SENTINEL_LAST = ${Math.floor(active.length / 2) - 1}\ndata.TECHIES = 0\n\nreturn data\n`);
write("hero_acronyms.lua", stamp + "return {\n" + active.map((a) => `  ["${a.slug}"] = "${a.label}",`).join("\n") + "\n}\n");

write("CHARACTERS.md", [
  "# BOTA character layer",
  "",
  "The full BOTA character slug is the only active hero identifier. It is used for unit IDs, draft entries, Lua keys, UI keys, model IDs, portrait IDs, catalogue IDs and all character-facing pack references.",
  "",
  `Active roster: ${active.length} fullname slugs from \`/workspace/bota/ROSTER.md\`, slots ${active[0].slot}-${active.at(-1).slot} excluding retired slots ${retiredSlots.join(", ")}.`,
  "",
  `Retired slots ${retiredSlots.join(", ")} are excluded from the pack (002 Chad and 011 Karen from the 2026-11-12 design finalization, and 003 Gigachad image-only Reserve). Slots 028-032 are Reserve with no images and are excluded.`,
  "",
  "The 127 original MOBA hero IDs below are historical source IDs being mapped FROM. They are not active BOTA identifiers.",
  "",
  `The \`<ACRO>-skinned.glb\` paths hold the ${ready.relayed} verified optimized static GLB meshes from hy3d's \`bota-ready.json\` (256px textures, glTF 2.0, no animation skins), of which the ${active.length} active heroes are packed. The historical \`-skinned\` suffix is retained for compatibility; rigged animation is not present.`,
  "",
  "## Acronym assignments",
  "",
  "| Slot | Acronym | Model | Portrait |",
  "| --- | --- | --- | --- |",
  ...active.map((a) => `| ${a.slot} | ${a.slug} | ${a.acro} | ${modelFor(a)} | ${portraitFor(a)} |`),
  "",
  "## Original ID to acronym mapping",
  "",
  "| Original MOBA source ID | Acronym |",
  "| --- | --- |",
  ...originals.map((id) => `| ${id} | ${mapping.get(id).acro} |`),
  "",
].join("\n"));

const identifierDoc = [
  "# BOTA identifiers: full character names/slugs only",
  "",
  "Active BOTA hero identifiers are the full character slugs below. Three-letter acronym identifiers are forbidden for BOTA active IDs. They appear only in the historical before/after provenance mapping.",
  "",
  "Validator mode: `--identifier-mode bota-fullname` with `--default-dota-acronyms`.",
  "",
  "The opt-in URL remains `http://127.0.0.1:<port>/js/client/play.html?package=scenarios/bota`. The default no-parameter URL resolves to `scenarios/moba-acronyms`, not BOTA.",
  "",
  `Slot \`003\` Gigachad is image-only Reserve and excluded; retired slots \`${retiredSlots.join("`, `")}\` are excluded. Slots \`028-032\` are excluded.`,
  "",
  "| Slot | Active BOTA ID | Legacy acronym (forbidden active ID) | Model | Portrait | Source model |",
  "| --- | --- | --- | --- | --- | --- |",
  ...active.map((a) => `| ${a.slot} | \`${a.slug}\` | \`${a.acro}\` | \`assets/models/${modelFor(a)}.glb\` | \`assets/${portraitFor(a)}\` | \`${readyBySlug.get(a.slug).model}\` |`),
  "",
  `The ${active.length} packed GLB byte hashes are unchanged from the ${ready.relayed} relayed meshes; only identifier-bearing filenames and references were renamed. Relayed \`bota-<slug>-skinned.glb\` assets remain as provenance assets.`,
  "",
].join("\n");
write("IDENTIFIERS.md", identifierDoc);

const baselineFiles = [];
const walkMoba = (dir, rel = "") => {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, entry.name);
    const relPath = path.posix.join(rel, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "tools") continue;
      walkMoba(abs, relPath);
    } else if (textExt.has(path.extname(entry.name))) baselineFiles.push(relPath);
  }
};
walkMoba(moba);
for (const rel of baselineFiles) {
  if (generatedNames.has(rel)) continue;
  const baselineText = fs.readFileSync(path.join(moba, rel), "utf8");
  let text = rel.endsWith(".lua") && rel !== "draft_data.lua" ? rewriteText(baselineText, rel) : rewriteText(baselineText, rel);
  if (rel.endsWith(".lua")) {
    for (const [stem, file] of Object.entries(JSON.parse(fs.readFileSync(path.join(moba, "manifest.json"), "utf8")).modules ?? {})) {
      const replacement = rewriteText(stem, rel);
      if (replacement !== stem) text = text.replace(new RegExp(`(?<![A-Za-z0-9_])${replacement}(?![A-Za-z0-9_])`, "g"), stem);
    }
  }
  if (rel === "mines.lua") text = text.replace(/\bFOREVER\b/g, "UNLIMITED_LIFE");
  if (rel === "manifest.json") {
    const manifest = JSON.parse(fs.readFileSync(path.join(moba, rel), "utf8"));
    manifest.name = "bota";
    manifest.version = "0.1.0";
    manifest.image = "bota.osdt";
    manifest.ui = "bota_ui.json";
    manifest.effects = "bota_ui.json";
    manifest.assets.textures.menu_backdrop = "textures/menu-backdrop.jpg";
    manifest.tables = [...new Set([...(manifest.tables ?? []), "bota-skinned.json"])];
    text = JSON.stringify(manifest, null, 2) + "\n";
  }
  if (path.posix.basename(rel) === "moba_ui.json") write("bota_ui.json", text);
  else write(rel, text);
}

const ui = JSON.parse(outputs.get(path.join(pack, "bota_ui.json")) ?? fs.readFileSync(path.join(pack, "bota_ui.json"), "utf8"));
ui.heroes = Object.fromEntries(active.map((a) => [a.slug, { label: a.label, slot: a.slot, model: modelFor(a), portrait: portraitFor(a), placeholder: "gallery_jpg", legacy_acronym: a.acro }]));
ui.heroAcronyms = Object.fromEntries(active.map((a) => [a.slug, a.slug]));
ui.legacyHeroAcronyms = Object.fromEntries(active.map((a) => [a.acro, a.slug]));
ui.identifierMode = "bota-fullname";
ui.menuBackdrop = "textures.menu_backdrop";
ui.characterSet = { name: "bota", active: active.length, retired: retiredSlots, reserve: ["028", "029", "030", "031", "032"], relayedModels: ready.relayed, optimizedModels: true };
write("bota_ui.json", JSON.stringify(ui, null, 2) + "\n");

const assets = JSON.parse(fs.readFileSync(path.join(moba, "assets", "assets.json"), "utf8"));
assets.items = assets.items.filter((item) => !(item.role === "character" && /-skinned\.glb$/.test(item.path ?? "")));
for (const a of active) {
  const row = readyBySlug.get(a.slug);
  assets.items.push({ id: modelFor(a), kind: "model", path: `models/${modelFor(a)}.glb`, placed_by: "props.json", bytes: row.bytes, role: "character", status: "optimized_static_mesh" });
  assets.items.push({ id: `${a.slug}-portrait`, kind: "texture", path: portraitFor(a), placed_by: "bota_ui.json", bytes: fs.statSync(path.join(botaRoot, "gallery", `${a.slot}-${a.fileSlug}.jpg`)).size, role: "portrait", status: "temporary_gallery_placeholder" });
}
const backdrop = path.join(pack, "assets", "textures", "menu-backdrop.jpg");
assets.items.push({ id: "menu-backdrop", kind: "texture", path: "textures/menu-backdrop.jpg", placed_by: "bota_ui.json", bytes: fs.statSync(backdrop).size, role: "menu_backdrop", status: "pack_asset" });
assets.count = assets.items.length;
assets.total_bytes = assets.items.reduce((n, item) => n + (item.bytes ?? 0), 0);
write("assets/assets.json", JSON.stringify(assets, null, 2) + "\n");

const props = JSON.parse(fs.readFileSync(path.join(moba, "assets", "props.json"), "utf8"));
const legacyHeroModelKeys = new Set(originals.map((id) => `${id}_skinned`).concat(["centaur_skinned"]));
for (const key of Object.keys(props)) {
  if (legacyHeroModelKeys.has(key) || /^bota[-_]/.test(key) || active.some((a) => key === a.slug || key === a.fileSlug)) delete props[key];
}
for (const [key, rel] of Object.entries(props)) {
  if (typeof rel === "string" && rel.endsWith(".glb") && !fs.existsSync(path.join(pack, "assets", rel))) delete props[key];
}
for (const a of active) props[a.slug] = `models/${modelFor(a)}.glb`;
write("assets/props.json", JSON.stringify(props, null, 2) + "\n");

const skinnedManifestPath = path.join(pack, "bota-skinned.json");
if (fs.existsSync(skinnedManifestPath)) write("bota-skinned.json", fs.readFileSync(skinnedManifestPath, "utf8"));
if (check) {
  const drift = [...outputs].filter(([abs, text]) => !fs.existsSync(abs) || fs.readFileSync(abs, "utf8") !== text).map(([abs]) => path.relative(pack, abs));
  if (drift.length) {
    console.error(`bota pack drift: ${drift.length} files\n${drift.join("\n")}`);
    process.exit(1);
  }
  console.log(`bota pack check ok: ${outputs.size} generated files current, ${active.length} fullname-only heroes`);
} else if (apply) {
  for (const [abs, text] of outputs) {
    fs.mkdirSync(path.dirname(abs), { recursive: true });
    fs.writeFileSync(abs, text);
  }
  const supportModels = [
    "tower-skinned.glb", "base-skinned.glb", "tender-skinned.glb", "courier-skinned.glb",
    "ghost-skinned.glb", "vampire-skinned.glb", "zombie-skinned.glb",
  ];
  for (const name of supportModels) {
    const source = path.join(moba, "assets", "models", name);
    const dest = path.join(pack, "assets", "models", name);
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    fs.copyFileSync(source, dest);
  }
  const skinnedDir = path.join(pack, ".bota-static-sources");
  fs.mkdirSync(skinnedDir, { recursive: true });
  for (const a of active) {
    const srcModel = sourceModelFor(a);
    const sourceCopy = path.join(skinnedDir, `${modelFor(a)}.glb`);
    fs.copyFileSync(srcModel, sourceCopy);
    const srcPortrait = path.join(botaRoot, "gallery", `${a.slot}-${a.fileSlug}.jpg`);
    const destPortrait = path.join(pack, "assets", portraitFor(a));
    fs.mkdirSync(path.dirname(destPortrait), { recursive: true });
    fs.copyFileSync(srcPortrait, destPortrait);
  }
  const canonicalModels = new Set([...active.map((a) => `${a.slug}-skinned.glb`), ...supportModels]);
  for (const p of fs.readdirSync(path.join(pack, "assets", "models"))) {
    const staleCharacter = p !== "roshan-skinned.glb" && /-skinned\.glb$/.test(p) && !canonicalModels.has(p) && (/^bota-/.test(p) || /^[A-Za-z]{1,8}-skinned\.glb$/.test(p));
    if (staleCharacter) fs.unlinkSync(path.join(pack, "assets", "models", p));
  }
  const canonicalPortraits = new Set(active.map((a) => `${a.slug}.jpg`));
  for (const p of fs.readdirSync(path.join(pack, "assets", "icons", "heroes"))) {
    if (/^(bota-.*|[A-Z]{1,3})\.jpg$/.test(p) && !canonicalPortraits.has(p)) fs.unlinkSync(path.join(pack, "assets", "icons", "heroes", p));
  }
  execFileSync(process.execPath, [path.join(pack, "tools", "skin-bota-models.mjs")], { cwd: root, stdio: "inherit", env: { ...process.env, BOTA_STATIC_SOURCE_DIR: skinnedDir } });
  fs.rmSync(skinnedDir, { recursive: true, force: true });
  outputs.set(skinnedManifestPath, fs.readFileSync(skinnedManifestPath, "utf8"));
  console.log(`bota pack generated: ${outputs.size} files, ${active.length} skinned fullname models, ${active.length} portraits, ${originals.length} mapped source ids`);
} else {
  console.log(`bota pack plan: ${outputs.size} files, ${active.length} fullname-only heroes, ${originals.length} mapped source ids`);
  console.log(`active ids: ${active.map((a) => a.slug).join(",")}`);
}
