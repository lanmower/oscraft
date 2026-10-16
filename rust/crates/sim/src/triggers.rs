//! Behaviour triggers: a behaviour declared in data fires during the tick when its trigger
//! happens to a unit. Spawned fires once, on the unit's first tick. Attacked fires on the
//! tick after a unit takes a hit. Idle fires on the edge where a unit comes to rest, not on
//! every resting tick. Ability_ready fires when a slot's cooldown runs out. A fired behaviour
//! runs its action: use_ability casts the named ability when the unit holds it (a unit that
//! does not hold it is skipped, not refused); any other action refuses as action-unsupported.
//! Behaviours carry no unit link, so every unit is offered each one. Nothing allocates: the
//! catalog is read by index and each pending flag is a byte in its unit's array.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::rules::{Refusal, TARGET_UNIT};
use crate::*;

impl Sim {
    /// Runs the pending spawned, attacked and ability_ready triggers of unit `i`, and clears them.
    pub(crate) fn fire_triggers(&mut self, i: usize) {
        if self.trig_spawn[i] != 0 {
            self.trig_spawn[i] = 0;
            self.run_behaviours(i, TRIGGER_SPAWNED, -1);
        }
        if self.trig_hit[i] != 0 {
            self.trig_hit[i] = 0;
            self.run_behaviours(i, TRIGGER_ATTACKED, -1);
        }
        if self.abil_on {
            for slot in 0..ABILITY_SLOTS {
                let idx = i * ABILITY_SLOTS + slot;
                if self.trig_ready[idx] != 0 {
                    self.trig_ready[idx] = 0;
                    let ability = self.abil_idx[idx];
                    self.run_behaviours(i, TRIGGER_ABILITY_READY, ability);
                }
            }
        }
    }

    /// Fires the idle trigger once when unit `i` comes to rest on its destination, and re-arms
    /// it when the unit moves off again.
    pub(crate) fn note_idle(&mut self, i: usize) {
        let resting = self.tgt_x[i] == self.pos_x[i] && self.tgt_y[i] == self.pos_y[i];
        if !resting {
            self.trig_idle[i] = 0;
            return;
        }
        if self.trig_idle[i] == 0 {
            self.trig_idle[i] = 1;
            self.run_behaviours(i, TRIGGER_IDLE, -1);
        }
    }

    /// Fires every behaviour declared for the named trigger (idle, attacked, spawned or
    /// ability_ready) on unit `i`, as if that trigger had just happened. Returns false for an
    /// out-of-range unit or an unknown trigger name; true once the behaviours have run.
    pub fn fire_trigger(&mut self, i: usize, name: &str) -> bool {
        if i >= self.n {
            return false;
        }
        let found = (0..=TRIGGER_ABILITY_READY).find(|&code| Trigger::from_code(code).map_or(false, |t| t.name() == name));
        let code = match found {
            Some(code) => code,
            None => return false,
        };
        if code == TRIGGER_ABILITY_READY {
            if self.abil_on {
                for slot in 0..ABILITY_SLOTS {
                    let ability = self.abil_idx[i * ABILITY_SLOTS + slot];
                    if ability >= 0 {
                        self.run_behaviours(i, code, ability);
                    }
                }
            }
        } else {
            self.run_behaviours(i, code, -1);
        }
        true
    }

    /// Runs every behaviour declared for `trigger` on unit `i`. For ability_ready only the
    /// behaviours naming `ability` (an ability index, or -1 for any) run.
    fn run_behaviours(&mut self, i: usize, trigger: u8, ability: i32) {
        for b in 0..self.cat.behaviours.len() {
            let beh = self.cat.behaviours[b];
            if beh.trigger != trigger {
                continue;
            }
            if trigger == TRIGGER_ABILITY_READY && beh.ability as i32 != ability {
                continue;
            }
            match beh.action {
                ACTION_USE_ABILITY => {
                    if let Some(slot) = self.slot_holding(i, beh.ability) {
                        let unit_target = self.cat.abilities.get(beh.ability as usize).map_or(false, |a| a.target == TARGET_UNIT);
                        let target = if unit_target { self.nearest_enemy(i) } else { -1 };
                        if !unit_target || target >= 0 {
                            let (x, y) = (self.pos_x[i], self.pos_y[i]);
                            self.cast_ability(i, slot, target, x, y);
                        }
                    }
                }
                _ => {
                    self.reject(Refusal::ActionUnsupported);
                }
            }
        }
    }

    /// The nearest living enemy that unit `i` can see and strike within its weapon reach, or -1.
    /// A unit-target behaviour needs one; with none in reach it is skipped.
    pub(crate) fn nearest_enemy(&self, i: usize) -> i32 {
        let reach = (self.range_sq[i] as i64) << fixed::SHIFT;
        let mut best = -1i32;
        let mut best_d2 = 0i64;
        for j in 0..self.n {
            if j == i || self.alive[j] == 0 || !self.hostile(self.player[i], self.player[j]) {
                continue;
            }
            if (self.targets[i] & self.layer_of(j)) == 0 || !self.visible(self.player[i], j) {
                continue;
            }
            let d2 = dist2(self.pos_x[i], self.pos_y[i], self.pos_x[j], self.pos_y[j]);
            if d2 <= reach && (best < 0 || d2 < best_d2) {
                best = j as i32;
                best_d2 = d2;
            }
        }
        best
    }

    /// The ability slot of unit `i` that holds catalog ability `ability`, if any.
    fn slot_holding(&self, i: usize, ability: u16) -> Option<usize> {
        (0..ABILITY_SLOTS).find(|&slot| self.abil_idx[i * ABILITY_SLOTS + slot] == ability as i32)
    }
}