// Which engine effect carries which Valve ability value. scripts/gen-abilities.mjs reads every number from Valve's file by key;
// a recipe names the key (H.V), the unit conversion and the engine effect that uses it. A key a recipe does not name is listed in
// the generated file as unmodelled. Engine effect semantics: rust/crates/data/FORMAT.md (effect table, AXTN).

const TYPE = { legacy: 0, physical: 1, magical: 2, pure: 3 };
const STAT = {
  damage: 0, armor: 1, health: 2, speed: 3, range: 4, evasion: 5, crit_chance: 6, crit_mult: 7, cleave: 8, cleave_radius: 9,
  lifesteal: 10, magic_resist: 11, attack_speed: 12, mana_regen: 13, health_regen: 14, max_mana: 15, spell_amp: 16,
  bash_chance: 17, bash_ticks: 18, speed_pct: 19, cdr: 20, status_resist: 21, strength: 22, agility: 23, intelligence: 24,
  spell_lifesteal: 25, armor_x100: 26, cast_range: 27, cleave_w0: 28, cleave_w1: 29, damage_reduction: 30,
};
const STATUS = {
  taunt: 5, hex: 6, disarm: 7, spell_immunity: 8, invulnerable: 9, break: 10, mute: 11, physical_immunity: 12, death_prevention: 13,
};
const TEAM = { enemy: 0, ally: 1, allyOther: 2, any: 3 };
const GLOBAL = 400000;

const count = (effects) => effects.reduce((n, e) => n + 1 + (e.nested ? count(e.nested) : 0), 0);

export const H = {
  V: (v, conv = "int", o = {}) => ({ v, conv, ...o }),
  dmg: (amount, type, radius = 0) => ({ k: "damage", p: [amount, radius, TYPE[type], 0] }),
  heal: (amount) => ({ k: "heal", p: [amount, 0, 1, 0] }),
  healPct: (hundredths) => ({ k: "heal", p: [hundredths, 0, 2, 0] }),
  stun: (t) => ({ k: "stun", p: [t, 0, 0, 0] }),
  silence: (t) => ({ k: "silence", p: [t, 0, 0, 0] }),
  root: (t) => ({ k: "root", p: [t, 0, 0, 0] }),
  slow: (t, mult) => ({ k: "slow", p: [t, mult, 0, 0] }),
  status: (name, t, extra = 0) => ({ k: "status", p: [STATUS[name], t, extra, 0] }),
  tstat: (stat, amount, t, pct = 0) => ({ k: "timed_stat", p: [STAT[stat], amount, t, pct] }),
  stat: (stat, amount) => ({ k: "stat", p: [STAT[stat], amount, 0, 0] }),
  dot: (amount, period, duration, type) => ({ k: "dot", p: [amount, period, duration, TYPE[type]] }),
  chain: (amount, jumps, radius, type) => ({ k: "chain", p: [amount, jumps, radius, TYPE[type]] }),
  mana: (amount, mode) => ({ k: "mana", p: [amount, mode, 0, 0] }),
  illusion: (count, duration, out, taken) => ({ k: "illusion", p: [count, duration, out, taken] }),
  move: (cap) => ({ k: "move", p: [cap, 0, 0, 0] }),
  dispel: (mask) => ({ k: "dispel", p: [mask, 0, 0, 0] }),
  attack: (pct = 100) => ({ k: "attack", p: [pct, 0, 0, 0] }),
  ward: (unit, life) => ({ k: "ward", p: [0, life, 0, 0], unit }),
  zone: (ability, period, life, o = {}) => ({ k: "zone", p: [0, period, life, o.follow ? 1 << 30 : o.delay ?? 0], ability }),
  ifKill: (nested) => ({ k: "if_kill", p: [count(nested), 0, 0, 0], nested }),
  self: (nested) => ({ k: "aoe", p: [0, 16, count(nested), 0], nested }),
  aoe: (radius, team, nested, o = {}) => {
    const bits = (o.filter ?? 0) | (o.heroes ? 1 << 30 : 0);
    const cap = o.max ?? 0;
    const p3 = typeof cap === "object" ? { ...cap, add: bits } : bits | (cap << 16);
    return { k: "aoe", p: [radius, TEAM[team] | (o.center ? 4 : 0) | (o.line ? 8 : 0), count(nested), p3], nested };
  },
  GLOBAL,
};
const { V, dmg, heal, healPct, stun, root, slow, status, tstat, stat, dot, chain, mana, move, attack, ward, zone, ifKill, aoe, illusion, dispel } = H;

// Slot order of each hero: basic abilities, then the ultimate, then the generic talent modifiers.
export const HEROES = {
  axe: ["axe_berserkers_call", "axe_battle_hunger", "axe_counter_helix", "axe_culling_blade", "@talents"],
  crystal_maiden: ["crystal_maiden_crystal_nova", "crystal_maiden_frostbite", "crystal_maiden_brilliance_aura", "crystal_maiden_freezing_field", "@talents"],
  sven: ["sven_storm_bolt", "sven_great_cleave", "sven_warcry", "sven_gods_strength", "@talents"],
  juggernaut: ["juggernaut_blade_fury", "juggernaut_healing_ward", "juggernaut_blade_dance", "juggernaut_omni_slash", "@talents"],
  zuus: ["zuus_arc_lightning", "zuus_lightning_bolt", "zuus_heavenly_jump", "zuus_thundergods_wrath", "@talents"],
  sniper: ["sniper_shrapnel", "sniper_headshot", "sniper_take_aim", "sniper_assassinate", "@talents"],
  lion: ["lion_impale", "lion_voodoo", "lion_mana_drain", "lion_finger_of_death", "@talents"],
  phantom_lancer: ["phantom_lancer_spirit_lance", "phantom_lancer_juxtapose", "@talents"],
  dazzle: ["dazzle_poison_touch", "dazzle_shallow_grave", "@talents"],
  antimage: ["antimage_mana_break", "antimage_blink", "antimage_counterspell", "@talents"],
  lina: ["lina_dragon_slave", "lina_light_strike_array", "lina_laguna_blade", "@talents"],
  lich: ["lich_frost_nova", "lich_frost_shield", "lich_chain_frost", "@talents"],
  tidehunter: ["tidehunter_gush", "tidehunter_kraken_shell", "tidehunter_ravage", "@talents"],
  slardar: ["slardar_sprint", "slardar_slithereen_crush", "slardar_bash", "slardar_amplify_damage", "@talents"],
  vengefulspirit: ["vengefulspirit_magic_missile", "vengefulspirit_wave_of_terror", "vengefulspirit_command_aura", "@talents"],
  omniknight: ["omniknight_purification", "omniknight_martyr", "omniknight_hammer_of_purity", "omniknight_degen_aura", "omniknight_guardian_angel", "@talents"],
};

// Helper abilities (not in a hero's slots): the pulse a zone, aura or ward runs. Each takes its Valve values from `parent`.
export const HELPERS = {
  juggernaut_blade_fury_pulse: "juggernaut_blade_fury",
  juggernaut_omni_slash_pulse: "juggernaut_omni_slash",
  juggernaut_healing_ward_aura: "juggernaut_healing_ward",
  sniper_shrapnel_pulse: "sniper_shrapnel",
  lina_light_strike_array_pulse: "lina_light_strike_array",
};

// Units the abilities place. Stats from npc_units.txt @282fad29 (scale 15.625 milli per unit, speed per tick at 16 ticks a second).
export const SUMMON_UNITS = [
  "# Summoned units of the hero abilities. juggernaut_healing_ward: StatusHealth 1, MovementSpeed 325, VisionDaytimeRange 600, BountyGold 75, BountyXP 75",
  "# (npc_dota_juggernaut_healing_ward, npc_units.txt @282fad29f0f74504fad465c52d197fb61f4d612b). Hull radius 125 is UNVERIFIED (hull sizes are engine constants).",
  'unit("juggernaut_healing_ward", 1, 317, 125, abilities=["juggernaut_healing_ward_aura"], filters=["ground", "light"], sight_milli=9375, model="pillar-obelisk", deny_pct=0)',
  'unit_extra("juggernaut_healing_ward", bounty=75, xp_bounty=75, deny_xp_pct=0)',
];

export const RECIPES = {
  // ---- Axe ----
  axe_berserkers_call: () => ({
    target: "none",
    flags: ["pierce"],
    effects: [aoe(V("radius", "dist"), "enemy", [status("taunt", V("duration", "time"))]), tstat("armor", V("bonus_armor"), V("duration", "time"))],
  }),
  axe_battle_hunger: () => ({
    target: "unit",
    flags: ["enemy_only"],
    effects: [dot(V("damage_per_second"), 16, V("duration", "time"), "pure"), slow(V("duration", "time"), V("slow", "slowpos"))],
  }),
  axe_counter_helix: () => ({
    target: "none",
    flags: ["passive", "proc_attacked", "counted"],
    ext: { period: V("trigger_attacks") },
    effects: [dmg(V("damage"), "pure", V("radius", "dist"))],
  }),
  axe_culling_blade: () => ({
    target: "unit",
    flags: ["pierce", "enemy_only"],
    effects: [
      dmg(V("damage"), "pure"),
      ifKill([aoe(V("speed_aoe", "dist"), "ally", [tstat("speed_pct", V("speed_bonus"), V("speed_duration", "time")), tstat("armor", V("armor_bonus"), V("speed_duration", "time"))], { center: true })]),
    ],
  }),

  // ---- Crystal Maiden ----
  crystal_maiden_crystal_nova: () => ({
    target: "point",
    effects: [
      aoe(V("radius", "dist"), "enemy", [
        dmg(V("nova_damage"), "magical"),
        slow(V("duration", "time"), V("movespeed_slow", "slowmul")),
        tstat("attack_speed", V("attackspeed_slow"), V("duration", "time")),
      ]),
    ],
  }),
  crystal_maiden_frostbite: () => ({
    target: "unit",
    flags: ["enemy_only"],
    effects: [root(V("duration", "time")), status("disarm", V("duration", "time")), dot(V("damage_per_second", "int", { mul: 0.25 }), 4, V("duration", "time"), "magical")],
  }),
  crystal_maiden_brilliance_aura: () => ({
    target: "none",
    flags: ["aura", "passive_stats"],
    ext: { period: 16 },
    note: "mana regeneration to every ally, tripled within proximity_bonus_radius (the proximity share is base x (proximity_bonus_factor 3 - 1))",
    effects: [
      aoe(H.GLOBAL, "ally", [tstat("mana_regen", V("base_mana_regen", "hundredths"), 32)]),
      aoe(V("proximity_bonus_radius", "dist"), "ally", [tstat("mana_regen", V("base_mana_regen", "hundredths", { mul: 2 }), 32)], { center: true }),
    ],
    ignore: ["proximity_mana_regen_tooltip", "proximity_bonus_factor"],
  }),
  crystal_maiden_freezing_field: () => ({
    target: "none",
    ext: { period: 2, scatter_min: V("explosion_min_dist", "dist"), scatter_max: V("explosion_max_dist", "dist") },
    note: "100 explosions at 0.1 s are modelled as one per 2 ticks (80 over the 10 s channel), each 1.25 times the Valve damage so the total is the same",
    effects: [
      aoe(V("explosion_radius", "dist"), "enemy", [
        dmg(V("damage", "int", { mul: 1.25 }), "magical"),
        slow(V("slow_duration", "time"), V("movespeed_slow", "slowmul")),
        tstat("attack_speed", V("attack_slow"), V("slow_duration", "time")),
      ]),
    ],
    ignore: ["num_explosions", "explosion_interval_tooltip", "radius"],
  }),

  // ---- Sven ----
  sven_storm_bolt: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: V("bolt_speed", "speed"),
    effects: [aoe(V("bolt_aoe", "dist"), "enemy", [dmg(V("AbilityDamage"), "magical"), stun(V("bolt_stun_duration", "time"))])],
  }),
  sven_great_cleave: () => ({
    target: "none",
    flags: ["passive"],
    effects: [
      stat("cleave", V("great_cleave_damage")),
      stat("cleave_radius", V("cleave_distance", "dist")),
      stat("cleave_w0", V("cleave_starting_width", "dist")),
      stat("cleave_w1", V("cleave_ending_width", "dist")),
    ],
    ignore: ["strength_bonus"],
  }),
  sven_warcry: () => ({
    target: "none",
    effects: [aoe(V("radius", "dist"), "ally", [tstat("armor", V("bonus_armor"), V("duration", "time")), tstat("speed_pct", V("movespeed"), V("duration", "time"))])],
  }),
  sven_gods_strength: () => ({
    target: "none",
    effects: [tstat("damage", V("gods_strength_damage"), V("AbilityDuration", "time"), 1)],
  }),

  // ---- Juggernaut ----
  juggernaut_blade_fury: () => ({
    target: "none",
    note: "the spin damage runs as a zone that follows Juggernaut, one pulse per 3 ticks (Valve ticks every 0.2 s)",
    effects: [
      status("spell_immunity", V("duration", "time")),
      tstat("speed_pct", V("bonus_movespeed"), V("duration", "time")),
      zone("juggernaut_blade_fury_pulse", 3, V("duration", "time"), { follow: true }),
    ],
    ignore: ["blade_fury_damage", "blade_fury_damage_tick", "blade_fury_radius", "immunity_resist"],
  }),
  juggernaut_blade_fury_pulse: () => ({
    target: "none",
    ranks: 4,
    noSkill: true,
    effects: [aoe(V("blade_fury_radius", "dist"), "enemy", [dmg(V("blade_fury_damage", "int", { mul: 0.1875 }), "magical")])],
  }),
  juggernaut_healing_ward: () => ({
    target: "point",
    effects: [ward("juggernaut_healing_ward", V("AbilityDuration", "time"))],
    ignore: ["healing_ward_heal_amount", "healing_ward_aura_radius", "healing_ward_movespeed_tooltip", "healing_ward_bonus_health", "healing_ward_hits_to_kill_tooltip"],
  }),
  juggernaut_healing_ward_aura: () => ({
    target: "none",
    ranks: 4,
    flags: ["aura"],
    ext: { period: 16 },
    effects: [aoe(V("healing_ward_aura_radius", "dist"), "ally", [healPct(V("healing_ward_heal_amount", "int", { mul: 100 }))])],
  }),
  juggernaut_blade_dance: () => ({
    target: "none",
    flags: ["passive"],
    effects: [stat("crit_chance", V("blade_dance_crit_chance")), stat("crit_mult", V("blade_dance_crit_mult")), stat("lifesteal", V("blade_dance_lifesteal"))],
  }),
  juggernaut_omni_slash: () => ({
    target: "unit",
    flags: ["pierce", "enemy_only"],
    note: "the bounce between targets runs as a zone that follows Juggernaut and strikes one nearby enemy every 9 ticks",
    effects: [
      move(V("AbilityCastRange", "dist")),
      H.self([
        status("invulnerable", V("duration", "time")),
        tstat("attack_speed", V("bonus_attack_speed"), V("duration", "time")),
        tstat("damage", V("bonus_damage"), V("duration", "time")),
      ]),
      zone("juggernaut_omni_slash_pulse", 9, V("duration", "time"), { follow: true }),
    ],
    ignore: ["attack_rate_multiplier", "omni_slash_radius", "dispel"],
  }),
  juggernaut_omni_slash_pulse: () => ({
    target: "none",
    ranks: 3,
    noSkill: true,
    effects: [aoe(V("omni_slash_radius", "dist"), "enemy", [attack(100)], { max: 1 })],
  }),

  // ---- Zeus ----
  zuus_arc_lightning: () => ({
    target: "unit",
    flags: ["enemy_only"],
    ext: { period: V("jump_delay", "time") },
    effects: [chain(V("arc_damage"), V("jump_count"), V("radius", "dist"), "magical")],
    ignore: ["has_unlimited_jumps"],
  }),
  zuus_lightning_bolt: () => ({
    target: "unit",
    flags: ["enemy_only"],
    effects: [dmg(V("damage"), "magical"), stun(V("ministun_duration", "time"))],
  }),
  zuus_heavenly_jump: () => ({
    target: "point",
    note: "Valve hops along the facing; the sim has no facing, so the hop goes toward the cast point (cap hop_distance)",
    effects: [
      move(V("hop_distance", "dist")),
      aoe(
        V("range", "dist"),
        "enemy",
        [dmg(V("damage"), "magical"), slow(V("duration", "time"), V("move_slow", "slowpos")), tstat("attack_speed", V("aspd_slow", "neg"), V("duration", "time"))],
        { center: true, max: V("targets", "int", { mul: 65536 }) },
      ),
    ],
    ignore: ["hop_duration", "hop_height", "vision_radius", "vision_duration"],
  }),
  zuus_thundergods_wrath: () => ({
    target: "none",
    effects: [aoe(H.GLOBAL, "enemy", [dmg(V("damage"), "magical")], { heroes: true })],
    ignore: ["sight_radius_day", "sight_radius_night", "sight_duration"],
  }),

  // ---- Sniper ----
  sniper_shrapnel: () => ({
    target: "point",
    ext: { charges: V("AbilityCharges"), charge_restore: V("AbilityChargeRestoreTime", "time") },
    note: "shrapnel_damage and slow run in a zone that starts after damage_delay and pulses once a second",
    effects: [zone("sniper_shrapnel_pulse", 16, V("duration", "time"), { delay: V("damage_delay", "time") })],
    ignore: ["radius", "shrapnel_damage", "slow_movement_speed", "slow_duration"],
  }),
  sniper_shrapnel_pulse: () => ({
    target: "none",
    ranks: 4,
    noSkill: true,
    effects: [
      aoe(V("radius", "dist"), "enemy", [dmg(V("shrapnel_damage"), "magical"), slow(32, V("slow_movement_speed", "slowmul"))]),
    ],
  }),
  sniper_headshot: () => ({
    target: "unit",
    flags: ["passive", "proc_attack"],
    ext: { chance: V("proc_chance") },
    effects: [dmg(V("damage"), "physical"), slow(V("slow_duration", "time"), V("slow", "slowmul"))],
    ignore: ["knockback_distance"],
  }),
  sniper_take_aim: () => ({
    target: "none",
    flags: ["passive_stats"],
    effects: [
      stat("range", V("passive_attack_range_bonus", "dist")),
      tstat("range", V("active_attack_range_bonus", "dist"), V("duration", "time")),
      tstat("attack_speed", V("attack_speed"), V("duration", "time")),
      slow(V("duration", "time"), V("slow", "slowpos")),
    ],
    ignore: ["headshot_chance", "bonus_vision", "view_restrict", "does_root", "require_target"],
  }),
  sniper_assassinate: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: V("projectile_speed", "speed"),
    effects: [dmg(V("damage"), "magical")],
  }),

  // ---- Lion ----
  lion_impale: () => ({
    target: "point",
    ext: { line_extra: V("length_buffer", "dist") },
    note: "the spike is instant along the line; Valve's travel speed (2800) is not modelled",
    effects: [aoe(V("width", "dist", { mul: 0.5 }), "enemy", [dmg(V("damage"), "magical"), stun(V("duration", "time"))], { line: true })],
    ignore: ["speed", "pierces_immunity", "cone_shaped"],
  }),
  lion_voodoo: () => ({
    target: "unit",
    flags: ["enemy_only"],
    effects: [aoe(V("radius", "dist"), "enemy", [status("hex", V("duration", "time"), V("movespeed", "speed"))])],
  }),
  lion_mana_drain: () => ({
    target: "unit",
    flags: ["enemy_only"],
    ext: { period: 4, break_range: V("break_distance", "dist") },
    note: "mana moves every 4 ticks (0.25 s; Valve: every 0.1 s) so each step is a whole number of mana",
    effects: [mana(V("mana_per_second", "int", { mul: 0.25 }), 2), slow(8, V("movespeed", "slowpos"))],
    ignore: ["duration", "tick_interval", "movespeed_bonus_when_empty_pct", "ally_pct", "damage_pct"],
  }),
  lion_finger_of_death: () => ({
    target: "unit",
    flags: ["enemy_only"],
    effects: [dmg(V("damage"), "magical")],
  }),

  // ---- Phantom Lancer ----
  phantom_lancer_spirit_lance: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: V("lance_speed", "speed"),
    effects: [
      dmg(V("lance_damage"), "magical"),
      slow(V("duration", "time"), V("movement_speed_pct", "slowmul")),
      illusion(1, V("illusion_duration", "time"), V("illusion_damage_out_pct", "int", { fn: (x) => x + 100 }), V("illusion_damage_in_pct", "int", { fn: (x) => x + 100 })),
    ],
    ignore: ["tooltip_illusion_damage", "tooltip_illusion_total_damage_in_pct", "fake_lance_distance"],
  }),
  phantom_lancer_juxtapose: () => ({
    target: "none",
    flags: ["passive", "proc_attack"],
    ext: { chance: V("proc_chance_pct") },
    note: "the illusion comes from a roll on each attack; max_illusions is not enforced",
    effects: [illusion(1, V("illusion_duration", "time"), V("illusion_damage_out_pct", "int", { fn: (x) => x + 100 }), V("illusion_damage_in_pct", "int", { fn: (x) => x + 100 }))],
    ignore: ["tooltip_illusion_damage", "tooltip_total_illusion_damage_in_pct"],
  }),

  // ---- Dazzle ----
  dazzle_poison_touch: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: V("projectile_speed", "speed"),
    note: "hits the target only (Valve also poisons up to `targets` units in a cone); the slow does not deepen over time",
    effects: [dot(V("damage"), 16, V("duration", "time"), "physical"), slow(V("duration", "time"), V("slow", "slowmul"))],
    ignore: ["start_radius", "end_radius", "end_distance", "targets"],
  }),
  dazzle_shallow_grave: () => ({
    target: "unit",
    flags: ["ally_only"],
    effects: [status("death_prevention", V("duration", "time"))],
    ignore: ["fx_halo_height"],
  }),

  // ---- Anti-Mage ----
  antimage_mana_break: () => ({
    target: "unit",
    flags: ["passive", "proc_attack"],
    ext: { chance: 100 },
    note: "burns the flat mana_per_hit only (not the percent of maximum mana) and deals percent_damage_per_burn of it as physical damage",
    effects: [mana(V("mana_per_hit"), 1), dmg(V("mana_per_hit", "int", { mul: 0.65 }), "physical")],
    ignore: ["percent_damage_per_burn", "mana_per_hit_pct", "illusion_percentage", "empowered_max_burn_pct", "empowered_mana_break_debuff_duration"],
  }),
  antimage_blink: () => ({
    target: "point",
    noRange: true,
    note: "the blink goes to the cast point, capped at the cast range; min_blink_range is not enforced",
    effects: [move(V("AbilityCastRange", "dist"))],
    ignore: ["min_blink_range", "empowered_mana_break_duration", "empowered_max_burn_pct_tooltip", "empowered_mana_break_debuff_duration_tooltip"],
  }),
  antimage_counterspell: () => ({
    target: "none",
    note: "the magic resistance only; Valve's spell reflection is not modelled",
    effects: [tstat("magic_resist", V("magic_resistance"), V("duration", "time"))],
    ignore: ["duration_illusion", "outgoing_damage", "incoming_damage", "does_reflect", "reflected_spell_amp", "heal_pct"],
  }),

  // ---- Lina ----
  lina_dragon_slave: () => ({
    target: "point",
    note: "the wave is a straight line of constant width (Valve tapers it from dragon_slave_width_initial to dragon_slave_width_end) and lands at once",
    effects: [aoe(V("dragon_slave_width_initial", "dist", { mul: 0.5 }), "enemy", [dmg(V("dragon_slave_damage"), "magical")], { line: true })],
    ignore: ["dragon_slave_speed", "dragon_slave_width_end", "dragon_slave_distance"],
  }),
  lina_light_strike_array: () => ({
    target: "point",
    effects: [zone("lina_light_strike_array_pulse", 1, 1, { delay: V("light_strike_array_delay_time", "time") })],
    ignore: ["light_strike_array_aoe", "light_strike_array_stun_duration", "light_strike_array_damage"],
  }),
  lina_light_strike_array_pulse: () => ({
    target: "none",
    ranks: 4,
    noSkill: true,
    effects: [aoe(V("light_strike_array_aoe", "dist"), "enemy", [dmg(V("light_strike_array_damage"), "magical"), stun(V("light_strike_array_stun_duration", "time"))])],
  }),
  lina_laguna_blade: () => ({
    target: "unit",
    flags: ["enemy_only"],
    effects: [dmg(V("damage"), "magical")],
    ignore: ["damage_delay"],
  }),

  // ---- Lich ----
  lich_frost_nova: () => ({
    target: "unit",
    flags: ["enemy_only"],
    effects: [
      dmg(V("damage"), "magical"),
      aoe(V("radius", "dist"), "enemy", [
        dmg(V("aoe_damage"), "magical"),
        slow(V("AbilityDuration", "time"), V("slow_movement_speed", "slowmul")),
        tstat("attack_speed", V("slow_attack_speed_primary"), V("AbilityDuration", "time")),
      ]),
    ],
  }),
  lich_frost_shield: () => ({
    target: "unit",
    flags: ["ally_only"],
    note: "only the damage reduction is modelled; the pulses that damage and slow nearby enemies are not",
    effects: [tstat("damage_reduction", V("damage_reduction"), V("duration", "time"))],
    ignore: ["movement_slow", "slow_duration", "damage", "interval", "radius", "bonus_duration_per_hero_killed", "bonus_duration_per_creep_killed", "AbilityChargeRestoreTime", "AbilityCharges"],
  }),
  lich_chain_frost: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: V("initial_projectile_speed", "speed"),
    ext: { period: 6 },
    note: "jumps every 6 ticks; the slow and the per-jump damage growth are not modelled",
    effects: [chain(V("damage"), V("jumps"), V("jump_range", "dist"), "magical")],
    ignore: ["projectile_speed", "vision_radius", "frostbound_duration", "slow_movement_speed", "slow_attack_speed", "slow_duration"],
  }),

  // ---- Tidehunter ----
  tidehunter_gush: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: V("projectile_speed", "speed"),
    effects: [
      dmg(V("gush_damage"), "magical"),
      slow(V("AbilityDuration", "time"), V("movement_speed", "slowmul")),
      tstat("armor", V("negative_armor", "neg"), V("AbilityDuration", "time")),
    ],
    ignore: ["speed_scepter", "aoe_scepter", "cooldown_scepter", "cast_range_scepter"],
  }),
  tidehunter_kraken_shell: () => ({
    target: "none",
    flags: ["passive_stats"],
    note: "the passive damage reduction and the active purge; the damage-triggered cleanse (blubber) is not modelled",
    effects: [stat("damage_reduction", V("damage_reduction")), dispel(6)],
    ignore: ["creep_reduction_penalty_pct", "active_pct_effectiveness", "active_move_speed_penalty_pct", "active_duration", "damage_cleanse", "damage_reset_interval"],
  }),
  tidehunter_ravage: () => ({
    target: "none",
    note: "the wave expands at 725 per second in Valve; here every enemy in the radius is hit at once",
    effects: [aoe(V("radius", "dist"), "enemy", [dmg(V("AbilityDamage"), "magical"), stun(V("duration", "time"))])],
    ignore: ["speed"],
  }),

  // ---- Slardar ----
  slardar_sprint: () => ({
    target: "none",
    effects: [tstat("speed_pct", V("bonus_speed"), V("duration", "time"))],
    ignore: ["speed_burst_max_duration", "slow_resistance_tooltip"],
  }),
  slardar_slithereen_crush: () => ({
    target: "none",
    effects: [
      aoe(V("crush_radius", "dist"), "enemy", [
        dmg(V("crush_damage"), "physical"),
        stun(V("stun_duration", "time")),
        slow(V("crush_extra_slow_duration", "time"), V("crush_extra_slow", "slowmul")),
        tstat("attack_speed", V("crush_attack_slow_tooltip"), V("crush_extra_slow_duration", "time")),
      ], { center: true }),
    ],
    ignore: ["puddle_duration", "puddle_radius"],
  }),
  slardar_bash: () => ({
    target: "unit",
    flags: ["passive", "proc_attack", "counted"],
    ext: { period: V("attack_count") },
    effects: [dmg(V("bonus_damage"), "physical"), stun(V("duration", "time"))],
  }),
  slardar_amplify_damage: () => ({
    target: "unit",
    flags: ["pierce", "enemy_only"],
    effects: [tstat("armor", V("armor_reduction"), V("duration", "time"))],
    ignore: ["puddle_radius", "puddle_duration", "undispellable"],
  }),

  // ---- Vengeful Spirit ----
  vengefulspirit_magic_missile: () => ({
    target: "unit",
    flags: ["enemy_only"],
    projectile: V("magic_missile_speed", "speed"),
    effects: [dmg(V("magic_missile_damage"), "magical"), stun(V("magic_missile_stun", "time"))],
    ignore: ["bounce_range_pct"],
  }),
  vengefulspirit_wave_of_terror: () => ({
    target: "point",
    effects: [
      aoe(V("wave_width", "dist", { mul: 0.5 }), "enemy", [dmg(V("damage"), "magical"), tstat("armor", V("armor_reduction"), V("AbilityDuration", "time"))], { line: true }),
    ],
    ignore: ["wave_speed", "attack_reduction", "vision_aoe", "vision_duration"],
  }),
  vengefulspirit_command_aura: () => ({
    target: "none",
    flags: ["aura"],
    ext: { period: 16 },
    note: "self_multiplier 25: Vengeful Spirit herself gets a quarter of the ally bonus",
    effects: [
      aoe(V("aura_radius", "dist"), "allyOther", [tstat("damage", V("bonus_base_damage"), 32)], { center: true }),
      H.self([tstat("damage", V("bonus_base_damage", "int", { mul: 0.25 }), 32)]),
    ],
    ignore: ["self_multiplier", "revival_time", "restitution_time"],
  }),

  // ---- Omniknight ----
  omniknight_purification: () => ({
    target: "unit",
    flags: ["ally_only"],
    effects: [heal(V("heal")), aoe(V("radius", "dist"), "enemy", [dmg(V("heal"), "pure")])],
    ignore: ["recast_delay", "recast_effectiveness_pct"],
  }),
  omniknight_martyr: () => ({
    target: "unit",
    flags: ["ally_only"],
    effects: [tstat("health_regen", V("base_hpregen", "hundredths"), V("duration", "time")), tstat("magic_resist", V("magic_resist"), V("duration", "time"))],
  }),
  omniknight_hammer_of_purity: () => ({
    target: "unit",
    flags: ["passive", "proc_attack"],
    ext: { chance: 100 },
    note: "autocast always on: the proc fires on every ready attack",
    effects: [dmg(V("base_damage"), "pure"), dmg(V("bonus_damage"), "physical")],
    ignore: ["heal_pct", "heal_duration", "total_ticks", "attack_cooldown", "attack_range_bonus", "duration", "movement_slow"],
  }),
  omniknight_degen_aura: () => ({
    target: "none",
    noSkill: true,
    flags: ["aura"],
    ext: { period: 16 },
    effects: [aoe(V("radius", "dist"), "enemy", [tstat("speed_pct", V("speed_bonus", "neg"), V("linger_duration", "time", { fn: (x) => x + 16 }))], { center: true })],
    ignore: ["bonus_damage_per_stack", "stack_interval", "max_stacks"],
  }),
  omniknight_guardian_angel: () => ({
    target: "none",
    effects: [aoe(V("radius", "dist"), "ally", [status("physical_immunity", V("duration", "time"))])],
    ignore: ["is_global", "affects_buildings", "heal_and_regen_amp", "model_scale", "flying"],
  }),
};
