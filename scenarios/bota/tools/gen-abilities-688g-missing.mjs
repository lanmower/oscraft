import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { emitCode, emitSplit } from "./emit.mjs";

const moba = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const check = process.argv.includes("--check");
const read = (file) => fs.readFileSync(path.join(moba, file), "utf8");

const table = (text) => {
  const [head, ...rows] = text.split("\n").filter(Boolean).map((line) => line.split("\t"));
  return rows.map((cells) => Object.fromEntries(head.map((key, i) => [key, cells[i] ?? ""])));
};
const slug = (name) => name.replace(/['’]/g, "").toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "");
const levelList = (text) => (text ? text.split("|").map(Number) : []);
const TICKS_PER_SECOND = 16;
const TARGET_BY_CODE = ["none", "unit", "point", "unit_or_point"];
const EXCLUDED_ABILITIES = new Set(["generic_hidden", "rubick_hidden", ""]);

const SPELL_ATTACK_TYPE_INDEX = 8;
const MAGICAL_DAMAGE_KIND = 2;
const SPELL_DAMAGE_TYPE = MAGICAL_DAMAGE_KIND + 4 * (1 + SPELL_ATTACK_TYPE_INDEX);
const DAMAGE_AMOUNT_LABELS = new Set(["伤害", "目标伤害", "伤害数值", "全伤害数值", "初始伤害"]);
const STUN_DURATION_LABEL = "晕眩持续时间";

const provenHeroes = table(read("hero_identity_688g.tsv")).filter((hero) => hero.identity_status === "PROVEN");
const inventory = table(read("abilities-inventory.tsv"));
const perlevel = table(read("abilities_688g_perlevel.tsv"));
const labelRules = new Map(table(read("abilities_688g_label_effects.tsv")).map((rule) => [rule.label, rule]));
const defined = new Set(
  fs.readdirSync(moba)
    .filter((file) => file.endsWith(".osd") && file !== "abilities_688g_missing.osd")
    .flatMap((file) => [...read(file).matchAll(/^ability\("([^"]+)"/gm)].map((m) => m[1])),
);

const expand = (values, levels) => Array.from({ length: levels }, (_, k) => values[Math.min(k, values.length - 1)] ?? 0);

function dataFields(text) {
  return text.split(/\s{2,}/).flatMap((entry) => {
    const m = /^Data[A-F]\[([^\]]*)\]=(.+)$/.exec(entry.trim());
    return m ? [{ values: m[1].split("/").map(Number), label: m[2].trim() }] : [];
  });
}

function targetOf(row) {
  const field = /DataB\[([^\]]*)\]=目标类型/.exec(row.data);
  if (!field) return null;
  return TARGET_BY_CODE[Number(field[1].split("/")[0])] ?? null;
}

function labelEffect(field, target) {
  const rule = labelRules.get(field.label);
  if (rule?.label_status === "NONE") return { cast: true };
  if (!rule?.effect_kind) return { reason: `${field.label} is UNVERIFIED` };
  if (field.values.every((v) => v === 0)) return { reason: `${field.label} has no value in perlevel` };
  const bindsDamage = DAMAGE_AMOUNT_LABELS.has(field.label) && rule.effect_kind === "damage";
  const bindsStun = field.label === STUN_DURATION_LABEL && rule.effect_kind === "stun";
  if (!bindsDamage && !bindsStun) return { reason: `${field.label} (${rule.effect_kind}) has no role binding` };
  if (target === null) return { reason: `${field.label}: no target code (DataB) in perlevel` };
  if (target !== "unit") return { reason: `${field.label} on a ${target} ability: no area in perlevel` };
  if (bindsStun) {
    return { effect: { kind: "stun", params: [field.values.map((s) => Math.round(s * TICKS_PER_SECOND)), [0], [0], [0]] } };
  }
  return { effect: { kind: "damage", params: [field.values, [0], [SPELL_DAMAGE_TYPE], [0]] } };
}

function abilityRows(id, row) {
  const cooldowns = levelList(row.cool_s).map((s) => Math.round(s * TICKS_PER_SECOND));
  const costs = levelList(row.cost).map((c) => Math.round(c));
  const ultimate = row.hotkey === "R";
  const passive = row.hotkey === "" && cooldowns.length === 0 && costs.length === 0;
  const maxLevels = ultimate ? 3 : 4;
  const levels = Math.min(maxLevels, Math.max(cooldowns.length, costs.length, 1));
  const gates = (ultimate ? [6, 11, 16] : [1, 3, 5, 7]).slice(0, levels);
  const target = targetOf(row);
  const castTarget = target ?? "none";
  const cd = expand(cooldowns, levels);
  const energy = expand(costs, levels);
  const verdicts = dataFields(row.data).map((field) => labelEffect(field, target));
  const reasons = verdicts.flatMap((v) => (v.reason ? [v.reason] : []));
  const bound = verdicts.flatMap((v) => (v.effect ? [v.effect] : []));
  if (bound.filter((e) => e.kind === "damage").length > 1) reasons.push("two damage amount labels");
  const mapped = reasons.length === 0 && bound.length > 0;
  const effects = mapped ? bound : [];
  const effectLines = effects.map((e) => `effect("${e.kind}", ${e.params.map((p, j) => `p${j}=${expand(p, levels)[0]}`).join(", ")})`);
  const scaledLines = effects.flatMap((e, idx) =>
    e.params.flatMap((p, j) => {
      const values = expand(p, levels);
      return values.some((v) => v !== values[0]) ? [`ability_scaled("${id}", ${idx}, ${j}, [${values}])`] : [];
    }),
  );
  const lines = [
    `ability("${id}", ${cd[0]}, ${energy[0]}, "${castTarget}", [${effectLines.join(", ")}])`,
    `skill("${id}", ultimate=${ultimate ? "True" : "False"}, gates=[${gates}], damage=[${expand([0], levels)}], cooldown=[${cd}], energy=[${energy}])`,
  ];
  if (passive) lines.push(`ability_ext("${id}", passive=True)`);
  lines.push(...scaledLines);
  const reason = reasons.length ? reasons.join("; ") : "no effect label with a value in the 6.88g perlevel row";
  return { lines, mapped, reason };
}

const emitted = [];
let abilityCount = 0;
let mappedCount = 0;
const unverified = ["id\thero\tunit_code\treason"];
for (const hero of provenHeroes) {
  const rows = perlevel.filter((row) => row.unit_code === hero.unit_code);
  const ids = inventory
    .filter((row) => row.hero === hero.oscraft_id && row.kind === "ability" && !EXCLUDED_ABILITIES.has(row.ability))
    .map((row) => row.ability);
  for (const id of ids) {
    if (defined.has(id)) continue;
    const suffix = id.startsWith(`${hero.oscraft_id}_`) ? id.slice(hero.oscraft_id.length + 1) : null;
    const match = rows.find((row) => suffix && slug(row.abil_name) === suffix);
    if (match) {
      const built = abilityRows(id, match);
      emitted.push(...built.lines);
      abilityCount += 1;
      if (built.mapped) mappedCount += 1;
      else unverified.push([id, hero.oscraft_id, hero.unit_code, built.reason].join("\t"));
    } else unverified.push([id, hero.oscraft_id, hero.unit_code, `Dota 2 id; no 6.88g perlevel row under ${hero.unit_code} named ${suffix}`].join("\t"));
  }
}

const labelTally = { cast: 0, bound: 0, notMapped: 0 };
for (const row of perlevel) {
  for (const field of dataFields(row.data)) {
    const verdict = labelEffect(field, targetOf(row));
    if (verdict.cast) labelTally.cast += 1;
    else if (verdict.effect) labelTally.bound += 1;
    else labelTally.notMapped += 1;
  }
}

const banner = "# Generated by scenarios/moba/tools/gen-abilities-688g-missing.mjs from abilities-inventory.tsv and abilities_688g_perlevel.tsv. Do not edit by hand.";
const stale = [
  ...emitSplit(moba, "abilities_688g_missing.osd", [banner, ...emitted, ""].join("\n"), check),
  ...emitCode(moba, "abilities_688g_missing_unverified.tsv", unverified.join("\n") + "\n", check),
];
if (check) {
  if (stale.length) {
    console.error(`gen-abilities-688g-missing --check: ${stale.join(", ")} differ; run node scenarios/moba/tools/gen-abilities-688g-missing.mjs`);
    process.exit(1);
  }
  console.log("gen-abilities-688g-missing --check: ok");
} else {
  console.log(`wrote ${abilityCount} abilities: ${mappedCount} with mapped effects, ${unverified.length - 1} unverified`);
  console.log(`label rows: ${labelTally.cast} cast (NONE), ${labelTally.bound} bound to an effect, ${labelTally.notMapped} not mapped`);
}
