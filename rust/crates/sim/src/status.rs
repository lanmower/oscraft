//! Status effects: stun, slow, silence, root and invisibility. Each is a per-unit countdown in
//! ticks. A status is active while its counter is above zero; the counter falls one per tick in
//! `tick_statuses` and a dead unit's counters are cleared. Reapplying keeps the longer duration.
//!
//! Stun: no orders, no cast, no attack, no movement. Slow: speed is scaled by a Q16 multiplier
//! (the strongest active slow wins). Silence: no ability cast (items still work). Root: no
//! movement, no blink. Invisibility: hidden from hostile players unless a detector covers the unit.

use crate::arrays::Sim;
use crate::fixed;
use crate::loader::Effect;

pub(crate) const EFFECT_STUN: u16 = 11;
pub(crate) const EFFECT_SLOW: u16 = 12;
pub(crate) const EFFECT_SILENCE: u16 = 13;
pub(crate) const EFFECT_ROOT: u16 = 14;
pub(crate) const EFFECT_INVISIBILITY: u16 = 15;
/// Effect kinds the data format defines for statuses are the codes `EFFECT_STUN..=EFFECT_INVISIBILITY`.
pub(crate) const STATUS_KINDS: usize = 5;
/// Q16 speed multiplier of an unslowed unit.
pub(crate) const SLOW_NONE: i32 = fixed::ONE;

/// Dota movement speed limits (550 maximum, 100 minimum; patch 7.38 notes) in Q16.16 per tick.
pub(crate) const MOVE_SPEED_MAX_Q16: i32 = 550 * 64;
pub(crate) const MOVE_SPEED_MIN_Q16: i32 = 100 * 64;

pub(crate) const STUN: usize = 0;
pub(crate) const SLOW: usize = 1;
pub(crate) const SILENCE: usize = 2;
pub(crate) const ROOT: usize = 3;
pub(crate) const INVISIBLE: usize = 4;

/// Status names in counter order, for hosts that report them.
pub const STATUS_NAMES: [&str; STATUS_KINDS] = ["stun", "slow", "silence", "root", "invisibility"];

impl Sim {
    /// Ticks left on status `kind` (index into `STATUS_NAMES`) of unit `i`; 0 when none, unknown unit or kind.
    pub fn status_left(&self, i: usize, kind: usize) -> i32 {
        if i >= self.n || kind >= STATUS_KINDS {
            return 0;
        }
        self.status_left[i * STATUS_KINDS + kind]
    }

    /// Q16 speed multiplier of unit `i`: `SLOW_NONE` unless a slow is active.
    pub fn slow_multiplier(&self, i: usize) -> i32 {
        if self.status_left(i, SLOW) > 0 { self.slow_mul[i] } else { SLOW_NONE }
    }

    pub(crate) fn stunned(&self, i: usize) -> bool {
        self.status_left(i, STUN) > 0
    }

    pub(crate) fn silenced(&self, i: usize) -> bool {
        self.status_left(i, SILENCE) > 0
    }

    pub(crate) fn rooted(&self, i: usize) -> bool {
        self.status_left(i, ROOT) > 0
    }

    pub(crate) fn invisible(&self, i: usize) -> bool {
        self.status_left(i, INVISIBLE) > 0
    }

    /// Unit `i`'s speed after its slow, in the same Q16.16 units as `speed`, held to Dota's limits:
    /// at most 550 (or the unit type's own base when that is higher) and at least 100 for a unit
    /// that can move (or its own speed when that is lower). One Dota speed point is 64 in Q16.16
    /// (1 Dota unit = 15.625 milli, 16 ticks per second).
    pub(crate) fn effective_speed(&self, i: usize) -> i32 {
        let mul = self.slow_multiplier(i);
        let speed = if mul == SLOW_NONE { self.speed[i] } else { ((self.speed[i] as i64 * mul as i64) >> fixed::SHIFT) as i32 };
        if self.speed[i] <= 0 {
            return speed;
        }
        let type_base = match self.type_of[i] {
            k if k >= 0 => fixed::from_milli(self.cat.units[k as usize].speed),
            _ => 0,
        };
        let cap = MOVE_SPEED_MAX_Q16.max(type_base);
        let floor = MOVE_SPEED_MIN_Q16.min(self.speed[i]);
        speed.clamp(floor, cap)
    }

    /// Gives unit `i` status `kind` for `ticks` ticks, keeping the longer of the held and new
    /// duration. A slow also takes the stronger (lower) multiplier `mul` while one is held.
    /// A stun ends the unit's channel. A dead or unknown unit and a non-positive duration change nothing.
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
        if kind == STUN {
            self.interrupt_channel(i);
        }
        self.touch(i);
    }

    /// Clears every status of unit `i`.
    pub(crate) fn clear_statuses(&mut self, i: usize) {
        for k in 0..STATUS_KINDS {
            self.status_left[i * STATUS_KINDS + k] = 0;
        }
        self.slow_mul[i] = SLOW_NONE;
    }

    /// Applies one status effect record to its receiver `target`. Parameters: p0 duration in ticks;
    /// slow also reads p1, the speed multiplier (Q16). A receiver that is no unit changes nothing.
    pub(crate) fn status_effect(&mut self, eff: Effect, target: i32) {
        if target < 0 || target as usize >= self.n {
            return;
        }
        let kind = match eff.kind {
            EFFECT_STUN => STUN,
            EFFECT_SLOW => SLOW,
            EFFECT_SILENCE => SILENCE,
            EFFECT_ROOT => ROOT,
            _ => INVISIBLE,
        };
        self.apply_status(target as usize, kind, eff.p0, eff.p1);
    }

    /// Counts every status down one tick; a dead unit loses all of them.
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

    /// Whether player `p` is blind to unit `j` by invisibility: the unit is invisible, hostile to
    /// `p`, and no detector of `p` covers it.
    pub(crate) fn hidden_by_invisibility(&self, p: u8, j: usize) -> bool {
        self.invisible(j) && self.hostile(p, self.player[j]) && !self.detected(p, j)
    }
}
