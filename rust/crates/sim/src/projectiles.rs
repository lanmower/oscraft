//! Projectiles: a projectile ability flies from its caster at its declared speed and
//! applies its effects where it lands. Travel is the Manhattan distance over the
//! per-tick step, rounded up, and at least one tick. A projectile whose caster has
//! died when it lands is counted as lost and applies nothing.

use crate::arrays::Sim;
use crate::fixed;
use crate::*;

impl Sim {
    pub(crate) fn free_missile(&self) -> Option<usize> {
        self.m_alive.iter().position(|&a| a == 0)
    }

    /// Launches a projectile. The caller has checked that a slot is free.
    pub(crate) fn launch_missile(&mut self, caster: usize, ai: usize, tx: i32, ty: i32, target: i32) -> bool {
        let Some(k) = self.free_missile() else {
            return false;
        };
        let speed = self.cat.abilities[ai].projectile_speed;
        let step = fixed::from_milli(speed).max(1) as i64;
        let dx = (self.pos_x[caster] as i64 - tx as i64).abs();
        let dy = (self.pos_y[caster] as i64 - ty as i64).abs();
        let ticks = ((dx + dy + step - 1) / step).clamp(1, i32::MAX as i64) as i32;
        self.m_alive[k] = 1;
        self.m_timer[k] = ticks;
        self.m_caster[k] = caster as i32;
        self.m_ability[k] = ai as i32;
        self.m_target[k] = target;
        self.m_x[k] = tx;
        self.m_y[k] = ty;
        true
    }

    /// Advances every projectile one tick and lands those whose travel is over.
    pub(crate) fn tick_projectiles(&mut self) {
        for k in 0..MISSILE_CAP {
            if self.m_alive[k] == 0 {
                continue;
            }
            self.m_timer[k] -= 1;
            if self.m_timer[k] > 0 {
                continue;
            }
            self.m_alive[k] = 0;
            let c = self.m_caster[k];
            if self.live_at(c) {
                let (ai, tx, ty, t) = (self.m_ability[k] as usize, self.m_x[k], self.m_y[k], self.m_target[k]);
                self.apply_effects(c as usize, ai, tx, ty, t);
            } else {
                self.stats.missiles_lost += 1;
            }
        }
    }
}
