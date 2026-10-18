//! Couriers: the `deliver` effect sends its caster to a friendly unit, moves every stash item of that
//! unit into its free inventory slots on arrival, then walks the caster back to the point it left
//! from. Any unit that holds an ability with the effect is a courier; nothing is named here.
//! Phase, target, reach and home are sim state: hashed and saved with the rest.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::inventory::INV_SLOTS;
use crate::ORDER_MOVE;

const IDLE: u8 = 0;
const OUTBOUND: u8 = 1;
const RETURNING: u8 = 2;
/// Ticks between route refreshes toward a moving target.
const REROUTE_PERIOD: u32 = 8;
/// Distance from home at which a returning courier is home, milli-units.
const HOME_REACH_MILLI: i32 = 1000;

#[derive(Clone, Debug)]
pub(crate) struct CourierState {
    pub(crate) phase: Vec<u8>,
    pub(crate) target: Vec<i32>,
    pub(crate) reach: Vec<i32>,
    pub(crate) home_x: Vec<i32>,
    pub(crate) home_y: Vec<i32>,
}

impl CourierState {
    pub(crate) fn new(cap: usize) -> CourierState {
        CourierState {
            phase: vec![IDLE; cap],
            target: vec![-1; cap],
            reach: vec![0; cap],
            home_x: vec![0; cap],
            home_y: vec![0; cap],
        }
    }

    pub(crate) fn clear(&mut self, i: usize) {
        self.phase[i] = IDLE;
        self.target[i] = -1;
    }
}

impl Sim {
    /// Phase of courier `i`: 0 idle, 1 delivering, 2 returning home; 0 for an unknown unit.
    pub fn courier_phase(&self, i: usize) -> u8 {
        self.couriers.phase.get(i).copied().unwrap_or(IDLE)
    }

    /// The `deliver` effect: p0 is the reach in milli-units. The receiver `target` must be a living
    /// unit friendly to the caster; anything else changes nothing. A courier already out keeps the
    /// home it left from.
    pub(crate) fn deliver_effect(&mut self, caster: usize, target: i32, reach_milli: i32) {
        if target < 0 || target as usize >= self.n || target as usize == caster {
            return;
        }
        let t = target as usize;
        if self.alive[t] == 0 || self.hostile(self.player[caster], self.player[t]) {
            return;
        }
        let c = &mut self.couriers;
        if c.phase[caster] == IDLE {
            c.home_x[caster] = self.pos_x[caster];
            c.home_y[caster] = self.pos_y[caster];
        }
        c.phase[caster] = OUTBOUND;
        c.target[caster] = target;
        c.reach[caster] = reach_milli;
        let _ = self.issue_order_kind(caster, ORDER_MOVE, self.pos_x[t], self.pos_y[t]);
    }

    /// Moves every stash item of `hero` into its free inventory slots, applying item bonuses.
    /// Returns how many items moved.
    pub(crate) fn deliver_stash(&mut self, hero: usize) -> usize {
        let mut moved = 0;
        for s in 0..crate::inventory::STASH_SLOTS {
            if self.stash_count[hero * INV_SLOTS + s] > 0 && self.unstash_slot(hero, s) {
                moved += 1;
            }
        }
        moved
    }

    /// Per tick: each courier walks toward its target, delivers within reach, then walks home.
    pub(crate) fn tick_couriers(&mut self) {
        for i in 0..self.n {
            let phase = self.couriers.phase[i];
            if phase == IDLE {
                continue;
            }
            if self.alive[i] == 0 {
                self.couriers.clear(i);
                continue;
            }
            let reroute = self.ticks % REROUTE_PERIOD == 0;
            if phase == OUTBOUND {
                let t = self.couriers.target[i];
                if t < 0 || self.alive[t as usize] == 0 {
                    self.couriers.phase[i] = RETURNING;
                    continue;
                }
                let t = t as usize;
                let reach = fixed::from_milli(self.couriers.reach[i]) as i64;
                if dist2(self.pos_x[i], self.pos_y[i], self.pos_x[t], self.pos_y[t]) <= reach * reach {
                    self.deliver_stash(t);
                    self.couriers.phase[i] = RETURNING;
                    self.couriers.target[i] = -1;
                    let (hx, hy) = (self.couriers.home_x[i], self.couriers.home_y[i]);
                    let _ = self.issue_order_kind(i, ORDER_MOVE, hx, hy);
                } else if reroute {
                    let _ = self.issue_order_kind(i, ORDER_MOVE, self.pos_x[t], self.pos_y[t]);
                }
            } else {
                let (hx, hy) = (self.couriers.home_x[i], self.couriers.home_y[i]);
                let reach = fixed::from_milli(HOME_REACH_MILLI) as i64;
                if dist2(self.pos_x[i], self.pos_y[i], hx, hy) <= reach * reach {
                    self.couriers.phase[i] = IDLE;
                } else if reroute {
                    let _ = self.issue_order_kind(i, ORDER_MOVE, hx, hy);
                }
            }
        }
    }
}
