//! Spell mechanics shared by every hero ability: the AXTN extension record (cast range, cast point, pulse
//! period, proc chance, rank-scaled parameters and scepter, shard and talent modifiers), per-unit stat
//! modifiers (passive and timed), damage over time, areas, chains, zones, illusions, procs, auras and the
//! attack modifiers (evasion, critical strikes, cleave, lifesteal, bash). Damage amounts of a typed hit
//! (type 1 physical, 2 magical, 3 pure) are whole hit points; type 0 keeps the legacy raw amount.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::loader::{Effect, LoadError};
use crate::rules::{Refusal, Slot};
use crate::status::{DISARM, HEX, INVULNERABLE, MUTE, PHYSICAL_IMMUNE, SPELL_IMMUNE, TAUNT};
use crate::*;

pub(crate) const FIRST_SPELL_KIND: u16 = 20;
const K_DOT: u16 = 20;
const K_STATUS: u16 = 21;
const K_TIMED_STAT: u16 = 22;
pub(crate) const K_AOE: u16 = 23;
const K_DISPEL: u16 = 24;
const K_CHAIN: u16 = 25;
const K_ZONE: u16 = 26;
const K_ILLUSION: u16 = 27;
const K_MANA: u16 = 28;
pub(crate) const K_IF_KILL: u16 = 29;
const K_ATTACK: u16 = 30;

pub const FLAG_PASSIVE: u16 = 1;
pub const FLAG_AURA: u16 = 2;
pub const FLAG_PROC_ATTACK: u16 = 4;
pub const FLAG_PROC_ATTACKED: u16 = 8;
pub const FLAG_PIERCE: u16 = 16;
pub const FLAG_GROUND_PULSE: u16 = 32;
/// A proc fires on every `period`-th hit instead of by chance.
pub const FLAG_COUNTED: u16 = 64;
/// The ability is castable, but its `stat` effects are passive: applied by rank, skipped on a cast.
pub const FLAG_PASSIVE_STATS: u16 = 128;
pub const FLAG_ALLY_ONLY: u16 = 256;
pub const FLAG_ENEMY_ONLY: u16 = 512;
const FLAG_MASK: u16 = 1023;

pub const STAT_COUNT: usize = 30;
pub const S_DAMAGE: usize = 0;
pub const S_ARMOR: usize = 1;
pub const S_HEALTH: usize = 2;
pub const S_SPEED: usize = 3;
pub const S_RANGE: usize = 4;
pub const S_EVASION: usize = 5;
pub const S_CRIT_CHANCE: usize = 6;
pub const S_CRIT_MULT: usize = 7;
pub const S_CLEAVE: usize = 8;
pub const S_CLEAVE_RADIUS: usize = 9;
pub const S_LIFESTEAL: usize = 10;
pub const S_MAGIC_RESIST: usize = 11;
pub const S_ATTACK_SPEED: usize = 12;
pub const S_MANA_REGEN: usize = 13;
pub const S_HEALTH_REGEN: usize = 14;
pub const S_MAX_MANA: usize = 15;
pub const S_SPELL_AMP: usize = 16;
pub const S_BASH_CHANCE: usize = 17;
pub const S_BASH_TICKS: usize = 18;
pub const S_SPEED_PCT: usize = 19;
pub const S_CDR: usize = 20;
pub const S_STATUS_RESIST: usize = 21;
pub const S_STRENGTH: usize = 22;
pub const S_SPELL_LIFESTEAL: usize = 25;
pub const S_ARMOR_X100: usize = 26;
pub const S_CAST_RANGE: usize = 27;
pub const S_CLEAVE_W0: usize = 28;
pub const S_CLEAVE_W1: usize = 29;

pub const SCALED_SLOTS: usize = 6;
pub const MOD_SLOTS: usize = 8;
const AXTN_RECORD: usize = 256;
const AXTN_SCALED_AT: usize = 40;
const AXTN_MODS_AT: usize = 160;
pub const MOD_FIELD_FIRST: u8 = 250;
/// Scaled-entry "effect" values naming a header field instead of an effect.
pub const SCALED_CAST_RANGE: u8 = 252;
pub const SCALED_PERIOD: u8 = 253;
pub const SCALED_CHANCE: u8 = 254;
const MOD_COOLDOWN: u8 = 250;
const MOD_ENERGY: u8 = 251;
const MOD_RANGE: u8 = 252;
const MOD_CAST_POINT: u8 = 253;
const MOD_CHARGE_RESTORE: u8 = 254;
const COND_SCEPTER: u32 = 0;
const COND_SHARD: u32 = 1;
const COND_TALENT: u32 = 2;
const TALENT_LEVELS: [u32; 4] = [10, 15, 20, 25];

pub(crate) const TMOD_SLOTS: usize = 8;
pub(crate) const DOT_SLOTS: usize = 4;
pub(crate) const ZONE_SLOTS: usize = 16;
pub(crate) const CHAIN_SLOTS: usize = 8;
const AOE_CAP: usize = 2048;
/// Passive stat effects tracked per ability slot.
const APPLIED: usize = 8;
const ZONE_DELAY_MASK: i32 = (1 << 30) - 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scaled {
    pub used: bool,
    pub effect: u8,
    pub param: u8,
    pub values: [i32; SKILL_RANKS],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mod {
    pub used: bool,
    pub cond: u8,
    pub effect: u8,
    pub param: u8,
    pub mode: u8,
    pub delta: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AbilityExt {
    pub flags: u16,
    pub cast_point: u16,
    pub period: i32,
    pub cast_range: i32,
    pub chance: i32,
    pub scatter_min: i32,
    pub scatter_max: i32,
    pub charges: i32,
    pub charge_restore: i32,
    pub break_range: i32,
    pub line_extra: i32,
    pub scaled: [Scaled; SCALED_SLOTS],
    pub mods: [Mod; MOD_SLOTS],
}

const SKILL_RANKS: usize = 4;

/// Integer square root, exact.
fn isqrt64(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = x / 2 + 1;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

fn le_i32(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Reads the AXTN section (one record per ability) and checks it against the abilities and effects.
pub(crate) fn parse_ext(
    bytes: &[u8],
    (off, count): (usize, usize),
    abilities: &[crate::loader::Ability],
    _effects: &[Effect],
) -> Result<Vec<AbilityExt>, LoadError> {
    if count != abilities.len() {
        return Err(LoadError::Range("AXTN count is not the ABIL count"));
    }
    let mut out = Vec::with_capacity(count);
    for (n, ability) in abilities.iter().enumerate() {
        let at = off + n * AXTN_RECORD;
        let rec = bytes.get(at..at + AXTN_RECORD).ok_or(LoadError::Truncated("AXTN record"))?;
        let flags = u16::from_le_bytes([rec[0], rec[1]]);
        if flags & !FLAG_MASK != 0 {
            return Err(LoadError::Range("AXTN flags"));
        }
        let mut e = AbilityExt {
            flags,
            cast_point: u16::from_le_bytes([rec[2], rec[3]]),
            period: le_i32(rec, 4),
            cast_range: le_i32(rec, 8),
            chance: le_i32(rec, 12),
            scatter_min: le_i32(rec, 16),
            scatter_max: le_i32(rec, 20),
            charges: le_i32(rec, 24),
            charge_restore: le_i32(rec, 28),
            break_range: le_i32(rec, 32),
            line_extra: le_i32(rec, 36),
            ..AbilityExt::default()
        };
        if e.period < 0 || e.cast_range < 0 || !(0..=100).contains(&e.chance) || e.scatter_min < 0 || e.scatter_max < e.scatter_min || e.charges < 0 || e.charge_restore < 0 || e.break_range < 0 || e.line_extra < 0 {
            return Err(LoadError::Range("AXTN header"));
        }
        for k in 0..SCALED_SLOTS {
            let s = AXTN_SCALED_AT + k * 20;
            if rec[s] == 0 {
                continue;
            }
            let (effect, param) = (rec[s + 1], rec[s + 2]);
            let header = (SCALED_CAST_RANGE..=SCALED_CHANCE).contains(&effect);
            if !(header || (effect as usize) < ability.effect_count as usize) || param >= 4 {
                return Err(LoadError::Range("AXTN scaled entry"));
            }
            let mut values = [0; SKILL_RANKS];
            for (r, v) in values.iter_mut().enumerate() {
                *v = le_i32(rec, s + 4 + 4 * r);
            }
            e.scaled[k] = Scaled { used: true, effect, param, values };
        }
        for k in 0..MOD_SLOTS {
            let m = AXTN_MODS_AT + k * 12;
            if rec[m] == 0 {
                continue;
            }
            let (cond, effect, param, mode) = (rec[m + 1], rec[m + 2], rec[m + 3], rec[m + 4]);
            let field = effect >= MOD_FIELD_FIRST;
            let ok = cond < 10 && mode <= 1 && if field { effect < MOD_FIELD_FIRST + 5 } else { (effect as usize) < ability.effect_count as usize && param < 4 };
            if !ok {
                return Err(LoadError::Range("AXTN modifier"));
            }
            e.mods[k] = Mod { used: true, cond, effect, param, mode, delta: le_i32(rec, m + 8) };
        }
        out.push(e);
    }
    Ok(out)
}

/// The parameter rule of the spell effect kinds (20..=29), mirrored from `format::effect_params_valid`.
pub(crate) fn spell_effect_error(e: &Effect) -> Option<&'static str> {
    let ok = match e.kind {
        K_DOT => e.p0 >= 0 && e.p1 >= 1 && e.p2 >= 1 && (0..=3).contains(&e.p3),
        K_STATUS => (0..crate::status::STATUS_KINDS as i32).contains(&e.p0) && e.p1 >= 1,
        K_TIMED_STAT => (0..STAT_COUNT as i32).contains(&e.p0) && e.p2 >= 1 && (0..=1).contains(&e.p3),
        K_AOE => e.p0 >= 0 && (0..=31).contains(&e.p1) && (0..=16).contains(&e.p2) && e.p3 >= 0,
        K_DISPEL => (1..=7).contains(&e.p0),
        K_CHAIN => e.p0 >= 0 && e.p1 >= 0 && e.p2 >= 0 && (0..=3).contains(&e.p3),
        K_ZONE => e.p0 >= 0 && e.p1 >= 1 && e.p2 >= 1 && e.p3 >= 0,
        K_ILLUSION => (1..=8).contains(&e.p0) && e.p1 >= 1 && e.p2 >= 0 && e.p3 >= 0,
        K_MANA => e.p0 >= 0 && (0..=2).contains(&e.p1),
        K_IF_KILL => (1..=16).contains(&e.p0),
        K_ATTACK => e.p0 >= 0,
        crate::teleport::K_TELEPORT => e.p0 >= 0,
        crate::refresh::K_REFRESH => (1..=3).contains(&e.p0),
        _ => true,
    };
    (!ok).then_some("spell effect parameters")
}

/// Per-unit and global spell state. Every field is a flat i32 table so hashing and saving walk one list.
#[derive(Clone, Debug)]
pub(crate) struct SpellState {
    /// Upgrade bits per unit: 0 scepter, 1 shard, 2..=9 the eight talent choices.
    pub(crate) upg: Vec<i32>,
    pub(crate) smod: Vec<i32>,
    /// Passive stat amounts already added, per ability slot and effect (first four effects).
    pub(crate) applied: Vec<i32>,
    pub(crate) tmod_code: Vec<i32>,
    pub(crate) tmod_amt: Vec<i32>,
    pub(crate) tmod_left: Vec<i32>,
    pub(crate) tmod_key: Vec<i32>,
    pub(crate) dot_src: Vec<i32>,
    pub(crate) dot_key: Vec<i32>,
    pub(crate) dot_amt: Vec<i32>,
    pub(crate) dot_period: Vec<i32>,
    pub(crate) dot_left: Vec<i32>,
    pub(crate) dot_next: Vec<i32>,
    pub(crate) dot_type: Vec<i32>,
    pub(crate) zone_left: Vec<i32>,
    pub(crate) zone_caster: Vec<i32>,
    pub(crate) zone_ai: Vec<i32>,
    pub(crate) zone_x: Vec<i32>,
    pub(crate) zone_y: Vec<i32>,
    pub(crate) zone_period: Vec<i32>,
    pub(crate) zone_next: Vec<i32>,
    pub(crate) zone_delay: Vec<i32>,
    pub(crate) zone_rank: Vec<i32>,
    pub(crate) zone_follow: Vec<i32>,
    pub(crate) charge_n: Vec<i32>,
    pub(crate) charge_t: Vec<i32>,
    pub(crate) chain_left: Vec<i32>,
    pub(crate) chain_caster: Vec<i32>,
    pub(crate) chain_cur: Vec<i32>,
    pub(crate) chain_prev: Vec<i32>,
    pub(crate) chain_dmg: Vec<i32>,
    pub(crate) chain_radius: Vec<i32>,
    pub(crate) chain_type: Vec<i32>,
    pub(crate) chain_wait: Vec<i32>,
    pub(crate) chain_delay: Vec<i32>,
    pub(crate) ill_out: Vec<i32>,
    pub(crate) ill_in: Vec<i32>,
    pub(crate) taunter: Vec<i32>,
    pub(crate) hex_speed: Vec<i32>,
    pub(crate) hits_taken: Vec<i32>,
    pub(crate) pend_slot: Vec<i32>,
    pub(crate) pend_target: Vec<i32>,
    pub(crate) pend_x: Vec<i32>,
    pub(crate) pend_y: Vec<i32>,
    /// Transient: the ability a running effect belongs to, the kill flag of the last damage, the
    /// pierce flag of the running ability. Not hashed or saved.
    pub(crate) cur_ai: i32,
    pub(crate) cur_eff: i32,
    pub(crate) kill_flag: bool,
    pub(crate) pierce: bool,
    /// Rank a zone pulse resolves its values at (its owner holds the zone ability, not the pulse ability); 0 for none.
    pub(crate) rank_override: i32,
    /// Whether any damage ends a channel (the legacy rule); Dota ends it by stun, silence or hex only.
    pub(crate) damage_interrupts: bool,
}

impl SpellState {
    pub(crate) fn new(cap: usize) -> SpellState {
        SpellState {
            upg: vec![0; cap],
            smod: vec![0; cap * STAT_COUNT],
            applied: vec![0; cap * ABILITY_SLOTS * APPLIED],
            tmod_code: vec![-1; cap * TMOD_SLOTS],
            tmod_amt: vec![0; cap * TMOD_SLOTS],
            tmod_left: vec![0; cap * TMOD_SLOTS],
            tmod_key: vec![0; cap * TMOD_SLOTS],
            dot_src: vec![-1; cap * DOT_SLOTS],
            dot_key: vec![0; cap * DOT_SLOTS],
            dot_amt: vec![0; cap * DOT_SLOTS],
            dot_period: vec![1; cap * DOT_SLOTS],
            dot_left: vec![0; cap * DOT_SLOTS],
            dot_next: vec![0; cap * DOT_SLOTS],
            dot_type: vec![0; cap * DOT_SLOTS],
            zone_left: vec![0; ZONE_SLOTS],
            zone_caster: vec![-1; ZONE_SLOTS],
            zone_ai: vec![-1; ZONE_SLOTS],
            zone_x: vec![0; ZONE_SLOTS],
            zone_y: vec![0; ZONE_SLOTS],
            zone_period: vec![1; ZONE_SLOTS],
            zone_next: vec![0; ZONE_SLOTS],
            zone_delay: vec![0; ZONE_SLOTS],
            zone_rank: vec![1; ZONE_SLOTS],
            zone_follow: vec![0; ZONE_SLOTS],
            charge_n: vec![-1; cap * ABILITY_SLOTS],
            charge_t: vec![0; cap * ABILITY_SLOTS],
            chain_left: vec![0; CHAIN_SLOTS],
            chain_caster: vec![-1; CHAIN_SLOTS],
            chain_cur: vec![-1; CHAIN_SLOTS],
            chain_prev: vec![-1; CHAIN_SLOTS],
            chain_dmg: vec![0; CHAIN_SLOTS],
            chain_radius: vec![0; CHAIN_SLOTS],
            chain_type: vec![0; CHAIN_SLOTS],
            chain_wait: vec![0; CHAIN_SLOTS],
            chain_delay: vec![0; CHAIN_SLOTS],
            ill_out: vec![0; cap],
            ill_in: vec![0; cap],
            taunter: vec![-1; cap],
            hex_speed: vec![0; cap],
            hits_taken: vec![0; cap],
            pend_slot: vec![-1; cap],
            pend_target: vec![-1; cap],
            pend_x: vec![0; cap],
            pend_y: vec![0; cap],
            cur_ai: -1,
            cur_eff: 0,
            kill_flag: false,
            pierce: false,
            rank_override: 0,
            damage_interrupts: true,
        }
    }

    /// Every saved table, in a fixed order, for the state hash and the save bytes.
    pub(crate) fn tables(&self) -> [&Vec<i32>; 44] {
        [
            &self.upg, &self.smod, &self.applied, &self.tmod_code, &self.tmod_amt, &self.tmod_left, &self.tmod_key,
            &self.dot_src, &self.dot_key, &self.dot_amt, &self.dot_period, &self.dot_left, &self.dot_next, &self.dot_type,
            &self.zone_left, &self.zone_caster, &self.zone_ai, &self.zone_x, &self.zone_y, &self.zone_period,
            &self.zone_next, &self.zone_delay, &self.zone_rank, &self.zone_follow, &self.charge_n, &self.charge_t, &self.chain_left, &self.chain_caster, &self.chain_cur, &self.chain_prev,
            &self.chain_dmg, &self.chain_radius, &self.chain_type, &self.chain_wait, &self.chain_delay, &self.ill_out,
            &self.ill_in, &self.taunter, &self.hex_speed, &self.hits_taken, &self.pend_slot, &self.pend_target,
            &self.pend_x, &self.pend_y,
        ]
    }

    pub(crate) fn tables_mut(&mut self) -> [&mut Vec<i32>; 44] {
        [
            &mut self.upg, &mut self.smod, &mut self.applied, &mut self.tmod_code, &mut self.tmod_amt, &mut self.tmod_left,
            &mut self.tmod_key, &mut self.dot_src, &mut self.dot_key, &mut self.dot_amt, &mut self.dot_period,
            &mut self.dot_left, &mut self.dot_next, &mut self.dot_type, &mut self.zone_left, &mut self.zone_caster,
            &mut self.zone_ai, &mut self.zone_x, &mut self.zone_y, &mut self.zone_period, &mut self.zone_next,
            &mut self.zone_delay, &mut self.zone_rank, &mut self.zone_follow, &mut self.charge_n, &mut self.charge_t, &mut self.chain_left, &mut self.chain_caster, &mut self.chain_cur, &mut self.chain_prev,
            &mut self.chain_dmg, &mut self.chain_radius, &mut self.chain_type, &mut self.chain_wait,
            &mut self.chain_delay, &mut self.ill_out, &mut self.ill_in, &mut self.taunter, &mut self.hex_speed,
            &mut self.hits_taken, &mut self.pend_slot, &mut self.pend_target, &mut self.pend_x, &mut self.pend_y,
        ]
    }

    /// Clears everything held for slot `i` (a fresh unit).
    pub(crate) fn clear_unit(&mut self, i: usize) {
        self.upg[i] = 0;
        self.smod[i * STAT_COUNT..(i + 1) * STAT_COUNT].fill(0);
        self.applied[i * ABILITY_SLOTS * APPLIED..(i + 1) * ABILITY_SLOTS * APPLIED].fill(0);
        self.clear_timed(i);
        self.ill_out[i] = 0;
        self.ill_in[i] = 0;
        self.taunter[i] = -1;
        self.hex_speed[i] = 0;
        self.hits_taken[i] = 0;
        self.pend_slot[i] = -1;
        self.charge_n[i * ABILITY_SLOTS..(i + 1) * ABILITY_SLOTS].fill(-1);
        self.charge_t[i * ABILITY_SLOTS..(i + 1) * ABILITY_SLOTS].fill(0);
    }

    /// Drops the timed modifiers and damage over time of slot `i`, leaving permanent stats alone.
    pub(crate) fn clear_timed(&mut self, i: usize) {
        for k in i * TMOD_SLOTS..(i + 1) * TMOD_SLOTS {
            if self.tmod_code[k] >= 0 {
                let code = self.tmod_code[k] as usize;
                self.smod[i * STAT_COUNT + code] -= self.tmod_amt[k];
            }
            self.tmod_code[k] = -1;
            self.tmod_amt[k] = 0;
            self.tmod_left[k] = 0;
        }
        for k in i * DOT_SLOTS..(i + 1) * DOT_SLOTS {
            self.dot_left[k] = 0;
            self.dot_src[k] = -1;
        }
    }
}

impl Sim {
    pub(crate) fn stat(&self, i: usize, code: usize) -> i32 {
        self.spell.smod.get(i * STAT_COUNT + code).copied().unwrap_or(0)
    }

    pub(crate) fn smod_add(&mut self, i: usize, code: usize, delta: i32) {
        self.spell.smod[i * STAT_COUNT + code] += delta;
        if code == S_RANGE {
            self.range_q[i] += fixed::from_milli(delta);
            self.range_sq[i] = fixed::mul(self.range_q[i], self.range_q[i]);
        }
        self.touch(i);
    }

    /// Weapon damage of unit `i` with its stat modifiers, Q16.16.
    pub(crate) fn eff_damage(&self, i: usize) -> i32 {
        self.damage[i].wrapping_add(fixed::from_int(self.stat(i, S_DAMAGE)))
    }

    /// Armour of unit `i` with its stat modifiers, Q16.16.
    pub(crate) fn eff_armor(&self, i: usize) -> i32 {
        let hundredths = (self.stat(i, S_ARMOR_X100) as i64 * fixed::ONE as i64 / 100) as i32;
        self.armor[i].wrapping_add(fixed::from_int(self.stat(i, S_ARMOR))).wrapping_add(hundredths)
    }

    /// Magic resistance of unit `i` in percent: the hero's base and its bonus stacked multiplicatively.
    pub(crate) fn magic_resist_pct(&self, i: usize) -> i32 {
        let base = self.hero_def(i).map_or(0, |d| d.magic_resist) as i64;
        let bonus = self.stat(i, S_MAGIC_RESIST) as i64;
        let kept = (100 - base) * (100 - bonus) / 100;
        (100 - kept) as i32
    }

    /// Ticks between attacks of unit `i`: its base period scaled by bonus attack speed (20 to 700 total).
    pub(crate) fn attack_period(&self, i: usize) -> i32 {
        let total = (100 + self.stat(i, S_ATTACK_SPEED)).clamp(20, 700) as i64;
        ((self.period[i] as i64 * 100 / total) as i32).max(1)
    }

    /// Rolls `pct` percent on the sim's generator. 0 never hits and 100 always does, without a roll.
    pub(crate) fn roll_pct(&mut self, pct: i32) -> bool {
        if pct <= 0 {
            return false;
        }
        if pct >= 100 {
            return true;
        }
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.rng >> 33) % 100) < pct as u64
    }

    fn rand_below(&mut self, n: u32) -> u32 {
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.rng >> 33) % n.max(1) as u64) as u32
    }

    /// A random offset between `min` and `max` milli-units from the origin, along one of sixteen directions.
    pub(crate) fn scatter_offset(&mut self, min: i32, max: i32) -> (i32, i32) {
        const DIRS: [(i64, i64); 16] = [
            (1000, 0), (924, 383), (707, 707), (383, 924), (0, 1000), (-383, 924), (-707, 707), (-924, 383),
            (-1000, 0), (-924, -383), (-707, -707), (-383, -924), (0, -1000), (383, -924), (707, -707), (924, -383),
        ];
        let (dx, dy) = DIRS[self.rand_below(16) as usize];
        let r = fixed::from_milli(min + self.rand_below((max - min).max(1) as u32) as i32) as i64;
        ((r * dx / 1000) as i32, (r * dy / 1000) as i32)
    }

    pub(crate) fn invulnerable(&self, i: usize) -> bool {
        self.status_left(i, INVULNERABLE) > 0
    }

    pub(crate) fn spell_immune(&self, i: usize) -> bool {
        self.status_left(i, SPELL_IMMUNE) > 0
    }

    pub(crate) fn hexed(&self, i: usize) -> bool {
        self.status_left(i, HEX) > 0
    }

    pub(crate) fn disarmed(&self, i: usize) -> bool {
        self.status_left(i, DISARM) > 0 || self.hexed(i)
    }

    pub(crate) fn muted(&self, i: usize) -> bool {
        self.status_left(i, MUTE) > 0
    }

    /// Whether a hostile spell of the running ability is turned away by the receiver's spell immunity.
    fn turned_away(&self, caster: usize, j: usize) -> bool {
        j < self.n && !self.spell.pierce && self.spell_immune(j) && self.hostile(self.player[caster], self.player[j])
    }

    /// The extension record of catalog ability `ai`.
    pub(crate) fn ext_of(&self, ai: usize) -> AbilityExt {
        self.cat.ext.get(ai).copied().unwrap_or_default()
    }

    /// Learned rank of catalog ability `ai` on unit `unit`: its slot's rank, 1 for an ability with no
    /// skill tree, 0 when not learned or not held.
    pub(crate) fn rank_of(&self, unit: usize, ai: usize) -> usize {
        if self.spell.rank_override > 0 {
            return self.spell.rank_override as usize;
        }
        let base = unit * ABILITY_SLOTS;
        match (base..base + ABILITY_SLOTS).find(|&k| self.abil_idx[k] == ai as i32) {
            Some(k) if self.cat.abilities[ai].skill.is_some() => self.abil_rank[k] as usize,
            Some(_) | None => 1,
        }
    }

    fn upgrade_on(&self, unit: usize, cond: u8) -> bool {
        (self.spell.upg[unit] >> cond) & 1 != 0
    }

    /// Effect `idx` of ability `ai` for `caster`: its stored parameters with the rank-scaled values and
    /// every active scepter, shard or talent modifier applied.
    pub(crate) fn resolve_effect(&self, caster: usize, ai: usize, idx: usize) -> Effect {
        let first = self.cat.abilities[ai].effect_first as usize;
        let mut eff = self.cat.effects[first + idx];
        let ext = match self.cat.ext.get(ai) {
            Some(e) => e,
            None => return eff,
        };
        let rank = self.rank_of(caster, ai).clamp(1, SKILL_RANKS) - 1;
        for s in ext.scaled.iter().filter(|s| s.used && s.effect as usize == idx) {
            *Self::param_mut(&mut eff, s.param) = s.values[rank];
        }
        for m in ext.mods.iter().filter(|m| m.used && m.effect as usize == idx && self.upgrade_on(caster, m.cond)) {
            let p = Self::param_mut(&mut eff, m.param);
            *p = if m.mode == 1 { *p + (*p as i64 * m.delta as i64 / 100) as i32 } else { *p + m.delta };
        }
        eff
    }

    fn param_mut(eff: &mut Effect, param: u8) -> &mut i32 {
        match param {
            0 => &mut eff.p0,
            1 => &mut eff.p1,
            2 => &mut eff.p2,
            _ => &mut eff.p3,
        }
    }

    /// A header field of ability `ai` for `caster`: rank scaling then modifiers. `which` is one of the
    /// `SCALED_*` or `MOD_*` codes; `base` is the unmodified value.
    pub(crate) fn ability_field(&self, caster: usize, ai: usize, which: u8, base: i32) -> i32 {
        let Some(ext) = self.cat.ext.get(ai) else { return base };
        let rank = self.rank_of(caster, ai).clamp(1, SKILL_RANKS) - 1;
        let scaled_code = match which {
            MOD_RANGE => Some(SCALED_CAST_RANGE),
            SCALED_PERIOD | SCALED_CHANCE => Some(which),
            _ => None,
        };
        let mut v = base;
        if let Some(code) = scaled_code {
            if let Some(s) = ext.scaled.iter().find(|s| s.used && s.effect == code) {
                v = s.values[rank];
            }
        }
        let field = match which {
            SCALED_PERIOD | SCALED_CHANCE => None,
            other => Some(other),
        };
        if let Some(field) = field {
            for m in ext.mods.iter().filter(|m| m.used && m.effect == field && self.upgrade_on(caster, m.cond)) {
                v = if m.mode == 1 { v + (v as i64 * m.delta as i64 / 100) as i32 } else { v + m.delta };
            }
        }
        v
    }

    /// Energy cost and cooldown of a cast after modifiers and cooldown reduction.
    pub(crate) fn cast_costs(&self, caster: usize, ai: usize, energy: i32, cooldown: i32) -> (i32, i32) {
        let energy = self.ability_field(caster, ai, MOD_ENERGY, energy).max(0);
        let cooldown = self.ability_field(caster, ai, MOD_COOLDOWN, cooldown).max(0);
        let cdr = self.stat(caster, S_CDR).clamp(0, 90) as i64;
        (energy, (cooldown as i64 * (100 - cdr) / 100) as i32)
    }

    pub(crate) fn cast_range_of(&self, caster: usize, ai: usize) -> i32 {
        let base = self.ext_of(ai).cast_range;
        let range = self.ability_field(caster, ai, MOD_RANGE, base).max(0);
        if range > 0 { range + self.stat(caster, S_CAST_RANGE) } else { 0 }
    }

    pub(crate) fn cast_point_of(&self, caster: usize, ai: usize) -> i32 {
        let base = self.ext_of(ai).cast_point as i32;
        self.ability_field(caster, ai, MOD_CAST_POINT, base).max(0)
    }

    // ---- damage ----

    /// Typed spell damage to unit `j` from `caster`: 1 physical (armour), 2 magical (resistance), 3 pure,
    /// each scaled by the caster's spell amplification. `amount` is whole hit points. Returns true when
    /// the hit killed. Other lane rules (shield, invulnerability, illusion intake) live in `inflict`.
    pub(crate) fn spell_damage(&mut self, caster: i32, j: usize, amount: i32, dtype: i32) -> bool {
        self.spell.kill_flag = false;
        if amount <= 0 || j >= self.n || self.alive[j] == 0 {
            return false;
        }
        if dtype == 1 && self.status_left(j, PHYSICAL_IMMUNE) > 0 {
            return false;
        }
        let mut dmg = fixed::from_int(amount);
        if caster >= 0 && (caster as usize) < self.n {
            let amp = self.stat(caster as usize, S_SPELL_AMP);
            if amp != 0 {
                dmg = (dmg as i64 * (100 + amp as i64) / 100) as i32;
            }
        }
        dmg = match dtype {
            1 => crate::combat::armour_reduced(dmg, self.eff_armor(j)),
            2 => (dmg as i64 * (100 - self.magic_resist_pct(j) as i64) / 100) as i32,
            _ => dmg,
        };
        let before = self.hp[j];
        self.hurt_typed(j, dmg, caster);
        let dealt = before - self.hp[j];
        if dealt > 0 && caster >= 0 && (caster as usize) < self.n {
            let c = caster as usize;
            let steal = if dtype == 1 { self.stat(c, S_LIFESTEAL) } else { self.stat(c, S_SPELL_LIFESTEAL) };
            if steal > 0 && self.alive[c] != 0 {
                self.heal_unit(c, (dealt as i64 * steal as i64 / 100) as i32);
            }
        }
        self.spell.kill_flag = self.alive[j] == 0;
        self.spell.kill_flag
    }

    /// Restores `amount` (Q16.16) hit points to unit `i`, up to its maximum.
    pub(crate) fn heal_unit(&mut self, i: usize, amount: i32) {
        if amount > 0 && self.alive[i] != 0 {
            self.hp[i] = self.hp[i].wrapping_add(amount).min(self.hp_max[i]);
            self.touch(i);
        }
    }

    /// A typed `damage` effect: p0 whole hit points of type p2, to the receiver or, with a radius in p1,
    /// to every living enemy around the cast point.
    pub(crate) fn typed_damage(&mut self, caster: usize, eff: Effect, tx: i32, ty: i32, target: i32) {
        let p = self.player[caster];
        if eff.p1 == 0 {
            if target >= 0 && (target as usize) < self.n {
                let j = target as usize;
                if j != caster && self.hostile(p, self.player[j]) && !self.turned_away(caster, j) {
                    self.spell_damage(caster as i32, j, eff.p0, eff.p2);
                }
            }
            return;
        }
        let mut found = [0u32; AOE_CAP];
        let n = self.collect(tx, ty, fixed::from_milli(eff.p1), &mut found);
        for &u in &found[..n] {
            let j = u as usize;
            if j != caster && self.alive[j] != 0 && self.hostile(p, self.player[j]) && !self.turned_away(caster, j) {
                self.spell_damage(caster as i32, j, eff.p0, eff.p2);
            }
        }
    }

    /// Living units within `rad` (Q16.16) of a point, any distance: the cell buckets for a short reach,
    /// a scan of the unit table beyond. Never writes past `buf`.
    pub(crate) fn collect(&self, x: i32, y: i32, rad: i32, buf: &mut [u32]) -> usize {
        let r2 = rad as i64 * rad as i64;
        let mut count = 0;
        for j in 0..self.n {
            if count == buf.len() {
                break;
            }
            if self.alive[j] != 0 && dist2(self.pos_x[j], self.pos_y[j], x, y) <= r2 {
                buf[count] = j as u32;
                count += 1;
            }
        }
        count
    }

    // ---- effect dispatch ----

    /// Applies the spell effect kinds (20..=29 except the group kinds `aoe` and `if_kill`, which
    /// `apply_effects` runs).
    pub(crate) fn spell_effect(&mut self, caster: usize, eff: Effect, tx: i32, ty: i32, target: i32) {
        let recv = (target >= 0 && (target as usize) < self.n && self.alive[target as usize] != 0).then_some(target as usize);
        match eff.kind {
            K_DOT => {
                if let Some(j) = recv.filter(|&j| !self.turned_away(caster, j)) {
                    self.dot_effect(caster, j, eff);
                }
            }
            K_STATUS => {
                if let Some(j) = recv.filter(|&j| !self.turned_away(caster, j)) {
                    if eff.p0 as usize == HEX {
                        self.spell.hex_speed[j] = eff.p2;
                    }
                    self.status_with_source(caster, j, eff.p0 as usize, eff.p1, crate::status::SLOW_NONE);
                }
            }
            crate::teleport::K_TELEPORT => self.teleport_effect(caster, eff, recv),
            crate::refresh::K_REFRESH => self.refresh_effect(caster, eff),
            K_ATTACK => {
                if let Some(j) = recv.filter(|&j| j != caster && self.hostile(self.player[caster], self.player[j])) {
                    self.strike_pct(caster, j, eff.p0);
                }
            }
            K_TIMED_STAT => {
                if let Some(j) = recv.filter(|&j| !self.turned_away(caster, j)) {
                    self.timed_stat(j, eff);
                }
            }
            K_DISPEL => {
                if let Some(j) = recv {
                    self.dispel(j, eff.p0);
                }
            }
            K_CHAIN => {
                if let Some(j) = recv.filter(|&j| !self.turned_away(caster, j)) {
                    self.chain_start(caster, j, eff);
                }
            }
            K_ZONE => self.zone_start(caster, tx, ty, eff),
            K_ILLUSION => self.illusion_effect(caster, eff),
            K_MANA => {
                if let Some(j) = recv {
                    self.mana_effect(caster, j, eff);
                }
            }
            _ => self.refuse(Refusal::EffectUnsupported),
        }
    }

    /// Applies status `code` to `j`, remembering the caster for a taunt; status resistance shortens it.
    pub(crate) fn status_effect_from(&mut self, caster: usize, eff: Effect, target: i32) {
        if target < 0 || target as usize >= self.n {
            return;
        }
        let j = target as usize;
        let code = (eff.kind as usize).saturating_sub(11);
        if code == crate::status::INVISIBLE {
            self.apply_status(j, code, eff.p0, eff.p1);
        } else if !self.turned_away(caster, j) {
            self.status_with_source(caster, j, code, eff.p0, eff.p1);
        }
    }

    pub(crate) fn status_with_source(&mut self, caster: usize, j: usize, code: usize, ticks: i32, mul: i32) {
        let hostile = self.hostile(self.player[caster], self.player[j]);
        let resist = if hostile { self.stat(j, S_STATUS_RESIST).clamp(0, 100) as i64 } else { 0 };
        let ticks = (ticks as i64 * (100 - resist) / 100) as i32;
        if ticks <= 0 {
            return;
        }
        if code == TAUNT {
            self.spell.taunter[j] = caster as i32;
        }
        self.apply_status(j, code, ticks, mul);
        if code == HEX {
            self.interrupt_channel(j);
        }
    }

    /// Identifies the running effect of the running ability, so two timed effects of one ability stack.
    fn effect_key(&self) -> i32 {
        self.spell.cur_ai * 64 + self.spell.cur_eff
    }

    fn dot_effect(&mut self, caster: usize, j: usize, eff: Effect) {
        let key = self.effect_key();
        let base = j * DOT_SLOTS;
        let slot = (base..base + DOT_SLOTS)
            .find(|&k| self.spell.dot_left[k] > 0 && self.spell.dot_src[k] == caster as i32 && self.spell.dot_key[k] == key)
            .or_else(|| (base..base + DOT_SLOTS).find(|&k| self.spell.dot_left[k] <= 0))
            .or_else(|| (base..base + DOT_SLOTS).min_by_key(|&k| self.spell.dot_left[k]));
        let Some(k) = slot else { return };
        self.spell.dot_src[k] = caster as i32;
        self.spell.dot_key[k] = key;
        self.spell.dot_amt[k] = eff.p0;
        self.spell.dot_period[k] = eff.p1;
        self.spell.dot_left[k] = eff.p2;
        self.spell.dot_next[k] = eff.p1;
        self.spell.dot_type[k] = eff.p3;
        self.touch(j);
    }

    fn timed_stat(&mut self, j: usize, eff: Effect) {
        let code = eff.p0 as usize;
        let key = self.effect_key();
        let mut amount = eff.p1;
        if eff.p3 == 1 {
            amount = match code {
                S_DAMAGE => (fixed::to_int(self.damage[j]) as i64 * eff.p1 as i64 / 100) as i32,
                S_ARMOR => (fixed::to_int(self.armor[j]) as i64 * eff.p1 as i64 / 100) as i32,
                _ => amount,
            };
        }
        let base = j * TMOD_SLOTS;
        let slot = (base..base + TMOD_SLOTS)
            .find(|&k| self.spell.tmod_code[k] == eff.p0 && self.spell.tmod_key[k] == key)
            .or_else(|| (base..base + TMOD_SLOTS).find(|&k| self.spell.tmod_code[k] < 0))
            .or_else(|| (base..base + TMOD_SLOTS).min_by_key(|&k| self.spell.tmod_left[k]));
        let Some(k) = slot else { return };
        if self.spell.tmod_code[k] >= 0 {
            let old = self.spell.tmod_code[k] as usize;
            let old_amt = self.spell.tmod_amt[k];
            self.smod_add(j, old, -old_amt);
        }
        self.spell.tmod_code[k] = eff.p0;
        self.spell.tmod_amt[k] = amount;
        self.spell.tmod_left[k] = eff.p2;
        self.spell.tmod_key[k] = key;
        self.smod_add(j, code, amount);
    }

    /// Removes effects from `j`: mask bit 1 timed buffs, bit 2 debuffs (slow, silence, root, taunt, disarm,
    /// mute, break and damage over time), bit 4 stuns and hexes as well.
    pub(crate) fn dispel(&mut self, j: usize, mask: i32) {
        use crate::status::{BREAK, ROOT, SILENCE, SLOW, STUN};
        if mask & 2 != 0 {
            for s in [SLOW, SILENCE, ROOT, TAUNT, DISARM, MUTE, BREAK] {
                self.clear_status(j, s);
            }
            if mask & 4 != 0 {
                self.clear_status(j, STUN);
                self.clear_status(j, HEX);
            }
            for k in j * DOT_SLOTS..(j + 1) * DOT_SLOTS {
                self.spell.dot_left[k] = 0;
            }
        }
        for k in j * TMOD_SLOTS..(j + 1) * TMOD_SLOTS {
            let code = self.spell.tmod_code[k];
            if code < 0 {
                continue;
            }
            let positive = self.spell.tmod_amt[k] > 0;
            if (positive && mask & 1 != 0) || (!positive && mask & 2 != 0) {
                let amt = self.spell.tmod_amt[k];
                self.smod_add(j, code as usize, -amt);
                self.spell.tmod_code[k] = -1;
                self.spell.tmod_left[k] = 0;
            }
        }
        self.touch(j);
    }

    fn mana_effect(&mut self, caster: usize, j: usize, eff: Effect) {
        let cap_of = |s: &Sim, u: usize| if s.is_hero(u) { s.heroes.mana_max[u] } else { i32::MAX };
        match eff.p1 {
            0 => {
                let cap = cap_of(self, j);
                self.energy[j] = self.energy[j].saturating_add(eff.p0).min(cap);
            }
            _ => {
                let taken = eff.p0.min(self.energy[j]).max(0);
                self.energy[j] -= taken;
                if eff.p1 == 2 {
                    let cap = cap_of(self, caster);
                    self.energy[caster] = self.energy[caster].saturating_add(taken).min(cap);
                    self.touch(caster);
                }
            }
        }
        self.touch(j);
    }

    // ---- areas ----

    /// Runs effects `first + 1 ..= first + n` of the `aoe` effect at `first` on every unit it selects.
    /// p0 is the radius (the half width of a line), p1 the team (0 enemies, 1 allies with the caster,
    /// 2 allies without it, 3 anyone) plus 4 to centre on the caster and 8 for a line from the caster
    /// through the cast point (length: the cast range plus the ability's `line_extra`), p2 how many
    /// effects follow, p3 the required filter bits, with bits 16..23 a cap on targets taken nearest
    /// first and bit 30 for heroes only.
    pub(crate) fn aoe_effects(&mut self, caster: usize, ai: usize, filter: u32, first: usize, tx: i32, ty: i32) {
        let eff = self.resolve_effect(caster, ai, first);
        let n = eff.p2 as usize;
        let p = self.player[caster];
        let (cx, cy) = if eff.p1 & 4 != 0 { (self.pos_x[caster], self.pos_y[caster]) } else { (tx, ty) };
        if eff.p1 & 16 != 0 {
            for e in 1..=n {
                let inner = self.resolve_effect(caster, ai, first + e);
                self.spell.cur_eff = (first + e) as i32;
                self.apply_effect(caster, filter, inner, self.pos_x[caster], self.pos_y[caster], caster as i32, true);
            }
            return;
        }
        let line = eff.p1 & 8 != 0;
        let mut found = [0u32; AOE_CAP];
        let count = if line {
            self.collect(self.pos_x[caster], self.pos_y[caster], fixed::from_milli(self.cast_range_of(caster, ai) + self.ext_of(ai).line_extra), &mut found)
        } else {
            self.collect(cx, cy, fixed::from_milli(eff.p0), &mut found)
        };
        let structure = 1u32 << 6;
        let wanted = eff.p3 as u32 & 0xFFF;
        let cap = ((eff.p3 as u32 >> 16) & 0xFF) as usize;
        let heroes_only = eff.p3 as u32 & (1 << 30) != 0;
        let (lx, ly) = (tx.wrapping_sub(self.pos_x[caster]) as i64, ty.wrapping_sub(self.pos_y[caster]) as i64);
        let len = isqrt64((lx * lx + ly * ly) as u64).max(1) as i128;
        let reach = fixed::from_milli(self.cast_range_of(caster, ai) + self.ext_of(ai).line_extra) as i128;
        let half = fixed::from_milli(eff.p0) as i128;
        let mut picked = [(0u32, 0i64); AOE_CAP];
        let mut taken = 0;
        for &u in &found[..count] {
            let j = u as usize;
            if self.alive[j] == 0 {
                continue;
            }
            let friend = !self.hostile(p, self.player[j]);
            let team_ok = match eff.p1 & 3 {
                0 => !friend,
                1 => friend,
                2 => friend && j != caster,
                _ => true,
            };
            let kind_ok = if wanted == 0 { self.filter[j] & structure == 0 } else { self.filter[j] & wanted == wanted };
            if !team_ok || !kind_ok || self.turned_away(caster, j) || (heroes_only && !self.is_hero(j)) {
                continue;
            }
            if line {
                let (wx, wy) = ((self.pos_x[j] - self.pos_x[caster]) as i128, (self.pos_y[j] - self.pos_y[caster]) as i128);
                let along = (wx * lx as i128 + wy * ly as i128) / len;
                let across = (wx * ly as i128 - wy * lx as i128).abs() / len;
                if along < 0 || along > reach || across > half + self.radius[j] as i128 {
                    continue;
                }
            }
            picked[taken] = (u, dist2(self.pos_x[j], self.pos_y[j], cx, cy));
            taken += 1;
        }
        if cap > 0 && taken > cap {
            picked[..taken].sort_by_key(|&(_, d)| d);
            taken = cap;
        }
        for &(u, _) in &picked[..taken] {
            let j = u as usize;
            if self.alive[j] == 0 {
                continue;
            }
            self.spell.kill_flag = false;
            let (jx, jy) = (self.pos_x[j], self.pos_y[j]);
            for e in 1..=n {
                let inner = self.resolve_effect(caster, ai, first + e);
                self.spell.cur_eff = (first + e) as i32;
                self.apply_effect(caster, filter, inner, jx, jy, j as i32, true);
            }
        }
    }

    // ---- chain ----

    fn chain_start(&mut self, caster: usize, j: usize, eff: Effect) {
        self.spell_damage(caster as i32, j, eff.p0, eff.p3);
        if eff.p1 == 0 {
            return;
        }
        let delay = self.ability_field(caster, self.spell.cur_ai.max(0) as usize, SCALED_PERIOD, self.ext_of(self.spell.cur_ai.max(0) as usize).period);
        let Some(k) = (0..CHAIN_SLOTS).find(|&k| self.spell.chain_left[k] <= 0) else { return };
        self.spell.chain_left[k] = eff.p1;
        self.spell.chain_caster[k] = caster as i32;
        self.spell.chain_cur[k] = j as i32;
        self.spell.chain_prev[k] = -1;
        self.spell.chain_dmg[k] = eff.p0;
        self.spell.chain_radius[k] = eff.p2;
        self.spell.chain_type[k] = eff.p3;
        self.spell.chain_delay[k] = delay.max(1);
        self.spell.chain_wait[k] = delay.max(1);
    }

    fn tick_chains(&mut self) {
        for k in 0..CHAIN_SLOTS {
            if self.spell.chain_left[k] <= 0 {
                continue;
            }
            self.spell.chain_wait[k] -= 1;
            if self.spell.chain_wait[k] > 0 {
                continue;
            }
            self.spell.chain_wait[k] = self.spell.chain_delay[k];
            let (caster, cur, prev) = (self.spell.chain_caster[k], self.spell.chain_cur[k], self.spell.chain_prev[k]);
            let (cx, cy) = if cur >= 0 { (self.pos_x[cur as usize], self.pos_y[cur as usize]) } else { (0, 0) };
            let mut found = [0u32; AOE_CAP];
            let n = self.collect(cx, cy, fixed::from_milli(self.spell.chain_radius[k]), &mut found);
            let p = self.player[caster.max(0) as usize];
            let mut best: Option<(usize, i64)> = None;
            let mut fallback: Option<(usize, i64)> = None;
            for &u in &found[..n] {
                let j = u as usize;
                if j as i32 == cur || self.alive[j] == 0 || !self.hostile(p, self.player[j]) || self.invulnerable(j) {
                    continue;
                }
                if self.spell_immune(j) {
                    continue;
                }
                let d = dist2(self.pos_x[j], self.pos_y[j], cx, cy);
                let slot = if j as i32 == prev { &mut fallback } else { &mut best };
                if slot.map_or(true, |(_, b)| d < b) {
                    *slot = Some((j, d));
                }
            }
            match best.or(fallback) {
                Some((j, _)) => {
                    self.spell_damage(caster, j, self.spell.chain_dmg[k], self.spell.chain_type[k]);
                    self.spell.chain_prev[k] = cur;
                    self.spell.chain_cur[k] = j as i32;
                    self.spell.chain_left[k] -= 1;
                }
                None => self.spell.chain_left[k] = 0,
            }
        }
    }

    // ---- zones ----

    fn zone_start(&mut self, caster: usize, x: i32, y: i32, eff: Effect) {
        let Some(k) = (0..ZONE_SLOTS).find(|&k| self.spell.zone_left[k] <= 0) else { return };
        self.spell.zone_left[k] = eff.p2;
        self.spell.zone_caster[k] = caster as i32;
        self.spell.zone_ai[k] = eff.p0;
        self.spell.zone_x[k] = x;
        self.spell.zone_y[k] = y;
        self.spell.zone_period[k] = eff.p1;
        self.spell.zone_next[k] = eff.p1;
        self.spell.zone_delay[k] = eff.p3 & ZONE_DELAY_MASK;
        self.spell.zone_follow[k] = (eff.p3 >> 30) & 1;
        let parent = self.spell.cur_ai.max(0) as usize;
        self.spell.zone_rank[k] = self.rank_of(caster, parent) as i32;
    }

    fn tick_zones(&mut self) {
        for k in 0..ZONE_SLOTS {
            if self.spell.zone_left[k] <= 0 {
                continue;
            }
            if self.spell.zone_follow[k] == 1 {
                let c = self.spell.zone_caster[k];
                if c < 0 || (c as usize) >= self.n || self.alive[c as usize] == 0 {
                    self.spell.zone_left[k] = 0;
                    continue;
                }
                self.spell.zone_x[k] = self.pos_x[c as usize];
                self.spell.zone_y[k] = self.pos_y[c as usize];
            }
            if self.spell.zone_delay[k] > 0 {
                self.spell.zone_delay[k] -= 1;
                continue;
            }
            self.spell.zone_left[k] -= 1;
            self.spell.zone_next[k] -= 1;
            if self.spell.zone_next[k] <= 0 {
                self.spell.zone_next[k] = self.spell.zone_period[k];
                let (c, ai, x, y) = (self.spell.zone_caster[k], self.spell.zone_ai[k], self.spell.zone_x[k], self.spell.zone_y[k]);
                if c >= 0 && (c as usize) < self.n && ai >= 0 && (ai as usize) < self.cat.abilities.len() {
                    self.spell.rank_override = self.spell.zone_rank[k];
                    self.run_effects(c as usize, ai as usize, x, y, -1);
                    self.spell.rank_override = 0;
                }
            }
        }
    }

    // ---- illusions ----

    fn illusion_effect(&mut self, caster: usize, eff: Effect) {
        let kind = self.type_of[caster];
        if kind < 0 {
            return;
        }
        let ring = fixed::from_milli(1600);
        let player = self.player[caster];
        for n in 0..eff.p0 {
            let (dx, dy) = match n % 8 {
                0 => (ring, 0),
                1 => (-ring, 0),
                2 => (0, ring),
                3 => (0, -ring),
                4 => (ring, ring),
                5 => (-ring, -ring),
                6 => (ring, -ring),
                _ => (-ring, ring),
            };
            let (x, y) = (self.pos_x[caster] + dx, self.pos_y[caster] + dy);
            let Some(j) = self.spawn_for(player, kind as u32, x, y) else { continue };
            self.hp_max[j] = self.hp_max[caster];
            self.hp[j] = self.hp_max[caster];
            self.damage[j] = self.damage[caster];
            self.armor[j] = self.armor[caster];
            self.speed[j] = self.speed[caster];
            self.spell.ill_out[j] = eff.p2.max(1);
            self.spell.ill_in[j] = eff.p3.max(1);
            self.wards.expires[j] = self.ticks.wrapping_add(eff.p1.max(1) as u32);
            self.touch(j);
        }
    }

    pub(crate) fn is_illusion(&self, j: usize) -> bool {
        self.spell.ill_out.get(j).is_some_and(|&v| v != 0)
    }

    // ---- attack modifiers ----

    /// A weapon hit of unit `i` on `b` after the target is chosen: evasion, critical strike, armour,
    /// illusion output, bash, cleave, lifesteal and the proc abilities.
    pub(crate) fn strike(&mut self, i: usize, b: usize) {
        self.strike_pct(i, b, 100);
    }

    /// A weapon hit at `pct` percent of the attacker's damage (the `attack` effect uses it).
    pub(crate) fn strike_pct(&mut self, i: usize, b: usize, pct: i32) {
        if self.status_left(b, PHYSICAL_IMMUNE) > 0 {
            return;
        }
        let evade = self.stat(b, S_EVASION);
        if evade > 0 && self.roll_pct(evade) {
            return;
        }
        let mut raw = (self.eff_damage(i) as i64 * pct as i64 / 100) as i32;
        let crit = self.stat(i, S_CRIT_CHANCE);
        if crit > 0 && self.roll_pct(crit) {
            raw = (raw as i64 * self.stat(i, S_CRIT_MULT).max(100) as i64 / 100) as i32;
        }
        let mut dmg = self.scale_hit(crate::combat::armour_reduced(raw, self.eff_armor(b)), i, b);
        if self.is_illusion(i) {
            dmg = (dmg as i64 * self.spell.ill_out[i] as i64 / 100) as i32;
        }
        let before = self.hp[b];
        self.inflict(b, dmg, i as i32);
        let dealt = (before - self.hp[b]).max(0);
        let steal = self.stat(i, S_LIFESTEAL);
        if steal > 0 && dealt > 0 {
            self.heal_unit(i, (dealt as i64 * steal as i64 / 100) as i32);
        }
        if self.alive[b] != 0 {
            let bash = self.stat(i, S_BASH_CHANCE);
            if bash > 0 && self.roll_pct(bash) {
                let ticks = self.stat(i, S_BASH_TICKS).max(1);
                self.apply_status(b, crate::status::STUN, ticks, crate::status::SLOW_NONE);
            }
        }
        self.cleave(i, b, raw);
        self.hit_procs(i, b);
    }

    /// Cleave: the stat percent of the hit to every other enemy in a cone from the attacker through the
    /// victim, `cleave_radius` long and widening from the start to the end width (all milli-units).
    fn cleave(&mut self, i: usize, b: usize, raw: i32) {
        let pct = self.stat(i, S_CLEAVE);
        let reach = self.stat(i, S_CLEAVE_RADIUS);
        if pct <= 0 || reach <= 0 {
            return;
        }
        let (w0, w1) = (self.stat(i, S_CLEAVE_W0) as i128, self.stat(i, S_CLEAVE_W1).max(self.stat(i, S_CLEAVE_W0)) as i128);
        let p = self.player[i];
        let (dx, dy) = ((self.pos_x[b] - self.pos_x[i]) as i128, (self.pos_y[b] - self.pos_y[i]) as i128);
        let len = isqrt64((dx * dx + dy * dy) as u64).max(1) as i128;
        let reach_q = fixed::from_milli(reach) as i128;
        let mut found = [0u32; AOE_CAP];
        let n = self.collect(self.pos_x[i], self.pos_y[i], fixed::from_milli(reach), &mut found);
        for &u in &found[..n] {
            let j = u as usize;
            if j == b || j == i || self.alive[j] == 0 || !self.hostile(p, self.player[j]) {
                continue;
            }
            let (wx, wy) = ((self.pos_x[j] - self.pos_x[i]) as i128, (self.pos_y[j] - self.pos_y[i]) as i128);
            let along = (wx * dx + wy * dy) / len;
            let across = (wx * dy - wy * dx).abs() / len;
            let width_milli = w0 + (w1 - w0) * along.max(0) / reach_q.max(1);
            let half = width_milli * fixed::ONE as i128 / 2000;
            if along < 0 || along > reach_q || across > half {
                continue;
            }
            let part = (raw as i64 * pct as i64 / 100) as i32;
            let dmg = crate::combat::armour_reduced(part, self.eff_armor(j));
            self.inflict(j, dmg, i as i32);
        }
    }

    /// Proc abilities after a weapon hit of `attacker` on `victim`: the attacker's on-attack procs and
    /// the victim's on-attacked procs.
    fn hit_procs(&mut self, attacker: usize, victim: usize) {
        self.run_procs(attacker, victim, FLAG_PROC_ATTACK, victim as i32);
        if self.alive[victim] != 0 {
            self.spell.hits_taken[victim] += 1;
            self.run_procs(victim, attacker, FLAG_PROC_ATTACKED, attacker as i32);
        }
    }

    /// The abilities unit `owner` runs as passives: its ability slots, then the abilities of the items
    /// in its active slots. Each comes with the slot that holds its cooldown.
    fn passive_sources(&self, owner: usize) -> ([(usize, Slot); ABILITY_SLOTS + crate::inventory::INV_SLOTS], usize) {
        let mut out = [(0, Slot::Ability(0)); ABILITY_SLOTS + crate::inventory::INV_SLOTS];
        let mut n = 0;
        for slot in 0..ABILITY_SLOTS {
            let idx = owner * ABILITY_SLOTS + slot;
            if self.abil_idx[idx] >= 0 {
                out[n] = (self.abil_idx[idx] as usize, Slot::Ability(idx));
                n += 1;
            }
        }
        for slot in (0..crate::inventory::INV_SLOTS).filter(|&s| crate::inventory::slot_active(s)) {
            let key = owner * crate::inventory::INV_SLOTS + slot;
            let ability = if self.inv_count[key] > 0 { self.item_def(self.inv_kind[key]).map(|d| d.ability) } else { None };
            if let Some(ai) = ability.filter(|&a| a != crate::loader::NO_INDEX) {
                out[n] = (ai as usize, Slot::Item(key));
                n += 1;
            }
        }
        (out, n)
    }

    fn run_procs(&mut self, owner: usize, other: usize, flag: u16, target: i32) {
        if self.stunned(owner) || self.alive[owner] == 0 {
            return;
        }
        let (sources, count) = self.passive_sources(owner);
        for &(ai, slot) in &sources[..count] {
            if self.cooldown(slot) > 0 {
                continue;
            }
            let ext = self.ext_of(ai);
            if ext.flags & flag == 0 {
                continue;
            }
            let ability = self.cat.abilities[ai];
            let rank = match slot {
                Slot::Ability(idx) => self.abil_rank[idx] as usize,
                Slot::Item(_) => 0,
            };
            if ability.skill.is_some() && rank == 0 {
                continue;
            }
            let fire = if ext.flags & FLAG_COUNTED != 0 {
                let every = self.ability_field(owner, ai, SCALED_PERIOD, ext.period).max(1);
                self.spell.hits_taken[owner] >= every && {
                    self.spell.hits_taken[owner] = 0;
                    true
                }
            } else {
                let chance = self.ability_field(owner, ai, SCALED_CHANCE, ext.chance);
                self.roll_pct(chance)
            };
            if !fire {
                continue;
            }
            let (_, cooldown) = self.cast_costs(owner, ai, 0, ability.skill.map_or(ability.cooldown, |s| s.cooldown[rank.max(1) - 1]));
            self.set_cooldown(slot, cooldown);
            let at = if flag == FLAG_PROC_ATTACKED { owner } else { other };
            let (x, y) = (self.pos_x[at], self.pos_y[at]);
            self.run_effects(owner, ai, x, y, target);
        }
    }

    // ---- passives, upgrades and talents ----

    /// Recomputes the stat contribution of every passive ability unit `i` holds: the difference between
    /// what its learned rank, scepter, shard and talents give now and what was already added.
    pub(crate) fn refresh_passives(&mut self, i: usize) {
        let mut changed = false;
        for slot in 0..ABILITY_SLOTS {
            let idx = i * ABILITY_SLOTS + slot;
            let ai = self.abil_idx[idx];
            if ai < 0 {
                continue;
            }
            let ai = ai as usize;
            let ability = self.cat.abilities[ai];
            let learned = ability.skill.is_none() || self.abil_rank[idx] > 0;
            if self.ext_of(ai).flags & (FLAG_PASSIVE | FLAG_PASSIVE_STATS) == 0 {
                continue;
            }
            for k in 0..(ability.effect_count as usize).min(APPLIED) {
                if self.cat.effects[ability.effect_first as usize + k].kind != crate::rules::EFFECT_STAT {
                    continue;
                }
                let eff = self.resolve_effect(i, ai, k);
                let want = if learned { eff.p1 } else { 0 };
                let at = idx * APPLIED + k;
                let delta = want - self.spell.applied[at];
                if delta != 0 {
                    self.spell.applied[at] = want;
                    self.smod_add(i, eff.p0 as usize, delta);
                    changed = true;
                }
            }
        }
        if changed {
            self.apply_hero_stats(i);
        }
    }

    /// Current value of stat `code` (see `STAT_COUNT`; the order of `data::format::STAT_NAMES`) on unit `i`: the
    /// sum of its passive and timed modifiers. Zero for an unknown unit or code.
    pub fn stat_value(&self, i: usize, code: usize) -> i32 {
        if i < self.n && code < STAT_COUNT { self.stat(i, code) } else { 0 }
    }

    /// The ability slot of unit `i` holding the ability called `name`.
    pub fn ability_slot(&self, i: usize, name: &str) -> Option<usize> {
        (0..ABILITY_SLOTS).find(|&slot| self.ability_in_slot(i, slot).is_some_and(|a| self.cat.name(self.cat.abilities[a].name) == name))
    }

    /// Whether unit `i` is walking into range to finish a cast.
    pub fn cast_pending(&self, i: usize) -> bool {
        self.spell.pend_slot.get(i).is_some_and(|&s| s >= 0)
    }

    /// Sets upgrade bit 0 (scepter) or 1 (shard) on unit `i`, and refreshes its passives.
    pub fn grant_upgrade(&mut self, i: usize, shard: bool, on: bool) {
        if i >= self.n {
            return;
        }
        let bit = if shard { COND_SHARD } else { COND_SCEPTER };
        self.spell.upg[i] = if on { self.spell.upg[i] | (1 << bit) } else { self.spell.upg[i] & !(1 << bit) };
        self.refresh_passives(i);
    }

    /// Chooses talent `side` (0 or 1) of tier `tier` (0..=3, levels 10, 15, 20, 25) for unit `i`.
    /// Refusals, each changing nothing: a dead or unknown unit, tier or side is NoSlot; a character
    /// level below the tier is SkillLevelGate; a tier already chosen is SkillMaxed.
    pub fn choose_talent(&mut self, i: usize, tier: usize, side: usize) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 || tier >= 4 || side >= 2 {
            return Err(self.reject(Refusal::NoSlot));
        }
        if self.character_level(i) < TALENT_LEVELS[tier] {
            return Err(self.reject(Refusal::SkillLevelGate));
        }
        let pair = 0b11 << (COND_TALENT + 2 * tier as u32);
        if self.spell.upg[i] & pair != 0 {
            return Err(self.reject(Refusal::SkillMaxed));
        }
        self.spell.upg[i] |= 1 << (COND_TALENT + 2 * tier as u32 + side as u32);
        self.refresh_passives(i);
        self.touch(i);
        Ok(())
    }

    /// The upgrade bits of unit `i`: bit 0 scepter, bit 1 shard, bits 2..=9 the chosen talents.
    pub fn upgrade_bits(&self, i: usize) -> i32 {
        self.spell.upg.get(i).copied().unwrap_or(0)
    }

    // ---- per tick ----

    pub(crate) fn tick_spells(&mut self) {
        self.tick_chains();
        self.tick_zones();
        for i in 0..self.n {
            if self.alive[i] == 0 {
                if self.spell.smod[i * STAT_COUNT..(i + 1) * STAT_COUNT].iter().any(|&v| v != 0) {
                    self.spell.clear_timed(i);
                }
                continue;
            }
            self.tick_charges(i);
            self.tick_tmods(i);
            self.tick_dots(i);
            self.tick_auras(i);
            self.tick_taunt(i);
            self.tick_pending(i);
        }
    }

    fn tick_tmods(&mut self, i: usize) {
        for k in i * TMOD_SLOTS..(i + 1) * TMOD_SLOTS {
            if self.spell.tmod_code[k] < 0 {
                continue;
            }
            self.spell.tmod_left[k] -= 1;
            if self.spell.tmod_left[k] <= 0 {
                let (code, amt) = (self.spell.tmod_code[k] as usize, self.spell.tmod_amt[k]);
                self.spell.tmod_code[k] = -1;
                self.smod_add(i, code, -amt);
            }
        }
    }

    fn tick_dots(&mut self, i: usize) {
        for k in i * DOT_SLOTS..(i + 1) * DOT_SLOTS {
            if self.spell.dot_left[k] <= 0 {
                continue;
            }
            self.spell.dot_left[k] -= 1;
            self.spell.dot_next[k] -= 1;
            if self.spell.dot_next[k] <= 0 {
                self.spell.dot_next[k] = self.spell.dot_period[k];
                let (src, amt, dtype) = (self.spell.dot_src[k], self.spell.dot_amt[k], self.spell.dot_type[k]);
                self.spell_damage(src, i, amt, dtype);
                if self.alive[i] == 0 {
                    return;
                }
            }
        }
    }

    fn tick_auras(&mut self, i: usize) {
        let (sources, count) = self.passive_sources(i);
        for &(ai, slot) in &sources[..count] {
            if self.cooldown(slot) > 0 {
                continue;
            }
            let ext = self.ext_of(ai);
            if ext.flags & FLAG_AURA == 0 {
                continue;
            }
            let ability = self.cat.abilities[ai];
            if let Slot::Ability(idx) = slot {
                if ability.skill.is_some() && self.abil_rank[idx] == 0 {
                    continue;
                }
            }
            let period = self.ability_field(i, ai, SCALED_PERIOD, ext.period).max(1);
            self.set_cooldown(slot, period);
            let (x, y) = (self.pos_x[i], self.pos_y[i]);
            self.run_effects(i, ai, x, y, i as i32);
        }
    }

    fn tick_taunt(&mut self, i: usize) {
        if self.status_left(i, TAUNT) <= 0 {
            return;
        }
        let t = self.spell.taunter[i];
        if t < 0 || (t as usize) >= self.n || self.alive[t as usize] == 0 {
            return;
        }
        self.tgt_x[i] = self.pos_x[t as usize];
        self.tgt_y[i] = self.pos_y[t as usize];
        self.order_kind[i] = ORDER_MOVE;
        self.routed[i] = 0;
    }

    /// The unit `i` must attack while taunted, when it stands in weapon reach.
    pub(crate) fn taunt_target(&self, i: usize) -> Option<usize> {
        if self.status_left(i, TAUNT) <= 0 {
            return None;
        }
        let t = self.spell.taunter[i];
        if t < 0 || (t as usize) >= self.n || self.alive[t as usize] == 0 {
            return None;
        }
        let t = t as usize;
        let reach = (self.range_sq[i] as i64) << fixed::SHIFT;
        (dist2(self.pos_x[i], self.pos_y[i], self.pos_x[t], self.pos_y[t]) <= reach).then_some(t)
    }

    // ---- charges ----

    fn max_charges(&self, ai: usize) -> i32 {
        self.ext_of(ai).charges
    }

    /// Whether ability slot key `k` (unit * slots + slot) of unit `i`, holding `ai`, has a charge to spend.
    pub(crate) fn has_charge(&self, k: usize, ai: usize) -> bool {
        self.max_charges(ai) == 0 || self.spell.charge_n[k] != 0
    }

    /// Spends one charge of slot key `k` and starts the restore timer when none is running.
    pub(crate) fn spend_charge(&mut self, i: usize, k: usize, ai: usize) {
        let max = self.max_charges(ai);
        if max == 0 {
            return;
        }
        if self.spell.charge_n[k] < 0 {
            self.spell.charge_n[k] = max;
        }
        if self.spell.charge_n[k] == max {
            let restore = self.ability_field(i, ai, MOD_CHARGE_RESTORE, self.ext_of(ai).charge_restore).max(1);
            self.spell.charge_t[k] = restore;
        }
        self.spell.charge_n[k] -= 1;
    }

    fn tick_charges(&mut self, i: usize) {
        for slot in 0..ABILITY_SLOTS {
            let k = i * ABILITY_SLOTS + slot;
            let ai = self.abil_idx[k];
            if ai < 0 {
                continue;
            }
            let ai = ai as usize;
            let max = self.max_charges(ai);
            if max == 0 {
                continue;
            }
            if self.spell.charge_n[k] < 0 {
                if self.cat.abilities[ai].skill.is_some() && self.abil_rank[k] == 0 {
                    continue;
                }
                self.spell.charge_n[k] = max;
            }
            if self.spell.charge_n[k] < max {
                self.spell.charge_t[k] -= 1;
                if self.spell.charge_t[k] <= 0 {
                    self.spell.charge_n[k] += 1;
                    if self.spell.charge_n[k] < max {
                        self.spell.charge_t[k] = self.ability_field(i, ai, MOD_CHARGE_RESTORE, self.ext_of(ai).charge_restore).max(1);
                    }
                }
            }
        }
    }

    /// Charges held by ability slot `slot` of unit `i`; -1 when the slot has no charges or has not started them.
    pub fn charges_of(&self, i: usize, slot: usize) -> i32 {
        if i >= self.n || slot >= ABILITY_SLOTS {
            return -1;
        }
        self.spell.charge_n[i * ABILITY_SLOTS + slot]
    }

    // ---- walking into cast range ----

    /// Parks a cast that is out of range: the unit walks toward the target and casts on arrival.
    pub(crate) fn begin_pending_cast(&mut self, i: usize, slot: usize, target: i32, x: i32, y: i32) {
        self.spell.pend_slot[i] = slot as i32;
        self.spell.pend_target[i] = target;
        self.spell.pend_x[i] = x;
        self.spell.pend_y[i] = y;
        self.tgt_x[i] = x;
        self.tgt_y[i] = y;
        self.order_kind[i] = ORDER_MOVE;
        self.routed[i] = 0;
        self.touch(i);
    }

    pub(crate) fn cancel_pending_cast(&mut self, i: usize) {
        if let Some(v) = self.spell.pend_slot.get_mut(i) {
            *v = -1;
        }
    }

    fn tick_pending(&mut self, i: usize) {
        let slot = self.spell.pend_slot[i];
        if slot < 0 {
            return;
        }
        if self.stunned(i) || self.hexed(i) {
            self.spell.pend_slot[i] = -1;
            return;
        }
        let ai = self.abil_idx[i * ABILITY_SLOTS + slot as usize];
        if ai < 0 {
            self.spell.pend_slot[i] = -1;
            return;
        }
        let (target, mut x, mut y) = (self.spell.pend_target[i], self.spell.pend_x[i], self.spell.pend_y[i]);
        if target >= 0 {
            let t = target as usize;
            if t >= self.n || self.alive[t] == 0 {
                self.spell.pend_slot[i] = -1;
                return;
            }
            x = self.pos_x[t];
            y = self.pos_y[t];
            self.tgt_x[i] = x;
            self.tgt_y[i] = y;
        }
        let range = fixed::from_milli(self.cast_range_of(i, ai as usize)) as i64;
        if dist2(self.pos_x[i], self.pos_y[i], x, y) <= range * range {
            self.spell.pend_slot[i] = -1;
            self.tgt_x[i] = self.pos_x[i];
            self.tgt_y[i] = self.pos_y[i];
            self.cast_ability(i, slot as usize, target, x, y);
        }
    }

    /// Gives each skilled ability of the placed unit `j` the rank its placer holds in the running ability.
    pub(crate) fn inherit_ranks(&mut self, caster: usize, j: usize) {
        if self.spell.cur_ai < 0 {
            return;
        }
        let rank = self.rank_of(caster, self.spell.cur_ai as usize);
        for slot in 0..ABILITY_SLOTS {
            let k = j * ABILITY_SLOTS + slot;
            let ai = self.abil_idx[k];
            if ai < 0 {
                continue;
            }
            if let Some(skill) = self.cat.abilities[ai as usize].skill {
                self.abil_rank[k] = rank.min(skill.max_rank as usize) as u8;
            }
        }
        self.refresh_passives(j);
    }

    /// Whether the cast point (x, y) is farther from unit `i` than the cast range of ability `ai`.
    pub(crate) fn out_of_cast_range(&self, i: usize, ai: usize, x: i32, y: i32) -> bool {
        let range = self.cast_range_of(i, ai);
        if range <= 0 {
            return false;
        }
        let r = fixed::from_milli(range) as i64;
        dist2(self.pos_x[i], self.pos_y[i], x, y) > r * r
    }
}
