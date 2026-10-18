//! Heroes: units that respawn after a death timer, buy back for gold, and earn passive gold.
//! Every timer and cost is in `HeroRules`, read from a `key=value` text file by
//! `Sim::set_hero_rules_text` (scenarios/moba/hero_rules.txt). Seconds convert to ticks at
//! `TICKS_PER_SECOND`. UNVERIFIED: the default values are placeholders, not sourced Dota 2 numbers.

use crate::arrays::Sim;
use crate::fixed;
use crate::{ABILITY_SLOTS, TICKS_PER_SECOND};

/// Dota 2 attribute coefficients (engine constants, absent from the npc dump; from the Dota 2 wiki
/// Attributes page): 22 health and 0.1 health regeneration per strength, 1/6 armour and 1 attack speed
/// per agility, 12 mana and 0.05 mana regeneration per intelligence, 1 damage per primary point (0.7 per
/// point of every attribute for a universal hero). Attributes and per-hero data are hundredths.
pub(crate) const HP_PER_STR: i32 = 22;
/// Damage per strength point of an item bonus (a strength hero); a hero's own damage uses its primary attribute.
pub(crate) const DAMAGE_PER_STR: i32 = 1;
const MANA_PER_INT: i64 = 12;
const ARMOUR_PER_AGI_DIV: i64 = 6;
const HP_REGEN_PER_STR_X100: i64 = 10;
const MANA_REGEN_PER_INT_X100: i64 = 5;
const UNIVERSAL_DAMAGE_PCT: i64 = 70;
const MANA_BASE_DEFAULT: i32 = 75;
/// Buyback cost is 200 + net worth / 13 (patch 7.29 notes), then 15 percent more (patch 7.36 notes).
/// The 200 and the 15 percent are in `buyback_base` (230); the net worth share is here.
const BUYBACK_NET_WORTH_DIV: i64 = 13;
const BUYBACK_NET_WORTH_PCT: i64 = 115;

/// Per hero type data of a `hero <unit> key=value ...` line in the rules text, written by
/// scripts/gen-heroes.mjs from npc_heroes. Attributes, gains, armor, damage and mana regeneration are
/// hundredths; `primary` is 0 strength, 1 agility, 2 intelligence, 3 universal; `bat` is the base attack
/// time in hundredths of a second (0 keeps the unit's weapon period); `damage` is the min/max midpoint
/// (0 keeps the unit's weapon damage).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeroDef {
    pub primary: u8,
    pub base: [i32; 3],
    pub gain: [i32; 3],
    pub mana: i32,
    pub mana_regen: i32,
    /// Base health regeneration, hundredths per second; may be negative.
    pub hp_regen: i32,
    pub armor: i32,
    pub magic_resist: i32,
    pub bat: i32,
    pub damage: i32,
}

impl HeroDef {
    fn from_pairs(rest: &str) -> Option<HeroDef> {
        let mut d = HeroDef { mana: MANA_BASE_DEFAULT, ..HeroDef::default() };
        for pair in rest.split_whitespace() {
            let (k, v) = pair.split_once('=')?;
            let v: i32 = v.parse().ok()?;
            match k {
                "primary" if (0..=3).contains(&v) => d.primary = v as u8,
                "str" => d.base[0] = v,
                "agi" => d.base[1] = v,
                "int" => d.base[2] = v,
                "str_gain" => d.gain[0] = v,
                "agi_gain" => d.gain[1] = v,
                "int_gain" => d.gain[2] = v,
                "mana" => d.mana = v,
                "mana_regen" => d.mana_regen = v,
                "hp_regen" => d.hp_regen = v,
                "armor" => d.armor = v,
                "mr" if (0..=100).contains(&v) => d.magic_resist = v,
                "bat" => d.bat = v,
                "damage" => d.damage = v,
                _ => return None,
            }
        }
        let nonneg = d.base.iter().chain(d.gain.iter()).chain([d.mana, d.bat, d.damage].iter()).all(|&v| v >= 0);
        nonneg.then_some(d)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeroRules {
    pub respawn_base_s: i32,
    pub respawn_per_level_s: i32,
    pub respawn_cap_s: i32,
    pub buyback_base: i32,
    pub buyback_per_level: i32,
    pub buyback_cooldown_s: i32,
    pub passive_gold_per_s: i32,
    pub team_share_pct: i32,
    /// Attributes at level 0, then the growth per level. UNVERIFIED growth values: placeholders.
    pub str_base: i32,
    pub agi_base: i32,
    pub int_base: i32,
    pub str_gain: i32,
    pub agi_gain: i32,
    pub int_gain: i32,
    /// Target priority and aggro (aggro.rs): 1 turns it on. Hold time of a drawn aggro target, seconds.
    pub aggro_priority: bool,
    /// Whether any damage ends a channel (1, the default) or only a stun, hex or silence does (0, as in Dota).
    pub channel_damage_interrupt: bool,
    pub aggro_hold_s: i32,
    /// Backdoor protection (aggro.rs): damage cut percent (0 = off), clear radius, quiet seconds before
    /// it applies, and health per second a protected structure regenerates.
    pub backdoor_reduction_pct: i32,
    pub backdoor_radius_milli: i32,
    pub backdoor_activation_s: i32,
    pub backdoor_regen_per_s: i32,
}

impl Default for HeroRules {
    fn default() -> Self {
        HeroRules {
            respawn_base_s: 5,
            respawn_per_level_s: 2,
            respawn_cap_s: 60,
            buyback_base: 100,
            buyback_per_level: 20,
            buyback_cooldown_s: 150,
            passive_gold_per_s: 1,
            team_share_pct: 20,
            str_base: 0,
            agi_base: 0,
            int_base: 0,
            str_gain: 2,
            agi_gain: 1,
            int_gain: 1,
            aggro_priority: false,
            channel_damage_interrupt: true,
            aggro_hold_s: 3,
            backdoor_reduction_pct: 0,
            backdoor_radius_milli: 7700,
            backdoor_activation_s: 15,
            backdoor_regen_per_s: 0,
        }
    }
}

impl HeroRules {
    /// Reads `key=value` lines over the current rules. Blank lines and `#` comments are skipped.
    /// False, with the rules unchanged, on an unknown key, a bad number or an invalid value.
    pub fn set_from_text(&mut self, text: &str) -> bool {
        let mut next = *self;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return false;
            };
            let Ok(v) = value.trim().parse::<i32>() else {
                return false;
            };
            let slot = match key.trim() {
                "respawn_base_s" => &mut next.respawn_base_s,
                "respawn_per_level_s" => &mut next.respawn_per_level_s,
                "respawn_cap_s" => &mut next.respawn_cap_s,
                "buyback_base" => &mut next.buyback_base,
                "buyback_per_level" => &mut next.buyback_per_level,
                "buyback_cooldown_s" => &mut next.buyback_cooldown_s,
                "passive_gold_per_s" => &mut next.passive_gold_per_s,
                "team_share_pct" => &mut next.team_share_pct,
                "str_base" => &mut next.str_base,
                "agi_base" => &mut next.agi_base,
                "int_base" => &mut next.int_base,
                "str_gain" => &mut next.str_gain,
                "agi_gain" => &mut next.agi_gain,
                "int_gain" => &mut next.int_gain,
                "channel_damage_interrupt" => {
                    if !(0..=1).contains(&v) {
                        return false;
                    }
                    next.channel_damage_interrupt = v == 1;
                    continue;
                }
                "aggro_priority" => {
                    if !(0..=1).contains(&v) {
                        return false;
                    }
                    next.aggro_priority = v == 1;
                    continue;
                }
                "aggro_hold_s" => &mut next.aggro_hold_s,
                "backdoor_reduction_pct" => &mut next.backdoor_reduction_pct,
                "backdoor_radius_milli" => &mut next.backdoor_radius_milli,
                "backdoor_activation_s" => &mut next.backdoor_activation_s,
                "backdoor_regen_per_s" => &mut next.backdoor_regen_per_s,
                _ => return false,
            };
            *slot = v;
        }
        let valid = [
            next.respawn_base_s,
            next.respawn_per_level_s,
            next.respawn_cap_s,
            next.buyback_base,
            next.buyback_per_level,
            next.buyback_cooldown_s,
            next.passive_gold_per_s,
            next.str_base,
            next.agi_base,
            next.int_base,
            next.str_gain,
            next.agi_gain,
            next.int_gain,
            next.aggro_hold_s,
            next.backdoor_radius_milli,
            next.backdoor_activation_s,
            next.backdoor_regen_per_s,
        ]
        .iter()
        .all(|&v| v >= 0)
            && (0..=100).contains(&next.team_share_pct)
            && (0..=100).contains(&next.backdoor_reduction_pct);
        if valid {
            *self = next;
        }
        valid
    }
}

/// Per-unit hero state, sized to the unit cap. Not included in the state hash or the save bytes.
#[derive(Clone, Debug)]
pub(crate) struct HeroState {
    pub(crate) rules: HeroRules,
    pub(crate) is_hero: Vec<u8>,
    pub(crate) home_x: Vec<i32>,
    pub(crate) home_y: Vec<i32>,
    pub(crate) waiting: Vec<u8>,
    pub(crate) respawn_at: Vec<u32>,
    pub(crate) buyback_ready: Vec<u32>,
    pub(crate) gold: Vec<i32>,
    pub(crate) mana_max: Vec<i32>,
    /// Hero data by catalog unit index; `None` for a type with no `hero` line (global attribute rules apply).
    pub(crate) defs: Vec<Option<HeroDef>>,
}

impl HeroState {
    pub(crate) fn new(cap: usize) -> HeroState {
        HeroState {
            rules: HeroRules::default(),
            is_hero: vec![0; cap],
            home_x: vec![0; cap],
            home_y: vec![0; cap],
            waiting: vec![0; cap],
            respawn_at: vec![0; cap],
            buyback_ready: vec![0; cap],
            gold: vec![0; cap],
            mana_max: vec![0; cap],
            defs: Vec::new(),
        }
    }
}

impl Sim {
    /// Spawns a hero of unit type `kind` for `player` at a Q16.16 point; that point is its home.
    pub fn spawn_hero(&mut self, player: u8, kind: u32, x: i32, y: i32) -> Option<usize> {
        let i = self.spawn_for(player, kind, x, y)?;
        self.enroll_hero(i, x, y);
        Some(i)
    }

    /// Makes the already spawned unit `i` a hero at the Q16.16 point (x, y): its home, its respawn
    /// and gold state, and its hero stats. Map placements of a player-controlled kind call this.
    pub(crate) fn enroll_hero(&mut self, i: usize, x: i32, y: i32) {
        let h = &mut self.heroes;
        h.is_hero[i] = 1;
        h.home_x[i] = x;
        h.home_y[i] = y;
        h.waiting[i] = 0;
        h.respawn_at[i] = 0;
        h.buyback_ready[i] = 0;
        h.gold[i] = 0;
        h.mana_max[i] = 0;
        self.energy[i] = 0;
        self.refresh_passives(i);
        self.apply_hero_stats(i);
        self.hp[i] = self.hp_max[i];
    }

    /// The `hero` line data of the type of unit `i`, if the rules text gave one.
    pub fn hero_def(&self, i: usize) -> Option<HeroDef> {
        let k = *self.type_of.get(i)?;
        if k < 0 {
            return None;
        }
        self.heroes.defs.get(k as usize).copied().flatten()
    }

    /// Strength, agility and intelligence of hero `i` in hundredths: the level-0 base plus the growth
    /// per level (level 1 is zero levels gained). All zero for a non-hero.
    pub fn hero_attributes_x100(&self, i: usize) -> [i32; 3] {
        let own = self.hero_own_attributes_x100(i);
        if !self.is_hero(i) {
            return own;
        }
        let items = self.item_attributes(i);
        [0, 1, 2].map(|k| own[k] + (items[k] + self.stat(i, crate::spell::S_STRENGTH + k)) * 100)
    }

    /// Attributes of hero `i` before item bonuses, in hundredths.
    fn hero_own_attributes_x100(&self, i: usize) -> [i32; 3] {
        if !self.is_hero(i) {
            return [0; 3];
        }
        let lvl = self.level(i) as i64;
        if let Some(d) = self.hero_def(i) {
            return [0, 1, 2].map(|k| (d.base[k] as i64 + d.gain[k] as i64 * lvl) as i32);
        }
        let r = self.heroes.rules;
        let whole = [r.str_base + r.str_gain * lvl as i32, r.agi_base + r.agi_gain * lvl as i32, r.int_base + r.int_gain * lvl as i32];
        whole.map(|v| v * 100)
    }

    /// Whole-point strength, agility and intelligence of hero `i`. All zero for a non-hero.
    pub fn hero_attributes(&self, i: usize) -> [i32; 3] {
        self.hero_attributes_x100(i).map(|v| v / 100)
    }

    /// Max mana of hero `i`: its base mana plus 12 per intelligence point. Zero for a non-hero.
    pub fn hero_max_mana(&self, i: usize) -> i32 {
        if !self.is_hero(i) {
            return 0;
        }
        let base = self.hero_def(i).map_or(MANA_BASE_DEFAULT, |d| d.mana) as i64;
        (base + MANA_PER_INT * self.hero_attributes_x100(i)[2] as i64 / 100) as i32 + self.stat(i, crate::spell::S_MAX_MANA)
    }

    /// Mana regeneration of hero `i` in hundredths per second: the base plus 0.05 per intelligence point.
    pub fn hero_mana_regen_x100(&self, i: usize) -> i32 {
        if !self.is_hero(i) {
            return 0;
        }
        let base = self.hero_def(i).map_or(0, |d| d.mana_regen) as i64;
        (base + MANA_REGEN_PER_INT_X100 * self.hero_attributes_x100(i)[2] as i64 / 100) as i32 + self.stat(i, crate::spell::S_MANA_REGEN)
    }

    /// Armour of unit `i` (Q16.16), its damage per attack (Q16.16), its attack period in ticks and its
    /// mana; zero for an unknown index.
    pub fn armor_of(&self, i: usize) -> i32 {
        self.armor.get(i).copied().unwrap_or(0)
    }

    pub fn damage_of(&self, i: usize) -> i32 {
        self.damage.get(i).copied().unwrap_or(0)
    }

    pub fn period_of(&self, i: usize) -> i32 {
        self.period.get(i).copied().unwrap_or(0)
    }

    pub fn mana_of(&self, i: usize) -> i32 {
        self.energy.get(i).copied().unwrap_or(0)
    }

    /// Magic resistance percent of hero `i`; 0 for a non-hero or a type with no `hero` line.
    pub fn hero_magic_resist_pct(&self, i: usize) -> i32 {
        if !self.is_hero(i) { 0 } else { self.hero_def(i).map_or(0, |d| d.magic_resist) }
    }

    /// Recomputes the attribute-driven stats of live hero `i` from its catalog type and attributes:
    /// max health, mana, damage, armour and attack period. Health and mana move by the change in max,
    /// so a level-up adds the gain and a death-free unit keeps its wounds. A non-hero or dead unit is
    /// left alone; revive and spawn apply it once the unit is alive.
    pub(crate) fn apply_hero_stats(&mut self, i: usize) {
        if !self.is_hero(i) || !self.alive(i) || self.type_of[i] < 0 {
            return;
        }
        let def = self.cat.units[self.type_of[i] as usize];
        let hd = self.hero_def(i);
        let [s, a, n] = self.hero_attributes_x100(i).map(|v| v as i64);
        let q16 = |x100: i64| (x100 * fixed::ONE as i64 / 100) as i32;
        let old_max = self.hp_max[i];
        let new_max = fixed::from_int(def.health + self.stat(i, crate::spell::S_HEALTH)).saturating_add(q16(HP_PER_STR as i64 * s));
        self.hp_max[i] = new_max;
        self.hp[i] = (self.hp[i] + (new_max - old_max)).clamp(0, new_max);
        let armor_x100 = hd.map_or(def.armor as i64 * 100, |d| d.armor as i64) + a / ARMOUR_PER_AGI_DIV;
        self.armor[i] = q16(armor_x100);
        if def.has_weapon() {
            let primary = hd.map_or(0, |d| d.primary);
            let bonus = match primary {
                0 => s,
                1 => a,
                2 => n,
                _ => (s + a + n) * UNIVERSAL_DAMAGE_PCT / 100,
            };
            let base = hd.map_or(0, |d| d.damage) as i64;
            let base = if base > 0 { base } else { def.weapon_damage as i64 * 100 };
            self.damage[i] = q16(base + bonus);
            if let Some(d) = hd.filter(|d| d.bat > 0) {
                let den = 10_000 + a;
                let ticks = (2 * TICKS_PER_SECOND as i64 * d.bat as i64 * 100 + den) / (2 * den);
                self.period[i] = ticks.max(1) as i32;
            }
        }
        let mana_max = self.hero_max_mana(i);
        let old_mana = self.heroes.mana_max[i];
        self.heroes.mana_max[i] = mana_max;
        self.energy[i] = (self.energy[i] + (mana_max - old_mana)).clamp(0, mana_max);
        self.touch(i);
    }

    /// Health regenerated per tick by the strength of hero `i`, Q16.16 (0.1 per second per point).
    /// Zero for a non-hero.
    pub(crate) fn hero_regen_per_tick(&self, i: usize) -> i32 {
        let s = self.hero_attributes_x100(i)[0] as i64;
        let from_str = s * HP_REGEN_PER_STR_X100 * fixed::ONE as i64 / (100 * 100 * TICKS_PER_SECOND as i64);
        let from_stat = self.stat(i, crate::spell::S_HEALTH_REGEN) as i64 * fixed::ONE as i64 / (100 * TICKS_PER_SECOND as i64);
        (from_str + from_stat) as i32
    }

    /// Recomputes every living hero's tracked max mana from its level and items without touching its
    /// mana. A loaded state keeps the energy it saved but has no tracker.
    pub(crate) fn resync_hero_mana(&mut self) {
        for i in 0..self.n {
            if self.is_hero(i) && self.alive(i) {
                self.heroes.mana_max[i] = self.hero_max_mana(i);
            }
        }
    }

    /// Adds the mana of hero `i` for this tick. The whole mana gained is the difference of two floors of
    /// the running total, so a fraction of a point is never lost and no per-unit remainder is stored.
    fn regen_mana(&mut self, i: usize) {
        let max = self.heroes.mana_max[i];
        let rate = self.hero_mana_regen_x100(i) as i64;
        if rate <= 0 || self.energy[i] >= max {
            return;
        }
        let per = 100 * TICKS_PER_SECOND as i64;
        let t = self.ticks as i64;
        let gain = ((t + 1) * rate / per - t * rate / per) as i32;
        if gain > 0 {
            self.energy[i] = (self.energy[i] + gain).min(max);
        }
    }

    pub fn is_hero(&self, i: usize) -> bool {
        self.heroes.is_hero.get(i).is_some_and(|&h| h != 0)
    }

    pub fn hero_rules(&self) -> HeroRules {
        self.heroes.rules
    }

    /// Loads the rules text: `hero <unit> key=value ...` lines set per-type hero data (the unit must
    /// exist in the catalog); every other line is a `key=value` rule. False, nothing changed, on a bad line.
    pub fn set_hero_rules_text(&mut self, text: &str) -> bool {
        let mut rest = String::new();
        let mut defs = self.heroes.defs.clone();
        defs.resize(self.cat.units.len(), None);
        for line in text.lines() {
            let Some(body) = line.trim().strip_prefix("hero ") else {
                rest.push_str(line);
                rest.push('\n');
                continue;
            };
            let (name, pairs) = body.trim().split_once(' ').unwrap_or((body.trim(), ""));
            let (Some(k), Some(d)) = (self.cat.unit_index(name), HeroDef::from_pairs(pairs)) else {
                return false;
            };
            defs[k] = Some(d);
        }
        if !self.heroes.rules.set_from_text(&rest) {
            return false;
        }
        self.heroes.defs = defs;
        self.spell.damage_interrupts = self.heroes.rules.channel_damage_interrupt;
        for i in 0..self.n {
            self.apply_hero_stats(i);
        }
        true
    }

    /// Gold held by hero `i`; 0 for a non-hero.
    pub fn hero_gold(&self, i: usize) -> i32 {
        if self.is_hero(i) { self.heroes.gold[i] } else { 0 }
    }

    /// Ticks until dead hero `i` respawns; 0 when it is alive; -1 for a non-hero.
    pub fn hero_respawn_left(&self, i: usize) -> i32 {
        if !self.is_hero(i) {
            return -1;
        }
        if self.heroes.waiting[i] == 0 {
            return 0;
        }
        self.heroes.respawn_at[i].saturating_sub(self.ticks) as i32
    }

    /// Gold a buyback of hero `i` costs at its current level; -1 for a non-hero.
    pub fn hero_buyback_cost(&self, i: usize) -> i32 {
        if !self.is_hero(i) {
            return -1;
        }
        let r = self.heroes.rules;
        let net_worth_part = self.hero_net_worth(i) as i64 * BUYBACK_NET_WORTH_PCT / (100 * BUYBACK_NET_WORTH_DIV);
        r.buyback_base + r.buyback_per_level * self.level(i) as i32 + net_worth_part as i32
    }

    /// Ticks until hero `i` may buy back again; 0 when ready; -1 for a non-hero.
    pub fn hero_buyback_cooldown_left(&self, i: usize) -> i32 {
        if !self.is_hero(i) {
            return -1;
        }
        self.heroes.buyback_ready[i].saturating_sub(self.ticks) as i32
    }

    /// Buys back dead hero `i` at its home: pays the buyback cost, revives it at once and starts the
    /// cooldown. False, with nothing changed, when it is alive, not dead-and-waiting, on cooldown,
    /// short of gold, or its home cell is full.
    pub fn buyback(&mut self, i: usize) -> bool {
        if !self.is_hero(i) || self.alive(i) || self.heroes.waiting[i] == 0 {
            return false;
        }
        if self.hero_buyback_cooldown_left(i) > 0 {
            return false;
        }
        let cost = self.hero_buyback_cost(i);
        if self.heroes.gold[i] < cost {
            return false;
        }
        self.heroes.gold[i] -= cost;
        if !self.revive(i) {
            self.heroes.gold[i] += cost;
            return false;
        }
        let cooldown = self.heroes.rules.buyback_cooldown_s * TICKS_PER_SECOND;
        self.heroes.buyback_ready[i] = self.ticks.wrapping_add(cooldown as u32);
        true
    }

    /// Books the death timer of hero `j`: base plus a per-level add, capped.
    pub(crate) fn hero_died(&mut self, j: usize) {
        if !self.is_hero(j) {
            return;
        }
        let r = self.heroes.rules;
        let secs = (r.respawn_base_s + r.respawn_per_level_s * self.level(j) as i32)
            .min(r.respawn_cap_s);
        self.heroes.waiting[j] = 1;
        self.heroes.respawn_at[j] = self.ticks.wrapping_add((secs * TICKS_PER_SECOND) as u32);
    }

    /// Pays a kill bounty to killer unit `k`: the team share goes to its side's resource pool, the
    /// rest to the unit's gold slot. UNVERIFIED: team_share_pct default 20 is a placeholder.
    pub(crate) fn hero_bounty(&mut self, k: usize, bounty: i32) {
        let share = bounty * self.heroes.rules.team_share_pct / 100;
        self.heroes.gold[k] = self.heroes.gold[k].saturating_add(bounty - share);
        let side = self.player[k];
        self.add_resources(side, share, 0);
    }

    /// Per tick: passive gold each second for every hero, then respawn of each dead hero whose
    /// timer has run out. A respawn blocked by a full home cell retries next tick.
    pub(crate) fn tick_heroes(&mut self) {
        if self.ticks % TICKS_PER_SECOND as u32 == 0 {
            let passive = self.heroes.rules.passive_gold_per_s;
            for i in 0..self.n {
                if self.is_hero(i) {
                    self.heroes.gold[i] = self.heroes.gold[i].saturating_add(passive);
                }
            }
        }
        for i in 0..self.n {
            if self.heroes.waiting[i] != 0 && self.ticks >= self.heroes.respawn_at[i] {
                self.revive(i);
            }
            if self.is_hero(i) && self.alive(i) {
                self.regen_mana(i);
            }
        }
    }

    /// Puts hero `i` back at its home with full health, keeping its xp and its kill and death
    /// tallies (a slot spawn resets them). False when the home cell cannot take it.
    fn revive(&mut self, i: usize) -> bool {
        let kind = self.type_of[i] as u32;
        let player = self.player[i];
        let (x, y) = (self.heroes.home_x[i], self.heroes.home_y[i]);
        let (xp, kills, deaths) = (self.xp_total[i], self.unit_kills[i], self.unit_deaths[i]);
        let ranks: Vec<u8> = self.abil_rank[i * ABILITY_SLOTS..(i + 1) * ABILITY_SLOTS].to_vec();
        let upgrades = self.spell.upg[i];
        if !self.spawn_at(i, kind, player, x, y) {
            return false;
        }
        self.abil_rank[i * ABILITY_SLOTS..(i + 1) * ABILITY_SLOTS].copy_from_slice(&ranks);
        self.spell.upg[i] = upgrades;
        self.refresh_passives(i);
        self.xp_total[i] = xp;
        self.unit_kills[i] = kills;
        self.unit_deaths[i] = deaths;
        self.heroes.mana_max[i] = 0;
        self.apply_hero_stats(i);
        self.hp[i] = self.hp_max[i];
        self.heroes.waiting[i] = 0;
        true
    }
}
