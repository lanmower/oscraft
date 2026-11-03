import fs from "node:fs";
import path from "node:path";

export const TICKS_PER_SECOND = 16;

export const norm = (text) => text.toLowerCase().replace(/[^a-z0-9]/g, "");

export function readRows(file) {
  return fs.readFileSync(file, "utf8").split("\n").slice(1).filter(Boolean).map((line) => line.split("\t"));
}

export function provenHeroes(moba) {
  const rows = readRows(path.join(moba, "heroes_688g.tsv"));
  return new Map(rows.filter(([, src]) => src && src !== "-").map(([id, src]) => [id, src]));
}

export function definedAbilityNames(moba, skipFile) {
  const names = new Set();
  for (const file of fs.readdirSync(moba).filter((f) => f.startsWith("abilities") && f.endsWith(".osd") && f !== skipFile)) {
    for (const line of fs.readFileSync(path.join(moba, file), "utf8").split("\n")) {
      const hit = /^ability\("([^"]+)"/.exec(line);
      if (hit) names.add(norm(hit[1]));
    }
  }
  return names;
}

export function perlevelByUnit(moba) {
  const byUnit = new Map();
  for (const row of readRows(path.join(moba, "abilities_688g_perlevel.tsv"))) {
    const key = `${row[0]}\t${row[3]}`;
    byUnit.set(key, row);
  }
  return byUnit;
}

export function parseLevels(text) {
  return String(text ?? "").trim().split("|").filter((t) => t !== "").map(Number);
}

export function parseDataLabels(text) {
  const labels = new Map();
  for (const part of String(text ?? "").split(/\s{2}(?=Data[A-Z]\[)/)) {
    const hit = /^Data([A-Z])\[([^\]]*)\]=(.+)$/.exec(part.trim());
    if (hit) labels.set(hit[3].trim(), hit[2].split("/").map(Number));
  }
  return labels;
}

export const TARGET_BY_CODE = ["none", "unit", "point", "unit_or_point"];

export const LABEL_EFFECTS = {
  初始伤害: { kind: "damage", scale: 1 },
  晕眩持续时间: { kind: "stun", scale: TICKS_PER_SECOND },
};

export const ALIAS = {
  nevermore_requiem: "A29J",
  bloodseeker_thirst: "A0I8",
  drow_ranger_trueshot: "A2O2",
  lone_druid_spirit_bear: "A0A5",
  mirana_arrow: "A0L8",
  nevermore_dark_lord: "A0FU",
  ogre_magi_unrefined_fireblast: "A04W",
  queenofpain_shadow_strike: "A0Q7",
  rattletrap_power_cogs: "A0Z5",
  skeleton_king_reincarnation: "A01Y",
  techies_suicide: "A06B",
  tusk_launch_snowball: "A1S7",
  wisp_spirits_out: "A1T8",
};

export const SKIP_REASON = {
  tiny_toss_tree: "tree variant of Toss (A0BZ); the same 6.88g ability",
  windrunner_windrun: "6.88g Windrunner (A14I) is a no-cooldown trigger named after the hero, not a verified Windrun",
  ancient_apparition_ice_blast_release: "release sub-state of Ice Blast (A1MI)",
  shredder_return_chakram: "return sub-state of Chakram (A2E5)",
  visage_summon_familiars_stone_form: "stone-form sub-state of Summon Familiars (A1NE)",
  nyx_assassin_impale: "duplicate of nyx_assassin_impale_nyx, already defined",
  troll_warlord_whirling_axes_melee: "melee variant of Whirling Axes (A21L)",
  troll_warlord_whirling_axes_ranged: "ranged variant of Whirling Axes (A21L)",
  morphling_adaptive_strike_agi: "agility variant of Adaptive Strike (A0G6)",
  morphling_morph_agi: "agility variant of Morph (A0KX)",
  morphling_morph_str: "strength variant of Morph (A0KX)",
  morphling_morph_replicate: "Replicate (A0G8) is a separate 6.88g ability; the Valve name is its morph form",
  alchemist_unstable_concoction_throw: "throw sub-state of Unstable Concoction (A1NI)",
  death_prophet_silence: "6.88g A0P6 is named Silence 0; the mapping is not verified",
  ember_spirit_fire_remnant: "6.88g A2JK is the Fire Remnant learn row, not the cast",
  phoenix_fire_spirits: "6.88g A35E is the Fire Spirits learn row, not the cast",
  wisp_tether: "6.88g A35G is the Tether learn row, not the cast",
  jakiro_liquid_fire: "6.88g A2VO is the Liquid Fire research row, not the cast",
  naga_siren_song_of_the_siren_cancel: "cancel sub-state of Song of the Siren (A07U)",
  nevermore_shadowraze2: "second Shadowraze variant of Shadowraze (A0EY)",
  bane_nightmare_end: "end sub-state of Nightmare (A04Y)",
  crystal_maiden_freezing_field_stop: "stop sub-state of Freezing Field (A03R)",
  keeper_of_the_light_illuminate_end: "end sub-state of Illuminate (A085)",
  phoenix_icarus_dive_stop: "stop sub-state of Icarus Dive (A1RJ)",
  phoenix_sun_ray_stop: "stop sub-state of Sun Ray (A1YY)",
  windrunner_focusfire_cancel: "cancel sub-state of Focus Fire (A12P)",
  rubick_telekinesis_land: "land sub-state of Telekinesis (A27F); rubick rows are excluded",
  rubick_telekinesis_land_self: "land sub-state of Telekinesis (A27F); rubick rows are excluded",
};

export function classify(hero, unitCode, name, present, perlevel) {
  const key = norm(name);
  if (name === "" || name === "generic_hidden" || name.startsWith("rubick_hidden")) {
    return { status: "EXCLUDED", code: "", reason: "Valve placeholder slot, not a hero ability" };
  }
  if (present.has(key)) return { status: "PRESENT", code: "", reason: "" };
  if (ALIAS[name]) {
    const code = ALIAS[name];
    const row = perlevel.get(`${unitCode}\t${code}`);
    if (row) return { status: "ROW", code, reason: "" };
    return { status: "UNVERIFIED", code: "", reason: `ALIAS ${code} is not in the 6.88g rows of ${unitCode}` };
  }
  if (SKIP_REASON[name]) return { status: "SKIP", code: "", reason: SKIP_REASON[name] };
  return { status: "UNVERIFIED", code: "", reason: `no exact 6.88g name under ${unitCode} for ${hero}` };
}
