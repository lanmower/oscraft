#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseGlb, parseGlbAnimations, readGlbChunks } from "../../../js/client/glb.js";
import { resolveGlbExtensions } from "../../../js/client/glb_resolve.js";
import { retargetLibrary } from "../../../js/client/anim.js";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const pack = path.join(root, "scenarios", "bota");
const moba = path.join(root, "scenarios", "moba");
const acronymPack = path.join(root, "scenarios", "moba-acronyms");
const hy3dRoot = path.resolve(root, "..", "hy3d");
const read = (...parts) => fs.readFileSync(path.join(...parts), "utf8");
const fail = (message) => { console.error(`bota pack: ${message}`); process.exit(1); };
const FULLNAMES = ["neckbeard", "wojak", "bloomer", "tradwife", "boomer", "coomer", "gondola", "spurdo", "howard", "sanic", "lankdad", "finger_dan", "willie", "noscope", "dino", "ecce", "bot_bernard", "shoop", "npc", "forever", "cole", "grey", "ghillie", "gothgirl"];
const ACTIVE = FULLNAMES.length;
const FORBIDDEN_BOTA_IDS = ["nec", "cha", "woj", "blo", "tra", "boo", "coo", "gon", "spu", "kar", "how", "san", "lan", "fin", "wil", "nos", "din", "ecc", "bot", "sho", "for", "col", "gre", "ghi", "got"];

const ready = JSON.parse(read(hy3dRoot, "output", "bota-ready.json"));
if (ready.activeSlots !== 26 || ready.ready !== 26 || ready.missing !== 0 || ready.relayed !== 26) fail("bota-ready.json does not report its 26 relayed characters");
const skinnedManifest = JSON.parse(read(pack, "bota-skinned.json"));
if (skinnedManifest.clips !== 8 || Object.keys(skinnedManifest.heroes ?? {}).length !== ACTIVE) fail(`bota-skinned.json does not report ${ACTIVE} heroes and 8 library clips`);
const rows = read(pack, "character-mapping.tsv").trim().split("\n").slice(1).map((line) => line.split("\t"));
if (rows.length !== 127) fail(`expected 127 original source mappings, got ${rows.length}`);
const activeRows = new Map();
const legacyToActive = new Map();
for (const [original, legacy, active, slot, model, portrait, optimized, source] of rows) {
  if (!original || !legacy || !active || !slot || !model || !portrait || !optimized || !source) fail(`incomplete mapping row for ${original}`);
  if (!/^[a-z][a-z0-9_]*$/.test(active) || FORBIDDEN_BOTA_IDS.includes(active)) fail(`${original}: ${active} is not a fullname active BOTA ID`);
  activeRows.set(active, { legacy, slot, model, portrait, optimized, source });
  legacyToActive.set(legacy, active);
}
if (activeRows.size !== ACTIVE || FULLNAMES.some((id) => !activeRows.has(id))) fail(`expected ${ACTIVE} unique owner-specified fullname heroes, got ${activeRows.size}`);
for (const id of Object.keys(skinnedManifest.heroes)) if (!activeRows.has(id)) fail(`bota-skinned.json has non-fullname key ${id}`);

const unitIds = [...read(pack, "units_heroes.osd").matchAll(/^unit\("([^"]+)"/gm)].map((m) => m[1]);
if (unitIds.length !== ACTIVE || unitIds.some((id) => !activeRows.has(id))) fail(`units_heroes.osd has non-fullname ids: ${unitIds.join(",")}`);
const draftBlock = /data\.heroes = \{([\s\S]*?)\}/.exec(read(pack, "draft_data.lua"))?.[1] ?? "";
const draft = [...draftBlock.matchAll(/"([^"]+)"/g)].map((m) => m[1]);
if (draft.length !== ACTIVE || draft.some((id) => !activeRows.has(id))) fail(`draft_data.lua has non-fullname ids: ${draft.join(",")}`);
const ui = JSON.parse(read(pack, "bota_ui.json"));
const uiHeroes = Object.keys(ui.heroes ?? {});
if (uiHeroes.length !== ACTIVE || uiHeroes.some((id) => !activeRows.has(id))) fail("bota_ui.json hero keys are not fullname-only");
if (ui.identifierMode !== "bota-fullname") fail("bota_ui.json does not declare bota-fullname identifier mode");
for (const [key, value] of Object.entries(ui.heroAcronyms ?? {})) {
  if (!activeRows.has(key) || key !== value || FORBIDDEN_BOTA_IDS.includes(key)) fail(`bota_ui.json heroAcronyms is not a fullname label map at ${key}`);
}
const botaAcronymTable = new Map([...read(pack, "hero_acronyms.lua").matchAll(/^\s*\["([^"]+)"\] = "([^"]*)",/gm)].map((m) => [m[1], m[2]]));
if (botaAcronymTable.size !== ACTIVE || [...botaAcronymTable.keys()].some((id) => !activeRows.has(id))) fail("BOTA hero_acronyms.lua is not a coherent fullname label table");
if ([...botaAcronymTable.values()].some((label) => /^[A-Z]{1,3}$/.test(label))) fail("BOTA hero_acronyms.lua still emits acronym-only labels");

const defaultPackParam = /const PACK_PARAM = [^\n]*\?\?|const PACK_PARAM = [^\n]*\|\| "([^"]+)"/.exec(read(root, "js", "client", "pack_load.js"));
const loaderText = read(root, "js", "client", "pack_load.js");
if (!loaderText.includes('"scenarios/moba-acronyms"')) fail("default PACK_PARAM does not resolve to scenarios/moba-acronyms");
if (!loaderText.includes('PACK_PARAMS.get("package")')) fail("package opt-in parameter was lost");
const defaultPack = JSON.parse(read(acronymPack, "manifest.json"));
if (defaultPack.name !== "moba-acronyms") fail("derived default pack manifest identity is not moba-acronyms");
const defaultTable = JSON.parse(read(acronymPack, "moba_ui.json"));
const defaultIds = Object.keys(defaultPack.modules ?? {}).includes("hero_acronyms") ? [...read(acronymPack, "hero_acronyms.lua").matchAll(/^\s*\["([^"]+)"\]/gm)].map((m) => m[1]) : [];
if (defaultIds.length !== 127 || defaultIds.some((id) => !/^[a-z]{1,3}$/.test(id))) fail(`default MOBA active hero IDs are not acronym-only (${defaultIds.length})`);
if (defaultTable.identifierMode !== "default-dota-acronyms") fail("derived default UI does not declare default-dota-acronyms");
const sourceTable = new Map([...read(moba, "hero_acronyms.lua").matchAll(/^\s*\["([^"]+)"\] = "([^"]*)",/gm)].map((m) => [m[1], m[2]]));
if (sourceTable.size !== 127 || [...sourceTable.values()].some((value) => !/^[A-Z]{1,3}$/.test(value))) fail("historical default MOBA acronym authority is not coherent");
const sourceUnitIds = [...read(moba, "units_heroes.osd").matchAll(/^unit\("(\w+)"/gm)].map((m) => m[1]);
if (sourceUnitIds.length !== 127) fail(`historical scenarios/moba unit roster changed: ${sourceUnitIds.length} units`);
if (read(root, "docs", "BOTA.md").includes("26 active acronym-only hero IDs")) fail("docs/BOTA.md still describes BOTA acronym IDs as active");
if (!read(pack, "IDENTIFIERS.md").includes("full character slugs") || !read(moba, "IDENTIFIERS.md").includes("acronyms")) fail("identifier regime docs are missing");

const readyBySlug = new Map(ready.characters.map((row) => [row.slug, row]));
const rosterAuthority = read(path.resolve(root, "..", "bota"), "ROSTER.md");
const sourceNames = [...rosterAuthority.matchAll(/^\| (0\d\d) \| ([^|]+) \|/gm)].map((m) => ({ slot: m[1], name: m[2].trim() }));
const allowedDocumentation = new Set(["CHARACTERS.md", "character-mapping.tsv", "IDENTIFIERS.md", "cross-contract-snapshot.json"]);
const textExt = new Set([".lua", ".osd", ".json", ".txt", ".tsv", ".md"]);
const scanned = [];
const walk = (dir, rel = "") => {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, entry.name), relPath = path.posix.join(rel, entry.name);
    if (entry.isDirectory()) { if (entry.name !== "tools" && entry.name !== "fixtures") walk(abs, relPath); }
    else if (textExt.has(path.extname(entry.name))) scanned.push([relPath, abs]);
  }
};
walk(pack);
const acronymIdPattern = new RegExp(`(?<![A-Za-z0-9_-])(?:${FORBIDDEN_BOTA_IDS.join("|")})(?![A-Za-z0-9_-])`, "i");
const forbidden = [];
for (const [rel, abs] of scanned) {
  if (allowedDocumentation.has(rel)) continue;
  const text = fs.readFileSync(abs, "utf8");
  const activeKeyLines = text.split("\n").filter((line) => /^\s*unit\(|\"[^\"]+\"\s*=|heroAcronyms|legacyHeroAcronyms|legacy_acronym|models\/.*-skinned|icons\/heroes|^\s*\"[a-z][a-z0-9_]*\",\s*$|data\.heroes/.test(line) && !/legacy_acronym|legacyHeroAcronyms/.test(line));
  if (activeKeyLines.some((line) => acronymIdPattern.test(line))) forbidden.push(`${rel}: acronym-only active hero ID`);
}
if (forbidden.length) fail(`forbidden BOTA acronym-only hero identifiers remain\n${forbidden.slice(0, 30).join("\n")}`);
const defaultPackScan = [];
for (const rel of ["units_heroes.osd", "draft_data.lua", "hero_acronyms.lua", "moba_ui.json", "IDENTIFIERS.md", "identifier-map.tsv"]) defaultPackScan.push(rel);
const defaultAcronymUnits = [...read(acronymPack, "units_heroes.osd").matchAll(/^unit\("(\w+)"/gm)].map((m) => m[1]);
if (defaultAcronymUnits.some((id) => !/^[a-z]{1,3}$/.test(id))) fail("derived default units_heroes.osd contains non-acronym active IDs");
const derivedDraft = [...(/data\.heroes = \{([\s\S]*?)\n\}/.exec(read(acronymPack, "draft_data.lua"))?.[1] ?? "").matchAll(/"([^"]+)"/g)].map((m) => m[1]);
if (derivedDraft.length === 0 || derivedDraft.some((id) => !/^[a-z]{1,3}$/.test(id)) || derivedDraft.some((id) => !defaultIds.includes(id))) fail(`derived default draft_data.lua contains non-acronym active IDs (${derivedDraft.length}: ${derivedDraft.filter((id) => !/^[a-z]{1,3}$/.test(id)).slice(0, 8).join(",")})`);
const acronymAuthority = new Set([...sourceTable.values()].map((value) => value.toLowerCase()));
const acronymUnitIds = [...read(acronymPack, "units_heroes.osd").matchAll(/^unit\("(\w+)"/gm)].map((m) => m[1]);
if (acronymUnitIds.length !== 127 || acronymUnitIds.some((id) => !acronymAuthority.has(id))) fail("default acronym pack units_heroes.osd has IDs outside the acronym authority");
for (const [rel, abs] of [["draft_data.lua", path.join(acronymPack, "draft_data.lua")], ["hero_acronyms.lua", path.join(acronymPack, "hero_acronyms.lua")]]) {
  const activeLines = fs.readFileSync(abs, "utf8").split("\n").filter((line) => /^\s*"[^"]+"|^\s*\["[^"]+"\]/.test(line));
  if (activeLines.some((line) => line.includes('"') && !acronymAuthority.has(line.split('"')[1] ?? ""))) fail(`default acronym pack ${rel} has IDs outside the acronym authority`);
}

const map = JSON.parse(read(pack, "assets", "anim", "maps", "humanoid.json"));
const library = JSON.parse(read(pack, "assets", "anim", "library.json"));
if (library.clips.length !== 8) fail(`animation library has ${library.clips.length} clips, expected 8`);
const catalogue = JSON.parse(read(pack, "assets", "assets.json"));
const props = JSON.parse(read(pack, "assets", "props.json"));
const roster = read(pack, "bota_roster.lua");
for (const [active, a] of activeRows) {
  const row = readyBySlug.get(path.basename(a.source, ".glb"));
  const provenance = skinnedManifest.heroes[active];
  if (!row || !provenance) fail(`${active}: missing readiness row or skinned provenance`);
  if (provenance.sourceStaticSha256 !== row.sha256 || provenance.sourceStaticBytes !== row.bytes) fail(`${active}: static source provenance does not match bota-ready.json`);
  const file = path.join(pack, a.optimized);
  const bytes = fs.readFileSync(file);
  const sha = crypto.createHash("sha256").update(bytes).digest("hex");
  if (bytes.length !== provenance.skinnedBytes || sha !== provenance.skinnedSha256) fail(`${active}: skinned model hash does not match bota-skinned.json`);
  const ab = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
  const glb = parseGlb(ab, a.model);
  if (!glb.joints || !glb.weights) fail(`${active}: client parser found no JOINTS_0/WEIGHTS_0`);
  const animations = parseGlbAnimations(ab);
  const skin = animations.skins[0];
  if (!skin || skin.joints.length !== 28 || skin.inverseBindMatrices === null) fail(`${active}: invalid skin or joint count ${skin?.joints.length}`);
  const primitive = readGlbChunks(ab).json.meshes[0].primitives[0];
  if (primitive.attributes.JOINTS_0 === undefined || primitive.attributes.WEIGHTS_0 === undefined) fail(`${active}: primitive lacks skin attributes`);
  const names = new Set(animations.nodes.map((node) => node.name).filter(Boolean));
  const resolved = Object.values(map.bones).filter((bone) => names.has(bone)).length;
  if (!names.has(map.bones.hips) || resolved < Object.keys(map.bones).length * 0.8) fail(`${active}: rig nodes are not humanoid (${resolved}/${Object.keys(map.bones).length})`);
  const clips = animations.clips.length ? animations.clips : retargetLibrary(library, map, animations.nodes);
  if (clips.length !== 8) fail(`${active}: expected 8 reachable clips, got ${clips.length}`);
  if (!catalogue.items.some((item) => item.path === `models/${a.model}.glb`)) fail(`${active}: catalogue does not name the skinned model`);
  if (!fs.existsSync(path.join(pack, "assets", a.portrait))) fail(`${active}: portrait is missing`);
  if (ui.heroes[active]?.model !== a.model || ui.heroes[active]?.portrait !== a.portrait) fail(`${active}: UI model/portrait references do not resolve`);
  if (props[active] !== `models/${a.model}.glb`) fail(`${active}: props.json does not resolve the active ID to its model`);
  if (!roster.includes(`["${active}"]`) || !unitIds.includes(active) || !draft.includes(active)) fail(`${active}: roster, unit or draft reference is missing`);
  if (!fs.existsSync(path.join(pack, "assets", "models", `${active}-skinned.glb`))) fail(`${active}: active model file is missing`);
}
const backdrop = path.join(pack, "assets", "textures", "menu-backdrop.jpg");
const backdropBytes = fs.existsSync(backdrop) ? fs.readFileSync(backdrop) : Buffer.alloc(0);
const backdropSha = crypto.createHash("sha256").update(backdropBytes).digest("hex");
if (backdropBytes.length !== 3654481 || backdropSha !== "431b1f861938c26edb7ce226e75d1c66a433c701d70a59463e064c402a077549") fail("menu backdrop JPEG bytes or SHA-256 do not match the supplied image");
if (ui.menuBackdrop !== "textures.menu_backdrop") fail("bota_ui.json does not name the backdrop asset key");
const manifestJson = JSON.parse(read(pack, "manifest.json"));
if (manifestJson.assets?.textures?.menu_backdrop !== "textures/menu-backdrop.jpg") fail("manifest.json does not expose the backdrop asset key");
const sourceBackdrop = path.join(moba, "assets", "textures", "menu-backdrop.jpg");
const defaultPacks = [["historical MOBA", moba], ["derived default", acronymPack]];
for (const [label, dir] of defaultPacks) {
  if (JSON.parse(read(dir, "manifest.json")).assets?.textures?.menu_backdrop !== "textures/menu-backdrop.jpg") fail(`${label} pack manifest does not expose the menu backdrop key`);
  if (JSON.parse(read(dir, "moba_ui.json")).menuBackdrop !== "textures.menu_backdrop") fail(`${label} pack UI does not name the menu backdrop key`);
  const file = path.join(dir, "assets", "textures", "menu-backdrop.jpg");
  const bytes = fs.existsSync(file) ? fs.readFileSync(file) : Buffer.alloc(0);
  if (bytes.length < 2 || bytes[0] !== 0xff || bytes[1] !== 0xd8) fail(`${label} pack menu backdrop is missing or not a JPEG`);
  if (bytes.length > 800 * 1024) fail(`${label} pack menu backdrop is ${bytes.length} bytes, over the 800 KiB page budget`);
}
if (!fs.readFileSync(sourceBackdrop).equals(fs.readFileSync(path.join(acronymPack, "assets", "textures", "menu-backdrop.jpg")))) fail("the derived default pack backdrop differs from the historical MOBA backdrop");
const refs = new Map();
for (const [key, rel] of Object.entries(props)) if (typeof rel === "string" && rel.endsWith(".glb")) refs.set(rel, { key, source: "props.json", kind: key });
for (const item of catalogue.items) if (item.kind === "model" && typeof item.path === "string" && item.path.endsWith(".glb")) if (!refs.has(item.path)) refs.set(item.path, { key: item.id, source: "assets.json", kind: item.role ?? "" });
const characterRef = (entry, rel) => activeRows.has(entry.key) || entry.kind === "character" || /^neutral_.*_skinned$/.test(entry.key) || activeRows.has(path.basename(rel, "-skinned.glb"));
const validatedCharacterModels = [];
await (async () => {
  for (const [rel, entry] of refs) {
    const file = path.join(pack, "assets", rel);
    if (!fs.existsSync(file)) fail(`${entry.source} ${entry.key}: ${rel} is missing`);
    if (!characterRef(entry, rel)) continue;
    const bytes = fs.readFileSync(file);
    const raw = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
    let glb, animations;
    try {
      const resolved = await resolveGlbExtensions(raw);
      glb = parseGlb(resolved, rel);
      animations = parseGlbAnimations(resolved);
    } catch (error) { fail(`${entry.source} ${entry.key}: ${rel} is not parseable by the client (${error.message})`); }
    const skin = animations.skins[0];
    if (!glb.joints || !glb.weights || !skin || skin.joints.length === 0 || skin.joints.length > 64 || skin.inverseBindMatrices === null) fail(`${entry.source} ${entry.key}: ${rel} is not a valid skinned character model`);
    const clips = animations.clips.length ? animations.clips : retargetLibrary(library, map, animations.nodes);
    if (clips.length !== 8) fail(`${entry.source} ${entry.key}: ${rel} has ${clips.length} reachable clips, expected 8`);
    validatedCharacterModels.push(`${entry.key}:${rel}`);
  }
  console.log(`bota menu backdrop ok: ${backdropBytes.length} bytes, sha256 ${backdropSha}, 2752x1536 JPEG, key ${ui.menuBackdrop}`);
  console.log(`default menu backdrop ok: ${fs.statSync(sourceBackdrop).size} bytes in both default packs, key textures.menu_backdrop`);
  console.log(`identifier regimes ok: BOTA ${activeRows.size} fullname-only heroes, default MOBA ${defaultIds.length} acronym-only heroes, PACK_PARAM scenarios/moba-acronyms`);
  console.log(`bota pack ok: ${activeRows.size} fullname heroes, ${rows.length} legacy acronym mappings, ${Object.keys(skinnedManifest.heroes).length} skinned models of ${ready.relayed} relayed, 28 joints, 8 reachable library clips`);
  console.log(`model reference scan ok: ${refs.size} unique props.json/assets.json GLB refs, ${validatedCharacterModels.length} character-class refs validated`);
  console.log(`forbidden identifier scan ok: ${scanned.length - allowedDocumentation.size} pack text files, 0 acronym-only BOTA hero IDs outside historical documentation`);
  console.log(`default acronym scan ok: units, draft and hero_acronyms contain no full Dota active IDs; historical scenarios/moba untouched`);
  console.log(`bota animation: real skins active; source static mesh bytes preserved in bota-ready.json provenance`);
})();
