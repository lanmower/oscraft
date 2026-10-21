//! Behaviour triggers: a behaviour declared in data fires during the tick when its trigger happens to a
//! unit. Spawned fires once, on the unit's first tick. Attacked fires on the tick after a unit takes a
//! hit. Idle fires on the edge where a unit comes to rest, not on every resting tick. Ability_ready fires
//! when a slot's cooldown runs out. Destroyed fires once when a unit's life runs out. A fired behaviour is
//! queued (`take_fired`) for the pack, which chooses the action and its target. A use_ability behaviour is
//! queued only when the unit holds its ability (a unit that does not hold it is skipped, not refused).
//! Behaviours carry no unit link, so every unit is offered each one. The queue is capped and transient; the
//! catalog is read by index and each pending flag is a byte in its unit's array.
//! The winner record is not a behaviour: the sim keeps the side a mod declares with `declare_winner`, and
//! the win rule itself is the mod's (on_check_win).

use crate::aggro::{Fired, FIRED_CAP};
use crate::arrays::Sim;
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

    /// Fires the destroyed trigger on every unit that lost its life since the last call.
    pub(crate) fn fire_destroyed(&mut self) {
        for i in 0..self.n {
            if self.trig_death[i] != 0 {
                self.trig_death[i] = 0;
                self.run_behaviours(i, TRIGGER_DESTROYED, -1);
            }
        }
    }

    /// The number of player slots the map starts (one start record per player).
    pub fn player_count(&self) -> usize {
        self.cat.starts.len()
    }

    /// The first side hostile to `side`, or None when no side is.
    pub fn first_hostile_side(&self, side: u8) -> Option<u8> {
        (0..MAX_PLAYERS as u8).find(|&q| q != side && self.hostile(side, q))
    }

    /// Declares side `side` the winner (the side a mod names from its win rule). The first winner stands.
    pub fn declare_winner(&mut self, side: u8) {
        if self.winner < 0 && (side as usize) < MAX_PLAYERS {
            self.winner = side as i32;
        }
    }

    /// The side declared winner, or None while no side has won.
    pub fn winner(&self) -> Option<u8> {
        u8::try_from(self.winner).ok()
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

    /// Queues every behaviour declared for the named trigger (idle, attacked, spawned or
    /// ability_ready) on unit `i`, as if that trigger had just happened. Returns false for an
    /// out-of-range unit or an unknown trigger name; true once the behaviours are queued.
    pub fn fire_trigger(&mut self, i: usize, name: &str) -> bool {
        if i >= self.n {
            return false;
        }
        let found = (0..=TRIGGER_DESTROYED).find(|&code| Trigger::from_code(code).map_or(false, |t| t.name() == name));
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

    /// Queues every behaviour declared for `trigger` on unit `i` for the pack. For ability_ready only the
    /// behaviours naming `ability` (an ability index, or -1 for any) queue. A use_ability behaviour queues
    /// with its ability's slot on the unit, and only when the unit holds that ability.
    fn run_behaviours(&mut self, i: usize, trigger: u8, ability: i32) {
        for b in 0..self.cat.behaviours.len() {
            let beh = self.cat.behaviours[b];
            if beh.trigger != trigger {
                continue;
            }
            if trigger == TRIGGER_ABILITY_READY && beh.ability as i32 != ability {
                continue;
            }
            let (slot, target_kind) = if beh.action == ACTION_USE_ABILITY {
                let Some(slot) = self.slot_holding(i, beh.ability) else {
                    continue;
                };
                let kind = self.cat.abilities.get(beh.ability as usize).map_or(0, |a| a.target as u8);
                (slot as i32, kind)
            } else {
                (-1, 0)
            };
            if self.aggro.fired.len() < FIRED_CAP {
                self.aggro.fired.push(Fired {
                    unit: i as u32,
                    trigger,
                    action: beh.action,
                    ability: beh.ability as i32,
                    slot,
                    target_kind,
                });
            }
        }
    }

    /// The ability slot of unit `i` that holds catalog ability `ability`, if any.
    fn slot_holding(&self, i: usize, ability: u16) -> Option<usize> {
        (0..ABILITY_SLOTS).find(|&slot| self.abil_idx[i * ABILITY_SLOTS + slot] == ability as i32)
    }
}
