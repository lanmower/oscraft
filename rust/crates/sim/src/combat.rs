//! Weapon attacks and damage. A hit is reduced by the target's armour, then its
//! damage response, then absorbed by its shield, then taken from life. Ability
//! damage goes through the same path, minus the armour.

use crate::arrays::{dist2, cell_of, Sim};
use crate::fixed;
use crate::*;

const RESPONSE_REDUCED: u8 = 1;
const RESPONSE_IMMUNE: u8 = 2;
/// The least a weapon hit can do after armour, so armour never zeroes a hit (Q16.16, one quarter).
const ARMOUR_FLOOR: i32 = fixed::ONE / 4;

/// Weapon damage after a flat armour reduction, in the units both arguments share (whole units or
/// Q16.16). Never below zero. A hit's floor (ARMOUR_FLOOR) is applied by the attack paths.
pub fn armour_reduced(damage: i32, armour: i32) -> i32 {
    damage.saturating_sub(armour).max(0)
}

impl Sim {
    /// Final damage factor (Q16) for a weapon hit by unit `attacker` on unit `target`: the DMGF
    /// record for the attacker's weapon damage type and the target's size category, or 1.0 when
    /// no record applies.
    pub(crate) fn final_factor(&self, attacker: usize, target: usize) -> i32 {
        let dtype = self.cat.unit_ext.get(self.type_of[attacker] as usize).map_or(0, |e| e.damage_type);
        let size = self.cat.unit_ext.get(self.type_of[target] as usize).map_or(0, |e| e.size);
        self.cat
            .damage_factors
            .iter()
            .find(|f| f.damage_type == dtype && f.size == size)
            .map_or(fixed::ONE, |f| f.factor)
    }

    /// A weapon hit after armour, scaled by the final damage factor of the attacker's type and
    /// the target's size category.
    pub fn scale_hit(&self, dmg: i32, attacker: usize, target: usize) -> i32 {
        ((dmg as i64 * self.final_factor(attacker, target) as i64) >> fixed::SHIFT) as i32
    }

    /// Whether a ranged shot by unit `i` misses. A shot is ranged when its weapon reaches past
    /// melee range. The chance is the UEXT miss chance (Q16), rolled on the sim's RNG, which advances
    /// once per ranged shot. The format has no high-ground or cover rule, so the chance applies to
    /// every ranged shot.
    pub fn misses(&mut self, i: usize) -> bool {
        let chance = self.cat.unit_ext.get(self.type_of[i] as usize).map_or(0, |e| e.miss_chance);
        if chance <= 0 || self.range_q[i] <= MELEE_RANGE {
            return false;
        }
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let roll = ((self.rng >> 33) & 0xFFFF) as i32;
        roll < chance
    }

    /// Whether unit `j` is hidden from enemies: cloaked, or burrowed.
    fn hidden(&self, j: usize) -> bool {
        let cloaked = self.cat.unit_ext.get(self.type_of[j] as usize).map_or(false, |e| e.cloak & 1 != 0);
        cloaked || self.burrowed[j] != 0
    }

    /// Whether one of player `p`'s own units with a detector radius reaches unit `j`.
    fn detected(&self, p: u8, j: usize) -> bool {
        (0..self.n).any(|k| {
            if self.alive[k] == 0 || self.player[k] != p {
                return false;
            }
            let radius = self.cat.unit_ext.get(self.type_of[k] as usize).map_or(0, |e| e.detector_radius);
            if radius <= 0 {
                return false;
            }
            let reach = fixed::from_milli(radius) as i64;
            dist2(self.pos_x[k], self.pos_y[k], self.pos_x[j], self.pos_y[j]) <= reach * reach
        })
    }

    /// Whether player `p` may target unit `j`: a hidden unit needs a detector covering it.
    pub(crate) fn targetable(&self, p: u8, j: usize) -> bool {
        !self.hidden(j) || self.detected(p, j)
    }

    /// Starts unit `i` burrowing for its UEXT burrow time; a zero time burrows at once. Only a
    /// unit with the can-burrow flag may burrow. Refusals change nothing: a dead or unknown unit
    /// is NoTarget; a unit that cannot burrow, or is already burrowed or burrowing, is InvalidOrderTarget.
    pub fn burrow_unit(&mut self, i: usize) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        let ext = self.cat.unit_ext.get(self.type_of[i] as usize).copied().unwrap_or_default();
        if ext.cloak & 2 == 0 || self.burrowed[i] != 0 || self.burrow_left[i] > 0 {
            return Err(self.reject(Refusal::InvalidOrderTarget));
        }
        if ext.burrow_ticks <= 0 {
            self.burrowed[i] = 1;
        } else {
            self.burrow_left[i] = ext.burrow_ticks;
        }
        self.touch(i);
        Ok(())
    }

    /// Counts down a burrow in progress on unit `i`; the unit is burrowed when it ends.
    pub(crate) fn burrow_step(&mut self, i: usize) {
        if self.burrow_left[i] > 0 {
            self.burrow_left[i] -= 1;
            if self.burrow_left[i] == 0 {
                self.burrowed[i] = 1;
                self.touch(i);
            }
        }
    }

    /// Auto-attack: nearest visible enemy on a layer this weapon may hit, within range.
    pub(crate) fn attack(&mut self, i: usize) {
        if self.damage[i] <= 0 {
            return;
        }
        if self.cooldown[i] > 0 {
            self.cooldown[i] -= 1;
            return;
        }
        let rc = (fixed::round(self.range_q[i]) + 1).clamp(1, 8);
        let cx = cell_of(self.pos_x[i]);
        let cy = cell_of(self.pos_y[i]);
        let (w, h) = (self.grid.w as i32, self.grid.h as i32);
        let mut best = -1i64;
        let mut best_d2 = (self.range_sq[i] as i64) << fixed::SHIFT;
        for oy in -rc..=rc {
            for ox in -rc..=rc {
                let nx = cx + ox;
                let ny = cy + oy;
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let cell = (ny * w + nx) as usize;
                let cnt = self.bucket_n[cell] as usize;
                for k in 0..cnt {
                    let j = self.bucket_idx[cell * BUCKET_CAP + k] as usize;
                    if j == i || self.alive[j] == 0 || !self.hostile(self.player[i], self.player[j]) {
                        continue;
                    }
                    if (self.targets[i] & self.layer_of(j)) == 0 || !self.visible(self.player[i], j) || !self.targetable(self.player[i], j) {
                        continue;
                    }
                    let d2 = dist2(self.pos_x[i], self.pos_y[i], self.pos_x[j], self.pos_y[j]);
                    if d2 <= best_d2 {
                        best = j as i64;
                        best_d2 = d2;
                    }
                }
            }
        }
        if best < 0 {
            return;
        }
        let b = best as usize;
        let dmg = self.scale_hit(armour_reduced(self.damage[i], self.armor[b]).max(ARMOUR_FLOOR), i, b);
        self.cooldown[i] = self.period[i];
        if self.misses(i) {
            return;
        }
        self.inflict(b, dmg);
    }

    /// Orders unit `i` to strike unit `target` through the weapon path (cooldown, armour,
    /// damage response, shield, then life). Refusals, each changing nothing: a dead or unknown
    /// unit is NoTarget; a target that is not a living visible enemy in weapon reach, or an
    /// attacker that cannot hit, is InvalidOrderTarget; a weapon on cooldown is Cooldown.
    pub fn order_attack(&mut self, i: usize, target: usize) -> Result<(), crate::rules::Refusal> {
        use crate::rules::Refusal;
        if i >= self.n || self.alive[i] == 0 || target >= self.n || self.alive[target] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        let reach = (self.range_sq[i] as i64) << fixed::SHIFT;
        let in_reach = i != target
            && self.damage[i] > 0
            && self.hostile(self.player[i], self.player[target])
            && (self.targets[i] & self.layer_of(target)) != 0
            && self.visible(self.player[i], target)
            && self.targetable(self.player[i], target)
            && dist2(self.pos_x[i], self.pos_y[i], self.pos_x[target], self.pos_y[target]) <= reach;
        if !in_reach {
            return Err(self.reject(Refusal::InvalidOrderTarget));
        }
        if self.cooldown[i] > 0 {
            return Err(self.reject(Refusal::Cooldown));
        }
        let dmg = self.scale_hit(armour_reduced(self.damage[i], self.armor[target]).max(ARMOUR_FLOOR), i, target);
        self.cooldown[i] = self.period[i];
        if self.misses(i) {
            return Ok(());
        }
        self.inflict(target, dmg);
        Ok(())
    }

    /// Ability damage: the response and shield apply, armour does not.
    /// Ability damage to unit `j`, through the shared damage path. An unknown unit is a no-op.
    pub fn hurt(&mut self, j: usize, dmg: i32) {
        if j >= self.n {
            return;
        }
        let before = self.hp[j];
        self.inflict(j, dmg);
        self.stats.ability_damage = self.stats.ability_damage.wrapping_add(before - self.hp[j]);
    }

    /// The damage path shared by weapons and abilities: damage response, then shield,
    /// then life, with death recorded when life runs out.
    fn inflict(&mut self, j: usize, mut dmg: i32) {
        self.touch(j);
        if dmg > 0 {
            self.trig_hit[j] = 1;
            self.interrupt_channel(j);
            self.last_hit[j] = self.ticks;
            match self.response[j] {
                RESPONSE_IMMUNE => dmg = 0,
                RESPONSE_REDUCED => dmg /= 2,
                _ => {}
            }
            dmg = self.absorb_shield(j, dmg);
        }
        self.hp[j] = self.hp[j].wrapping_sub(dmg);
        if self.hp[j] <= 0 {
            self.hp[j] = 0;
            self.alive[j] = 0;
            self.stats.deaths += 1;
        }
    }

    /// Damage to the cast target unit alone, for a damage effect with radius 0. Only a living
    /// enemy other than the caster takes it.
    pub fn damage_target(&mut self, caster: i32, p: u8, dmg: i32, target: i32) {
        if dmg <= 0 || target < 0 || target as usize >= self.n {
            return;
        }
        let j = target as usize;
        if target == caster || self.alive[j] == 0 || !self.hostile(p, self.player[j]) {
            return;
        }
        self.hurt(j, dmg);
    }

    /// Damages every living enemy unit within `rad` of a point. `caster` is the unit that
    /// cast it, or -1; the caster is spared.
    pub fn damage_area(&mut self, caster: i32, p: u8, dmg: i32, rad: i32, x: i32, y: i32) {
        if dmg <= 0 {
            return;
        }
        let mut buf = std::mem::take(&mut self.scratch);
        let n = self.gather(x, y, rad, &mut buf);
        for k in 0..n {
            let j = buf[k] as usize;
            if j as i32 == caster || self.alive[j] == 0 || !self.hostile(p, self.player[j]) {
                continue;
            }
            self.hurt(j, dmg);
        }
        self.scratch = buf;
    }

    /// Writes the indices of units inside a disc into `buf`, in cell then slot order.
    /// Returns how many were written; `buf` is never grown.
    pub(crate) fn gather(&self, x: i32, y: i32, rad: i32, buf: &mut [u32]) -> usize {
        let rad = rad.max(0);
        let rcells = (fixed::round(rad) + 1).min(8);
        let rad2 = rad as i64 * rad as i64;
        let cx = fixed::to_int(x);
        let cy = fixed::to_int(y);
        let (w, h) = (self.grid.w as i32, self.grid.h as i32);
        let mut count = 0usize;
        for oy in -rcells..=rcells {
            for ox in -rcells..=rcells {
                let nx = cx + ox;
                let ny = cy + oy;
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let cell = (ny * w + nx) as usize;
                let cnt = self.bucket_n[cell] as usize;
                for k in 0..cnt {
                    if count == buf.len() {
                        return count;
                    }
                    let j = self.bucket_idx[cell * BUCKET_CAP + k];
                    let ju = j as usize;
                    if dist2(self.pos_x[ju], self.pos_y[ju], x, y) > rad2 {
                        continue;
                    }
                    buf[count] = j;
                    count += 1;
                }
            }
        }
        count
    }
}
