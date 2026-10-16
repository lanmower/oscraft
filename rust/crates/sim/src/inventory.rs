//! Inventory: a fixed number of item slots per unit, stored struct-of-arrays like every other
//! per-unit component. A slot holds an item kind (a catalog id, 0 when empty) and a count.
//! Nothing here names a kind, so any item fits any slot. A slot's count is 0 exactly when its
//! kind is 0.

use crate::arrays::Sim;

pub(crate) const INV_SLOTS: usize = 6;

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

    /// Adds `count` of `kind` to a unit: tops up the slot already holding the kind, else takes
    /// the first empty slot. False, with the unit unchanged, for kind 0, a zero count, an
    /// out-of-range unit or a full inventory.
    pub fn inventory_add(&mut self, unit: usize, kind: u32, count: u32) -> bool {
        if kind == 0 || count == 0 || unit >= self.cap {
            return false;
        }
        let base = unit * INV_SLOTS;
        let slot = (0..INV_SLOTS)
            .find(|&s| self.inv_kind[base + s] == kind)
            .or_else(|| (0..INV_SLOTS).find(|&s| self.inv_count[base + s] == 0));
        match slot {
            Some(s) => {
                self.inv_kind[base + s] = kind;
                self.inv_count[base + s] = self.inv_count[base + s].saturating_add(count);
                true
            }
            None => false,
        }
    }

    /// Removes `count` of `kind` from a unit, taking from its slots in slot order. All or
    /// nothing: false, with the unit unchanged, when it holds less than `count`.
    pub fn inventory_remove(&mut self, unit: usize, kind: u32, count: u32) -> bool {
        if kind == 0 || count == 0 || unit >= self.cap || self.inventory_count(unit, kind) < count {
            return false;
        }
        let base = unit * INV_SLOTS;
        let mut left = count;
        for at in base..base + INV_SLOTS {
            if left == 0 {
                break;
            }
            if self.inv_kind[at] != kind {
                continue;
            }
            let take = left.min(self.inv_count[at]);
            self.inv_count[at] -= take;
            left -= take;
            if self.inv_count[at] == 0 {
                self.inv_kind[at] = 0;
            }
        }
        true
    }

    /// Empties a unit's slots. Called when the unit spawns.
    pub(crate) fn inventory_clear(&mut self, unit: usize) {
        let base = unit * INV_SLOTS;
        for at in base..base + INV_SLOTS {
            self.inv_kind[at] = 0;
            self.inv_count[at] = 0;
        }
    }
}
