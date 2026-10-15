//! Behaviour triggers: a behaviour declared in data fires during the tick when its trigger
//! happens to a unit. Spawned fires once, on the unit's first tick. Attacked fires on the
//! tick after a unit takes a hit. Idle fires on the edge where a unit comes to rest, not on
//! every resting tick. Ability_ready fires when a slot's cooldown runs out. A fired behaviour
//! runs its action: use_ability casts the named ability when the unit holds it (a unit that
//! does not hold it is skipped, not refused); any other action refuses as action-unsupported.
//! Behaviours carry no unit link, so every unit is offered each one. Nothing allocates: the
//! catalog is read by index and each pending flag is a byte in its unit's array.

use crate::arrays::Sim;
use crate::rules::Refusal;
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
                        let (x, y) = (self.pos_x[i], self.pos_y[i]);
                        self.cast_ability(i, slot, -1, x, y);
                    }
                }
                _ => {
                    self.reject(Refusal::ActionUnsupported);
                }
            }
        }
    }

    /// The ability slot of unit `i` that holds catalog ability `ability`, if any.
    fn slot_holding(&self, i: usize, ability: u16) -> Option<usize> {
        (0..ABILITY_SLOTS).find(|&slot| self.abil_idx[i * ABILITY_SLOTS + slot] == ability as i32)
    }
}
