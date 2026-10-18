//! Wards: the `ward` effect places a unit of a catalog type at the cast point for its caster's
//! player, and the unit expires after a lifetime in ticks. What a ward does comes from its unit
//! data: sight gives vision, a detector radius reveals invisible units (see `Sim::detected`), health
//! lets an enemy that sees it kill it. A ward item is consumed by the cast that places it.
//! Expiry stamps are sim state: hashed and saved with the rest.

use crate::arrays::Sim;
use crate::inventory::INV_SLOTS;

#[derive(Clone, Debug)]
pub(crate) struct WardState {
    /// Tick at which each ward expires; 0 for a unit that is not a ward.
    pub(crate) expires: Vec<u32>,
    /// Inventory slot being cast during `use_item`, -1 otherwise. Transient: not hashed or saved.
    pub(crate) casting_slot: i32,
}

impl WardState {
    pub(crate) fn new(cap: usize) -> WardState {
        WardState { expires: vec![0; cap], casting_slot: -1 }
    }
}

impl Sim {
    /// Ticks left on ward `i`; 0 when it is no ward, dead, or unknown.
    pub fn ward_left(&self, i: usize) -> u32 {
        if i >= self.n || self.alive[i] == 0 {
            return 0;
        }
        self.wards.expires[i].saturating_sub(self.ticks)
    }

    /// The `ward` effect: p0 unit type, p1 lifetime ticks. Places the ward for the caster's player at
    /// (x, y); a full unit table or a point the unit does not fit places nothing and keeps the item.
    pub(crate) fn ward_effect(&mut self, caster: usize, kind: i32, life: i32, x: i32, y: i32) {
        let p = self.player[caster];
        let Some(j) = self.spawn_for(p, kind as u32, x, y) else {
            return;
        };
        self.stats.summoned += 1;
        self.wards.expires[j] = self.ticks.wrapping_add(life.max(1) as u32);
        let slot = self.wards.casting_slot;
        if slot >= 0 {
            self.consume_charge(caster, slot as usize);
        }
    }

    /// Spends one charge of the item in inventory slot `slot` of unit `i`; an empty slot loses its
    /// item and the stat bonuses stay at zero because a ward item carries none.
    fn consume_charge(&mut self, i: usize, slot: usize) {
        let key = i * INV_SLOTS + slot;
        if self.inv_count[key] > 1 {
            self.inv_count[key] -= 1;
        } else {
            self.inv_kind[key] = 0;
            self.inv_count[key] = 0;
        }
        self.touch(i);
    }

    /// Per tick: every ward past its expiry is removed without bounty.
    pub(crate) fn tick_wards(&mut self) {
        for i in 0..self.n {
            let at = self.wards.expires[i];
            if at != 0 && self.alive[i] != 0 && self.ticks >= at {
                self.hp[i] = 0;
                self.alive[i] = 0;
                self.touch(i);
            }
        }
    }
}
