//! Attributed units: per-type attribute data (three attributes with a base and a growth each, the
//! primary attribute, mana, regeneration, armour, damage and base attack time) stored per unit type,
//! and the sim-side application of the stat bonuses a mod sets (`add_stat_bonus`). The mod decides every
//! derived stat (scenarios/moba/heroes.lua); the engine stores those numbers and applies them.
//! The engine rules are read from a `key=value` text file by `Sim::set_attr_rules_text` (the pack's
//! attr_rules.txt, named by its manifest). Every other `key=value` line of that file is a named number that
//! the mod reads through `Sim::rule`: the respawn, buy-back, gold, xp and attribute coefficient numbers live there.
//! A dead attributed unit waits for the respawn the mod sets (`set_respawn`, from the mod's on_died) and comes
//! back through `revive_at_point`; with no respawn set it stays dead.

use std::collections::BTreeMap;

use crate::arrays::Sim;
use crate::fixed;
use crate::{ABILITY_SLOTS, TICKS_PER_SECOND};

/// Per unit type attribute data of a `unit <unit> key=value ...` line in the rules text, written by
/// scripts/gen-units.mjs from npc_heroes. Attributes, gains, armor, damage and mana regeneration are
/// hundredths; `primary` is the index of the attribute that drives damage (0 a, 1 b, 2 c, 3 all three); `bat` is the base attack
/// time in hundredths of a second (0 keeps the unit's weapon period); `damage` is the min/max midpoint
/// (0 keeps the unit's weapon damage).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttrDef {
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

/// The engine's own rule values. Copyable; the named values of the rules text live beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttrRules {
    /// Target priority and aggro (aggro.rs): 1 turns it on. Hold time of a drawn aggro target, seconds.
    pub aggro_priority: bool,
    /// Whether any damage ends a channel (1, the default) or only a stun, hex or silence does (0, as in reference).
    pub channel_damage_interrupt: bool,
    pub aggro_hold_s: i32,
    /// Distance, in milli-units, at which a unit moves on to its next waypoint (waypoints.rs).
    pub waypoint_reach_milli: i32,
    /// Percent of a hit a `reduced` damage response keeps (combat.rs inflict): 50 is half.
    pub reduced_response_pct: i32,
}

/// Keys the engine must find in every rules text: a file without one is refused (`set_from_text` returns None).
const REQUIRED_KEYS: &[&str] = &["waypoint_reach_milli", "reduced_response_pct"];

impl Default for AttrRules {
    fn default() -> Self {
        AttrRules {
            aggro_priority: false,
            channel_damage_interrupt: true,
            aggro_hold_s: 3,
            // Neutral until a rules text sets them: a unit reaches a waypoint only on it, and a reduced response keeps the whole hit.
            waypoint_reach_milli: 0,
            reduced_response_pct: 100,
        }
    }
}

impl AttrRules {
    /// Reads `key=value` lines over the current rules. The engine keys set the rules; every other key is
    /// a named value, returned in the map. Blank lines and `#` comments are skipped. None, with the rules
    /// unchanged, on a bad line, a missing engine key or an invalid engine value.
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
                "reduced_response_pct" => &mut next.reduced_response_pct,
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
        if next.aggro_hold_s < 0 || next.waypoint_reach_milli < 0 || !(0..=100).contains(&next.reduced_response_pct) {
            return None;
        }
        *self = next;
        Some(named)
    }
}

/// Per-unit attribute state, sized to the unit cap. Not included in the state hash or the save bytes.
#[derive(Clone, Debug)]
pub(crate) struct AttributedState {
    pub(crate) rules: AttrRules,
    /// The named values of the rules text (every key the engine does not own), read by mods through `Sim::rule`.
    pub(crate) named: BTreeMap<String, i32>,
    pub(crate) is_attributed: Vec<u8>,
    pub(crate) home_x: Vec<i32>,
    pub(crate) home_y: Vec<i32>,
    /// 0 alive, 1 dead awaiting the respawn the mod set (`u32::MAX` until it sets one).
    pub(crate) waiting: Vec<u8>,
    pub(crate) respawn_at: Vec<u32>,
    pub(crate) gold: Vec<i32>,
    pub(crate) mana_max: Vec<i32>,
    /// Unit data by catalog unit index; `None` for a type with no `unit` line (global attribute rules apply).
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

impl Sim {
    /// Spawns unit type `kind` for `player` at a Q16.16 point and gives it the attribute data of its type;
    /// that point is its home.
    pub fn spawn_attributed(&mut self, player: u8, kind: u32, x: i32, y: i32) -> Option<usize> {
        let i = self.spawn_for(player, kind, x, y)?;
        self.attribute_unit(i, x, y);
        Some(i)
    }

    /// Gives the already spawned unit `i` attributed state at the Q16.16 point (x, y): its home, its
    /// respawn and gold state, and its attribute stats. The mod's seat-pick spawn and map placements call this.
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

    /// Marks attributed unit `j` dead and waiting. Its respawn stays unset (never due) until the mod's
    /// on_died calls `set_respawn`. A unit that is not attributed is left alone.
    pub(crate) fn hold_dead(&mut self, j: usize) {
        if !self.is_attributed(j) {
            return;
        }
        self.attributed.waiting[j] = 1;
        self.attributed.respawn_at[j] = u32::MAX;
    }

    /// A value of the rules text, or None. The engine's own keys read from the typed rules (booleans as 0 or 1);
    /// every other `key=value` line is a named value.
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

    /// The xp a kill of unit `j` is worth, its assist radius in milli-units, and its denial xp percent,
    /// from the catalog's unit data (the mod's xp share reads these).
    pub fn xp_bounty(&self, j: usize) -> i32 {
        self.cat.unit_ext.get(self.type_of.get(j).copied().unwrap_or(-1).max(0) as usize).map_or(0, |e| e.xp_bounty)
    }

    pub fn assist_radius_milli(&self, j: usize) -> i32 {
        self.cat.unit_ext.get(self.type_of.get(j).copied().unwrap_or(-1).max(0) as usize).map_or(0, |e| e.assist_radius)
    }

    pub fn deny_xp_pct(&self, j: usize) -> i32 {
        self.cat.unit_ext.get(self.type_of.get(j).copied().unwrap_or(-1).max(0) as usize).map_or(0, |e| i32::from(e.deny_xp_pct))
    }

    /// A named counter of unit `i` the mods keep (`set_counter`); 0 when never set.
    pub fn counter(&self, i: usize, name: &str) -> i64 {
        self.counters.get(&(i, name.to_string())).copied().unwrap_or(0)
    }

    /// Sets the named counter of unit `i`.
    pub fn set_counter(&mut self, i: usize, name: &str, value: i64) {
        self.counters.insert((i, name.to_string()), value);
    }

    /// A field of the `unit` line data of unit `i` by name: primary, str, agi, int, str_gain, agi_gain, int_gain,
    /// mana, mana_regen, hp_regen, armor, mr, bat or damage. None for a unit with no unit line or an unknown name.
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

    /// Adds `delta` to stat `code` of unit `i` (a generic stat modifier, the mod's attribute bonuses)
    /// and recomputes the stats of a unit from them.
    pub fn add_stat_bonus(&mut self, i: usize, code: usize, delta: i32) {
        self.smod_add(i, code, delta);
        self.apply_attr_stats(i);
    }

    /// The `unit` line data of the type of unit `i`, if the rules text gave one.
    pub fn attr_def(&self, i: usize) -> Option<AttrDef> {
        let k = *self.type_of.get(i)?;
        if k < 0 {
            return None;
        }
        self.attributed.defs.get(k as usize).copied().flatten()
    }

    /// Max mana of unit `i`: the base mana of its unit line plus its S_SLOT_15 stat (the mod's mana bonus, set
    /// by the pack's heroes.lua). Zero for a non-unit.
    pub fn unit_max_mana(&self, i: usize) -> i32 {
        if !self.is_attributed(i) {
            return 0;
        }
        let base = self.attr_def(i).map_or(0, |d| d.mana);
        base + self.stat(i, crate::spell::S_SLOT_15)
    }

    /// Mana regeneration of unit `i` in hundredths per second: the base regen plus its S_SLOT_13 stat
    /// (the mod's intelligence bonus).
    pub fn unit_mana_regen_x100(&self, i: usize) -> i32 {
        if !self.is_attributed(i) {
            return 0;
        }
        self.attr_def(i).map_or(0, |d| d.mana_regen) + self.stat(i, crate::spell::S_SLOT_13)
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

    /// Enrols unit `i` as a unit at (x, y): unit flags, home, gold and mana reset, passives and attribute
    /// stats applied, health full. The mod's seat-pick and placement rules call this through `game.enroll`.
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

    /// Recomputes the stats of live unit `i` from its catalog type, its unit line and the stat bonuses the mod set
    /// (heroes.lua derives those bonuses from attributes): max health, mana, damage, armour and attack period. Health and mana move by the change in max,
    /// so a level-up adds the gain and a death-free unit keeps its wounds. A non-unit or dead unit is
    /// left alone; revive and spawn apply it once the unit is alive.
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
                // The base period; the agility bonus is the mod's S_ATTACK_SPEED stat (attack_period).
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

    /// The S_HEALTH_REGEN stat (hundredths per second, the mod's strength bonus) per tick, Q16.16.
    pub(crate) fn unit_regen_per_tick(&self, i: usize) -> i32 {
        let from_stat = self.stat(i, crate::spell::S_HEALTH_REGEN) as i64 * fixed::ONE as i64 / (100 * TICKS_PER_SECOND as i64);
        from_stat as i32
    }

    /// Recomputes every living unit's tracked max mana from its unit line and mana stat without touching its
    /// mana. A loaded state keeps the energy it saved but has no tracker.
    pub(crate) fn resync_unit_mana(&mut self) {
        for i in 0..self.n {
            if self.is_attributed(i) && self.alive(i) {
                self.attributed.mana_max[i] = self.unit_max_mana(i);
            }
        }
    }

    /// Adds the mana of unit `i` for this tick. The whole mana gained is the difference of two floors of
    /// the running total, so a fraction of a point is never lost and no per-unit remainder is stored.
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

    /// Loads the rules text: `unit <unit> key=value ...` lines set per-type unit data (the unit must
    /// exist in the catalog); the engine keys set the rules and every other `key=value` line is a named
    /// value for `rule`. False, nothing changed, on a bad line.
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
        for i in 0..self.n {
            self.apply_attr_stats(i);
        }
        true
    }

    /// Gold held by unit `i`; 0 for a unit with no gold state.
    pub fn unit_gold(&self, i: usize) -> i32 {
        if self.is_attributed(i) { self.attributed.gold[i] } else { 0 }
    }

    /// Sets the respawn of dead unit `i` (the `set_respawn` primitive): `Some(s)` revives it `s` seconds
    /// from now at its home, `None` leaves it dead. Refused for a live unit or a non-unit.
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

    /// Per tick: the respawn timer of each dead unit once it has run out (a revive at its home), and the mana
    /// regeneration of each living unit. A revive blocked by a full cell retries next tick.
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

    /// Puts dead unit `i` back at the Q16.16 point (x, y) with full health, keeping its xp and its kill
    /// and death tallies (a slot spawn resets them). False when the cell cannot take it.
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

    /// Re-derives the attribute stats of unit `i` (max health, mana, damage, armour, attack period) from its
    /// catalog type, unit line and stat bonuses, as a spawn or the mod's level-up does. An unknown index changes nothing.
    pub fn refresh_attr_stats(&mut self, i: usize) {
        if i < self.n {
            self.apply_attr_stats(i);
        }
    }

    /// Sets the base attack period of live unit `i` to `ticks`, before the attack-speed bonus. False for a dead
    /// or unknown unit or a period that is not positive, which changes nothing.
    pub fn set_base_period(&mut self, i: usize, ticks: i32) -> bool {
        if !self.alive(i) || ticks <= 0 {
            return false;
        }
        self.period[i] = ticks;
        self.touch(i);
        true
    }

    /// Sets ability slot `slot` of live unit `unit` to `rank`, clamped to the ability's max rank, with no level
    /// gate. Returns the rank held, or None for a dead unit, an empty slot or an ability with no rank tree.
    pub fn set_skill_rank(&mut self, unit: usize, slot: usize, rank: u8) -> Option<u8> {
        let tree = self.rank_tree(unit, slot)?;
        let rank = (rank as usize).min(tree.max_rank as usize) as u8;
        self.abil_rank[unit * ABILITY_SLOTS + slot] = rank;
        self.refresh_passives(unit);
        Some(rank)
    }

    /// The unit that applied the taunt on unit `i`, while the taunt is active and that unit is alive; None otherwise.
    pub fn taunter_of(&self, i: usize) -> Option<usize> {
        if i >= self.n || self.status_left(i, crate::status::TAUNT) <= 0 {
            return None;
        }
        usize::try_from(self.spell.taunter[i]).ok().filter(|&t| self.alive(t))
    }
}
