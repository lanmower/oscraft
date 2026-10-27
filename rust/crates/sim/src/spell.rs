use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::loader::{Effect, LoadError};
use crate::rules::{Refusal, Slot};
use crate::status::{DISARM, HEX, INVULNERABLE, PHYSICAL_IMMUNE, SPELL_IMMUNE};
use crate::*;

pub(crate) const FIRST_SPELL_KIND: u16 = 20;
const K_DOT: u16 = 20;
const K_STATUS: u16 = 21;
const K_TIMED_STAT: u16 = 22;
pub(crate) const K_AOE: u16 = 23;
const K_CHAIN: u16 = 25;
const CHAIN_TYPE_MASK: i32 = 0x00FF_FFFF;
const CHAIN_GROWTH_SHIFT: u32 = 24;
const K_ZONE: u16 = 26;
pub(crate) const K_RESOURCE: u16 = 28;
pub(crate) const K_IF_KILL: u16 = 29;
const K_ATTACK: u16 = 30;

pub const FLAG_PASSIVE: u16 = 1;
pub const FLAG_AURA: u16 = 2;
pub const FLAG_PROC_ATTACK: u16 = 4;
pub const FLAG_PROC_ATTACKED: u16 = 8;
pub const FLAG_PIERCE: u16 = 16;
pub const FLAG_GROUND_PULSE: u16 = 32;
pub const FLAG_COUNTED: u16 = 64;
pub const FLAG_PASSIVE_STATS: u16 = 128;
const FLAG_MASK: u16 = 1023;

pub const STATUS_ENDS_CHANNEL: i32 = 1;
pub const STATUS_PIERCES_IMMUNITY: i32 = 2;
pub const STATUS_NO_RESIST: i32 = 4;
pub const STATUS_RECORDS_SOURCE: i32 = 8;
pub const STATUS_SETS_SPEED: i32 = 16;
pub const STATUS_BLOCKS_PROCS: i32 = 32;
const PROC_LOG_CAP: usize = 1024;

pub const SPELL_KIND_MASK: i32 = 3;
pub const SPELL_ATTACK_SHIFT: i32 = 2;
pub const SPELL_ATTACK_MAX: i32 = 9;

pub fn spell_type_valid(value: i32) -> bool {
    value >= 0 && (value >> SPELL_ATTACK_SHIFT) <= SPELL_ATTACK_MAX
}

pub const STAT_COUNT: usize = 32;
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
pub const S_SLOT_13: usize = 13;
pub const S_HEALTH_REGEN: usize = 14;
pub const S_SLOT_15: usize = 15;
pub const S_SPELL_AMP: usize = 16;
pub const S_BASH_CHANCE: usize = 17;
pub const S_BASH_TICKS: usize = 18;
pub const S_SPEED_PCT: usize = 19;
pub const S_CDR: usize = 20;
pub const S_STATUS_RESIST: usize = 21;
pub const S_SPELL_LIFESTEAL: usize = 25;
pub const S_ARMOR_X100: usize = 26;
pub const S_CAST_RANGE: usize = 27;
pub const S_CLEAVE_W0: usize = 28;
pub const S_CLEAVE_W1: usize = 29;
pub const S_DENY_MARK: usize = 30;
pub const S_ATTACK_BONUS: usize = 31;

pub const SCALED_SLOTS: usize = 6;
pub const MOD_SLOTS: usize = 8;
const AXTN_RECORD: usize = 256;
const AXTN_SCALED_AT: usize = 40;
const AXTN_MODS_AT: usize = 160;
pub const MOD_FIELD_FIRST: u8 = 250;
pub const SCALED_CAST_POINT: u8 = 251;
pub const SCALED_CAST_RANGE: u8 = 252;
pub const SCALED_PERIOD: u8 = 253;
pub const SCALED_CHANCE: u8 = 254;
const MOD_COOLDOWN: u8 = 250;
const MOD_ENERGY: u8 = 251;
const MOD_RANGE: u8 = 252;
const MOD_CAST_POINT: u8 = 253;
const MOD_CHARGE_RESTORE: u8 = 254;

pub(crate) const TMOD_SLOTS: usize = 8;
pub(crate) const DOT_SLOTS: usize = 4;
pub(crate) const ZONE_SLOTS: usize = 16;
pub(crate) const CHAIN_SLOTS: usize = 8;
pub(crate) const AOE_CAP: usize = 2048;
const AOE_MELEE_ONLY: u32 = 1 << 12;
const AOE_RANGED_ONLY: u32 = 1 << 13;
const AOE_LOW_LIFE_ONLY: u32 = 1 << 14;
const AOE_LOW_LIFE_DIVISOR: i64 = 5;
const RANGED_REACH_MILLI: i32 = 3125;
const APPLIED: usize = 8;
const ZONE_DELAY_MASK: i32 = (1 << 29) - 1;
const ZONE_PIN_TARGET: i32 = 1 << 29;
const ZONE_FOLLOW_CASTER: i32 = 1 << 30;
const AOE_OWNED_ONLY: u32 = 1 << 15;
const AOE_NON_ATTRIBUTED_ONLY: u32 = 1 << 29;

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

pub(crate) fn isqrt64(n: u64) -> u64 {
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
            let header = (SCALED_CAST_POINT..=SCALED_CHANCE).contains(&effect);
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

fn grown(damage: i32, p3: i32) -> i32 {
    let percent = (p3 >> CHAIN_GROWTH_SHIFT) & 0x7F;
    (damage as i64 * (100 + percent as i64) / 100) as i32
}

pub(crate) fn spell_effect_error(e: &Effect) -> Option<&'static str> {
    let ok = match e.kind {
        K_DOT => e.p0 >= 0 && e.p1 >= 1 && e.p2 >= 1 && spell_type_valid(e.p3),
        K_STATUS => (0..crate::status::STATUS_KINDS as i32).contains(&e.p0) && e.p1 >= 1,
        K_TIMED_STAT => (0..STAT_COUNT as i32).contains(&e.p0) && e.p2 >= 1 && (0..=1).contains(&e.p3),
        K_AOE => e.p0 >= 0 && (0..=31).contains(&e.p1) && (0..=16).contains(&e.p2) && e.p3 >= 0,
        K_CHAIN => e.p0 >= 0 && e.p1 >= 0 && e.p2 >= 0 && spell_type_valid(e.p3 & CHAIN_TYPE_MASK),
        K_ZONE => e.p0 >= 0 && e.p1 >= 1 && e.p2 >= 1 && e.p3 >= 0,
        K_RESOURCE => e.p0 >= 0 && (0..=3).contains(&e.p1) && e.p2 >= 0 && (e.p2 == 0 || e.p1 == 1),
        K_IF_KILL => (1..=16).contains(&e.p0),
        K_ATTACK => e.p0 >= 0 && e.p1 >= 0,
        crate::point_move::K_POINT_MOVE => e.p0 >= 0,
        crate::refresh::K_REFRESH => (1..=3).contains(&e.p0),
        crate::displace::K_DISPLACE => crate::displace::params_valid(e.p0, e.p1, e.p2, e.p3),
        crate::wall::K_WALL => e.p0 > 0 && e.p1 >= 1 && (0..=1).contains(&e.p3),
        crate::damage_by::K_DAMAGE_BY => crate::damage_by::params_valid(e.p0, e.p1, e.p2, e.p3),
        crate::item_form::K_ITEM_FORM => e.p0 >= 0 && crate::item_form::params_valid(e.p1, e.p2, e.p3),
        crate::execute::K_EXECUTE => e.p0 >= 0 && e.p1 >= 0 && e.p2 & 3 != 0 && (0..=1).contains(&e.p3),
        _ => true,
    };
    (!ok).then_some("spell effect parameters")
}

#[derive(Clone, Debug)]
pub(crate) struct SpellState {
    pub(crate) upg: Vec<i32>,
    pub(crate) smod: Vec<i32>,
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
    pub(crate) hits_made: Vec<i32>,
    pub(crate) status_flags: Vec<i32>,
    pub(crate) stat_bounds: Vec<i32>,
    pub(crate) disp_left: Vec<i32>,
    pub(crate) disp_vx: Vec<i32>,
    pub(crate) disp_vy: Vec<i32>,
    pub(crate) blind_pct: Vec<i32>,
    pub(crate) cur_ai: i32,
    pub(crate) cur_eff: i32,
    pub(crate) kill_flag: bool,
    pub(crate) attack_hit: bool,
    pub(crate) aoe_origin: Option<(i32, i32)>,
    pub(crate) pierce: bool,
    pub(crate) rank_override: i32,
    pub(crate) damage_interrupts: bool,
    pub(crate) heavy_hit_min: i32,
    pub(crate) proc_log: Vec<(u32, u16)>,
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
            hits_made: vec![0; cap],
            status_flags: vec![0; crate::status::STATUS_KINDS],
            stat_bounds: (0..STAT_COUNT).flat_map(|_| [i32::MIN, i32::MAX]).collect(),
            disp_left: vec![0; cap],
            disp_vx: vec![0; cap],
            disp_vy: vec![0; cap],
            blind_pct: vec![0; cap],
            cur_ai: -1,
            cur_eff: 0,
            kill_flag: false,
            attack_hit: false,
            aoe_origin: None,
            pierce: false,
            rank_override: 0,
            damage_interrupts: true,
            heavy_hit_min: i32::MAX,
            proc_log: Vec::new(),
        }
    }

    pub(crate) fn tables(&self) -> [&Vec<i32>; 47] {
        [
            &self.upg, &self.smod, &self.applied, &self.tmod_code, &self.tmod_amt, &self.tmod_left, &self.tmod_key,
            &self.dot_src, &self.dot_key, &self.dot_amt, &self.dot_period, &self.dot_left, &self.dot_next, &self.dot_type,
            &self.zone_left, &self.zone_caster, &self.zone_ai, &self.zone_x, &self.zone_y, &self.zone_period,
            &self.zone_next, &self.zone_delay, &self.zone_rank, &self.zone_follow, &self.charge_n, &self.charge_t, &self.chain_left, &self.chain_caster, &self.chain_cur, &self.chain_prev,
            &self.chain_dmg, &self.chain_radius, &self.chain_type, &self.chain_wait, &self.chain_delay, &self.ill_out,
            &self.ill_in, &self.taunter, &self.hex_speed, &self.hits_taken, &self.hits_made, &self.status_flags, &self.stat_bounds, &self.disp_left, &self.disp_vx, &self.disp_vy, &self.blind_pct,
        ]
    }

    pub(crate) fn tables_mut(&mut self) -> [&mut Vec<i32>; 47] {
        [
            &mut self.upg, &mut self.smod, &mut self.applied, &mut self.tmod_code, &mut self.tmod_amt, &mut self.tmod_left,
            &mut self.tmod_key, &mut self.dot_src, &mut self.dot_key, &mut self.dot_amt, &mut self.dot_period,
            &mut self.dot_left, &mut self.dot_next, &mut self.dot_type, &mut self.zone_left, &mut self.zone_caster,
            &mut self.zone_ai, &mut self.zone_x, &mut self.zone_y, &mut self.zone_period, &mut self.zone_next,
            &mut self.zone_delay, &mut self.zone_rank, &mut self.zone_follow, &mut self.charge_n, &mut self.charge_t, &mut self.chain_left, &mut self.chain_caster, &mut self.chain_cur, &mut self.chain_prev,
            &mut self.chain_dmg, &mut self.chain_radius, &mut self.chain_type, &mut self.chain_wait,
            &mut self.chain_delay, &mut self.ill_out, &mut self.ill_in, &mut self.taunter, &mut self.hex_speed,
            &mut self.hits_taken, &mut self.hits_made, &mut self.status_flags, &mut self.stat_bounds, &mut self.disp_left, &mut self.disp_vx, &mut self.disp_vy, &mut self.blind_pct,
        ]
    }

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
        self.hits_made[i] = 0;
        self.disp_left[i] = 0;
        self.charge_n[i * ABILITY_SLOTS..(i + 1) * ABILITY_SLOTS].fill(-1);
        self.charge_t[i * ABILITY_SLOTS..(i + 1) * ABILITY_SLOTS].fill(0);
    }

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
    pub fn stat(&self, i: usize, code: usize) -> i32 {
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

    pub(crate) fn bounded_stat(&self, i: usize, code: usize) -> i32 {
        let (lo, hi) = (self.spell.stat_bounds[2 * code], self.spell.stat_bounds[2 * code + 1]);
        self.stat(i, code).clamp(lo, hi)
    }

    pub fn set_stat_bounds(&mut self, code: usize, lo: i32, hi: i32) -> bool {
        if code >= STAT_COUNT || lo > hi {
            return false;
        }
        self.spell.stat_bounds[2 * code] = lo;
        self.spell.stat_bounds[2 * code + 1] = hi;
        true
    }

    fn proc_blocked(&self, i: usize) -> bool {
        (0..crate::status::STATUS_KINDS).any(|k| self.status_flag(k) & STATUS_BLOCKS_PROCS != 0 && self.status_left(i, k) > 0)
    }

    pub(crate) fn status_flag(&self, code: usize) -> i32 {
        self.spell.status_flags.get(code).copied().unwrap_or(0)
    }

    pub fn set_status_flags(&mut self, kind: usize, bits: i32) -> bool {
        match self.spell.status_flags.get_mut(kind) {
            Some(flags) => {
                *flags = bits;
                true
            }
            None => false,
        }
    }

    pub fn status_index(&self, name: &str) -> Option<usize> {
        crate::status::STATUS_NAMES.iter().position(|&n| n == name)
    }

    pub(crate) fn eff_damage(&self, i: usize) -> i32 {
        self.damage[i].wrapping_add(fixed::from_int(self.stat(i, S_DAMAGE)))
    }

    pub(crate) fn eff_armor(&self, i: usize) -> i32 {
        let hundredths = (self.stat(i, S_ARMOR_X100) as i64 * fixed::ONE as i64 / 100) as i32;
        self.armor[i].wrapping_add(fixed::from_int(self.stat(i, S_ARMOR))).wrapping_add(hundredths)
    }

    pub(crate) fn attack_period(&self, i: usize) -> i32 {
        let total = (100 + self.bounded_stat(i, S_ATTACK_SPEED)).max(1) as i64;
        ((self.period[i] as i64 * 100 / total) as i32).max(1)
    }

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

    pub fn random_bits(&mut self) -> u64 {
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.rng >> 11
    }

    pub(crate) fn rand_below(&mut self, n: u32) -> u32 {
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.rng >> 33) % n.max(1) as u64) as u32
    }

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

    pub(crate) fn turned_away(&self, caster: usize, j: usize) -> bool {
        j < self.n && !self.spell.pierce && self.spell_immune(j) && self.hostile(self.player[caster], self.player[j])
    }

    pub(crate) fn ext_of(&self, ai: usize) -> AbilityExt {
        self.cat.ext.get(ai).copied().unwrap_or_default()
    }

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

    pub(crate) fn cast_costs(&self, caster: usize, ai: usize, energy: i32, cooldown: i32) -> (i32, i32) {
        let energy = self.ability_field(caster, ai, MOD_ENERGY, energy).max(0);
        let cooldown = self.ability_field(caster, ai, MOD_COOLDOWN, cooldown).max(0);
        let cdr = self.bounded_stat(caster, S_CDR) as i64;
        (energy, (cooldown as i64 * (100 - cdr) / 100) as i32)
    }

    pub(crate) fn cast_range_of(&self, caster: usize, ai: usize) -> i32 {
        let base = self.ext_of(ai).cast_range;
        let range = self.ability_field(caster, ai, MOD_RANGE, base).max(0);
        if range > 0 { range + self.stat(caster, S_CAST_RANGE) } else { 0 }
    }

    pub(crate) fn cast_point_of(&self, caster: usize, ai: usize) -> i32 {
        let ext = self.ext_of(ai);
        let rank = self.rank_of(caster, ai).clamp(1, SKILL_RANKS) - 1;
        let mut v = ext.cast_point as i32;
        if let Some(s) = ext.scaled.iter().find(|s| s.used && s.effect == SCALED_CAST_POINT) {
            v = s.values[rank];
        }
        for m in ext.mods.iter().filter(|m| m.used && m.effect == MOD_CAST_POINT && self.upgrade_on(caster, m.cond)) {
            v = if m.mode == 1 { v + (v as i64 * m.delta as i64 / 100) as i32 } else { v + m.delta };
        }
        v.max(0)
    }

    pub fn spell_damage(&mut self, caster: i32, j: usize, amount: i32, dtype: i32) -> bool {
        self.spell.kill_flag = false;
        if amount <= 0 || j >= self.n || self.alive[j] == 0 {
            return false;
        }
        let kind = dtype & SPELL_KIND_MASK;
        let attack = dtype >> SPELL_ATTACK_SHIFT;
        if kind == 1 && self.status_left(j, PHYSICAL_IMMUNE) > 0 {
            return false;
        }
        let mut dmg = fixed::from_int(amount);
        if caster >= 0 && (caster as usize) < self.n {
            let amp = self.stat(caster as usize, S_SPELL_AMP);
            if amp != 0 {
                dmg = (dmg as i64 * (100 + amp as i64) / 100) as i32;
            }
        }
        dmg = match kind {
            1 => self.armour_scaled(dmg, j),
            2 if attack > 0 => self.bonus_magic_scaled(dmg, j),
            2 => self.magic_scaled(dmg, j),
            _ => dmg,
        };
        if attack > 0 {
            dmg = self.attack_factor_scaled(dmg, (attack - 1) as u8, j);
        }
        let before = self.hp[j];
        self.hurt_typed(j, dmg, caster);
        let dealt = before - self.hp[j];
        if dealt > 0 && caster >= 0 && (caster as usize) < self.n {
            let c = caster as usize;
            let steal = if kind == 1 { self.stat(c, S_LIFESTEAL) } else { self.stat(c, S_SPELL_LIFESTEAL) };
            if steal > 0 && self.alive[c] != 0 {
                self.heal_unit(c, (dealt as i64 * steal as i64 / 100) as i32);
            }
        }
        self.spell.kill_flag = self.alive[j] == 0;
        self.spell.kill_flag
    }

    pub fn set_hp_within(&mut self, i: usize, hp: i32) -> bool {
        if i >= self.n || self.alive[i] == 0 {
            return false;
        }
        self.hp[i] = hp.clamp(fixed::ONE, self.hp_max[i].max(fixed::ONE));
        self.touch(i);
        true
    }

    pub(crate) fn heal_unit(&mut self, i: usize, amount: i32) {
        if amount > 0 && self.alive[i] != 0 {
            self.hp[i] = self.hp[i].wrapping_add(amount).min(self.hp_max[i]);
            self.touch(i);
        }
    }

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

    pub(crate) fn spell_effect(&mut self, caster: usize, eff: Effect, tx: i32, ty: i32, target: i32) {
        let recv = (target >= 0 && (target as usize) < self.n && self.alive[target as usize] != 0).then_some(target as usize);
        match eff.kind {
            K_DOT => {
                if let Some(j) = recv.filter(|&j| !self.turned_away(caster, j)) {
                    self.dot_effect(caster, j, eff);
                }
            }
            K_STATUS => {
                let code = eff.p0 as usize;
                let pierce = self.status_flag(code) & STATUS_PIERCES_IMMUNITY != 0;
                if let Some(j) = recv.filter(|&j| pierce || !self.turned_away(caster, j)) {
                    if self.status_flag(code) & STATUS_SETS_SPEED != 0 {
                        self.spell.hex_speed[j] = eff.p2;
                    }
                    if code == crate::status::BLIND {
                        let pct = if eff.p2 <= 0 { 100 } else { eff.p2.min(100) };
                        let held = self.status_left(j, crate::status::BLIND) > 0;
                        self.spell.blind_pct[j] = if held { self.spell.blind_pct[j].max(pct) } else { pct };
                    }
                    self.status_with_source(caster, j, code, eff.p1, crate::status::SLOW_NONE);
                }
            }
            crate::point_move::K_POINT_MOVE => self.point_move_effect(caster, tx, ty),
            crate::refresh::K_REFRESH => self.refresh_effect(caster, eff),
            crate::displace::K_DISPLACE => self.displace_effect(caster, eff, tx, ty, target),
            crate::wall::K_WALL => self.wall_effect(caster, eff, tx, ty),
            crate::damage_by::K_DAMAGE_BY => self.damage_by_effect(caster, eff, target),
            crate::item_form::K_ITEM_FORM => self.item_form_effect(caster, eff),
            crate::execute::K_EXECUTE => self.execute_effect(caster, eff, target),
            K_ATTACK => {
                if let Some(j) = recv.filter(|&j| j != caster && self.hostile(self.player[caster], self.player[j])) {
                    self.strike_pct_stealing(caster, j, eff.p0, eff.p1);
                }
            }
            K_TIMED_STAT => {
                if let Some(j) = recv.filter(|&j| !self.turned_away(caster, j)) {
                    self.timed_stat(j, eff);
                }
            }
            K_CHAIN => {
                if let Some(j) = recv.filter(|&j| !self.turned_away(caster, j)) {
                    self.chain_start(caster, j, eff);
                }
            }
            K_ZONE => self.zone_start(caster, tx, ty, eff, target),
            K_RESOURCE => {
                if let Some(j) = recv {
                    self.resource_effect(caster, j, eff);
                }
            }
            _ => self.refuse(Refusal::EffectUnsupported),
        }
    }

    pub(crate) fn status_effect_from(&mut self, caster: usize, eff: Effect, target: i32) {
        if target < 0 || target as usize >= self.n {
            return;
        }
        let j = target as usize;
        let code = (eff.kind as usize).saturating_sub(11);
        if self.status_flag(code) & STATUS_PIERCES_IMMUNITY != 0 || !self.turned_away(caster, j) {
            let ticks = if eff.p2 > 0 && self.is_attributed(j) { eff.p2 } else { eff.p0 };
            self.status_with_source(caster, j, code, ticks, eff.p1);
        }
    }

    pub(crate) fn status_with_source(&mut self, caster: usize, j: usize, code: usize, ticks: i32, mul: i32) {
        let flags = self.status_flag(code);
        let hostile = self.hostile(self.player[caster], self.player[j]);
        let resist = if hostile && flags & STATUS_NO_RESIST == 0 { self.stat(j, S_STATUS_RESIST).clamp(0, 100) as i64 } else { 0 };
        let ticks = (ticks as i64 * (100 - resist) / 100) as i32;
        if ticks <= 0 {
            return;
        }
        if flags & STATUS_RECORDS_SOURCE != 0 {
            self.spell.taunter[j] = caster as i32;
        }
        self.apply_status(j, code, ticks, mul);
        if flags & STATUS_ENDS_CHANNEL != 0 {
            self.interrupt_channel(j);
        }
    }

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
        let key = self.effect_key();
        self.timed_stat_keyed(j, eff, key);
    }

    pub fn pack_timed_stat(&mut self, j: usize, stat: usize, amount: i32, ticks: i32, key: i32) -> bool {
        if j >= self.n || self.alive[j] == 0 || stat >= STAT_COUNT || ticks <= 0 || key >= 0 {
            return false;
        }
        let eff = Effect { kind: 0, p0: stat as i32, p1: amount, p2: ticks, p3: 0 };
        self.timed_stat_keyed(j, eff, key);
        true
    }

    fn timed_stat_keyed(&mut self, j: usize, eff: Effect, key: i32) {
        let code = eff.p0 as usize;
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

    pub fn clear_timed_mods(&mut self, j: usize, buffs: bool) {
        for k in j * TMOD_SLOTS..(j + 1) * TMOD_SLOTS {
            let code = self.spell.tmod_code[k];
            if code < 0 || (self.spell.tmod_amt[k] > 0) != buffs {
                continue;
            }
            let amt = self.spell.tmod_amt[k];
            self.smod_add(j, code as usize, -amt);
            self.spell.tmod_code[k] = -1;
            self.spell.tmod_left[k] = 0;
        }
        self.touch(j);
    }

    pub(crate) fn clear_timed_stat(&mut self, j: usize, code: usize) {
        for k in j * TMOD_SLOTS..(j + 1) * TMOD_SLOTS {
            if self.spell.tmod_code[k] != code as i32 {
                continue;
            }
            let amt = self.spell.tmod_amt[k];
            self.smod_add(j, code, -amt);
            self.spell.tmod_code[k] = -1;
            self.spell.tmod_left[k] = 0;
        }
    }

    pub fn clear_dots(&mut self, j: usize) {
        for k in j * DOT_SLOTS..(j + 1) * DOT_SLOTS {
            self.spell.dot_left[k] = 0;
        }
        self.touch(j);
    }

    fn resource_effect(&mut self, caster: usize, j: usize, eff: Effect) {
        let cap_of = |s: &Sim, u: usize| if s.is_attributed(u) { s.attributed.mana_max[u] } else { i32::MAX };
        match eff.p1 {
            0 => {
                let cap = cap_of(self, j);
                self.energy[j] = self.energy[j].saturating_add(eff.p0).min(cap);
            }
            3 => {
                if j == caster || self.is_attributed(j) || self.hostile(self.player[caster], self.player[j]) {
                    return;
                }
                let gain = (fixed::to_int(self.hp[j]) as i64 * eff.p0 as i64 / 100) as i32;
                let cap = cap_of(self, caster);
                self.energy[caster] = self.energy[caster].saturating_add(gain).min(cap);
                self.touch(caster);
                self.spell_damage(caster as i32, j, 32767, 3);
            }
            _ => {
                let taken = eff.p0.min(self.energy[j]).max(0);
                self.energy[j] -= taken;
                if eff.p2 > 0 && taken > 0 {
                    let dealt = ((taken as i64 * eff.p2 as i64 + 50) / 100).min(32767) as i32;
                    self.spell_damage(caster as i32, j, dealt, eff.p3);
                }
                if eff.p1 == 2 {
                    let cap = cap_of(self, caster);
                    self.energy[caster] = self.energy[caster].saturating_add(taken).min(cap);
                    self.touch(caster);
                }
            }
        }
        self.touch(j);
    }

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
        let attributed_only = eff.p3 as u32 & (1 << 30) != 0;
        let melee_only = eff.p3 as u32 & AOE_MELEE_ONLY != 0;
        let ranged_only = eff.p3 as u32 & AOE_RANGED_ONLY != 0;
        let low_life_only = eff.p3 as u32 & AOE_LOW_LIFE_ONLY != 0;
        let taper = (eff.p3 as u32 >> 24) & 31;
        let owned_only = eff.p3 as u32 & AOE_OWNED_ONLY != 0;
        let non_attributed_only = eff.p3 as u32 & AOE_NON_ATTRIBUTED_ONLY != 0;
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
            let ranged = self.is_ranged(j);
            let low_life = (self.hp[j] as i64) * AOE_LOW_LIFE_DIVISOR < self.hp_max[j] as i64;
            if !team_ok
                || !kind_ok
                || (owned_only && self.player[j] != p)
                || (non_attributed_only && self.is_attributed(j))
                || self.turned_away(caster, j)
                || (attributed_only && !self.is_attributed(j))
                || (melee_only && ranged)
                || (ranged_only && !ranged)
                || (low_life_only && !low_life)
            {
                continue;
            }
            if line {
                let (wx, wy) = ((self.pos_x[j] - self.pos_x[caster]) as i128, (self.pos_y[j] - self.pos_y[caster]) as i128);
                let along = (wx * lx as i128 + wy * ly as i128) / len;
                let across = (wx * ly as i128 - wy * lx as i128).abs() / len;
                let width = if taper == 0 { half } else { half - half * (32 - taper as i128) / 32 * along.clamp(0, reach) / reach.max(1) };
                if along < 0 || along > reach || across > width + self.radius[j] as i128 {
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
        let outer_origin = self.spell.aoe_origin.replace((cx, cy));
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
        self.spell.aoe_origin = outer_origin;
    }

    fn chain_start(&mut self, caster: usize, j: usize, eff: Effect) {
        self.spell_damage(caster as i32, j, eff.p0, eff.p3 & CHAIN_TYPE_MASK);
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
        self.spell.chain_dmg[k] = grown(eff.p0, eff.p3);
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
                    self.spell_damage(caster, j, self.spell.chain_dmg[k], self.spell.chain_type[k] & CHAIN_TYPE_MASK);
                    self.spell.chain_dmg[k] = grown(self.spell.chain_dmg[k], self.spell.chain_type[k]);
                    self.spell.chain_prev[k] = cur;
                    self.spell.chain_cur[k] = j as i32;
                    self.spell.chain_left[k] -= 1;
                }
                None => self.spell.chain_left[k] = 0,
            }
        }
    }

    fn zone_start(&mut self, caster: usize, x: i32, y: i32, eff: Effect, target: i32) {
        let Some(k) = (0..ZONE_SLOTS).find(|&k| self.spell.zone_left[k] <= 0) else { return };
        self.spell.zone_left[k] = eff.p2;
        self.spell.zone_caster[k] = caster as i32;
        self.spell.zone_ai[k] = eff.p0;
        self.spell.zone_x[k] = x;
        self.spell.zone_y[k] = y;
        self.spell.zone_period[k] = eff.p1;
        self.spell.zone_next[k] = eff.p1;
        self.spell.zone_delay[k] = eff.p3 & ZONE_DELAY_MASK;
        let pinned = eff.p3 & ZONE_PIN_TARGET != 0 && target >= 0 && (target as usize) < self.n;
        self.spell.zone_follow[k] = if pinned { target + 2 } else { (eff.p3 & ZONE_FOLLOW_CASTER != 0) as i32 };
        let parent = self.spell.cur_ai.max(0) as usize;
        self.spell.zone_rank[k] = self.rank_of(caster, parent) as i32;
    }

    fn tick_zones(&mut self) {
        for k in 0..ZONE_SLOTS {
            if self.spell.zone_left[k] <= 0 {
                continue;
            }
            if self.spell.zone_follow[k] >= 2 {
                let t = (self.spell.zone_follow[k] - 2) as usize;
                if t >= self.n || self.alive[t] == 0 {
                    self.spell.zone_left[k] = 0;
                    continue;
                }
                self.spell.zone_x[k] = self.pos_x[t];
                self.spell.zone_y[k] = self.pos_y[t];
            } else if self.spell.zone_follow[k] == 1 {
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
                    let pinned = if self.spell.zone_follow[k] >= 2 { self.spell.zone_follow[k] - 2 } else { -1 };
                    self.run_effects(c as usize, ai as usize, x, y, pinned);
                    self.spell.rank_override = 0;
                }
            }
        }
    }

    pub fn spawn_image(&mut self, caster: usize, x: i32, y: i32, life: i32, out: i32, inc: i32) -> Option<usize> {
        if caster >= self.n || self.type_of[caster] < 0 {
            return None;
        }
        let kind = self.type_of[caster] as u32;
        let player = self.player[caster];
        let j = self.spawn_for(player, kind, x, y)?;
        self.hp_max[j] = self.hp_max[caster];
        self.hp[j] = self.hp_max[caster];
        self.damage[j] = self.damage[caster];
        self.armor[j] = self.armor[caster];
        self.speed[j] = self.speed[caster];
        self.spell.ill_out[j] = out.max(1);
        self.spell.ill_in[j] = inc.max(1);
        self.expiry.expires[j] = self.ticks.wrapping_add(life.max(1) as u32);
        self.touch(j);
        Some(j)
    }

    pub(crate) fn is_illusion(&self, j: usize) -> bool {
        self.spell.ill_out.get(j).is_some_and(|&v| v != 0)
    }

    pub(crate) fn strike(&mut self, i: usize, b: usize) {
        self.strike_pct(i, b, 100);
    }

    pub(crate) fn strike_pct(&mut self, i: usize, b: usize, pct: i32) {
        self.strike_pct_stealing(i, b, pct, 0);
    }

    pub(crate) fn strike_pct_stealing(&mut self, i: usize, b: usize, pct: i32, extra_steal: i32) {
        if self.status_left(b, PHYSICAL_IMMUNE) > 0 {
            return;
        }
        let evade = self.stat(b, S_EVASION);
        if evade > 0 && self.item_stat(i, crate::inventory::STAT_TRUE_STRIKE) <= 0 && self.roll_pct(evade) {
            return;
        }
        let mut raw = (self.eff_damage(i) as i64 * pct as i64 / 100) as i32;
        let creep_pct = self.item_stat(i, crate::inventory::STAT_CREEP_DAMAGE_PCT);
        if creep_pct > 0 && !self.is_attributed(b) && !self.is_structure(b) && self.cat.units[self.type_of[b] as usize].filter & crate::inventory::FILTER_MASSIVE == 0 {
            raw = (raw as i64 * (100 + creep_pct as i64) / 100) as i32;
        }
        let bonus = self.stat(i, S_ATTACK_BONUS);
        if bonus > 0 {
            raw = (raw as i64 * (100 + bonus as i64) / 100) as i32;
            self.clear_timed_stat(i, S_ATTACK_BONUS);
        }
        let crit = self.stat(i, S_CRIT_CHANCE);
        if crit > 0 && self.roll_pct(crit) {
            raw = (raw as i64 * self.stat(i, S_CRIT_MULT).max(100) as i64 / 100) as i32;
        }
        let mut dmg = self.scale_hit(self.armour_scaled(raw, b), i, b);
        if self.is_illusion(i) {
            dmg = (dmg as i64 * self.spell.ill_out[i] as i64 / 100) as i32;
        }
        let nonhero_block = self.item_stat(b, crate::inventory::STAT_BLOCK_CHANCE_NONHERO);
        let block = if nonhero_block > 0 && !self.is_attributed(i) { nonhero_block } else { self.item_stat(b, crate::inventory::STAT_BLOCK_CHANCE) };
        if block > 0 && self.roll_pct(block) {
            let blocked = fixed::from_int(self.item_stat(b, crate::inventory::STAT_BLOCK_AMOUNT));
            dmg = (dmg - blocked).max(0);
        }
        let before = self.hp[b];
        self.spell.attack_hit = true;
        self.inflict(b, dmg, i as i32);
        self.spell.attack_hit = false;
        let dealt = (before - self.hp[b]).max(0);
        let steal = self.stat(i, S_LIFESTEAL) + extra_steal.max(0);
        if steal > 0 && dealt > 0 {
            self.heal_unit(i, (dealt as i64 * steal as i64 / 100) as i32);
        }
        if self.alive[b] != 0 {
            let ranged = self.is_ranged(i);
            let ranged_chance = self.item_stat(i, crate::inventory::STAT_BASH_CHANCE_RANGED);
            let bash = if ranged && ranged_chance > 0 { ranged_chance } else { self.stat(i, S_BASH_CHANCE) };
            if bash > 0 && self.ticks >= self.bash_ready[i] && self.roll_pct(bash) {
                let ticks = self.stat(i, S_BASH_TICKS).max(1);
                self.apply_status(b, crate::status::STUN, ticks, crate::status::SLOW_NONE);
                let bonus = self.item_stat(i, crate::inventory::STAT_BASH_DAMAGE);
                if bonus > 0 {
                    let extra = self.armour_scaled(fixed::from_int(bonus), b);
                    self.inflict(b, extra, i as i32);
                }
                let cooldown = self.item_stat(i, crate::inventory::STAT_BASH_COOLDOWN);
                if cooldown > 0 {
                    self.bash_ready[i] = self.ticks.saturating_add(cooldown as u32);
                }
            }
        }
        self.cleave(i, b, raw);
        self.hit_procs(i, b);
    }

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
            let dmg = self.armour_scaled(part, j);
            self.inflict(j, dmg, i as i32);
        }
    }

    fn hit_procs(&mut self, attacker: usize, victim: usize) {
        self.spell.hits_made[attacker] += 1;
        self.run_procs(attacker, victim, FLAG_PROC_ATTACK, victim as i32);
        if self.alive[victim] != 0 {
            self.spell.hits_taken[victim] += 1;
            self.run_procs(victim, attacker, FLAG_PROC_ATTACKED, attacker as i32);
        }
    }

    fn passive_sources(&self, owner: usize) -> ([(usize, Slot); ABILITY_SLOTS + 2 * crate::inventory::INV_SLOTS], usize) {
        let mut out = [(0, Slot::Ability(0)); ABILITY_SLOTS + 2 * crate::inventory::INV_SLOTS];
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
            let def = if self.inv_count[key] > 0 { self.item_def(self.inv_kind[key]) } else { None };
            if let Some(ai) = def.map(|d| d.ability).filter(|&a| a != crate::loader::NO_INDEX) {
                out[n] = (ai as usize, Slot::Item(key));
                n += 1;
            }
            let proc_only = |a: usize| self.ext_of(a).flags & (FLAG_PROC_ATTACK | FLAG_PROC_ATTACKED) != 0 && self.ext_of(a).flags & FLAG_AURA == 0;
            if let Some(ai) = def.map(|d| d.aura).filter(|&a| a != crate::loader::NO_INDEX && proc_only(a as usize)) {
                out[n] = (ai as usize, Slot::Item(key));
                n += 1;
            }
        }
        (out, n)
    }

    fn run_procs(&mut self, owner: usize, other: usize, flag: u16, target: i32) {
        if self.proc_blocked(owner) || self.alive[owner] == 0 {
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
                let counter = if flag == FLAG_PROC_ATTACK { &mut self.spell.hits_made[owner] } else { &mut self.spell.hits_taken[owner] };
                *counter >= every && {
                    *counter = 0;
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
            if self.spell.proc_log.len() < PROC_LOG_CAP {
                self.spell.proc_log.push((owner as u32, ai as u16));
            }
            self.run_effects(owner, ai, x, y, target);
        }
    }

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
            self.apply_attr_stats(i);
        }
    }

    pub fn set_upgrade_bit(&mut self, i: usize, bit: u32, on: bool) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 || bit > 9 {
            return Err(self.reject(Refusal::NoSlot));
        }
        let mask = 1 << bit;
        self.spell.upg[i] = if on { self.spell.upg[i] | mask } else { self.spell.upg[i] & !mask };
        self.refresh_passives(i);
        self.touch(i);
        Ok(())
    }

    pub fn upgrade_bits(&self, i: usize) -> i32 {
        self.spell.upg.get(i).copied().unwrap_or(0)
    }

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
            self.tick_item_auras(i);
            self.tick_displace(i);
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

    pub(crate) fn is_ranged(&self, i: usize) -> bool {
        self.range_q[i] >= fixed::from_milli(RANGED_REACH_MILLI)
    }

    fn tick_item_auras(&mut self, i: usize) {
        for slot in (0..crate::inventory::INV_SLOTS).filter(|&s| crate::inventory::slot_active(s)) {
            let key = i * crate::inventory::INV_SLOTS + slot;
            if self.inv_count[key] == 0 {
                continue;
            }
            let Some(def) = self.item_def(self.inv_kind[key]) else { continue };
            if def.aura == crate::loader::NO_INDEX {
                continue;
            }
            let ai = def.aura as usize;
            if self.ext_of(ai).flags & FLAG_AURA == 0 && self.ext_of(ai).flags & (FLAG_PROC_ATTACK | FLAG_PROC_ATTACKED) != 0 {
                continue;
            }
            let period = self.ability_field(i, ai, SCALED_PERIOD, self.ext_of(ai).period).max(1);
            if (self.ticks as i32).wrapping_add(i as i32).rem_euclid(period) != 0 {
                continue;
            }
            let (x, y) = (self.pos_x[i], self.pos_y[i]);
            self.run_effects(i, ai, x, y, i as i32);
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

    fn max_charges(&self, ai: usize) -> i32 {
        self.ext_of(ai).charges
    }

    pub(crate) fn has_charge(&self, k: usize, ai: usize) -> bool {
        self.max_charges(ai) == 0 || self.spell.charge_n[k] != 0
    }

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

    pub fn ability_slot(&self, i: usize, name: &str) -> Option<usize> {
        (0..ABILITY_SLOTS).find(|&slot| self.ability_in_slot(i, slot).is_some_and(|a| self.cat.name(self.cat.abilities[a].name) == name))
    }

    pub fn grant_energy(&mut self, i: usize, amount: i32) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        if amount <= 0 {
            return Err(self.reject(Refusal::Energy));
        }
        self.energy[i] = self.energy[i].saturating_add(amount);
        self.touch(i);
        Ok(())
    }

    pub fn ability_gate(&self, ai: usize) -> Option<(u16, u32, u8)> {
        let ability = self.cat.abilities.get(ai)?;
        Some((self.ext_of(ai).flags, ability.filter as u32, ability.target))
    }

    pub fn cast_in_range(&self, i: usize, ai: usize, x: i32, y: i32) -> bool {
        !self.out_of_cast_range(i, ai, x, y)
    }

    pub fn item_ability_at(&self, unit: usize, slot: usize) -> Option<usize> {
        use crate::loader::NO_INDEX;
        if unit >= self.n || slot >= crate::inventory::INV_SLOTS || !crate::inventory::slot_active(slot) {
            return None;
        }
        let key = unit * crate::inventory::INV_SLOTS + slot;
        let ability = self.item_def(self.inv_kind[key]).map(|d| d.ability)?;
        (ability != NO_INDEX).then_some(ability as usize)
    }

    pub fn take_procs(&mut self) -> Vec<(usize, usize)> {
        std::mem::take(&mut self.spell.proc_log).into_iter().map(|(u, a)| (u as usize, a as usize)).collect()
    }

    pub(crate) fn out_of_cast_range(&self, i: usize, ai: usize, x: i32, y: i32) -> bool {
        let range = self.cast_range_of(i, ai);
        if range <= 0 {
            return false;
        }
        let r = fixed::from_milli(range) as i64;
        dist2(self.pos_x[i], self.pos_y[i], x, y) > r * r
    }
}
