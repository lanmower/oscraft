#!/usr/bin/env node
// Builds the 6.88w3 doodad type table: every type code of the target map with the doodad kind it
// becomes, for place-doodads-688g.mjs to place.
//
// Usage: node scenarios/moba/tools/gen-doodad-types-w3.mjs [--dir scenarios/moba]
//        [--types /workspace/hy3d/output/map-diff/doodad-types-w3.tsv]
//
// The target map's placements name 100 type codes. A code becomes a doodad line only when the table
// says what it is, so the whole job here is identification:
//   - a code 6.88g also has keeps that map's answer, including its tree rule;
//   - the D00x custom block resolves to a parent kind read from the map's own doodad table, so a
//     lantern variant ships as a lantern rather than as nothing;
//   - a code neither map names is left blank with its reason, and its placements are dropped rather
//     than shipped as a guess.
//
// Trees are marked as trees so the placer skips them: the tree layer carries the terrain's blocked
// cells, and moving it means regenerating pathability too, which is a separate pass.
import fs from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
const flag = (name, fallback) => {
  const hit = args.find((a) => a.startsWith(`--${name}=`));
  return hit ? hit.slice(name.length + 3) : fallback;
};
const dir = flag("dir", "scenarios/moba");
const typesPath = flag("types", "/workspace/hy3d/output/map-diff/doodad-types-w3.tsv");

// Codes the target map has and 6.88g never did, identified by name rather than by a 6.88g row. Each
// ships as the declared kind that already means the same thing, so nothing new is invented: a bundle
// of wheat is a hay bale.
const NAMED = {
  LPwb: { kind: "hay_bale", why: "a bundle of wheat, read from the editor strings" },
};
// Codes 6.88g mapped by inference from where they clustered, where the editor strings give a
// different answer. Direct evidence beats a guess: a glowing brazier is not a gate, and a long fence
// is not a fountain. A code with no declared kind for its meaning is dropped instead of stretched.
const OVERRIDE = {
  APms: { kind: "mushrooms", why: "mushrooms from the English editor strings; 6.88g inferred grass_large" },
  AOnt: { kind: "lantern", why: "totem lantern from the English editor strings; 6.88g inferred rocks" },
  APtv: { kind: "hedge", why: "thorny vines from the English editor strings; no vine kind is declared, so hedge is the nearest declared kind (6.88g inferred grass_large)" },
  AOgs: { kind: "", why: "statue from the English editor strings, but no statue kind is declared; dropped rather than shipped as the fountain 6.88g inferred" },

  LObz: { kind: "torch_glowing", why: "glowing brazier (\u70bd\u70ed\u7684\u706b\u76c6) from the editor strings; 6.88g inferred iron_fence_border_gate" },
  VOfl: { kind: "fence", why: "long fence (\u957f\u7bf1\u7b06) from the editor strings; 6.88g inferred fountain" },
  LWw0: { kind: "", why: "waterfall (\u7011\u5e03) from the editor strings, but no declared kind is a waterfall; dropped rather than shipped as the catwalk 6.88g inferred" },
};
// The custom block, as the map's doodad table resolves it: each is a variant of another code. The
// value is a rawcode, not a kind name, so it is looked up in the kinds below and ships as whatever
// that code already ships as -- D00B is a ZPsh variant and ships as bench, because ZPsh ships as bench.
const PARENT = {
  D00B: "ZPsh", D003: "ZPsh", D000: "NOfl", D001: "AObr", D002: "VOfl", D009: "AObd", D00C: "LPwb",
  D004: "LOlp", D005: "LOlp", D006: "LOlp", D007: "LOlp", D008: "LOlp",
  D00D: "LOlp", D00E: "LOlp", D00F: "LOlp", D00H: "LOlp", D00R: "LOlp",
  D00A: "DOtb", D00I: "DOtb", D00J: "DOtb", D00K: "DOtb", D00L: "DOtb", D00M: "DOtb",
  D00N: "DOtb", D00O: "DOtb", D00P: "DOtb", D00Q: "DOtb",
};

const rows = (p) => {
  const [header, ...rest] = fs.readFileSync(p, "utf8").trim().split("\n").map((l) => l.split("\t"));
  const col = Object.fromEntries(header.map((h, i) => [h, i]));
  return rest.map((r) => Object.fromEntries(Object.entries(col).map(([h, i]) => [h, r[i]])));
};

const w3 = rows(typesPath);
const g = new Map(rows(path.join(dir, "doodad-types-688g.tsv")).map((r) => [r.type, r]));
// What each code ships as: the 6.88g answer where there is one, plus the codes identified by name.
const kindOf = new Map([...g].map(([t, r]) => [t, r.tree_rule === "tree" ? "" : r.shipped_kind]));
for (const [code, named] of Object.entries(NAMED)) kindOf.set(code, named.kind);
// An override also changes what a variant of that code ships as, so it goes into the lookup too.
for (const [code, o] of Object.entries(OVERRIDE)) kindOf.set(code, o.kind);

const out = ["type\tcount\ttree_rule\tname\tshipped_kind\treason"];
let reused = 0;
let fromParent = 0;
let namedCount = 0;
let overridden = 0;
let unresolved = 0;
let trees = 0;
for (const r of w3) {
  const override = OVERRIDE[r.type];
  if (override) {
    if (override.kind) { overridden += 1; out.push([r.type, r.count, "", "", override.kind, override.why].join("\t")); }
    else { unresolved += 1; out.push([r.type, r.count, "", "", "", override.why].join("\t")); }
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
    out.push([r.type, r.count, "", "", named.kind, named.why].join("\t"));
    continue;
  }
  const parentCode = PARENT[r.type];
  const parentKind = parentCode && kindOf.get(parentCode);
  if (parentKind) {
    fromParent += 1;
    out.push([r.type, r.count, "", "", parentKind, `custom code: a ${parentCode} variant, which ships as ${parentKind}`].join("\t"));
    continue;
  }
  unresolved += 1;
  out.push([r.type, r.count, "", "", "", "unresolved: no name in either map's doodad table, so its placements are dropped"].join("\t"));
}

const target = path.join(dir, "doodad-types-w3.tsv");
fs.writeFileSync(target, out.join("\n") + "\n");
console.log(
  `${w3.length} codes: ${reused} reused from 6.88g, ${namedCount} identified by name, ${overridden} corrected from the editor strings, ${fromParent} from the custom-code parent map, ` +
    `${trees} trees (kept from 6.88g), ${unresolved} unresolved\nwritten to ${target}`,
);
