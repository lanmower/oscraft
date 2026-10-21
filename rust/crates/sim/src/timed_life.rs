//! Timed life: the generic `timed_spawn` primitive places a unit of a catalog type at a point for its
//! caster's player, and the unit expires after a lifetime in ticks, removed without bounty. Nothing here
//! says what the unit is for: its behaviour comes from its unit data (sight, detector radius, health), and
//! which unit, where and for how long is the mod's choice (the pack's spells.lua), and the item charge a
//! cast spends is the mod's too (`game.consume_item_charge`). A data effect that places timed units is a
//! caller of this primitive. Expiry stamps are sim state: hashed and saved with the rest.

use crate::arrays::Sim;

#[derive(Clone, Debug)]
pub(crate) struct ExpiryState {
    /// Tick at which each timed unit expires; 0 for a unit with no lifetime.
    pub(crate) expires: Vec<u32>,
    /// Inventory slot being cast during `use_item`, -1 otherwise, read by the refresh effect. Transient: not hashed or saved.
    pub(crate) casting_slot: i32,
}

impl ExpiryState {
    pub(crate) fn new(cap: usize) -> ExpiryState {
        ExpiryState { expires: vec![0; cap], casting_slot: -1 }
    }
}

impl Sim {
    /// Ticks left on timed unit `i`; 0 when it has no lifetime, is dead, or is unknown.
    pub fn timed_left(&self, i: usize) -> u32 {
        if i >= self.n || self.alive[i] == 0 {
            return 0;
        }
        self.expiry.expires[i].saturating_sub(self.ticks)
    }

    /// Places unit type `kind` for the caster's player at (x, y) with a lifetime of `life` ticks. A full unit
    /// table or a point the unit does not fit places nothing.
    pub fn timed_spawn_effect(&mut self, caster: usize, kind: i32, life: i32, x: i32, y: i32) {
        let p = self.player[caster];
        let Some(j) = self.spawn_for(p, kind as u32, x, y) else {
            return;
        };
        self.stats.summoned += 1;
        self.expiry.expires[j] = self.ticks.saturating_add(life.max(1) as u32);
    }

    /// Per tick: every timed unit past its expiry is removed without bounty.
    pub(crate) fn tick_timed_life(&mut self) {
        for i in 0..self.n {
            let at = self.expiry.expires[i];
            if at != 0 && self.alive[i] != 0 && self.ticks >= at {
                self.hp[i] = 0;
                self.alive[i] = 0;
                self.touch(i);
            }
        }
    }
}
