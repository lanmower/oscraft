//! Heroes: units that respawn after a death timer, buy back for gold, and earn passive gold.
//! Every timer and cost is in `HeroRules`, read from a `key=value` text file by
//! `Sim::set_hero_rules_text` (scenarios/moba/hero_rules.txt). Seconds convert to ticks at
//! `TICKS_PER_SECOND`. UNVERIFIED: the default values are placeholders, not sourced Dota 2 numbers.

use crate::arrays::Sim;
use crate::fixed;
use crate::TICKS_PER_SECOND;

/// Dota 2 stat coefficients the hero stats use. UNVERIFIED: recalled, not checked against a source.
/// Max health gains 20 per strength point; max mana is 75 plus 12 per intelligence point; strength
/// is the primary attribute, adding 1 damage per point (UNVERIFIED primary choice); each agility
/// point adds 1/6 armour; regeneration adds 0.03 health per second per strength point.
const HP_PER_STR: i32 = 20;
const MANA_BASE: i32 = 75;
const MANA_PER_INT: i32 = 12;
const DAMAGE_PER_STR: i32 = 1;
const ARMOUR_PER_AGI_DIV: i64 = 6;
const HP_REGEN_PER_STR_Q16: i64 = 1966;

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
        ]
        .iter()
        .all(|&v| v >= 0)
            && (0..=100).contains(&next.team_share_pct);
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
        }
    }
}

impl Sim {
    /// Spawns a hero of unit type `kind` for `player` at a Q16.16 point; that point is its home.
    pub fn spawn_hero(&mut self, player: u8, kind: u32, x: i32, y: i32) -> Option<usize> {
        let i = self.spawn_for(player, kind, x, y)?;
        let h = &mut self.heroes;
        h.is_hero[i] = 1;
        h.home_x[i] = x;
        h.home_y[i] = y;
        h.waiting[i] = 0;
        h.respawn_at[i] = 0;
        h.buyback_ready[i] = 0;
        h.gold[i] = 0;
        self.apply_hero_stats(i);
        self.hp[i] = self.hp_max[i];
        Some(i)
    }

    /// Strength, agility and intelligence of hero `i`: the level-0 base plus the growth per level.
    /// All zero for a non-hero.
    pub fn hero_attributes(&self, i: usize) -> [i32; 3] {
        if !self.is_hero(i) {
            return [0; 3];
        }
        let r = self.heroes.rules;
        let lvl = self.level(i) as i32;
        [r.str_base + r.str_gain * lvl, r.agi_base + r.agi_gain * lvl, r.int_base + r.int_gain * lvl]
    }

    /// Max mana of hero `i` from its intelligence. Zero for a non-hero. There is no mana pool yet,
    /// so this is a stat only.
    pub fn hero_max_mana(&self, i: usize) -> i32 {
        if !self.is_hero(i) {
            return 0;
        }
        MANA_BASE + MANA_PER_INT * self.hero_attributes(i)[2]
    }

    /// Recomputes the attribute-driven stats of live hero `i` from its catalog type and attributes:
    /// max health, damage and armour. Health moves by the change in max, so a level-up adds the gain
    /// and a death-free unit keeps its wounds. A non-hero or dead unit is left alone; revive and
    /// spawn apply it once the unit is alive.
    pub(crate) fn apply_hero_stats(&mut self, i: usize) {
        if !self.is_hero(i) || !self.alive(i) || self.type_of[i] < 0 {
            return;
        }
        let def = self.cat.units[self.type_of[i] as usize];
        let [str_, agi, _] = self.hero_attributes(i);
        let old_max = self.hp_max[i];
        let new_max = fixed::from_int(def.health + HP_PER_STR * str_);
        self.hp_max[i] = new_max;
        self.hp[i] = (self.hp[i] + (new_max - old_max)).clamp(0, new_max);
        self.armor[i] = fixed::from_int(def.armor) + (fixed::ONE as i64 * agi as i64 / ARMOUR_PER_AGI_DIV) as i32;
        if def.has_weapon() {
            self.damage[i] = fixed::from_int(def.weapon_damage + DAMAGE_PER_STR * str_);
        }
        self.touch(i);
    }

    /// Health regenerated per tick by the strength of hero `i`, Q16.16. Zero for a non-hero.
    pub(crate) fn hero_regen_per_tick(&self, i: usize) -> i32 {
        let [str_, _, _] = self.hero_attributes(i);
        (str_ as i64 * HP_REGEN_PER_STR_Q16 / TICKS_PER_SECOND as i64) as i32
    }

    pub fn is_hero(&self, i: usize) -> bool {
        self.heroes.is_hero.get(i).is_some_and(|&h| h != 0)
    }

    pub fn hero_rules(&self) -> HeroRules {
        self.heroes.rules
    }

    pub fn set_hero_rules_text(&mut self, text: &str) -> bool {
        self.heroes.rules.set_from_text(text)
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
        r.buyback_base + r.buyback_per_level * self.level(i) as i32
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
        }
    }

    /// Puts hero `i` back at its home with full health, keeping its xp and its kill and death
    /// tallies (a slot spawn resets them). False when the home cell cannot take it.
    fn revive(&mut self, i: usize) -> bool {
        let kind = self.type_of[i] as u32;
        let player = self.player[i];
        let (x, y) = (self.heroes.home_x[i], self.heroes.home_y[i]);
        let (xp, kills, deaths) = (self.xp_total[i], self.unit_kills[i], self.unit_deaths[i]);
        if !self.spawn_at(i, kind, player, x, y) {
            return false;
        }
        self.xp_total[i] = xp;
        self.unit_kills[i] = kills;
        self.unit_deaths[i] = deaths;
        self.apply_hero_stats(i);
        self.hp[i] = self.hp_max[i];
        self.heroes.waiting[i] = 0;
        true
    }
}
