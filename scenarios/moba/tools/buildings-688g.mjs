import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { provenanceFile, rowsToTsv } from "./provenance.mjs";

const GX = 35;
const GY = 35;
const UNIT_MILLI = 1000 / 64;
export const toMilli = (wx, wy) => [Math.round((GX * 64 + wx + 8192) * UNIT_MILLI), Math.round((GY * 64 + 8192 - wy) * UNIT_MILLI)];

const SIDE_OF_OWNER = { "ne[0]": 0, "Ve[0]": 1 };

const TOWER_TIER = { e00R: 1, u00M: 1, e011: 2, u00D: 2, e00S: 3, u00N: 3, e019: 4, u00T: 4 };
const RAX_KIND = { eaom: "barracks_melee", usep: "barracks_melee", eaoe: "barracks_ranged", utod: "barracks_ranged" };
const ANCIENT = { etol: 0, unpl: 1 };
const FOUNTAIN = { nfoh: 0, ndfl: 1 };
const OUTBUILDING = { emow: "base_outbuilding", uzig: "base_outbuilding", eaow: "base_outbuilding_large", edob: "base_outbuilding_large", ubon: "base_outbuilding_large", usap: "base_outbuilding_large" };
const LANES = ["top", "mid", "bot"];

const HOME_SHOP = new Set(["n12B", "n12Y", "hC95", "n01K", "nC38", "e025", "n00V", "n00W", "n002", "n00X", "n009", "n0HE", "n12Z", "n12N", "nC35", "u00Q"]);
const SIDE_SHOP = new Set(["u00Z", "u010", "n12K"]);
const SECRET_SHOP = new Set(["uC74"]);

const RELEVANT = (code) => TOWER_TIER[code] || RAX_KIND[code] || ANCIENT[code] !== undefined || FOUNTAIN[code] !== undefined
  || OUTBUILDING[code] || HOME_SHOP.has(code) || SIDE_SHOP.has(code) || SECRET_SHOP.has(code);

function makeEvaluator(lines) {
  const globals = {};
  for (const l of lines) {
    const m = l.match(/^real (\w+)=(.+)$/);
    if (m) { const v = literal(m[2]); if (v !== undefined) globals[m[1]] = v; }
  }
  const units = {};
  function literal(s) {
    s = s.trim();
    if (/^-?0x[0-9a-f]+$/i.test(s)) return s[0] === "-" ? -parseInt(s.slice(1), 16) : parseInt(s, 16);
    if (/^-?\d+(\.\d*)?$/.test(s)) return parseFloat(s);
    return undefined;
  }
  function evaluate(e) {
    e = e.trim();
    const g = e.match(/^GetUnit([XY])\((\w+)\)$/);
    if (g) { const u = units[g[2]]; return u ? u[g[1] === "X" ? "wx" : "wy"] : undefined; }
    const lit = literal(e);
    if (lit !== undefined) return lit;
    const terms = e.match(/[+-]?[^+-]+/g);
    if (terms && terms.length > 1) {
      let sum = 0;
      for (const t of terms) {
        const sign = t.trim()[0] === "-" ? -1 : 1;
        const v = evaluate(t.trim().replace(/^[+-]/, ""));
        if (v === undefined) return undefined;
        sum += sign * v;
      }
      return sum;
    }
    return globals[e];
  }
  return { globals, units, evaluate };
}

export function mainUnits(script) {
  const lines = script.split(/\r?\n/);
  const start = lines.findIndex((l) => l.startsWith("function main takes"));
  if (start < 0) throw new Error("main() not found in the script");
  const { globals, units, evaluate } = makeEvaluator(lines);
  const created = [];
  let owner = "?";
  for (let i = start; i < lines.length && !lines[i].startsWith("endfunction"); i++) {
    const l = lines[i];
    let m;
    if ((m = l.match(/^set p=(.+)$/))) owner = m[1];
    if ((m = l.match(/^set (\w+)=(GetUnit[XY]\(\w+\))$/))) { const v = evaluate(m[2]); if (v !== undefined) globals[m[1]] = v; continue; }
    if ((m = l.match(/^set (\w+)=([-\w.+]+)$/))) { const v = evaluate(m[2]); if (v !== undefined) globals[m[1]] = v; }
    if ((m = l.match(/^call RemoveUnit\((\w+)\)/))) { if (units[m[1]]) units[m[1]].removed = true; }
    if ((m = l.match(/^(?:set (\w+)=)?CreateUnit\(([^,]+),'(\w{4})',([^,]+),([^,]+),([^)]+)\)$/))) {
      const [, v, own, code, xs, ys] = m;
      const wx = evaluate(xs), wy = evaluate(ys);
      if (wx === undefined || wy === undefined) {
        if (RELEVANT(code)) throw new Error(`main line ${i + 1}: coordinates of ${code} do not resolve (${xs}, ${ys})`);
        continue;
      }
      const rec = { code, owner: own === "p" ? owner : own, wx, wy, line: i + 1, removed: false };
      if (v) units[v] = rec;
      created.push(rec);
    }
  }
  return created.filter((u) => !u.removed);
}

function rectCentre(script, name) {
  const m = script.match(new RegExp(`set ${name}=Rect\\((-?[\\d.]+),(-?[\\d.]+),(-?[\\d.]+),(-?[\\d.]+)\\)`));
  if (!m) throw new Error(`rect ${name} not found in the script`);
  const [x0, y0, x1, y1] = m.slice(1).map(Number);
  return { wx: (x0 + x1) / 2, wy: (y0 + y1) / 2, halfW: Math.abs(x1 - x0) / 2, halfH: Math.abs(y1 - y0) / 2 };
}

function heroSpawnRects(script) {
  const fn = script.match(/function jU takes player hT returns nothing[\s\S]*?endfunction/);
  if (!fn || !/if Uu\(hT\)then[\s\S]*?GetRectCenter\(Bn\)[\s\S]*?else[\s\S]*?GetRectCenter\(Xn\)/.test(fn[0])) throw new Error("hero spawn function jU no longer matches Bn (Sentinel) / Xn (Scourge)");
  return [rectCentre(script, "Bn"), rectCentre(script, "Xn")];
}

const SLOT_OFFSET = [[0, 0], [-128, 0], [128, 0], [0, -96], [0, 96]];

export function buildingPlan(script) {
  const all = mainUnits(script);
  const plan = { starts: [], base: [], towers: [], rax: [], outbuildings: [], fountains: [], shops: { home: [], side: [], secret: [] } };
  const tally = {};
  for (const u of all) {
    const side = SIDE_OF_OWNER[u.owner];
    if (ANCIENT[u.code] !== undefined) plan.base.push({ side: ANCIENT[u.code], ...u });
    else if (FOUNTAIN[u.code] !== undefined) plan.fountains.push({ side: FOUNTAIN[u.code], ...u });
    else if (side !== undefined && TOWER_TIER[u.code]) {
      const key = `${side}:${u.code}`;
      const lane = tally[key] ?? 0;
      tally[key] = lane + 1;
      plan.towers.push({ side, tier: TOWER_TIER[u.code], lane, ...u });
    } else if (side !== undefined && RAX_KIND[u.code]) {
      const key = `${side}:${u.code}`;
      const lane = tally[key] ?? 0;
      tally[key] = lane + 1;
      plan.rax.push({ side, kind: RAX_KIND[u.code], lane, ...u });
    } else if (side !== undefined && OUTBUILDING[u.code]) plan.outbuildings.push({ side, kind: OUTBUILDING[u.code], ...u });
    else if (HOME_SHOP.has(u.code)) plan.shops.home.push({ ...u, side: u.wx < 0 ? 0 : 1 });
    else if (SIDE_SHOP.has(u.code) && u.line < 101000) plan.shops.side.push(u);
    else if (SECRET_SHOP.has(u.code) && u.line < 101000) plan.shops.secret.push(u);
  }
  const rects = heroSpawnRects(script);
  for (let k = 0; k < 10; k++) {
    const side = k % 2, slot = Math.floor(k / 2);
    const r = rects[side];
    const [dx, dy] = SLOT_OFFSET[slot];
    if (Math.abs(dx) > r.halfW || Math.abs(dy) > r.halfH) throw new Error("slot offset leaves the hero spawn rect");
    plan.starts.push({ k, side, slot, wx: r.wx + dx, wy: r.wy + dy });
  }
  const need = { base: 2, towers: 22, rax: 12, fountains: 2 };
  for (const [k, n] of Object.entries(need)) if (plan[k].length !== n) throw new Error(`expected ${n} ${k}, the script gives ${plan[k].length}`);
  return plan;
}

const m2 = (u) => toMilli(u.wx, u.wy).join(", ");
const TIER_KIND = { 1: "tower", 2: "tower_t2", 3: "tower_t3", 4: "tower_t4" };

const entry = (line, note) => ({ line, note });

export function buildingEntries(plan) {
  const out = [];
  plan.starts.forEach((s, i) => out.push(entry(`start(${s.k}, ${toMilli(s.wx, s.wy).join(", ")}, team=${s.side})`, i === 0 ? "Hero starts: every Sentinel seat (even player index) takes the centre of the script's Bn rect and every Scourge seat the centre of Xn (jU in war3map.j), spread by SLOT_OFFSET inside the rect because the script stacks all five heroes on the centre." : null)));
  const spawn = (kind, side, u, note) => out.push(entry(`spawn("${kind}", ${side}, ${m2(u)})`, note));
  plan.base.forEach((b, i) => spawn("base", b.side, b, `${i === 0 ? "Buildings from the literal CreateUnit calls of main(); lane order top, mid, bot. Do not edit by hand: scenarios/moba/tools/buildings-688g.mjs. " : ""}ancient ${b.code} at ${b.wx},${b.wy} from the source script`));
  for (const side of [0, 1]) {
    for (const tier of [1, 2, 3, 4]) {
      for (const t of plan.towers.filter((x) => x.side === side && x.tier === tier)) spawn(TIER_KIND[tier], side, t, `${t.code} tier ${tier} ${tier === 4 ? `no. ${t.lane + 1}` : LANES[t.lane]} at ${t.wx},${t.wy}`);
    }
  }
  for (const side of [0, 1]) {
    for (let lane = 0; lane < 3; lane++) {
      for (const r of plan.rax.filter((x) => x.side === side && x.lane === lane).sort((a, b) => (a.kind < b.kind ? -1 : 1))) spawn(r.kind, side, r, `${r.code} ${LANES[lane]} at ${r.wx},${r.wy}`);
    }
  }
  for (const f of plan.fountains) spawn("fountain", f.side, f, `fountain ${f.code} at ${f.wx},${f.wy}`);
  for (const o of plan.outbuildings) spawn(o.kind, o.side, o, `${o.code} at ${o.wx},${o.wy}`);
  return out;
}

export const buildingLines = (plan) => buildingEntries(plan).map((e) => e.line);

export const SHOP_ID_BASE = 5;
export function shopEntries(plan) {
  const out = [];
  let id = SHOP_ID_BASE;
  const put = (kind, list) => { for (const s of list) out.push(entry(`prop(${id++}, "${kind}", ${m2(s)})`, `shop ${s.code} at ${s.wx},${s.wy}`)); };
  put("depot", [...plan.shops.home].sort((a, b) => a.side - b.side || a.line - b.line));
  put("side_depot", [...plan.shops.side].sort((a, b) => a.line - b.line));
  put("secret_depot", [...plan.shops.secret].sort((a, b) => a.line - b.line));
  if (id > 100) throw new Error("shop prop ids overrun the tree id range");
  return out;
}
export const shopLines = (plan) => shopEntries(plan).map((e) => e.line);

export const parseSidecar = (text) => text.split("\n").slice(1).filter(Boolean).map((line) => {
  const [anchor, kind, ...note] = line.split("\t");
  return { anchor, kind, note: note.join("\t") };
});
export const sidecarText = (rows) => rowsToTsv(rows.map((r) => [r.anchor, r.kind, r.note]));
const anchorOfLine = (line) => {
  const call = /^(\w+)\((.*)\)$/.exec(line);
  const positional = call[2].split(", ").filter((a) => !/^\w+=/.test(a)).map((a) => a.replace(/^"|"$/g, ""));
  return [call[1], ...positional.slice(0, 4)].join(":");
};
export const provenanceRows = (plan) => [...buildingEntries(plan), ...shopEntries(plan)].filter((e) => e.note).map((e) => ({ anchor: anchorOfLine(e.line), kind: "source", note: e.note }));

const OWNED_ANCHOR = /^start:|^spawn:(base|tower|tower_t2|tower_t3|tower_t4|barracks_melee|barracks_ranged|fountain|base_outbuilding|base_outbuilding_large):|^prop:\d+:(depot|side_depot|secret_depot):/;
export const applyToSidecar = (rows, plan) => [...rows.filter((r) => !OWNED_ANCHOR.test(r.anchor)), ...provenanceRows(plan)];

const isBuildingLine = (l) => /^start\(|^spawn\("(base|tower|tower_t2|tower_t3|tower_t4|barracks_melee|barracks_ranged|fountain|base_outbuilding|base_outbuilding_large)"/.test(l);
const isShopLine = (l) => /^prop\((\d+), "(depot|side_depot|secret_depot)"/.test(l) && Number(l.match(/^prop\((\d+)/)[1]) < 100;

export function applyToMap(mapText, plan) {
  const src = mapText.split("\n");
  const kept = src.filter((l) => !isBuildingLine(l) && !isShopLine(l));
  const firstTree = kept.findIndex((l) => /^prop\(\d+, "tree"/.test(l));
  const firstStart = src.findIndex((l) => /^start\(/.test(l));
  const at = firstStart >= 0 ? src.slice(0, firstStart).filter((l) => !isBuildingLine(l) && !isShopLine(l)).length : kept.findIndex((l) => l.startsWith("spawn("));
  const insertAt = at >= 0 ? at : (firstTree >= 0 ? firstTree : kept.length);
  const out = [...kept.slice(0, insertAt), ...buildingLines(plan), ...kept.slice(insertAt)];
  const shopAt = out.findIndex((l) => /^prop\(\d+, "tree"/.test(l));
  const withShops = [...out.slice(0, shopAt >= 0 ? shopAt : out.length), ...shopLines(plan), ...out.slice(shopAt >= 0 ? shopAt : out.length)];
  return withShops.join("\n");
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const opt = (name, dflt) => (args.includes(name) ? args[args.indexOf(name) + 1] : dflt);
  const scriptPath = opt("--script", null);
  if (!scriptPath) throw new Error("--script <war3map.j> is required");
  const mapPath = opt("--map", "scenarios/moba/map.osd");
  const plan = buildingPlan(fs.readFileSync(scriptPath, "latin1"));
  const current = fs.readFileSync(mapPath, "utf8");
  const next = applyToMap(current, plan);
  const sidecarPath = provenanceFile(path.dirname(mapPath), path.basename(mapPath));
  const sidecarNow = fs.existsSync(sidecarPath) ? fs.readFileSync(sidecarPath, "utf8") : null;
  const sidecarNext = sidecarText(applyToSidecar(sidecarNow === null ? [] : parseSidecar(sidecarNow), plan));
  if (args.includes("--write")) {
    fs.mkdirSync(path.dirname(sidecarPath), { recursive: true });
    fs.writeFileSync(mapPath, next);
    fs.writeFileSync(sidecarPath, sidecarNext);
    console.log(JSON.stringify({ wrote: mapPath, towers: plan.towers.length, rax: plan.rax.length, outbuildings: plan.outbuildings.length, shops: Object.fromEntries(Object.entries(plan.shops).map(([k, v]) => [k, v.length])) }));
  } else if (next !== current || sidecarNext !== sidecarNow) {
    console.error(`${mapPath} or ${sidecarPath} differs from the building lines and rows derived from ${scriptPath}; run with --write`);
    process.exit(1);
  } else console.log("buildings-688g: map.osd matches the script");
}
