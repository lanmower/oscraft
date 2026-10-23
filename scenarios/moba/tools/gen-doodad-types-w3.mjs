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

// The custom block, as the map's doodad table resolves it: each is a variant of another code. The
// value is a rawcode, not a kind name, so it is looked up in the 6.88g table to get the kind that code
// already ships as -- D00B is a ZPsh variant and ships as bench, because ZPsh ships as bench.
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

const out = ["type\tcount\ttree_rule\tname\tshipped_kind\treason"];
let reused = 0;
let fromParent = 0;
let unresolved = 0;
let trees = 0;
for (const r of w3) {
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
  const parentCode = PARENT[r.type];
  const parent = parentCode && g.get(parentCode);
  if (parent && parent.tree_rule !== "tree" && parent.shipped_kind) {
    fromParent += 1;
    out.push([r.type, r.count, "", "", parent.shipped_kind, `custom code: a ${parentCode} variant, which 6.88g ships as ${parent.shipped_kind}`].join("\t"));
    continue;
  }
  unresolved += 1;
  out.push([r.type, r.count, "", "", "", "unresolved: no name in either map's doodad table, so its placements are dropped"].join("\t"));
}

const target = path.join(dir, "doodad-types-w3.tsv");
fs.writeFileSync(target, out.join("\n") + "\n");
console.log(
  `${w3.length} codes: ${reused} reused from 6.88g, ${fromParent} from the custom-code parent map, ` +
    `${trees} trees (kept from 6.88g), ${unresolved} unresolved\nwritten to ${target}`,
);
