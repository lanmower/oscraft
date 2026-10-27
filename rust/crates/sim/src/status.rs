
use crate::arrays::Sim;
use crate::fixed;
use crate::spell::{STATUS_ENDS_CHANNEL, STATUS_SETS_SPEED};

pub(crate) const EFFECT_STUN: u16 = 11;
pub(crate) const EFFECT_INVISIBILITY: u16 = 15;
pub(crate) const STATUS_KINDS: usize = 17;
pub(crate) const SLOW_NONE: i32 = fixed::ONE;

pub(crate) const STUN: usize = 0;
pub(crate) const SLOW: usize = 1;
pub(crate) const ROOT: usize = 3;
pub(crate) const INVISIBLE: usize = 4;
pub(crate) const TAUNT: usize = 5;
pub(crate) const HEX: usize = 6;
pub(crate) const DISARM: usize = 7;
pub(crate) const SPELL_IMMUNE: usize = 8;
pub(crate) const INVULNERABLE: usize = 9;
pub(crate) const PHYSICAL_IMMUNE: usize = 12;
pub(crate) const DEATH_PREVENTION: usize = 13;
pub(crate) const BLIND: usize = 14;
pub(crate) const REVEALED: usize = 15;
pub(crate) const SLEEP: usize = 16;

pub const STATUS_NAMES: [&str; STATUS_KINDS] = [
    "stun", "slow", "silence", "root", "invisibility", "taunt", "hex", "disarm", "spell_immunity", "invulnerable", "break", "mute",
    "physical_immunity", "death_prevention", "blind", "revealed", "sleep",
];

impl Sim {
    pub fn status_left(&self, i: usize, kind: usize) -> i32 {
        if i >= self.n || kind >= STATUS_KINDS {
            return 0;
        }
        self.status_left[i * STATUS_KINDS + kind]
    }

    pub fn slow_multiplier(&self, i: usize) -> i32 {
        if self.status_left(i, SLOW) > 0 { self.slow_mul[i] } else { SLOW_NONE }
    }

    pub(crate) fn stunned(&self, i: usize) -> bool {
        self.status_left(i, STUN) > 0 || self.status_left(i, SLEEP) > 0
    }

    pub fn clear_status(&mut self, i: usize, kind: usize) {
        if i < self.n && kind < STATUS_KINDS {
            self.status_left[i * STATUS_KINDS + kind] = 0;
            if kind == SLOW {
                self.slow_mul[i] = SLOW_NONE;
            }
            self.touch(i);
        }
    }

    pub(crate) fn rooted(&self, i: usize) -> bool {
        self.status_left(i, ROOT) > 0
    }

    pub(crate) fn invisible(&self, i: usize) -> bool {
        self.status_left(i, INVISIBLE) > 0
    }

    pub(crate) fn effective_speed(&self, i: usize) -> i32 {
        let sets_speed = (0..STATUS_KINDS).any(|k| self.status_left(i, k) > 0 && self.status_flag(k) & STATUS_SETS_SPEED != 0);
        if sets_speed {
            let own = self.spell.hex_speed.get(i).copied().unwrap_or(0);
            if own > 0 {
                return fixed::from_milli(own);
            }
        }
        let mul = self.slow_multiplier(i);
        let boosted = (self.speed[i] as i64 + self.stat(i, crate::spell::S_SPEED) as i64 * fixed::ONE as i64 / 1000)
            * (100 + self.stat(i, crate::spell::S_SPEED_PCT) as i64).max(0)
            / 100;
        let speed = if mul == SLOW_NONE { boosted as i32 } else { ((boosted * mul as i64) >> fixed::SHIFT) as i32 };
        if self.speed[i] <= 0 {
            return speed;
        }
        speed.max(self.speed_floor[i]).min(self.speed_cap[i])
    }

    pub fn set_speed_bounds(&mut self, i: usize, floor: i32, cap: i32) {
        if i < self.n {
            self.speed_floor[i] = floor;
            self.speed_cap[i] = cap;
        }
    }

    pub fn speed_bounds_of(&self, i: usize) -> (i32, i32) {
        (self.speed_floor.get(i).copied().unwrap_or(0), self.speed_cap.get(i).copied().unwrap_or(i32::MAX))
    }

    pub fn base_speed_of(&self, i: usize) -> i32 {
        self.speed.get(i).copied().unwrap_or(0)
    }

    pub fn type_speed_milli(&self, i: usize) -> i32 {
        if i >= self.n {
            return 0;
        }
        usize::try_from(self.type_of[i]).ok().and_then(|k| self.cat.units.get(k)).map_or(0, |u| u.speed)
    }

    pub fn apply_status(&mut self, i: usize, kind: usize, ticks: i32, mul: i32) {
        if i >= self.n || self.alive[i] == 0 || kind >= STATUS_KINDS || ticks <= 0 {
            return;
        }
        let at = i * STATUS_KINDS + kind;
        if kind == SLOW {
            let held = self.status_left[at] > 0;
            let mul = mul.clamp(0, SLOW_NONE);
            self.slow_mul[i] = if held { self.slow_mul[i].min(mul) } else { mul };
        }
        self.status_left[at] = self.status_left[at].max(ticks);
        if self.status_flag(kind) & STATUS_ENDS_CHANNEL != 0 {
            self.interrupt_channel(i);
        }
        self.touch(i);
    }

    pub fn grant_status(&mut self, i: usize, kind: usize, ticks: i32) {
        self.apply_status(i, kind, ticks, SLOW_NONE);
    }

    pub fn grant_status_scaled(&mut self, i: usize, kind: usize, ticks: i32, mul: i32) {
        self.apply_status(i, kind, ticks, mul.clamp(0, SLOW_NONE));
    }

    pub(crate) fn clear_statuses(&mut self, i: usize) {
        for k in 0..STATUS_KINDS {
            self.status_left[i * STATUS_KINDS + k] = 0;
        }
        self.slow_mul[i] = SLOW_NONE;
    }

    pub(crate) fn tick_statuses(&mut self) {
        for i in 0..self.n {
            let base = i * STATUS_KINDS;
            if self.alive[i] == 0 {
                if self.status_left[base..base + STATUS_KINDS].iter().any(|&t| t != 0) {
                    self.clear_statuses(i);
                }
                continue;
            }
            for k in 0..STATUS_KINDS {
                if self.status_left[base + k] > 0 {
                    self.status_left[base + k] -= 1;
                    if self.status_left[base + k] == 0 {
                        if k == SLOW {
                            self.slow_mul[i] = SLOW_NONE;
                        }
                        self.touch(i);
                    }
                }
            }
        }
    }

    pub(crate) fn hidden_by_invisibility(&self, p: u8, j: usize) -> bool {
        self.invisible(j) && self.hostile(p, self.player[j]) && !self.detected(p, j)
    }
}
