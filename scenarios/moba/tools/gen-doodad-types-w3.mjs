#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
const KNOWN = new Set(["dir", "types"]);
for (const a of args) if (a.startsWith("--") && !KNOWN.has(a.slice(2).split("=")[0])) throw new Error(`unknown argument ${a}; use --dir <dir> and --types <file>`);
const flag = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  if (i >= 0) {
    if (!args[i + 1] || args[i + 1].startsWith("--")) throw new Error(`--${name} needs a value`);
    return args[i + 1];
  }
  const hit = args.find((a) => a.startsWith(`--${name}=`));
  return hit ? hit.slice(name.length + 3) : fallback;
};
const dir = flag("dir", "scenarios/moba");
const typesPath = flag("types");
if (!typesPath) throw new Error("--types <file> is required: the w3 type table is produced outside this repo");
const INI = "sumneko/w3x2lni data/enUS-1.27.1/prebuilt/Melee";
const WES = "Crainax/War3Lib fdf/WorldEditStrings.txt";
const cite = (key, text, textLine, iniLine) => `${text} (WESTRING_DOOD_${key}, ${WES}:${textLine}; ${INI}/doodad.ini:${iniLine})`;
const destructable = (key, iniLine) => `destructable, not a doodad (WESTRING_DEST_${key}, ${INI}/destructable.ini:${iniLine}); its placements are dropped`;

const NAMED = {
  LPwb: { kind: "hay_bale", why: "a bundle of wheat, read from the editor strings" },
  AWlp: { kind: "lily_pad", why: `same object as 6.88g CPlp (Lily Pad): ${cite("APLP", "Lily Pads", 2799, 1401)}` },
  APct: { kind: "cattail", why: `same object as 6.88g CPct (Cattail): ${cite("APCT", "Cattail", 2798, 521)}` },
};
const OVERRIDE = {
  APms: { kind: "mushrooms", why: "mushrooms from the English editor strings; 6.88g inferred grass_large" },
  AOnt: { kind: "lantern", why: "totem lantern from the English editor strings; 6.88g inferred rocks" },
  APtv: { kind: "hedge", why: "thorny vines from the English editor strings; no vine kind is declared, so hedge is the nearest declared kind (6.88g inferred grass_large)" },
  AOgs: { kind: "", why: "statue from the English editor strings, but no statue kind is declared; dropped rather than shipped as the fountain 6.88g inferred" },
  LObz: { kind: "torch_glowing", why: "glowing brazier (\u70bd\u70ed\u7684\u706b\u76c6) from the editor strings; 6.88g inferred iron_fence_border_gate" },
  VOfl: { kind: "fence", why: "long fence (\u957f\u7bf1\u7b06) from the editor strings; 6.88g inferred fountain" },
  LWw0: { kind: "", why: "waterfall (\u7011\u5e03) from the editor strings, but no declared kind is a waterfall; dropped rather than shipped as the catwalk 6.88g inferred" },
};
const PARENT = {
  D00B: "ZPsh", D003: "ZPsh", D000: "NOfl", D001: "AObr", D002: "VOfl", D009: "AObd", D00C: "LPwb",
  D004: "LOlp", D005: "LOlp", D006: "LOlp", D007: "LOlp", D008: "LOlp",
  D00D: "LOlp", D00E: "LOlp", D00F: "LOlp", D00H: "LOlp", D00R: "LOlp",
  D00A: "DOtb", D00I: "DOtb", D00J: "DOtb", D00K: "DOtb", D00L: "DOtb", D00M: "DOtb",
  D00N: "DOtb", D00O: "DOtb", D00P: "DOtb", D00Q: "DOtb",
};
const IDENTIFIED = {
  CSbr: cite("ASBR", "Rubble", 2819, 2881),
  ASbr: cite("ASBR", "Rubble", 2819, 761),
  CRfs: cite("BRFS", "Fissure", 2829, 2681),
  CObo: cite("NOBK", "Obelisk Broken", 2938, 2281),
  ZWsw: cite("ZWSW", "Seaweed", 3201, 18561),
  LOsh: cite("LOSH", "Head on Spear", 2879, 7281),
  LPcr: cite("LPCR", "Corn", 2842, 7881),
  YOec: cite("YOEC", "Crates Empty", 2976, 13441),
  NOft: cite("NOFT", "Fire Pit Trashed", 2950, 9201),
  OOgr: cite("OOGR", "Flame Grate", 3336, 10401),
  ZObz: cite("ZOBR", "Ruins Brazier", 3105, 16841),
  IWbg: cite("IWBG", "Bubbles Steam", 3204, 5401),
  COhs: cite("APHS", "Stump Hollow", 2802, 2361),
  ASpr: cite("ASPR", "Pier", 2817, 801),
  LSwm: cite("LSWM", "Windmill", 2909, 8761),
  LOct: cite("LOCT", "Cage Trashed", 2863, 6641),
  LOt1: cite("LOT1", "Trash", 2885, 7521),
  DOtp: cite("DOTP", "Pile of Treasure", 3072, 3761),
  NOgv: cite("NOGV", "Grave Stone", 2944, 9241),
  DOtb: cite("DOTB", "Table", 3083, 3681),
  OTip: destructable("INVISIBLE_PLATFORM", 7673),
  JTtw: destructable("DALARAN_RUINS_TREE_WALL", 4145),
  YTfb: destructable("PATHBLOCKER_BOTH", 11929),
};

const rows = (p) => {
  const [header, ...rest] = fs.readFileSync(p, "utf8").trim().split("\n").map((l) => l.split("\t"));
  const col = Object.fromEntries(header.map((h, i) => [h, i]));
  return rest.map((r) => Object.fromEntries(Object.entries(col).map(([h, i]) => [h, r[i]])));
};

const w3 = rows(typesPath);
const g = new Map(rows(path.join(dir, "doodad-types-688g.tsv")).map((r) => [r.type, r]));
const kindOf = new Map([...g].map(([t, r]) => [t, r.tree_rule === "tree" ? "" : r.shipped_kind]));
for (const [code, named] of Object.entries(NAMED)) kindOf.set(code, named.kind);
for (const [code, o] of Object.entries(OVERRIDE)) kindOf.set(code, o.kind);

const out = ["type\count\ttree_rule\tname\tshipped_kind\treason"];
let reused = 0;
let fromParent = 0;
let namedCount = 0;
let overridden = 0;
let identifiedOnly = 0;
let unverified = 0;
let trees = 0;
const markSilent = (code, why) => (why.startsWith("UNVERIFIED") || (g.has(code) && !OVERRIDE[code]) ? why : `UNVERIFIED (6.88g silent): ${why}`);
for (const r of w3) {
  const override = OVERRIDE[r.type];
  if (override) {
    if (override.kind) { overridden += 1; out.push([r.type, r.count, "", "", override.kind, markSilent(r.type, override.why)].join("\t")); }
    else { identifiedOnly += 1; out.push([r.type, r.count, "", "", "", markSilent(r.type, override.why)].join("\t")); }
    continue;
  }
  const prior = g.get(r.type);
  if (prior && prior.tree_rule === "tree") {
    trees += 1;
    out.push([r.type, r.count, "tree", prior.name ?? "", "tree", `${prior.reason} (6.88g tree layer kept)`].join("\t"));
    continue;
  }
  if (prior) {
    reused += 1;
    out.push([r.type, r.count, prior.tree_rule ?? "", prior.name ?? "", prior.shipped_kind, `6.88g mapped this code to ${prior.shipped_kind}`].join("\t"));
    continue;
  }
  const named = NAMED[r.type];
  if (named) {
    namedCount += 1;
    out.push([r.type, r.count, "", "", named.kind, markSilent(r.type, named.why)].join("\t"));
    continue;
  }
  const parentCode = PARENT[r.type];
  const parentKind = parentCode && kindOf.get(parentCode);
  if (parentKind) {
    fromParent += 1;
    out.push([r.type, r.count, "", "", parentKind, markSilent(r.type, `custom code: a ${parentCode} variant, which ships as ${parentKind}`)].join("\t"));
    continue;
  }
  const identity = IDENTIFIED[r.type];
  if (identity) { identifiedOnly += 1; out.push([r.type, r.count, "", "", "", markSilent(r.type, identity)].join("\t")); continue; }
  if (parentCode) {
    const parentText = IDENTIFIED[parentCode] ? `${parentCode} is ${IDENTIFIED[parentCode]}` : `${parentCode} is UNVERIFIED`;
    if (IDENTIFIED[parentCode]) identifiedOnly += 1; else unverified += 1;
    out.push([r.type, r.count, "", "", "", markSilent(r.type, `custom code: a ${parentCode} variant; ${parentText}, which has no declared kind, so dropped`)].join("\t"));
    continue;
  }
  unverified += 1;
  out.push([r.type, r.count, "", "", "", "UNVERIFIED: no name in the 6.88g table, the w3x2lni doodad or destructable tables, or the WorldEditStrings lookup; its placements are dropped"].join("\t"));
}

const target = path.join(dir, "doodad-types-w3.tsv");
fs.writeFileSync(target, out.join("\n") + "\n");
const placements = { shipped: 0, dropped: 0 };
const droppedCodes = [];
for (const line of out.slice(1)) {
  const [type, count, , , kind] = line.split("\t");
  if (kind) placements.shipped += Number(count);
  else { placements.dropped += Number(count); droppedCodes.push(`${type}:${count}`); }
}
console.log(`placement coverage: ${placements.shipped} shipped, ${placements.dropped} dropped (${droppedCodes.join(" ")})`);
console.log(
  `${w3.length} codes: ${reused} reused from 6.88g, ${namedCount} identified by name, ${overridden} corrected from the editor strings, ${fromParent} from the custom-code parent map, ` +
    `${trees} trees (kept from 6.88g), ${identifiedOnly} identified with no declared kind, ${unverified} UNVERIFIED\nwritten to ${target}`,
);
