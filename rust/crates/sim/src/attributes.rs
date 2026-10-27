use std::collections::BTreeMap;

use crate::arrays::Sim;
use crate::fixed;
use crate::{ABILITY_SLOTS, TICKS_PER_SECOND};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttrDef {
    pub primary: u8,
    pub base: [i32; 3],
    pub gain: [i32; 3],
    pub mana: i32,
    pub mana_regen: i32,
    pub hp_regen: i32,
    pub armor: i32,
    pub magic_resist: i32,
    pub bat: i32,
    pub damage: i32,
}

impl AttrDef {
    fn from_pairs(rest: &str) -> Option<AttrDef> {
        let mut d = AttrDef::default();
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
pub struct AttrRules {
    pub aggro_priority: bool,
    pub channel_damage_interrupt: bool,
    pub aggro_hold_s: i32,
    pub waypoint_reach_milli: i32,
}

const REQUIRED_KEYS: &[&str] = &["waypoint_reach_milli"];

impl Default for AttrRules {
    fn default() -> Self {
        AttrRules {
            aggro_priority: false,
            channel_damage_interrupt: true,
            aggro_hold_s: 3,
            waypoint_reach_milli: 0,
        }
    }
}

impl AttrRules {
    pub fn set_from_text(&mut self, text: &str) -> Option<BTreeMap<String, i32>> {
        let mut next = *self;
        let mut named = BTreeMap::new();
        let mut seen: Vec<&str> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=')?;
            let v: i32 = value.trim().parse().ok()?;
            let key = key.trim();
            seen.push(key);
            let slot = match key {
                "channel_damage_interrupt" => {
                    if !(0..=1).contains(&v) {
                        return None;
                    }
                    next.channel_damage_interrupt = v == 1;
                    continue;
                }
                "aggro_priority" => {
                    if !(0..=1).contains(&v) {
                        return None;
                    }
                    next.aggro_priority = v == 1;
                    continue;
                }
                "aggro_hold_s" => &mut next.aggro_hold_s,
                "waypoint_reach_milli" => &mut next.waypoint_reach_milli,
                _ => {
                    named.insert(key.to_string(), v);
                    continue;
                }
            };
            *slot = v;
        }
        if !REQUIRED_KEYS.iter().all(|k| seen.contains(k)) {
            return None;
        }
        if next.aggro_hold_s < 0 || next.waypoint_reach_milli < 0 {
            return None;
        }
        *self = next;
        Some(named)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AttributedState {
    pub(crate) rules: AttrRules,
    pub(crate) named: BTreeMap<String, i32>,
    pub(crate) is_attributed: Vec<u8>,
    pub(crate) home_x: Vec<i32>,
    pub(crate) home_y: Vec<i32>,
    pub(crate) waiting: Vec<u8>,
    pub(crate) respawn_at: Vec<u32>,
    pub(crate) gold: Vec<i32>,
    pub(crate) mana_max: Vec<i32>,
    pub(crate) defs: Vec<Option<AttrDef>>,
}

impl AttributedState {
    pub(crate) fn new(cap: usize) -> AttributedState {
        AttributedState {
            rules: AttrRules::default(),
            named: BTreeMap::new(),
            is_attributed: vec![0; cap],
            home_x: vec![0; cap],
            home_y: vec![0; cap],
            waiting: vec![0; cap],
            respawn_at: vec![0; cap],
            gold: vec![0; cap],
            mana_max: vec![0; cap],
            defs: Vec::new(),
        }
    }
}

pub(crate) const MAX_COUNTERS: usize = 1 << 20;

impl Sim {
    pub fn spawn_attributed(&mut self, player: u8, kind: u32, x: i32, y: i32) -> Option<usize> {
        let i = self.spawn_for(player, kind, x, y)?;
        self.attribute_unit(i, x, y);
        Some(i)
    }

    pub(crate) fn attribute_unit(&mut self, i: usize, x: i32, y: i32) {
        let h = &mut self.attributed;
        h.is_attributed[i] = 1;
        h.home_x[i] = x;
        h.home_y[i] = y;
        h.waiting[i] = 0;
        h.respawn_at[i] = 0;
        h.gold[i] = 0;
        h.mana_max[i] = 0;
        self.energy[i] = 0;
        self.refresh_passives(i);
        self.apply_attr_stats(i);
        self.hp[i] = self.hp_max[i];
    }

    pub(crate) fn hold_dead(&mut self, j: usize) {
        if !self.is_attributed(j) {
            return;
        }
        self.attributed.waiting[j] = 1;
        self.attributed.respawn_at[j] = u32::MAX;
    }

    pub fn rule(&self, key: &str) -> Option<i32> {
        let r = &self.attributed.rules;
        match key {
            "aggro_priority" => Some(i32::from(r.aggro_priority)),
            "channel_damage_interrupt" => Some(i32::from(r.channel_damage_interrupt)),
            "aggro_hold_s" => Some(r.aggro_hold_s),
            "waypoint_reach_milli" => Some(r.waypoint_reach_milli),
            _ => self.attributed.named.get(key).copied(),
        }
    }

    pub fn xp_bounty(&self, j: usize) -> i32 {
        self.cat.unit_ext.get(self.type_of.get(j).copied().unwrap_or(-1).max(0) as usize).map_or(0, |e| e.xp_bounty)
    }

    pub fn assist_radius_milli(&self, j: usize) -> i32 {
        self.cat.unit_ext.get(self.type_of.get(j).copied().unwrap_or(-1).max(0) as usize).map_or(0, |e| e.assist_radius)
    }

    pub fn deny_xp_pct(&self, j: usize) -> i32 {
        self.cat.unit_ext.get(self.type_of.get(j).copied().unwrap_or(-1).max(0) as usize).map_or(0, |e| i32::from(e.deny_xp_pct))
    }

    pub fn counter(&self, i: usize, name: &str) -> i64 {
        self.counters.get(&(i, name.to_string())).copied().unwrap_or(0)
    }

    pub fn set_counter(&mut self, i: usize, name: &str, value: i64) {
        self.counters.insert((i, name.to_string()), value);
    }

    pub fn attr_field(&self, i: usize, name: &str) -> Option<i32> {
        let d = self.attr_def(i)?;
        Some(match name {
            "primary" => d.primary as i32,
            "str" => d.base[0],
            "agi" => d.base[1],
            "int" => d.base[2],
            "str_gain" => d.gain[0],
            "agi_gain" => d.gain[1],
            "int_gain" => d.gain[2],
            "mana" => d.mana,
            "mana_regen" => d.mana_regen,
            "hp_regen" => d.hp_regen,
            "armor" => d.armor,
            "mr" => d.magic_resist,
            "bat" => d.bat,
            "damage" => d.damage,
            _ => return None,
        })
    }

    pub fn add_stat_bonus(&mut self, i: usize, code: usize, delta: i32) {
        self.smod_add(i, code, delta);
        self.apply_attr_stats(i);
    }

    pub fn attr_def(&self, i: usize) -> Option<AttrDef> {
        let k = *self.type_of.get(i)?;
        if k < 0 {
            return None;
        }
        self.kind_attr_def(k as usize)
    }

    /// The `unit` line data of unit kind `k` (its catalog index), if the rules text gave one.
    pub fn kind_attr_def(&self, k: usize) -> Option<AttrDef> {
        self.attributed.defs.get(k).copied().flatten()
    }

    pub fn unit_max_mana(&self, i: usize) -> i32 {
        if !self.is_attributed(i) {
            return 0;
        }
        let base = self.attr_def(i).map_or(0, |d| d.mana);
        base + self.stat(i, crate::spell::S_SLOT_15)
    }

    pub fn unit_mana_regen_x100(&self, i: usize) -> i32 {
        if !self.is_attributed(i) {
            return 0;
        }
        self.attr_def(i).map_or(0, |d| d.mana_regen) + self.stat(i, crate::spell::S_SLOT_13)
    }

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

    pub fn set_mana_within(&mut self, i: usize, mana: i32) -> bool {
        if i >= self.n || self.alive[i] == 0 {
            return false;
        }
        let cap = if self.is_attributed(i) { self.attributed.mana_max[i] } else { i32::MAX };
        self.energy[i] = mana.clamp(0, cap);
        self.touch(i);
        true
    }

    pub fn enroll_attributed(&mut self, i: usize, x: i32, y: i32) {
        let h = &mut self.attributed;
        h.is_attributed[i] = 1;
        h.home_x[i] = x;
        h.home_y[i] = y;
        h.waiting[i] = 0;
        h.respawn_at[i] = 0;
        h.gold[i] = 0;
        h.mana_max[i] = 0;
        self.energy[i] = 0;
        self.refresh_passives(i);
        self.apply_attr_stats(i);
        self.hp[i] = self.hp_max[i];
    }

    pub(crate) fn apply_attr_stats(&mut self, i: usize) {
        if !self.is_attributed(i) || !self.alive(i) || self.type_of[i] < 0 {
            return;
        }
        let def = self.cat.units[self.type_of[i] as usize];
        let hd = self.attr_def(i);
        let q16 = |x100: i64| (x100 * fixed::ONE as i64 / 100) as i32;
        let old_max = self.hp_max[i];
        let new_max = fixed::from_int(def.health + self.stat(i, crate::spell::S_HEALTH));
        self.hp_max[i] = new_max;
        self.hp[i] = (self.hp[i] + (new_max - old_max)).clamp(0, new_max);
        let armor_x100 = hd.map_or(def.armor as i64 * 100, |d| d.armor as i64);
        self.armor[i] = q16(armor_x100);
        if def.has_weapon() {
            let base = hd.map_or(0, |d| d.damage) as i64;
            let base = if base > 0 { base } else { def.weapon_damage as i64 * 100 };
            self.damage[i] = q16(base);
            if let Some(d) = hd.filter(|d| d.bat > 0) {
                let den = 10_000;
                let ticks = (2 * TICKS_PER_SECOND as i64 * d.bat as i64 * 100 + den) / (2 * den);
                self.period[i] = ticks.max(1) as i32;
            }
        }
        let mana_max = self.unit_max_mana(i);
        let old_mana = self.attributed.mana_max[i];
        self.attributed.mana_max[i] = mana_max;
        self.energy[i] = (self.energy[i] + (mana_max - old_mana)).clamp(0, mana_max);
        self.touch(i);
    }

    pub(crate) fn unit_regen_per_tick(&self, i: usize) -> i32 {
        let from_stat = self.stat(i, crate::spell::S_HEALTH_REGEN) as i64 * fixed::ONE as i64 / (100 * TICKS_PER_SECOND as i64);
        from_stat as i32
    }

    pub(crate) fn resync_unit_mana(&mut self) {
        for i in 0..self.n {
            if self.is_attributed(i) && self.alive(i) {
                self.attributed.mana_max[i] = self.unit_max_mana(i);
            }
        }
    }

    fn regen_mana(&mut self, i: usize) {
        let max = self.attributed.mana_max[i];
        let rate = self.unit_mana_regen_x100(i) as i64;
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

    pub fn is_attributed(&self, i: usize) -> bool {
        self.attributed.is_attributed.get(i).is_some_and(|&h| h != 0)
    }

    pub fn attr_rules(&self) -> AttrRules {
        self.attributed.rules
    }

    pub fn set_attr_rules_text(&mut self, text: &str) -> bool {
        let mut rest = String::new();
        let mut defs = self.attributed.defs.clone();
        defs.resize(self.cat.units.len(), None);
        for line in text.lines() {
            let Some(body) = line.trim().strip_prefix("attr ") else {
                rest.push_str(line);
                rest.push('\n');
                continue;
            };
            let (name, pairs) = body.trim().split_once(' ').unwrap_or((body.trim(), ""));
            let (Some(k), Some(d)) = (self.cat.unit_index(name), AttrDef::from_pairs(pairs)) else {
                return false;
            };
            defs[k] = Some(d);
        }
        let mut rules = self.attributed.rules;
        let Some(named) = rules.set_from_text(&rest) else {
            return false;
        };
        self.attributed.rules = rules;
        self.attributed.named = named;
        self.attributed.defs = defs;
        self.spell.damage_interrupts = self.attributed.rules.channel_damage_interrupt;
        self.spell.heavy_hit_min = self.attributed.named.get("heavy_hit_min").map_or(i32::MAX, |&v| fixed::from_int(v));
        for i in 0..self.n {
            self.apply_attr_stats(i);
        }
        true
    }

    pub fn unit_gold(&self, i: usize) -> i32 {
        if self.is_attributed(i) { self.attributed.gold[i] } else { 0 }
    }

    pub(crate) fn set_attributed_respawn(&mut self, i: usize, delay_s: Option<u32>) -> Result<(), &'static str> {
        if !self.is_attributed(i) || self.alive(i) || self.attributed.waiting[i] != 1 {
            return Err("not a dead attributed");
        }
        self.attributed.respawn_at[i] = match delay_s {
            Some(s) => self.ticks.wrapping_add(s.saturating_mul(TICKS_PER_SECOND as u32)),
            None => u32::MAX,
        };
        Ok(())
    }

    pub(crate) fn tick_respawns(&mut self) {
        for i in 0..self.n {
            if self.attributed.waiting[i] != 0 && self.ticks >= self.attributed.respawn_at[i] {
                let (x, y) = (self.attributed.home_x[i], self.attributed.home_y[i]);
                self.revive_at_point(i, x, y);
            }
            if self.is_attributed(i) && self.alive(i) {
                self.regen_mana(i);
            }
        }
    }

    pub fn revive_at_point(&mut self, i: usize, x: i32, y: i32) -> bool {
        let kind = self.type_of[i] as u32;
        let player = self.player[i];
        let (xp, kills, deaths) = (self.xp_total[i], self.unit_kills[i], self.unit_deaths[i]);
        let ranks: Vec<u8> = self.abil_rank[i * ABILITY_SLOTS..(i + 1) * ABILITY_SLOTS].to_vec();
        let upgrades = self.spell.upg[i];
        let items = self.snapshot_items(i);
        if !self.spawn_at(i, kind, player, x, y) {
            return false;
        }
        self.restore_items(i, items);
        self.abil_rank[i * ABILITY_SLOTS..(i + 1) * ABILITY_SLOTS].copy_from_slice(&ranks);
        self.spell.upg[i] = upgrades;
        self.refresh_passives(i);
        self.xp_total[i] = xp;
        self.unit_kills[i] = kills;
        self.unit_deaths[i] = deaths;
        self.attributed.mana_max[i] = 0;
        self.apply_attr_stats(i);
        self.hp[i] = self.hp_max[i];
        self.attributed.waiting[i] = 0;
        true
    }

    pub fn refresh_attr_stats(&mut self, i: usize) {
        if i < self.n {
            self.apply_attr_stats(i);
        }
    }

    pub fn set_base_period(&mut self, i: usize, ticks: i32) -> bool {
        if !self.alive(i) || ticks <= 0 {
            return false;
        }
        self.period[i] = ticks;
        self.touch(i);
        true
    }

    pub fn set_skill_rank(&mut self, unit: usize, slot: usize, rank: u8) -> Option<u8> {
        let tree = self.rank_tree(unit, slot)?;
        let rank = (rank as usize).min(tree.max_rank as usize) as u8;
        self.abil_rank[unit * ABILITY_SLOTS + slot] = rank;
        self.refresh_passives(unit);
        Some(rank)
    }

    pub fn taunter_of(&self, i: usize) -> Option<usize> {
        if i >= self.n || self.status_left(i, crate::status::TAUNT) <= 0 {
            return None;
        }
        usize::try_from(self.spell.taunter[i]).ok().filter(|&t| self.alive(t))
    }
}
