import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const pack = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const KEYS = ["backswing_ms", "missile_homing", "missile_arc"];
const rows = fs.readFileSync(path.join(pack, "weapon_feel_688g.tsv"), "utf8").split("\n").filter((l) => l && !l.startsWith("#"));
const head = rows[0].split("\t");
const feel = new Map(rows.slice(1).map((l) => {
  const r = Object.fromEntries(l.split("\t").map((v, i) => [head[i], v]));
  const args = [];
  if (Number(r.backswing_ms) > 0) args.push(`backswing_ms=${r.backswing_ms}`);
  if (r.homing === "1") args.push("missile_homing=True");
  if (Number(r.arc) > 0) args.push(`missile_arc=${r.arc}`);
  return [r.id, args];
}));
let changed = 0;
for (const file of ["units_extra.osd", "units_heroes.osd", "units_lane.osd"]) {
  const p = path.join(pack, file);
  if (!fs.existsSync(p)) continue;
  const lines = fs.readFileSync(p, "utf8").split("\n").map((line) => {
    const m = /^unit_extra\("(\w+)", /.exec(line);
    if (!m || !feel.has(m[1])) return line;
    const bare = line.replace(new RegExp(`, (?:${KEYS.join("|")})=[^,)]+`, "g"), "");
    const args = feel.get(m[1]);
    const next = args.length ? bare.replace(/\)\s*$/, `, ${args.join(", ")})`) : bare;
    if (next !== line) changed++;
    return next;
  });
  fs.writeFileSync(p, lines.join("\n"));
}
console.log(`apply-weapon-feel: ${changed} unit_extra lines changed`);
