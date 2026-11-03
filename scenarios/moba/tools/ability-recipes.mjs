
const TYPE = { legacy: 0, physical: 1, magical: 2, pure: 3 };
const ATTACK = ["normal", "concussive", "explosive", "pierce", "siege", "magic", "chaos", "hero", "spells"];
const typeCode = (name) => {
  const [kind, attack] = name.split(":");
  return TYPE[kind] + (attack ? 4 * (ATTACK.indexOf(attack) + 1) : 0);
};
const STAT = {
  damage: 0, armor: 1, health: 2, speed: 3, range: 4, evasion: 5, crit_chance: 6, crit_mult: 7, cleave: 8, cleave_radius: 9,
  lifesteal: 10, magic_resist: 11, attack_speed: 12, mana_regen: 13, health_regen: 14, max_mana: 15, spell_amp: 16,
  bash_chance: 17, bash_ticks: 18, speed_pct: 19, cdr: 20, status_resist: 21, strength: 22, agility: 23, intelligence: 24,
  spell_lifesteal: 25, armor_x100: 26, cast_range: 27, cleave_w0: 28, cleave_w1: 29, damage_reduction: 30, attack_bonus: 31,
};
const STATUS = {
  taunt: 5, hex: 6, disarm: 7, spell_immunity: 8, invulnerable: 9, break: 10, mute: 11, physical_immunity: 12, death_prevention: 13, blind: 14, revealed: 15,
};
const TEAM = { enemy: 0, ally: 1, allyOther: 2, any: 3 };
const GLOBAL = 400000;

const count = (effects) => effects.reduce((n, e) => n + 1 + (e.nested ? count(e.nested) : 0), 0);

export const H = {
  V: (v, conv = "int", o = {}) => ({ v, conv, ...o }),
  dmg: (amount, type, radius = 0) => ({ k: "damage", p: [amount, radius, typeCode(type), 0] }),
  heal: (amount) => ({ k: "heal", p: [amount, 0, 1, 0] }),
  healPct: (hundredths) => ({ k: "heal", p: [hundredths, 0, 2, 0] }),
  stun: (t) => ({ k: "stun", p: [t, 0, 0, 0] }),
  silence: (t) => ({ k: "silence", p: [t, 0, 0, 0] }),
  root: (t) => ({ k: "root", p: [t, 0, 0, 0] }),
  slow: (t, mult) => ({ k: "slow", p: [t, mult, 0, 0] }),
  status: (name, t, extra = 0) => ({ k: "status", p: [STATUS[name], t, extra, 0] }),
  tstat: (stat, amount, t, pct = 0) => ({ k: "timed_stat", p: [STAT[stat], amount, t, pct] }),
  stat: (stat, amount) => ({ k: "stat", p: [STAT[stat], amount, 0, 0] }),
  dot: (amount, period, duration, type) => ({ k: "dot", p: [amount, period, duration, typeCode(type)] }),
  chain: (amount, jumps, radius, type) => ({ k: "chain", p: [amount, jumps, radius, typeCode(type)] }),
  mana: (amount, mode, damagePercent = 0, type = "legacy") => ({ k: "mana", p: [amount, mode, damagePercent, typeCode(type)] }),
  illusion: (count, duration, out, taken) => ({ k: "illusion", p: [count, duration, out, taken] }),
  move: (cap) => ({ k: "move", p: [cap, 0, 0, 0] }),
  dispel: (mask) => ({ k: "dispel", p: [mask, 0, 0, 0] }),
  attack: (pct = 100, steal = 0) => ({ k: "attack", p: [pct, steal, 0, 0] }),
  execute: (threshold, damage, type, resetOnHeroKill = false) => ({ k: "execute", p: [threshold, damage, typeCode(type), resetOnHeroKill ? 1 : 0] }),
  ward: (unit, life) => ({ k: "ward", p: [0, life, 0, 0], unit }),
  zone: (ability, period, life, o = {}) => ({ k: "zone", p: [0, period, life, (o.follow ? 1 << 30 : 0) | (o.pin ? 1 << 29 : 0) | (o.delay ?? 0)], ability }),
  ifKill: (nested) => ({ k: "if_kill", p: [count(nested), 0, 0, 0], nested }),
  self: (nested) => ({ k: "aoe", p: [0, 16, count(nested), 0], nested }),
  aoe: (radius, team, nested, o = {}) => {
    const bits = (o.filter ?? 0) | (o.heroes ? 1 << 30 : 0) | ((o.taper ?? 0) << 24);
    const cap = o.max ?? 0;
    const p3 = typeof cap === "object" ? { ...cap, add: bits } : bits | (cap << 16);
    return { k: "aoe", p: [radius, TEAM[team] | (o.center ? 4 : 0) | (o.line ? 8 : 0), count(nested), p3], nested };
  },
  T: (...seconds) => seconds.map((x) => Math.round(x * 16)),
  D: (...units) => units.map((x) => Math.round(x * 15.625)),
  SLOWREM: (...percent) => percent.map((x) => Math.round(65536 * (1 - x / 100))),
  GLOBAL,
};
const { T, D, SLOWREM } = H;
const { V, dmg, heal, healPct, stun, root, slow, status, tstat, stat, dot, chain, mana, move, attack, execute, ward, zone, ifKill, aoe, illusion, dispel } = H;

export const HEROES = {
  axe: ["axe_berserkers_call", "axe_battle_hunger", "axe_counter_helix", "axe_culling_blade", "@talents"],
  crystal_maiden: ["crystal_maiden_crystal_nova", "crystal_maiden_frostbite", "crystal_maiden_brilliance_aura", "crystal_maiden_freezing_field", "@talents"],
  sven: ["sven_storm_bolt", "sven_great_cleave", "sven_warcry", "sven_gods_strength", "@talents"],
  juggernaut: ["juggernaut_healing_ward", "juggernaut_blade_fury", "juggernaut_blade_dance", "juggernaut_omni_slash", "@talents"],
  zuus: ["zuus_arc_lightning", "zuus_static_field", "zuus_lightning_bolt", "zuus_thundergods_wrath", "@talents"],
  sniper: ["sniper_assassinate", "sniper_take_aim", "sniper_headshot", "sniper_shrapnel", "@talents"],
  lion: ["lion_impale", "lion_voodoo", "lion_mana_drain", "lion_finger_of_death", "@talents"],
  phantom_lancer: ["phantom_lancer_phantom_rush", "phantom_lancer_spirit_lance", "phantom_lancer_doppelganger", "phantom_lancer_juxtapose", "@talents"],
  dazzle: ["dazzle_shallow_grave", "dazzle_shadow_wave", "dazzle_weave", "dazzle_poison_touch", "@talents"],
  antimage: ["antimage_spell_shield", "antimage_mana_break", "antimage_blink", "antimage_mana_void", "@talents"],
  lina: ["lina_dragon_slave", "lina_light_strike_array", "lina_fiery_soul", "lina_laguna_blade", "@talents"],
  lich: ["lich_dark_ritual", "lich_frost_nova", "lich_frost_armor_lich", "lich_chain_frost", "@talents"],
  tidehunter: ["tidehunter_ravage", "tidehunter_anchor_smash", "tidehunter_gush", "tidehunter_kraken_shell", "@talents"],
  slardar: ["slardar_sprint", "slardar_slithereen_crush", "slardar_bash", "slardar_amplify_damage", "@talents"],
  vengefulspirit: ["vengefulspirit_magic_missile", "vengefulspirit_command_aura", "vengefulspirit_wave_of_terror", "vengefulspirit_nether_swap", "@talents"],
  omniknight: ["omniknight_purification", "omniknight_repel", "omniknight_guardian_angel", "omniknight_degen_aura", "@talents"],
};

export const HELPERS = {
  juggernaut_blade_fury_pulse: "juggernaut_blade_fury",
  juggernaut_omni_slash_pulse: "juggernaut_omni_slash",
  juggernaut_healing_ward_aura: "juggernaut_healing_ward",
  sniper_shrapnel_pulse: "sniper_shrapnel",
  lina_light_strike_array_pulse: "lina_light_strike_array",
  lina_laguna_blade_hit: "lina_laguna_blade",
  lion_finger_of_death_hit: "lion_finger_of_death",
  lion_impale_line: "lion_impale",
};

export const SUMMON_UNITS = [
  "# Summoned units of the hero abilities. juggernaut_healing_ward is 6.88g o00C (unit.ini:323284, parent ohwd): spd 420 (unit.ini:323391) = 410 milli per tick.",
  "# Bounty: the script pays only its UUR table (war3map.j 91684-92300), which has no row for o00C or ohwd, so a kill pays 0 gold and 0 xp; an allied kill halves the table xp for any unit (W_R 93567), so deny_xp_pct is 50.",
  'unit("juggernaut_healing_ward", 1, 410, 250, abilities=["juggernaut_healing_ward_aura"], filters=["ground", "light"], sight_milli=9375, model="pillar-obelisk", deny_pct=50)',
  'unit_extra("juggernaut_healing_ward", reward=0, xp_reward=0, deny_xp_pct=50)',
  'unit("necronomicon_archer_1", 700, 322, 375, abilities=["necronomicon_mana_burn_1", "necronomicon_endurance_aura_1", "necronomicon_spell_damage_reduction"], filters=["ground", "light", "army"], weapon_damage=60, weapon_range_milli=5469, weapon_period=16, sight_milli=20312, defense=4, health_regen=312, model="ranged-creep-skinned", deny_pct=50)',
  'unit_extra("necronomicon_archer_1", reward=100, deny_xp_pct=0, night_sight_milli=12500)',
  'unit("necronomicon_warrior_1", 700, 342, 375, abilities=["necronomicon_mana_break_1", "necronomicon_spell_damage_reduction"], filters=["ground", "light", "army"], weapon_damage=45, weapon_range_milli=1562, weapon_period=12, sight_milli=20312, defense=4, health_regen=312, model="melee-creep-skinned", deny_pct=50)',
  'unit_extra("necronomicon_warrior_1", reward=100, deny_xp_pct=0, night_sight_milli=12500)',
  'unit("necronomicon_archer_2", 800, 352, 375, abilities=["necronomicon_mana_burn_2", "necronomicon_endurance_aura_2", "necronomicon_spell_damage_reduction"], filters=["ground", "light", "army"], weapon_damage=90, weapon_range_milli=7031, weapon_period=16, sight_milli=21875, defense=4, health_regen=312, model="ranged-creep-skinned", deny_pct=50)',
  'unit_extra("necronomicon_archer_2", reward=150, deny_xp_pct=0, night_sight_milli=12500)',
  'unit("necronomicon_warrior_2", 800, 342, 375, abilities=["necronomicon_mana_break_2", "necronomicon_spell_damage_reduction"], filters=["ground", "light", "army"], weapon_damage=60, weapon_range_milli=1562, weapon_period=12, sight_milli=21875, defense=4, health_regen=312, model="melee-creep-skinned", deny_pct=50)',
  'unit_extra("necronomicon_warrior_2", reward=150, deny_xp_pct=0, night_sight_milli=12500)',
  'unit("necronomicon_archer_3", 900, 381, 375, abilities=["necronomicon_mana_burn_3", "necronomicon_endurance_aura_3", "necronomicon_spell_damage_reduction"], filters=["ground", "light", "army"], weapon_damage=120, weapon_range_milli=8594, weapon_period=16, sight_milli=23437, defense=4, health_regen=312, model="ranged-creep-skinned", deny_pct=50)',
  'unit_extra("necronomicon_archer_3", reward=200, deny_xp_pct=0, night_sight_milli=12500)',
  'unit("necronomicon_warrior_3", 900, 342, 375, abilities=["necronomicon_mana_break_3", "necronomicon_spell_damage_reduction"], filters=["ground", "light", "army"], weapon_damage=75, weapon_range_milli=1562, weapon_period=12, sight_milli=23437, defense=4, health_regen=312, model="melee-creep-skinned", deny_pct=50)',
  'unit_extra("necronomicon_warrior_3", reward=200, deny_xp_pct=0, night_sight_milli=12500, detector_radius=15625)',  'unit("shadow_shaman_serpent_ward_1", 2, 0, 250, filters=["ground", "light"], weapon_damage=42, weapon_range_milli=9375, weapon_period=24, sight_milli=18750, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("shadow_shaman_serpent_ward_1", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("shadow_shaman_serpent_ward_2", 2, 0, 250, filters=["ground", "light"], weapon_damage=57, weapon_range_milli=9375, weapon_period=24, sight_milli=18750, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("shadow_shaman_serpent_ward_2", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("shadow_shaman_serpent_ward_3", 2, 0, 250, filters=["ground", "light"], weapon_damage=72, weapon_range_milli=9375, weapon_period=24, sight_milli=18750, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("shadow_shaman_serpent_ward_3", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("venomancer_plague_ward_1", 75, 0, 250, filters=["ground", "light"], weapon_damage=11, weapon_range_milli=9375, weapon_period=16, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("venomancer_plague_ward_1", reward=15, xp_reward=0, deny_xp_pct=0, missile_speed=1900)',
  'unit("venomancer_plague_ward_2", 200, 0, 250, filters=["ground", "light"], weapon_damage=21, weapon_range_milli=9375, weapon_period=16, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("venomancer_plague_ward_2", reward=15, xp_reward=0, deny_xp_pct=0, missile_speed=1900)',
  'unit("venomancer_plague_ward_3", 325, 0, 250, filters=["ground", "light"], weapon_damage=32, weapon_range_milli=9375, weapon_period=16, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("venomancer_plague_ward_3", reward=15, xp_reward=0, deny_xp_pct=0, missile_speed=1900)',
  'unit("venomancer_plague_ward_4", 450, 0, 250, filters=["ground", "light"], weapon_damage=42, weapon_range_milli=9375, weapon_period=16, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("venomancer_plague_ward_4", reward=15, xp_reward=0, deny_xp_pct=0, missile_speed=1900)',
  'unit("techies_land_mine", 1, 0, 125, filters=["ground", "light"], sight_milli=1000, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("techies_land_mine", reward=0, xp_reward=0, deny_xp_pct=0, cloak=["cloaked"])',
  'unit("techies_stasis_trap_ward", 1, 0, 125, filters=["ground", "light"], sight_milli=1000, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("techies_stasis_trap_ward", reward=0, xp_reward=0, deny_xp_pct=0, cloak=["cloaked"])',
  'unit("storm_spirit_static_remnant_unit", 1, 0, 125, filters=["ground", "light"], sight_milli=1000, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("storm_spirit_static_remnant_unit", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("witch_doctor_death_ward_1", 200, 0, 250, filters=["ground", "light"], weapon_damage=60, weapon_range_milli=10938, weapon_period=4, sight_milli=18750, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("witch_doctor_death_ward_1", reward=0, xp_reward=0, deny_xp_pct=0, missile_speed=1000)',
  'unit("witch_doctor_death_ward_2", 200, 0, 250, filters=["ground", "light"], weapon_damage=105, weapon_range_milli=10938, weapon_period=4, sight_milli=18750, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("witch_doctor_death_ward_2", reward=0, xp_reward=0, deny_xp_pct=0, missile_speed=1000)',
  'unit("witch_doctor_death_ward_3", 200, 0, 250, filters=["ground", "light"], weapon_damage=150, weapon_range_milli=10938, weapon_period=4, sight_milli=18750, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("witch_doctor_death_ward_3", reward=0, xp_reward=0, deny_xp_pct=0, missile_speed=1000)',
  'unit("weaver_beetle_1", 3, 350, 125, filters=["ground", "light"], weapon_damage=20, weapon_range_milli=2000, weapon_period=22, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("weaver_beetle_1", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("weaver_beetle_2", 3, 350, 125, filters=["ground", "light"], weapon_damage=20, weapon_range_milli=2000, weapon_period=20, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("weaver_beetle_2", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("weaver_beetle_3", 4, 350, 125, filters=["ground", "light"], weapon_damage=20, weapon_range_milli=2000, weapon_period=18, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("weaver_beetle_3", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("weaver_beetle_4", 4, 350, 125, filters=["ground", "light"], weapon_damage=20, weapon_range_milli=2000, weapon_period=15, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("weaver_beetle_4", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("visage_familiar_1", 30000, 380, 125, filters=["ground", "light"], weapon_damage=10, weapon_range_milli=2500, weapon_period=6, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("visage_familiar_1", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("visage_familiar_2", 30000, 390, 125, filters=["ground", "light"], weapon_damage=10, weapon_range_milli=2500, weapon_period=6, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("visage_familiar_2", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("visage_familiar_3", 30000, 400, 125, filters=["ground", "light"], weapon_damage=10, weapon_range_milli=2500, weapon_period=6, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("visage_familiar_3", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("undying_zombie", 1, 375, 125, filters=["ground", "light"], weapon_damage=30, weapon_range_milli=1562, weapon_period=26, sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("undying_zombie", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("undying_tombstone_unit", 30000, 0, 125, filters=["ground", "light"], sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("undying_tombstone_unit", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("tusk_frozen_sigil_unit", 200, 0, 125, filters=["ground", "light"], sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("tusk_frozen_sigil_unit", reward=0, xp_reward=0, deny_xp_pct=0)',
  'unit("techies_remote_mine", 1, 0, 125, filters=["ground", "light"], sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("techies_remote_mine", reward=0, xp_reward=0, deny_xp_pct=0, cloak=["cloaked"])',
  'unit("templar_assassin_psionic_trap_unit", 1, 0, 125, filters=["ground", "light"], sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("templar_assassin_psionic_trap_unit", reward=0, xp_reward=0, deny_xp_pct=0, cloak=["cloaked"])',
];

export const RECIPES = {
  axe_berserkers_call: () => ({
    target: "none",
    flags: ["pierce"],
    v688: { cd: [16, 14, 12, 10], mana: [80, 90, 100, 110], castPoint: 0.1 },
    effects: [aoe(5078, "enemy", [status("taunt", [32, 38, 45, 51])]), tstat("armor", 40, [32, 38, 45, 51])],
  }),
  axe_battle_hunger: () => ({
    target: "unit",
    flags: ["enemy_only"],
    v688: { cd: [20, 15, 10, 5], mana: [75, 75, 75, 75], range: [750] },
    effects: [dot([15, 20, 25, 30], 16, 160, "magical:spells"), slow(160, 57672)],
  }),
  axe_counter_helix: () => ({
    target: "none",
    flags: ["passive", "proc_attacked"],
    v688: { cd: [0.45, 0.4, 0.35, 0.3], mana: [0], levels: 4 },
    ext: { chance: 20 },
    effects: [dmg([100, 135, 170, 205], "physical", 4688)],
  }),
  axe_culling_blade: () => ({
    target: "unit",
    flags: ["pierce", "enemy_only"],
    v688: { cd: [75, 65, 55], mana: [60, 120, 180], range: [150], levels: 3, ultimate: true },
    effects: [
      execute([250, 325, 400], [150, 250, 300], "magical:spells", true),
      ifKill([aoe(14453, "ally", [tstat("speed_pct", 30, 96), tstat("attack_speed", 30, 96)], { center: true, heroes: true })]),
    ],
  }),

  crystal_maiden_crystal_nova: () => ({
    target: "point",
    v688: { cd: [13, 13, 13, 13], mana: [100, 120, 140, 160], range: [700], castPoint: 0 },
    effects: [aoe(6641, "enemy", [dmg([100, 150, 200, 250], "magical:spells"), slow(72, [52429, 45875, 39322, 32768]), tstat("attack_speed", [-20, -30, -40, -50], 72)])],
  }),
  crystal_maiden_frostbite: () => ({
    target: "unit",
    flags: ["enemy_only"],
    v688: { cd: [9, 8, 7, 6], mana: [115, 125, 130, 140], range: [600] },
    effects: [dmg(50, "magical:spells"), root([24, 32, 40, 48]), dot(50, 8, [16, 24, 32, 40], "magical:spells")],
  }),
  crystal_maiden_brilliance_aura: () => ({
    target: "none",
    flags: ["aura", "passive_stats"],
    v688: { levels: 4 },
    ext: { period: 16 },
    effects: [aoe(H.GLOBAL, "ally", [tstat("mana_regen", [100, 150, 200, 250], 32)]), H.self([tstat("mana_regen", [100, 150, 200, 250], 32)])],
  }),
  crystal_maiden_freezing_field: () => ({
    target: "none",
    v688: { cd: [90, 90, 90], mana: [200, 400, 600], channel: 10, levels: 3, ultimate: true },
    ext: { period: 2, scatter_min: 3047, scatter_max: 12266 },
    effects: [aoe(5078, "enemy", [dmg([131, 213, 313], "magical:normal")]), aoe(12500, "enemy", [slow(16, 45875), tstat("attack_speed", -30, 16)], { center: true })],
  }),

  sven_storm_bolt: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: 977,
    v688: { cd: [13], mana: [140], range: [600], castPoint: 0.1875, levels: 4 },
    effects: [aoe(4375, "enemy", [dmg([100, 175, 250, 325], "magical:normal"), stun([22, 26, 29, 32])])],
  }),
  sven_great_cleave: () => ({
    target: "none",
    flags: ["passive"],
    v688: { levels: 4 },
    effects: [stat("cleave", [30, 42, 54, 66]), stat("cleave_radius", 4688), stat("cleave_w0", 2344), stat("cleave_w1", 4219)],
  }),
  sven_warcry: () => ({
    target: "none",
    v688: { cd: [32, 26, 20, 14], mana: [25], levels: 4 },
    effects: [aoe(14453, "ally", [tstat("armor", [5, 10, 15, 20], 128), tstat("speed_pct", 12, 128)])],
  }),
  sven_gods_strength: () => ({
    target: "none",
    v688: { cd: [100], mana: [100, 150, 200], castPoint: 0.3125, levels: 3, ultimate: true },
    effects: [tstat("damage", [100, 150, 200], 400, 1)],
  }),

  juggernaut_blade_fury: () => ({
    target: "none",
    v688: { cd: [42, 34, 26, 18], mana: [120, 110, 100, 90], castPoint: 0, levels: 4 },
    effects: [status("spell_immunity", 80), zone("juggernaut_blade_fury_pulse", 4, 76, { follow: true })],
  }),
  juggernaut_blade_fury_pulse: () => ({
    target: "none",
    ranks: 4,
    noSkill: true,
    effects: [aoe(4297, "enemy", [dmg([20, 26, 33, 39], "magical")])],
  }),
  juggernaut_healing_ward: () => ({
    target: "unit_or_point",
    v688: { cd: [60], mana: [120, 125, 130, 135], range: [350], castPoint: 0.3125, levels: 4 },
    effects: [ward("juggernaut_healing_ward", 400)],
  }),
  juggernaut_healing_ward_aura: () => ({
    target: "none",
    ranks: 4,
    noSkill: true,
    flags: ["aura"],
    ext: { period: 16 },
    effects: [aoe(12500, "ally", [healPct([200, 300, 400, 500])])],
  }),
  juggernaut_blade_dance: () => ({
    target: "none",
    flags: ["passive"],
    v688: { levels: 4 },
    effects: [stat("crit_chance", [20, 25, 30, 35]), stat("crit_mult", 200)],
  }),
  juggernaut_omni_slash: () => ({
    target: "unit",
    flags: ["pierce", "enemy_only"],
    v688: { cd: [130, 120, 110], mana: [200, 275, 350], range: [350], castPoint: 0.3125, levels: 3, ultimate: true },
    effects: [
      move(5469),
      H.self([status("invulnerable", [19, 38, 58])]),
      zone("juggernaut_omni_slash_pulse", 6, [19, 38, 58], { follow: true }),
    ],
  }),
  juggernaut_omni_slash_pulse: () => ({
    target: "none",
    ranks: 3,
    noSkill: true,
    effects: [aoe(7031, "enemy", [dmg(213, "physical")], { max: 1 })],
  }),

  zuus_arc_lightning: () => ({
    target: "unit",
    flags: ["enemy_only"],
    ext: { period: 4 },
    v688: { cd: [1.75], mana: [65, 70, 75, 80], range: [850], castPoint: 0.1875, levels: 4 },
    effects: [chain([85, 100, 115, 145], [4, 6, 8, 14], 7813, "magical:spells")],
  }),
  zuus_static_field: () => ({
    target: "none",
    flags: ["passive"],
    v688: { levels: 4 },
    effects: [],
  }),
  zuus_lightning_bolt: () => ({
    target: "unit_or_point",
    flags: ["enemy_only"],
    v688: { cd: [6], mana: [75, 95, 115, 135], range: [700], castPoint: 0.2, levels: 4 },
    effects: [dmg([100, 175, 250, 325], "magical:normal"), stun(1), status("revealed", T(4.5))],
  }),
  zuus_thundergods_wrath: () => ({
    target: "none",
    v688: { cd: [90], mana: [225, 325, 450], castPoint: 0.2, levels: 3, ultimate: true },
    effects: [aoe(H.GLOBAL, "enemy", [dmg([225, 350, 475], "magical:spells"), status("revealed", T(3))], { heroes: true })],
  }),

  sniper_assassinate: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: 2441,
    v688: { cd: [20, 15, 10], mana: [175, 275, 375], range: [2000, 2500, 3000], castPoint: [1.7, 1.65, 1.5], levels: 3, ultimate: true },
    effects: [dmg([320, 485, 650], "magical:spells"), stun(1), status("revealed", T(4))],
  }),
  sniper_take_aim: () => ({
    target: "none",
    flags: ["passive_stats"],
    v688: { levels: 4 },
    effects: [stat("range", [1563, 3125, 4688, 6250])],
  }),
  sniper_headshot: () => ({
    target: "unit",
    flags: ["passive", "proc_attack"],
    ext: { chance: 40 },
    v688: { levels: 4 },
    effects: [dmg([15, 40, 65, 90], "physical:hero"), slow(8, 0)],
  }),
  sniper_shrapnel: () => ({
    target: "point",
    ext: { charges: 3, charge_restore: 880 },
    v688: { cd: [0], mana: [50], range: [1800], castPoint: 0, levels: 4 },
    effects: [zone("sniper_shrapnel_pulse", 16, 160, { delay: 19 })],
  }),
  sniper_shrapnel_pulse: () => ({
    target: "none",
    ranks: 4,
    noSkill: true,
    effects: [aoe(7422, "enemy", [dmg([12, 24, 36, 48], "magical"), slow(32, [55706, 52429, 49152, 45875])])],
  }),

  lion_impale: () => ({
    target: "unit_or_point",
    v688: { cd: [12], mana: [100, 120, 145, 170], range: [650], castPoint: 0.3125, levels: 4 },
    effects: [zone("lion_impale_line", 1, 1, { delay: 7 })],
  }),
  lion_impale_line: () => ({
    target: "none",
    ranks: 4,
    noSkill: true,
    ext: { line_extra: 10938 },
    effects: [aoe(2344, "enemy", [dmg([80, 140, 200, 260], "magical:spells"), stun([8, 16, 24, 32])], { line: true })],
  }),
  lion_voodoo: () => ({
    target: "unit",
    flags: ["enemy_only"],
    v688: { cd: [30, 24, 18, 12], mana: [125, 150, 175, 200], range: [500], castPoint: 0, levels: 4 },
    effects: [status("hex", [40, 48, 56, 64], 117)],
  }),
  lion_mana_drain: () => ({
    target: "unit",
    flags: ["enemy_only"],
    ext: { period: 4, break_range: 18750 },
    v688: { cd: [20, 15, 10, 5], mana: [0], range: [850], castPoint: 0.3125, channel: 5, levels: 4 },
    effects: [mana([5, 10, 15, 30], 2)],
  }),
  lion_finger_of_death: () => ({
    target: "unit",
    flags: ["enemy_only"],
    v688: { cd: [160, 100, 40], mana: [200, 420, 650], range: [900], castPoint: 0.3125, levels: 3, ultimate: true },
    effects: [zone("lion_finger_of_death_hit", 1, 1, { delay: 3, pin: true })],
  }),
  lion_finger_of_death_hit: () => ({
    target: "unit",
    ranks: 3,
    noSkill: true,
    effects: [dmg([600, 725, 850], "magical:normal")],
  }),

  phantom_lancer_phantom_rush: () => ({
    target: "none",
    flags: ["passive"],
    v688: { cd: [16, 12, 8, 4], levels: 4 },
    effects: [],
  }),
  phantom_lancer_spirit_lance: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: 977,
    v688: { cd: [7], mana: [125, 130, 135, 140], range: [750], castPoint: 0.2, levels: 4 },
    effects: [
      dmg([100, 150, 200, 250], "magical:normal"),
      slow(52, [58982, 52429, 45875, 39322]),
      tstat("attack_speed", [-10, -20, -30, -40], 52),
      illusion(1, [32, 64, 96, 128], 20, 400),
    ],
  }),
  phantom_lancer_doppelganger: () => ({
    target: "point",
    v688: { cd: [25, 20, 15, 10], mana: [50, 50, 50, 50], range: [600], castPoint: 0, levels: 4 },
    effects: [move(9375), illusion(1, 128, 20, 600), illusion(1, 128, 0, 100)],
  }),
  phantom_lancer_juxtapose: () => ({
    target: "none",
    flags: ["passive", "proc_attack"],
    ext: { chance: [40, 45, 50] },
    v688: { levels: 3, ultimate: true },
    effects: [illusion(1, 128, 16, 500)],
  }),

  dazzle_shallow_grave: () => ({
    target: "unit",
    flags: ["ally_only"],
    v688: { cd: [60, 45, 30, 15], mana: [140, 130, 120, 110], range: [550, 700, 850, 1000] },
    effects: [status("death_prevention", T(5))],
  }),
  dazzle_shadow_wave: () => ({
    target: "unit",
    flags: ["ally_only"],
    v688: { cd: [12, 10, 8, 6], mana: [80, 90, 100, 110], range: [900, 900, 900, 900] },
    effects: [heal([80, 100, 120, 140]), H.self([heal([80, 100, 120, 140])]), aoe(D(210), "enemy", [dmg([80, 100, 120, 140], "physical:hero")]), aoe(D(210), "enemy", [dmg([80, 100, 120, 140], "physical:hero")], { center: true })],
  }),
  dazzle_weave: () => ({
    target: "point",
    v688: { cd: [40, 40, 40], mana: [100, 100, 100], range: [2000, 2000, 2000], levels: 3, ultimate: true },
    effects: [],
  }),
  dazzle_poison_touch: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: 1270,
    v688: { cd: [15, 13, 11, 9], mana: [100, 100, 100, 100], range: [600, 600, 600, 600] },
    effects: [],
  }),

  antimage_spell_shield: () => ({
    target: "none",
    flags: ["passive_stats"],
    v688: { cd: [0, 0, 0, 0], mana: [0, 0, 0, 0] },
    effects: [stat("magic_resist", [26, 34, 42, 50])],
  }),
  antimage_mana_break: () => ({
    target: "unit",
    flags: ["passive", "proc_attack"],
    ext: { chance: 100 },
    v688: { cd: [0, 0, 0, 0], mana: [0, 0, 0, 0] },
    effects: [mana([28, 40, 52, 64], 1, 60, "physical:hero")],
  }),
  antimage_blink: () => ({
    target: "point",
    noRange: true,
    v688: { cd: [12, 9, 7, 5], mana: [60, 60, 60, 60] },
    effects: [move(D(925, 1000, 1075, 1150))],
  }),
  antimage_mana_void: () => ({
    target: "unit",
    flags: ["enemy_only"],
    v688: { cd: [70, 70, 70], mana: [125, 200, 275], range: [600, 600, 600], levels: 3, ultimate: true },
    effects: [stun(T(0.3))],
  }),

  lina_dragon_slave: () => ({
    target: "point",
    v688: { cd: [8.5, 8.5, 8.5, 8.5], mana: [100, 115, 130, 145], range: [800, 800, 800, 800] },
    effects: [aoe(D(275), "enemy", [dmg([110, 180, 250, 320], "magical:spells")], { line: true, taper: 23 })],
  }),
  lina_light_strike_array: () => ({
    target: "point",
    v688: { cd: [7, 7, 7, 7], mana: [100, 110, 120, 130], range: [625, 625, 620, 620] },
    effects: [zone("lina_light_strike_array_pulse", 1, 1, { delay: 8 })],
  }),
  lina_light_strike_array_pulse: () => ({
    target: "none",
    ranks: 4,
    noSkill: true,
    effects: [aoe(D(245), "enemy", [dmg([80, 120, 160, 200], "magical"), stun(T(1.6, 1.9, 2.2, 2.5))])],
  }),
  lina_fiery_soul: () => ({
    target: "none",
    flags: ["passive"],
    v688: { cd: [0, 0, 0, 0], mana: [0, 0, 0, 0] },
    effects: [],
  }),
  lina_laguna_blade: () => ({
    target: "unit",
    flags: ["enemy_only"],
    v688: { cd: [70, 60, 50], mana: [280, 420, 680], range: [600, 600, 600], levels: 3, ultimate: true },
    effects: [zone("lina_laguna_blade_hit", 1, 1, { delay: 3, pin: true })],
  }),
  lina_laguna_blade_hit: () => ({
    target: "unit",
    ranks: 3,
    noSkill: true,
    effects: [dmg([450, 650, 850], "magical")],
  }),

  lich_dark_ritual: () => ({
    target: "unit",
    flags: ["ally_only"],
    v688: { cd: [44, 36, 28, 20], mana: [25, 25, 25, 25], range: [400, 400, 400, 400] },
    effects: [mana([25, 40, 55, 70], 3)],
  }),
  lich_frost_nova: () => ({
    target: "unit",
    flags: ["enemy_only"],
    v688: { cd: [8, 8, 8, 8], mana: [125, 150, 170, 190], range: [600, 600, 600, 600] },
    effects: [
      dmg([50, 100, 150, 200], "magical:spells"),
      aoe(D(200), "enemy", [dmg([75, 100, 125, 150], "magical:spells"), slow(T(4), 49152), tstat("attack_speed", [-30, -40, -50, -60], T(4))]),
    ],
  }),
  lich_frost_armor_lich: () => ({
    target: "unit",
    flags: ["ally_only"],
    v688: { cd: [5, 5, 5, 5], mana: [50, 50, 50, 50], range: [1000, 1000, 1000, 1000] },
    effects: [tstat("armor", [3, 5, 7, 9], T(40))],
  }),
  lich_chain_frost: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: 830,
    ext: { period: 3 },
    v688: { cd: [145, 115, 60], mana: [200, 325, 500], range: [750, 750, 750], levels: 3, ultimate: true },
    effects: [chain([280, 370, 460], 10, D(600), "magical:spells")],
  }),

  tidehunter_ravage: () => ({
    target: "none",
    v688: { cd: [150, 150, 150], mana: [150, 225, 325], levels: 3, ultimate: true },
    effects: [aoe(D(800), "enemy", [dmg([200, 290, 380], "magical:spells"), stun(T(1.5, 1.8, 2.25))], { center: true })],
  }),
  tidehunter_anchor_smash: () => ({
    target: "none",
    v688: { cd: [7, 6, 5, 4], mana: [30, 40, 50, 60] },
    effects: [aoe(D(400), "enemy", [dmg([75, 125, 175, 225], "physical"), tstat("damage", [-45, -50, -55, -60], T(6), 1)], { center: true })],
  }),
  tidehunter_gush: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: 3906,
    v688: { cd: [12, 12, 12, 12], mana: [120, 120, 120, 120], range: [700, 700, 700, 700] },
    effects: [dmg([110, 160, 210, 260], "magical:normal"), slow(T(4), SLOWREM(40)), tstat("armor", [-3, -4, -5, -6], T(4))],
  }),
  tidehunter_kraken_shell: () => ({
    target: "none",
    flags: ["passive"],
    v688: { cd: [0, 0, 0, 0], mana: [0, 0, 0, 0] },
    effects: [],
  }),

  slardar_sprint: () => ({
    target: "none",
    v688: { cd: [17, 17, 17, 17], mana: [25, 25, 25, 25] },
    effects: [tstat("speed_pct", [16, 25, 34, 43], T(12)), tstat("damage_reduction", -15, T(12))],
  }),
  slardar_slithereen_crush: () => ({
    target: "none",
    v688: { cd: [8, 8, 8, 8], mana: [80, 95, 105, 115] },
    effects: [
      aoe(
        D(375),
        "enemy",
        [dmg([80, 120, 160, 200], "physical:hero"), stun(T(1, 1.3, 1.6, 1.9)), slow(T(3, 3.5, 4, 4.5), SLOWREM(20)), tstat("attack_speed", -20, T(3, 3.5, 4, 4.5))],
        { center: true },
      ),
    ],
  }),
  slardar_bash: () => ({
    target: "unit",
    flags: ["passive", "proc_attack"],
    ext: { chance: [10, 15, 20, 25] },
    v688: { cd: [0, 0, 0, 0], mana: [0, 0, 0, 0] },
    effects: [dmg([60, 80, 100, 120], "physical"), stun(T(1))],
  }),
  slardar_amplify_damage: () => ({
    target: "unit",
    flags: ["pierce", "enemy_only"],
    v688: { cd: [5, 5, 5], mana: [25, 25, 25], range: [700, 700, 700], levels: 3, ultimate: true },
    effects: [tstat("armor", [-10, -15, -20], T(25)), status("revealed", T(25))],
  }),

  vengefulspirit_magic_missile: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: 1221,
    v688: { cd: [13, 12, 11, 10], mana: [110, 120, 130, 140], range: [650, 650, 650, 650] },
    effects: [dmg([100, 175, 250, 325], "magical:spells"), stun(T(1.45, 1.55, 1.65, 1.75))],
  }),
  vengefulspirit_command_aura: () => ({
    target: "none",
    flags: ["aura"],
    ext: { period: 16 },
    v688: { cd: [0, 0, 0, 0], mana: [0, 0, 0, 0] },
    effects: [aoe(D(900), "ally", [tstat("damage", [12, 20, 28, 36], 32, 1)], { center: true })],
  }),
  vengefulspirit_wave_of_terror: () => ({
    target: "point",
    ext: { line_extra: 6641 },
    v688: { cd: [20, 20, 20, 20], mana: [40, 40, 40, 40], range: [1700, 1700, 1700, 1700] },
    effects: [aoe(D(325), "enemy", [dmg([30, 50, 70, 90], "pure:hero"), tstat("armor", [-3, -4, -5, -6], T(15))], { line: true })],
  }),
  vengefulspirit_nether_swap: () => ({
    target: "unit",
    v688: { cd: [45, 45, 45], mana: [100, 150, 200], range: [700, 950, 1200], levels: 3, ultimate: true },
    effects: [],
  }),

  omniknight_purification: () => ({
    target: "unit",
    flags: ["ally_only"],
    v688: { cd: [10, 10, 10, 10], mana: [100, 120, 140, 160], range: [700, 700, 700, 700] },
    effects: [heal([90, 180, 270, 360]), aoe(D(285), "enemy", [dmg([90, 180, 270, 360], "magical:hero")])],
  }),
  omniknight_repel: () => ({
    target: "unit",
    flags: ["ally_only"],
    v688: { cd: [14, 14, 14, 14], mana: [50, 50, 50, 50], range: [500, 500, 500, 500], castPoint: 0.25 },
    effects: [status("spell_immunity", T(4, 5, 6, 7))],
  }),
  omniknight_guardian_angel: () => ({
    target: "none",
    v688: { cd: [150, 150, 150], mana: [125, 175, 250], castPoint: 0.25, levels: 3, ultimate: true },
    effects: [aoe(D(600), "ally", [status("physical_immunity", T(6, 7, 8))], { center: true })],
  }),
  omniknight_degen_aura: () => ({
    target: "none",
    flags: ["aura"],
    ext: { period: 16 },
    v688: { cd: [0, 0, 0, 0], mana: [0, 0, 0, 0] },
    effects: [aoe(D(350), "enemy", [tstat("speed_pct", [-10, -18, -26, -34], 32), tstat("attack_speed", [-10, -18, -26, -34], 32)], { center: true })],
  }),

};
