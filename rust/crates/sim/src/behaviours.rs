use crate::aggro::{Fired, FIRED_CAP};
use crate::arrays::Sim;
use crate::*;

impl Sim {
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

    pub(crate) fn fire_destroyed(&mut self) {
        for i in 0..self.n {
            if self.trig_death[i] != 0 {
                self.trig_death[i] = 0;
                self.run_behaviours(i, TRIGGER_DESTROYED, -1);
            }
        }
    }

    pub fn player_count(&self) -> usize {
        self.cat.starts.len()
    }

    pub fn first_hostile_side(&self, side: u8) -> Option<u8> {
        (0..MAX_PLAYERS as u8).find(|&q| q != side && self.hostile(side, q))
    }

    pub fn declare_winner(&mut self, side: u8) {
        if self.winner < 0 && (side as usize) < MAX_PLAYERS {
            self.winner = side as i32;
        }
    }

    pub fn winner(&self) -> Option<u8> {
        u8::try_from(self.winner).ok()
    }

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

    fn slot_holding(&self, i: usize, ability: u16) -> Option<usize> {
        (0..ABILITY_SLOTS).find(|&slot| self.abil_idx[i * ABILITY_SLOTS + slot] == ability as i32)
    }
}
