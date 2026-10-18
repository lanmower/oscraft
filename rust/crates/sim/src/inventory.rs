//! Inventory: a fixed number of item slots per unit, stored struct-of-arrays like every other
//! per-unit component. A slot holds an item kind (a catalog id, 0 when empty) and a count.
//! Nothing here names a kind, so any item fits any slot. A slot's count is 0 exactly when its
//! kind is 0. The stash is a second bank of the same shape: see `stash_slot`. Shop purchases
//! use `item_kind` for the id and put one item per slot (a stackable item adds charges to a stack).
//!
//! A unit has ten slots: six main (`0..MAIN_SLOTS`), three backpack and one neutral. Items in the
//! main slots and the neutral slot are active: they add their stat bonuses and cast their abilities.
//! A backpack item does neither. Only a neutral item (`ITEM_FLAG_NEUTRAL`) fits the neutral slot.
//! Stat bonuses are never stored: `item_totals` sums the active slots, and every change to the
//! slots applies the difference between the totals before and after (`change_items`).

use crate::arrays::Sim;
use crate::fixed;
use crate::spell::{
    S_ATTACK_SPEED, S_CDR, S_DAMAGE, S_EVASION, S_LIFESTEAL, S_MAGIC_RESIST, S_SPEED, S_SPELL_AMP, S_SPELL_LIFESTEAL,
    S_STATUS_RESIST, S_HEALTH, S_MAX_MANA, S_HEALTH_REGEN, S_MANA_REGEN, S_ARMOR_X100, S_CRIT_CHANCE, S_CRIT_MULT, S_CLEAVE,
    S_CLEAVE_RADIUS, S_CLEAVE_W0, S_CLEAVE_W1, S_BASH_CHANCE, S_BASH_TICKS, S_SPEED_PCT,
};
use crate::rules::catalog_index;

pub(crate) const MAIN_SLOTS: usize = 6;
pub(crate) const BACKPACK_SLOTS: usize = 3;
pub(crate) const NEUTRAL_SLOT: usize = MAIN_SLOTS + BACKPACK_SLOTS;
pub(crate) const INV_SLOTS: usize = NEUTRAL_SLOT + 1;
/// Slots of the stash bank in use: the stash shares the inventory stride but has no neutral slot.
pub(crate) const STASH_SLOTS: usize = NEUTRAL_SLOT;

pub(crate) const ITEM_COMPONENTS: usize = 6;
pub(crate) const ITEM_STAT_COUNT: usize = 36;
pub(crate) const ITEM_FLAG_CONSUMABLE: u16 = 1;
pub(crate) const ITEM_FLAG_NEUTRAL: u16 = 2;
pub(crate) const ITEM_FLAG_NO_BUY: u16 = 4;
pub(crate) const ITEM_FLAG_NO_SELL: u16 = 8;
pub(crate) const ITEM_FLAG_ENHANCEMENT: u16 = 16;
pub(crate) const ITEM_FLAG_MASK: u16 = 31;
pub(crate) const ITEM_SHOP_MASK: u8 = 7;
pub(crate) const NEUTRAL_TIERS: u8 = 5;
pub(crate) const SHOP_HOME: u8 = 1;
pub(crate) const SHOP_SIDE: u8 = 2;
pub(crate) const SHOP_SECRET: u8 = 4;

/// Indices into `Item::stats`, in the order of `ITEM_STATS` in the data crate.
pub(crate) const STAT_ARMOR: usize = 0;
pub(crate) const STAT_DAMAGE: usize = 1;
pub(crate) const STAT_HEALTH: usize = 2;
pub(crate) const STAT_MANA: usize = 3;
pub(crate) const STAT_STRENGTH: usize = 4;
pub(crate) const STAT_AGILITY: usize = 5;
pub(crate) const STAT_INTELLIGENCE: usize = 6;
pub(crate) const STAT_MOVE_SPEED: usize = 7;
pub(crate) const STAT_ATTACK_SPEED: usize = 8;
pub(crate) const STAT_HEALTH_REGEN: usize = 9;
pub(crate) const STAT_MANA_REGEN: usize = 10;
pub(crate) const STAT_MAGIC_RESIST: usize = 11;
pub(crate) const STAT_EVASION: usize = 12;
pub(crate) const STAT_SPELL_AMP: usize = 13;
pub(crate) const STAT_LIFESTEAL: usize = 14;
pub(crate) const STAT_SPELL_LIFESTEAL: usize = 15;
pub(crate) const STAT_ATTACK_RANGE: usize = 17;
pub(crate) const STAT_STATUS_RESIST: usize = 18;
pub(crate) const STAT_COOLDOWN_REDUCTION: usize = 19;
pub(crate) const STAT_CRIT_CHANCE: usize = 22;
pub(crate) const STAT_CRIT_MULTIPLIER: usize = 23;
pub(crate) const STAT_CLEAVE: usize = 24;
pub(crate) const STAT_CLEAVE_DISTANCE: usize = 25;
pub(crate) const STAT_CLEAVE_START_WIDTH: usize = 26;
pub(crate) const STAT_CLEAVE_END_WIDTH: usize = 27;
pub(crate) const STAT_BASH_CHANCE: usize = 28;
pub(crate) const STAT_BASH_DURATION: usize = 29;
pub(crate) const STAT_SPEED_PCT: usize = 30;
pub(crate) const STAT_NIGHT_VISION: usize = 35;

/// Stats that do not add up across items: a unit takes the largest value any active item gives.
const BEST_ONLY: [usize; 9] = [
    STAT_CRIT_CHANCE,
    STAT_CRIT_MULTIPLIER,
    STAT_CLEAVE,
    STAT_CLEAVE_DISTANCE,
    STAT_CLEAVE_START_WIDTH,
    STAT_CLEAVE_END_WIDTH,
    STAT_BASH_CHANCE,
    STAT_BASH_DURATION,
    STAT_NIGHT_VISION,
];

/// Names of the item stats, in `Item::stats` order (the data crate's `ITEM_STATS`).
pub const ITEM_STATS: [&str; ITEM_STAT_COUNT] = [
    "armor", "damage", "health", "mana", "strength", "agility", "intelligence", "move_speed", "attack_speed",
    "health_regen", "mana_regen", "magic_resist", "evasion", "spell_amp", "lifesteal", "spell_lifesteal",
    "cast_range", "attack_range", "status_resist", "cooldown_reduction", "gold_per_min", "xp_per_min",
    "crit_chance", "crit_multiplier", "cleave", "cleave_distance", "cleave_start_width", "cleave_end_width",
    "bash_chance", "bash_duration", "speed_pct", "slow_resist", "hp_regen_amp", "mana_regen_amp",
    "mana_cost_reduction", "night_vision",
];

/// Sums the stat bonuses of a set of item kinds.
pub(crate) type ItemTotals = [i32; ITEM_STAT_COUNT];

/// A unit's arrays that item changes touch, for the before/after difference in `change_items`.
struct Derived {
    hp_max: i32,
    hp: i32,
    armor: i32,
    damage: i32,
    period: i32,
    energy: i32,
}

/// Item stats that are stat modifiers of the spell layer, with the modifier each feeds.
const ITEM_SMODS: [(usize, usize); 24] = [
    (STAT_ARMOR, S_ARMOR_X100),
    (STAT_DAMAGE, S_DAMAGE),
    (STAT_MOVE_SPEED, S_SPEED),
    (STAT_ATTACK_SPEED, S_ATTACK_SPEED),
    (STAT_MAGIC_RESIST, S_MAGIC_RESIST),
    (STAT_EVASION, S_EVASION),
    (STAT_SPELL_AMP, S_SPELL_AMP),
    (STAT_LIFESTEAL, S_LIFESTEAL),
    (STAT_SPELL_LIFESTEAL, S_SPELL_LIFESTEAL),
    (STAT_STATUS_RESIST, S_STATUS_RESIST),
    (STAT_COOLDOWN_REDUCTION, S_CDR),
    (STAT_HEALTH, S_HEALTH),
    (STAT_MANA, S_MAX_MANA),
    (STAT_HEALTH_REGEN, S_HEALTH_REGEN),
    (STAT_MANA_REGEN, S_MANA_REGEN),
    (STAT_CRIT_CHANCE, S_CRIT_CHANCE),
    (STAT_CRIT_MULTIPLIER, S_CRIT_MULT),
    (STAT_CLEAVE, S_CLEAVE),
    (STAT_CLEAVE_DISTANCE, S_CLEAVE_RADIUS),
    (STAT_CLEAVE_START_WIDTH, S_CLEAVE_W0),
    (STAT_CLEAVE_END_WIDTH, S_CLEAVE_W1),
    (STAT_BASH_CHANCE, S_BASH_CHANCE),
    (STAT_BASH_DURATION, S_BASH_TICKS),
    (STAT_SPEED_PCT, S_SPEED_PCT),
];

pub(crate) fn slot_active(slot: usize) -> bool {
    slot < MAIN_SLOTS || slot == NEUTRAL_SLOT
}

impl Sim {
    /// Slots per unit.
    pub fn inventory_slots(&self) -> usize {
        INV_SLOTS
    }

    /// The kind and count held in one slot. Kind 0 means empty; an out-of-range read is empty.
    pub fn inventory_slot(&self, unit: usize, slot: usize) -> (u32, u32) {
        if unit >= self.cap || slot >= INV_SLOTS {
            return (0, 0);
        }
        let at = unit * INV_SLOTS + slot;
        (self.inv_kind[at], self.inv_count[at])
    }

    /// Total count of one kind across a unit's slots. Zero for kind 0.
    pub fn inventory_count(&self, unit: usize, kind: u32) -> u32 {
        if kind == 0 {
            return 0;
        }
        (0..INV_SLOTS)
            .map(|s| self.inventory_slot(unit, s))
            .filter(|&(k, _)| k == kind)
            .fold(0u32, |sum, (_, c)| sum.saturating_add(c))
    }

    /// Adds `count` of `kind` to a unit: tops up the main slot already holding the kind, else takes
    /// the first empty main slot. False, with the unit unchanged, for kind 0, a zero count, an
    /// out-of-range unit or full main slots. A neutral item goes to the empty neutral slot instead.
    pub fn inventory_add(&mut self, unit: usize, kind: u32, count: u32) -> bool {
        if kind == 0 || count == 0 || unit >= self.cap {
            return false;
        }
        let base = unit * INV_SLOTS;
        let neutral = self.item_def(kind).is_some_and(|d| d.flags & ITEM_FLAG_NEUTRAL != 0);
        let slot = if neutral {
            (self.inv_count[base + NEUTRAL_SLOT] == 0).then_some(NEUTRAL_SLOT)
        } else {
            (0..MAIN_SLOTS)
                .find(|&s| self.inv_kind[base + s] == kind)
                .or_else(|| (0..MAIN_SLOTS).find(|&s| self.inv_count[base + s] == 0))
        };
        let Some(s) = slot else {
            return false;
        };
        self.change_items(unit, |sim| {
            sim.inv_kind[base + s] = kind;
            sim.inv_count[base + s] = sim.inv_count[base + s].saturating_add(count);
        });
        true
    }

    /// Removes `count` of `kind` from a unit, taking from its slots in slot order. All or
    /// nothing: false, with the unit unchanged, when it holds less than `count`.
    pub fn inventory_remove(&mut self, unit: usize, kind: u32, count: u32) -> bool {
        if kind == 0 || count == 0 || unit >= self.cap || self.inventory_count(unit, kind) < count {
            return false;
        }
        let base = unit * INV_SLOTS;
        self.change_items(unit, |sim| {
            let mut left = count;
            for at in base..base + INV_SLOTS {
                if left == 0 {
                    break;
                }
                if sim.inv_kind[at] != kind {
                    continue;
                }
                let take = left.min(sim.inv_count[at]);
                sim.inv_count[at] -= take;
                left -= take;
                if sim.inv_count[at] == 0 {
                    sim.inv_kind[at] = 0;
                }
            }
        });
        true
    }

    /// Empties a unit's slots. Called when the unit spawns.
    pub(crate) fn inventory_clear(&mut self, unit: usize) {
        let base = unit * INV_SLOTS;
        for at in base..base + INV_SLOTS {
            self.inv_kind[at] = 0;
            self.inv_count[at] = 0;
            self.item_cd[at] = 0;
        }
    }

    /// Empties a unit's stash bank. Called when the unit spawns.
    pub(crate) fn stash_clear(&mut self, unit: usize) {
        let base = unit * INV_SLOTS;
        for at in base..base + INV_SLOTS {
            self.stash_kind[at] = 0;
            self.stash_count[at] = 0;
        }
    }

    /// The kind and count in one stash slot. Kind 0 means empty; an out-of-range read is empty.
    pub fn stash_slot(&self, unit: usize, slot: usize) -> (u32, u32) {
        if unit >= self.cap || slot >= STASH_SLOTS {
            return (0, 0);
        }
        let at = unit * INV_SLOTS + slot;
        (self.stash_kind[at], self.stash_count[at])
    }

    /// The catalog definition of the item stored under `kind`, or None for an empty or unknown kind.
    pub(crate) fn item_def(&self, kind: u32) -> Option<crate::loader::Item> {
        catalog_index(kind, self.cat.items.len()).map(|k| self.cat.items[k])
    }

    /// Sum of the stat bonuses of the items in unit `unit`'s active slots.
    pub(crate) fn item_totals(&self, unit: usize) -> ItemTotals {
        let mut sum = [0i32; ITEM_STAT_COUNT];
        if unit >= self.cap {
            return sum;
        }
        let base = unit * INV_SLOTS;
        for slot in (0..INV_SLOTS).filter(|&s| slot_active(s)) {
            if self.inv_count[base + slot] == 0 {
                continue;
            }
            if let Some(def) = self.item_def(self.inv_kind[base + slot]) {
                for (k, (total, stat)) in sum.iter_mut().zip(def.stats).enumerate() {
                    *total = if BEST_ONLY.contains(&k) { (*total).max(stat) } else { total.saturating_add(stat) };
                }
            }
        }
        sum
    }

    /// Runs `edit` on unit `unit`'s slots, then applies the change in item stat totals to the unit.
    /// A hero's attribute-driven stats come from `apply_hero_stats`, which reads the item totals: it
    /// runs before and after the edit and the unit takes the difference, so effects that raised its
    /// stats by other means stay. Every other stat moves by the difference of the totals.
    pub(crate) fn change_items<R>(&mut self, unit: usize, edit: impl FnOnce(&mut Sim) -> R) -> R {
        let before = self.item_totals(unit);
        let hero = unit < self.n && self.is_hero(unit) && self.alive(unit) && self.type_of[unit] >= 0;
        let kept = hero.then(|| self.derived_stats(unit));
        if hero {
            self.apply_hero_stats(unit);
        }
        let old = hero.then(|| self.derived_stats(unit));
        let out = edit(self);
        let after = self.item_totals(unit);
        self.apply_item_delta(unit, &before, &after, hero);
        if let (Some(kept), Some(old)) = (kept, old) {
            self.apply_hero_stats(unit);
            let new = self.derived_stats(unit);
            let max = kept.hp_max + new.hp_max - old.hp_max;
            self.hp_max[unit] = max;
            self.hp[unit] = (kept.hp + new.hp - old.hp).clamp(1, max.max(1));
            self.armor[unit] = kept.armor + new.armor - old.armor;
            self.damage[unit] = kept.damage + new.damage - old.damage;
            self.period[unit] = (kept.period + new.period - old.period).max(1);
            let mana_max = self.heroes.mana_max[unit];
            self.energy[unit] = (kept.energy + new.energy - old.energy).clamp(0, mana_max);
        }
        out
    }

    fn derived_stats(&self, i: usize) -> Derived {
        Derived {
            hp_max: self.hp_max[i],
            hp: self.hp[i],
            armor: self.armor[i],
            damage: self.damage[i],
            period: self.period[i],
            energy: self.energy[i],
        }
    }

    /// Applies the difference between two item totals. Stats the spell layer reads (armor, damage,
    /// speed, attack speed, evasion, resistances, lifesteal, spell amplification, cooldown reduction)
    /// go into the unit's stat modifiers. Health, and attack range, go to their unit arrays; a hero's
    /// health, mana and attributes come from `apply_hero_stats`, which reads the totals.
    fn apply_item_delta(&mut self, i: usize, before: &ItemTotals, after: &ItemTotals, hero: bool) {
        if i >= self.n || before == after {
            return;
        }
        for (stat, code) in ITEM_SMODS {
            let delta = after[stat] - before[stat];
            if delta != 0 && (hero || stat != STAT_HEALTH) {
                self.smod_add(i, code, delta);
            }
        }
        let d = |stat: usize| after[stat] - before[stat];
        if !hero {
            let health = fixed::from_int(d(STAT_HEALTH));
            self.hp_max[i] += health;
            self.hp[i] = if health >= 0 { self.hp[i] + health } else { self.hp[i].min(self.hp_max[i]) };
        }
        if d(STAT_ATTACK_RANGE) != 0 {
            self.range_q[i] += fixed::from_milli(d(STAT_ATTACK_RANGE));
            self.range_sq[i] = fixed::mul(self.range_q[i], self.range_q[i]);
        }
        self.touch(i);
    }

    /// The sum of one item stat (a name of `ITEM_STATS`) over the active slots of unit `unit`; 0 for an unknown name.
    pub fn item_bonus(&self, unit: usize, stat: &str) -> i32 {
        ITEM_STATS.iter().position(|&n| n == stat).map_or(0, |k| self.item_totals(unit)[k])
    }

    /// Whole attribute points the items of unit `i` add: strength, agility, intelligence.
    pub(crate) fn item_attributes(&self, i: usize) -> [i32; 3] {
        let t = self.item_totals(i);
        [t[STAT_STRENGTH], t[STAT_AGILITY], t[STAT_INTELLIGENCE]]
    }

    /// Spends one charge of the item in inventory slot `slot` of unit `i`; an emptied slot loses
    /// the item and its stat bonuses.
    pub(crate) fn consume_item_charge(&mut self, i: usize, slot: usize) {
        let key = i * INV_SLOTS + slot;
        self.change_items(i, |sim| {
            if sim.inv_count[key] > 1 {
                sim.inv_count[key] -= 1;
            } else {
                sim.inv_kind[key] = 0;
                sim.inv_count[key] = 0;
            }
        });
        self.touch(i);
    }
}

/// The catalog id stored in a slot for catalog item `item`. Id 0 is reserved for an empty slot,
/// so the id is the item index plus one.
pub(crate) fn item_kind(item: usize) -> u32 {
    item as u32 + 1
}
