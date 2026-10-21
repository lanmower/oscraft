//! Order memory, and the `carry` effect's request. A mod that sends a unit away saves the unit's
//! order point first and restores it when the unit comes back. The sim keeps only that point (a
//! generic per-unit slot) and the request. What carrying means, how a unit walks and when it hands
//! over are the mod's rule (the pack's store.lua), driven by `ItemRequest::Carry` and move-to-point orders.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::store::ItemRequest;

/// Saved order points, one slot per unit. `set` is 1 when a point is saved.
#[derive(Clone, Debug)]
pub(crate) struct OrderMemory {
    pub(crate) set: Vec<u8>,
    pub(crate) x: Vec<i32>,
    pub(crate) y: Vec<i32>,
}

impl OrderMemory {
    pub(crate) fn new(cap: usize) -> OrderMemory {
        OrderMemory { set: vec![0; cap], x: vec![0; cap], y: vec![0; cap] }
    }

    pub(crate) fn clear(&mut self, i: usize) {
        self.set[i] = 0;
    }
}

impl Sim {
    /// Saves the order point (x, y) for unit `i`, replacing any earlier one. False for an unknown unit.
    pub fn save_order(&mut self, i: usize, x: i32, y: i32) -> bool {
        if i >= self.n {
            return false;
        }
        let m = &mut self.order_memory;
        m.set[i] = 1;
        m.x[i] = x;
        m.y[i] = y;
        true
    }

    /// The saved order point of unit `i`, if one is saved.
    pub fn saved_order(&self, i: usize) -> Option<(i32, i32)> {
        (self.order_memory.set.get(i).copied() == Some(1)).then(|| (self.order_memory.x[i], self.order_memory.y[i]))
    }

    /// Orders living unit `i` to its saved point and keeps the point. False when none is saved or the
    /// unit is dead.
    pub fn restore_order(&mut self, i: usize) -> bool {
        match self.saved_order(i) {
            Some((x, y)) if self.alive(i) => {
                self.order(i, x, y);
                true
            }
            _ => false,
        }
    }

    /// Whether unit `i` stands within `reach_milli` of the point (x, y), in fixed units.
    pub fn within_reach(&self, i: usize, x: i32, y: i32, reach_milli: i32) -> bool {
        if i >= self.n {
            return false;
        }
        let reach = fixed::from_milli(reach_milli) as i64;
        dist2(self.pos_x[i], self.pos_y[i], x, y) <= reach * reach
    }

    /// The `carry` effect: `target` must be a living unit friendly to the caster, other than the
    /// caster; anything else changes nothing. Queues the request for the mod (`ItemRequest::Carry`).
    pub(crate) fn carry_effect(&mut self, caster: usize, target: i32) {
        if target < 0 || target as usize >= self.n || target as usize == caster {
            return;
        }
        let t = target as usize;
        if self.alive[t] == 0 || self.hostile(self.player[caster], self.player[t]) {
            return;
        }
        self.item_requests.requests.push(ItemRequest::Carry { caster, target: t });
    }
}
