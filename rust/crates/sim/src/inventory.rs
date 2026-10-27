use crate::arrays::Sim;
use crate::fixed;
use crate::spell::{
    S_ATTACK_SPEED, S_CDR, S_DAMAGE, S_EVASION, S_LIFESTEAL, S_MAGIC_RESIST, S_SPEED, S_SPELL_AMP, S_SPELL_LIFESTEAL,
    S_STATUS_RESIST, S_HEALTH, S_SLOT_15, S_HEALTH_REGEN, S_SLOT_13, S_ARMOR_X100, S_CRIT_CHANCE, S_CRIT_MULT, S_CLEAVE,
    S_CLEAVE_RADIUS, S_CLEAVE_W0, S_CLEAVE_W1, S_BASH_CHANCE, S_BASH_TICKS, S_SPEED_PCT,
};
use crate::rules::{catalog_index, Refusal};

pub(crate) const MAIN_SLOTS: usize = 6;
pub(crate) const BACKPACK_SLOTS: usize = 3;
pub(crate) const NEUTRAL_SLOT: usize = MAIN_SLOTS + BACKPACK_SLOTS;
pub(crate) const INV_SLOTS: usize = NEUTRAL_SLOT + 1;
pub(crate) const STASH_SLOTS: usize = NEUTRAL_SLOT;

pub(crate) const ITEM_COMPONENTS: usize = 6;
pub(crate) const ITEM_STAT_COUNT: usize = 48;
pub(crate) const ITEM_FLAG_CONSUMABLE: u16 = 1;
pub(crate) const ITEM_FLAG_NEUTRAL: u16 = 2;
pub(crate) const ITEM_FLAG_NO_DROP: u16 = 32;
pub(crate) const ITEM_FLAG_MASK: u16 = 63;
pub(crate) const NEUTRAL_TIERS: u8 = 5;

pub(crate) const FILTER_MASSIVE: u32 = 1 << 8;
pub(crate) const STAT_SLOT_0: usize = 0;
pub(crate) const STAT_DAMAGE: usize = 1;
pub(crate) const STAT_HEALTH: usize = 2;
pub(crate) const STAT_SLOT_3: usize = 3;
pub(crate) const STAT_MOVE_SPEED: usize = 7;
pub(crate) const STAT_ATTACK_SPEED: usize = 8;
pub(crate) const STAT_HEALTH_REGEN: usize = 9;
pub(crate) const STAT_SLOT_10: usize = 10;
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
pub(crate) const STAT_BASH_DAMAGE: usize = 38;
pub(crate) const STAT_BASH_CHANCE_RANGED: usize = 39;
pub(crate) const STAT_BASH_COOLDOWN: usize = 40;
pub(crate) const STAT_BLOCK_CHANCE: usize = 41;
pub(crate) const STAT_BLOCK_AMOUNT: usize = 42;
pub(crate) const STAT_SPELL_BLOCK: usize = 43;
pub(crate) const STAT_TRUE_STRIKE: usize = 44;
pub(crate) const STAT_BLOCK_CHANCE_NONHERO: usize = 45;
pub(crate) const STAT_CREEP_DAMAGE_PCT: usize = 46;
pub(crate) const STAT_TRUE_SIGHT: usize = 47;

pub const ITEM_STATS: [&str; ITEM_STAT_COUNT] = [
    "armor", "damage", "health", "mana", "strength", "agility", "intelligence", "move_speed", "attack_speed",
    "health_regen", "mana_regen", "magic_resist", "evasion", "spell_amp", "lifesteal", "spell_lifesteal",
    "cast_range", "attack_range", "status_resist", "cooldown_reduction", "gold_per_min", "xp_per_min",
    "crit_chance", "crit_multiplier", "cleave", "cleave_distance", "cleave_start_width", "cleave_end_width",
    "bash_chance", "bash_duration", "speed_pct", "slow_resist", "hp_regen_amp", "mana_regen_amp",
    "mana_cost_reduction", "night_vision", "scepter", "shard", "bash_damage", "bash_chance_ranged", "bash_cooldown",
    "block_chance", "block_amount", "spell_block", "true_strike", "block_chance_nonhero",
    "creep_damage_pct", "true_sight",
];

pub(crate) type ItemTotals = [i32; ITEM_STAT_COUNT];

pub(crate) struct ItemSnapshot {
    inv: Bank,
    stash: Bank,
    cd: [i32; INV_SLOTS],
}

type Bank = [(u32, u32); INV_SLOTS];

struct Derived {
    hp_max: i32,
    hp: i32,
    armor: i32,
    damage: i32,
    period: i32,
    energy: i32,
}

const ITEM_SMODS: [(usize, usize); 24] = [
    (STAT_SLOT_0, S_ARMOR_X100),
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
    (STAT_SLOT_3, S_SLOT_15),
    (STAT_HEALTH_REGEN, S_HEALTH_REGEN),
    (STAT_SLOT_10, S_SLOT_13),
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
    pub fn inventory_slots(&self) -> usize {
        INV_SLOTS
    }

    pub fn inventory_slot(&self, unit: usize, slot: usize) -> (u32, u32) {
        if unit >= self.cap || slot >= INV_SLOTS {
            return (0, 0);
        }
        let at = unit * INV_SLOTS + slot;
        (self.inv_kind[at], self.inv_count[at])
    }

    pub fn inventory_count(&self, unit: usize, kind: u32) -> u32 {
        if kind == 0 {
            return 0;
        }
        (0..INV_SLOTS)
            .map(|s| self.inventory_slot(unit, s))
            .filter(|&(k, _)| k == kind)
            .fold(0u32, |sum, (_, c)| sum.saturating_add(c))
    }

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

    pub fn replace_item(&mut self, unit: usize, slot: usize, kind: u32, count: u32) -> Result<(), Refusal> {
        if unit >= self.cap || slot >= INV_SLOTS || kind == 0 || count == 0 {
            return Err(self.reject(Refusal::NoItem));
        }
        let at = unit * INV_SLOTS + slot;
        let Some(def) = self.item_def(kind) else {
            return Err(self.reject(Refusal::NoItem));
        };
        if self.inv_count[at] == 0 {
            return Err(self.reject(Refusal::NoItem));
        }
        if (def.flags & ITEM_FLAG_NEUTRAL != 0) != (slot == NEUTRAL_SLOT) {
            return Err(self.reject(Refusal::NoSlot));
        }
        let owner = self.inv_owner[at];
        self.change_items(unit, |sim| {
            sim.inv_kind[at] = kind;
            sim.inv_count[at] = count;
        });
        self.inv_owner[at] = owner;
        self.touch(unit);
        Ok(())
    }

    pub fn item_owner(&self, unit: usize, slot: usize) -> Option<usize> {
        if unit >= self.cap || slot >= INV_SLOTS {
            return None;
        }
        let at = unit * INV_SLOTS + slot;
        if self.inv_count[at] == 0 {
            return None;
        }
        match self.inv_owner[at] {
            0 => (unit < self.n).then(|| self.player[unit] as usize),
            p => Some(p as usize - 1),
        }
    }

    pub fn set_item_owner(&mut self, unit: usize, slot: usize, player: usize) -> Result<(), Refusal> {
        if unit >= self.cap || slot >= INV_SLOTS || self.inv_count[unit * INV_SLOTS + slot] == 0 {
            return Err(self.reject(Refusal::NoItem));
        }
        if player >= crate::MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidPlayer));
        }
        self.inv_owner[unit * INV_SLOTS + slot] = player as u8 + 1;
        self.bump_item_gen(unit);
        self.touch(unit);
        Ok(())
    }

    fn held_kinds(&self, unit: usize) -> [u32; INV_SLOTS] {
        let mut held = [0u32; INV_SLOTS];
        if unit < self.cap {
            held.copy_from_slice(&self.inv_kind[unit * INV_SLOTS..(unit + 1) * INV_SLOTS]);
        }
        held
    }

    fn drop_stale_owners(&mut self, unit: usize, before: &[u32; INV_SLOTS]) {
        if unit >= self.cap {
            return;
        }
        for (slot, was) in before.iter().enumerate() {
            let at = unit * INV_SLOTS + slot;
            if self.inv_kind[at] != *was || self.inv_count[at] == 0 {
                self.inv_owner[at] = 0;
            }
        }
    }

    pub fn item_lock_left(&self, unit: usize, slot: usize) -> u32 {
        if unit >= self.cap || slot >= INV_SLOTS {
            return 0;
        }
        let at = unit * INV_SLOTS + slot;
        if self.inv_count[at] == 0 || self.item_lock_kind[at] != self.inv_kind[at] {
            return 0;
        }
        self.item_lock_until[at].saturating_sub(self.ticks)
    }

    pub fn item_generation(&self, unit: usize) -> u32 {
        self.item_gen.get(unit).copied().unwrap_or(0)
    }

    pub(crate) fn bump_item_gen(&mut self, unit: usize) {
        if let Some(g) = self.item_gen.get_mut(unit) {
            *g = g.wrapping_add(1);
        }
    }

    pub(crate) fn inventory_clear(&mut self, unit: usize) {
        let base = unit * INV_SLOTS;
        for at in base..base + INV_SLOTS {
            self.inv_kind[at] = 0;
            self.inv_count[at] = 0;
            self.item_cd[at] = 0;
        }
        self.bump_item_gen(unit);
    }

    pub(crate) fn stash_clear(&mut self, unit: usize) {
        let base = unit * INV_SLOTS;
        for at in base..base + INV_SLOTS {
            self.stash_kind[at] = 0;
            self.stash_count[at] = 0;
        }
        self.bump_item_gen(unit);
    }

    pub fn stash_slot(&self, unit: usize, slot: usize) -> (u32, u32) {
        if unit >= self.cap || slot >= STASH_SLOTS {
            return (0, 0);
        }
        let at = unit * INV_SLOTS + slot;
        (self.stash_kind[at], self.stash_count[at])
    }

    pub(crate) fn item_def(&self, kind: u32) -> Option<crate::loader::Item> {
        catalog_index(kind, self.cat.items.len()).map(|k| self.cat.items[k])
    }

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
                    let best = k < 64 && (self.cat.tune.best_only >> k) & 1 != 0;
                    *total = if best { (*total).max(stat) } else { total.saturating_add(stat) };
                }
            }
        }
        sum
    }

    pub(crate) fn item_stat(&self, unit: usize, stat: usize) -> i32 {
        if unit >= self.cap {
            return 0;
        }
        let best = (self.cat.tune.best_only >> stat) & 1 != 0;
        let base = unit * INV_SLOTS;
        let mut total = 0i32;
        for slot in (0..INV_SLOTS).filter(|&s| slot_active(s)) {
            if self.inv_count[base + slot] == 0 {
                continue;
            }
            if let Some(def) = self.item_def(self.inv_kind[base + slot]) {
                let value = def.stats[stat];
                total = if best { total.max(value) } else { total.saturating_add(value) };
            }
        }
        total
    }

    pub(crate) fn change_items<R>(&mut self, unit: usize, edit: impl FnOnce(&mut Sim) -> R) -> R {
        let before = self.item_totals(unit);
        let held_before = self.held_kinds(unit);
        let attributed = unit < self.n && self.is_attributed(unit) && self.alive(unit) && self.type_of[unit] >= 0;
        let kept = attributed.then(|| self.derived_stats(unit));
        if attributed {
            self.apply_attr_stats(unit);
        }
        let old = attributed.then(|| self.derived_stats(unit));
        let out = edit(self);
        self.drop_stale_owners(unit, &held_before);
        self.bump_item_gen(unit);
        let after = self.item_totals(unit);
        self.apply_item_delta(unit, &before, &after, attributed);
        if let (Some(kept), Some(old)) = (kept, old) {
            self.apply_attr_stats(unit);
            let new = self.derived_stats(unit);
            let max = kept.hp_max + new.hp_max - old.hp_max;
            self.hp_max[unit] = max;
            self.hp[unit] = (kept.hp + new.hp - old.hp).clamp(1, max.max(1));
            self.armor[unit] = kept.armor + new.armor - old.armor;
            self.damage[unit] = kept.damage + new.damage - old.damage;
            self.period[unit] = (kept.period + new.period - old.period).max(1);
            let mana_max = self.attributed.mana_max[unit];
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

    fn apply_item_delta(&mut self, i: usize, before: &ItemTotals, after: &ItemTotals, attributed: bool) {
        if i >= self.n || before == after {
            return;
        }
        for (stat, code) in ITEM_SMODS {
            let delta = after[stat] - before[stat];
            if delta != 0 && (attributed || stat != STAT_HEALTH) {
                self.smod_add(i, code, delta);
            }
        }
        let d = |stat: usize| after[stat] - before[stat];
        if !attributed {
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

    pub fn item_bonus(&self, unit: usize, stat: &str) -> i32 {
        ITEM_STATS.iter().position(|&n| n == stat).map_or(0, |k| self.item_totals(unit)[k])
    }

    pub(crate) fn snapshot_items(&self, i: usize) -> ItemSnapshot {
        let base = i * INV_SLOTS;
        ItemSnapshot {
            inv: std::array::from_fn(|s| (self.inv_kind[base + s], self.inv_count[base + s])),
            stash: std::array::from_fn(|s| (self.stash_kind[base + s], self.stash_count[base + s])),
            cd: std::array::from_fn(|s| self.item_cd[base + s]),
        }
    }

    pub(crate) fn restore_items(&mut self, i: usize, snap: ItemSnapshot) {
        let base = i * INV_SLOTS;
        for s in 0..INV_SLOTS {
            (self.inv_kind[base + s], self.inv_count[base + s]) = snap.inv[s];
            (self.stash_kind[base + s], self.stash_count[base + s]) = snap.stash[s];
            self.item_cd[base + s] = snap.cd[s];
        }
        self.bump_item_gen(i);
        let attributed = self.is_attributed(i);
        let totals = self.item_totals(i);
        self.apply_item_delta(i, &[0; ITEM_STAT_COUNT], &totals, attributed);
    }

    pub fn consume_item_charge(&mut self, i: usize, slot: usize) {
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

pub(crate) fn item_kind(item: usize) -> u32 {
    item as u32 + 1
}

#[cfg(test)]
mod replace_item_tests {
    use super::*;
    use crate::{fixed, load};

    fn holder() -> (Sim, usize) {
        let image = osdt_data::generate(&crate::test_pack_dir()).expect("pack image");
        let mut s = load(&image, 1024).expect("pack sim");
        s.set_fog(false);
        let kind = s.catalog().unit_index("antimage").expect("antimage") as u32;
        let (w, h) = (s.grid().w as i32, s.grid().h as i32);
        let u = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .find_map(|(x, y)| s.spawn_for(0, kind, fixed::from_int(x), fixed::from_int(y)))
            .expect("an open cell for the unit");
        let (x, y) = s.position_fixed(u);
        s.attribute_unit(u, x, y);
        (s, u)
    }

    fn kind_of(s: &Sim, name: &str) -> u32 {
        item_kind(s.catalog().item_index(name).unwrap_or_else(|| panic!("catalog has no {name}")))
    }

    #[test]
    fn replace_item_swaps_in_place_keeping_slot_and_cooldown_without_stacking() {
        let (mut s, u) = holder();
        let (bottle, empty) = (kind_of(&s, "bottle"), kind_of(&s, "bottle_empty"));
        assert!(s.inventory_add(u, bottle, 3));
        assert!(s.inventory_add(u, empty, 1));
        let base = u * INV_SLOTS;
        let slot = (0..MAIN_SLOTS).find(|&k| s.inv_kind[base + k] == bottle).expect("bottle slot");
        s.item_cd[base + slot] = 77;
        s.replace_item(u, slot, empty, 1).expect("replace");
        assert_eq!(s.inventory_slot(u, slot), (empty, 1), "the slot holds the new item");
        assert_eq!(s.item_cd[base + slot], 77, "the cooldown stays with the slot");
        assert_eq!(s.inventory_count(u, empty), 2, "the replacement did not stack with the empty bottle held elsewhere");
        let other = (0..MAIN_SLOTS).find(|&k| k != slot && s.inv_kind[base + k] == empty).expect("other empty");
        assert_eq!(s.inventory_slot(u, other), (empty, 1), "the other bottle keeps its own slot and count");
    }

    #[test]
    fn replace_item_refuses_an_empty_slot_a_missing_kind_and_a_neutral_mismatch() {
        let (mut s, u) = holder();
        let bottle = kind_of(&s, "bottle");
        assert!(s.replace_item(u, 0, bottle, 1).is_err(), "an empty slot has nothing to replace");
        assert!(s.inventory_add(u, bottle, 1));
        assert!(s.replace_item(u, 0, 0, 1).is_err(), "kind 0 is not an item");
        assert!(s.replace_item(u, 0, bottle, 0).is_err(), "a zero count is not a stack");
        assert!(s.replace_item(u, 0, u32::MAX, 1).is_err(), "an unknown kind is refused");
        assert!(s.replace_item(u, INV_SLOTS, bottle, 1).is_err(), "a slot past the bank is refused");
        assert_eq!(s.inventory_slot(u, 0), (bottle, 1), "refusals leave the slot alone");
    }

    #[test]
    fn item_owner_defaults_to_the_holder_follows_a_swap_and_clears_with_the_item() {
        let (mut s, u) = holder();
        let (bottle, empty) = (kind_of(&s, "bottle"), kind_of(&s, "bottle_empty"));
        assert!(s.inventory_add(u, bottle, 3));
        assert_eq!(s.item_owner(u, 0), Some(0), "an unowned item belongs to its holder's player");
        s.set_item_owner(u, 0, 3).expect("owner");
        assert_eq!(s.item_owner(u, 0), Some(3));
        s.replace_item(u, 0, empty, 1).expect("replace");
        assert_eq!(s.item_owner(u, 0), Some(3), "a replacement keeps the item's owner");
        assert!(s.move_item(u, 0, 1));
        assert_eq!(s.item_owner(u, 1), Some(3), "a moved item takes its owner along");
        assert_eq!(s.item_owner(u, 0), None, "the emptied slot has no owner");
        assert!(s.inventory_remove(u, empty, 1));
        assert!(s.inventory_add(u, bottle, 1));
        assert_eq!(s.item_owner(u, 0), Some(0), "a new item in a freed slot starts with its holder as owner");
        assert!(s.set_item_owner(u, 5, 1).is_err(), "an empty slot has no owner to set");
        assert!(s.set_item_owner(u, 0, crate::MAX_PLAYERS).is_err(), "a player past the table is refused");
    }
}
