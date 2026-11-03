use crate::arrays::{dist2, Sim};
use crate::fixed::ONE;
use crate::movement::isqrt;
use crate::spell::S_ATTACK_SPEED;
use crate::TICKS_PER_SECOND;

const ORDERED: i32 = 1 << 24;

#[derive(Clone, Debug)]
pub(crate) struct Swing {
    pub(crate) point: Vec<i32>,
    pub(crate) recover: Vec<i32>,
    pub(crate) mx: Vec<i32>,
    pub(crate) my: Vec<i32>,
    pub(crate) tx: Vec<i32>,
    pub(crate) ty: Vec<i32>,
}

impl Swing {
    pub(crate) fn new(cap: usize) -> Swing {
        Swing { point: vec![0; cap], recover: vec![0; cap], mx: vec![0; cap], my: vec![0; cap], tx: vec![0; cap], ty: vec![0; cap] }
    }

    pub(crate) fn tables(&self) -> [&Vec<i32>; 6] {
        [&self.point, &self.recover, &self.mx, &self.my, &self.tx, &self.ty]
    }

    pub(crate) fn tables_mut(&mut self) -> [&mut Vec<i32>; 6] {
        [&mut self.point, &mut self.recover, &mut self.mx, &mut self.my, &mut self.tx, &mut self.ty]
    }

    pub(crate) fn active(&self) -> bool {
        self.point.iter().chain(self.recover.iter()).any(|&t| t > 0)
    }

    pub(crate) fn busy(&self, i: usize) -> bool {
        self.point[i] > 0 || self.recover[i] > 0
    }

    pub(crate) fn clear(&mut self, i: usize) {
        self.point[i] = 0;
        self.recover[i] = 0;
    }
}

impl Sim {
    fn attack_ticks(&self, i: usize, ms: u16) -> i32 {
        let total = (100 + self.bounded_stat(i, S_ATTACK_SPEED)).max(1) as i64;
        let ticks = (i64::from(ms) * TICKS_PER_SECOND as i64 * 100 + 500 * total) / (1000 * total);
        ticks.clamp(0, i32::MAX as i64) as i32
    }

    pub(crate) fn launch_strike(&mut self, i: usize, b: usize, ordered: bool) {
        if self.swing_left[i] > 0 {
            self.swing_left[i] = 0;
            self.land_swing(i);
        }
        let point = self.cat.unit_ext.get(self.type_of[i] as usize).map_or(0, |e| self.attack_ticks(i, e.attack_point));
        self.swing_target[i] = b as i32 | if ordered { ORDERED } else { 0 };
        if point <= 0 {
            self.release_swing(i);
            return;
        }
        self.swing_left[i] = point;
        self.swing.point[i] = point;
    }

    pub fn hit_in_flight(&self, i: usize) -> bool {
        self.swing_left.get(i).is_some_and(|&t| t > 0)
    }

    pub fn backswing_left(&self, i: usize) -> i32 {
        self.swing.recover.get(i).copied().unwrap_or(0)
    }

    pub fn weapon_missile(&self, i: usize) -> Option<(i32, i32)> {
        (self.swing_left.get(i).copied().unwrap_or(0) > 0 && self.swing.point[i] == 0).then(|| (self.swing.mx[i], self.swing.my[i]))
    }

    pub(crate) fn swing_busy(&self, i: usize) -> bool {
        self.swing.busy(i)
    }

    pub(crate) fn swing_victim_is(&self, u: usize, victim: usize) -> bool {
        self.swing_victim(u) == Some(victim)
    }

    fn swing_victim(&self, i: usize) -> Option<usize> {
        let t = self.swing_target[i];
        (t >= 0 && (t & !ORDERED) < self.n as i32).then_some((t & !ORDERED) as usize)
    }

    fn cancel_swing(&mut self, i: usize) {
        self.swing_left[i] = 0;
        self.swing_target[i] = -1;
        self.swing.point[i] = 0;
    }

    fn release_swing(&mut self, i: usize) {
        let Some(b) = self.swing_victim(i) else {
            self.cancel_swing(i);
            return;
        };
        self.swing.point[i] = 0;
        let ext = self.cat.unit_ext.get(self.type_of[i] as usize).copied().unwrap_or_default();
        self.swing.recover[i] = self.attack_ticks(i, ext.attack_backswing).min((self.cooldown[i] - 1).max(0));
        if self.misses(i) {
            self.swing_left[i] = 0;
            self.swing_target[i] = -1;
            return;
        }
        if ext.missile_speed == 0 {
            self.swing_left[i] = 0;
            self.land_swing(i);
            return;
        }
        self.swing.mx[i] = self.pos_x[i];
        self.swing.my[i] = self.pos_y[i];
        self.swing.tx[i] = self.pos_x[b];
        self.swing.ty[i] = self.pos_y[b];
        self.swing_left[i] = 1;
    }

    pub(crate) fn tick_swing(&mut self, i: usize) {
        if self.swing.recover[i] > 0 {
            self.swing.recover[i] -= 1;
        }
        if self.swing_left[i] <= 0 {
            return;
        }
        if self.swing.point[i] > 0 {
            self.tick_point(i);
        } else {
            self.fly_missile(i);
        }
    }

    fn tick_point(&mut self, i: usize) {
        let Some(b) = self.swing_victim(i) else {
            self.cancel_swing(i);
            return;
        };
        let ordered = self.swing_target[i] & ORDERED != 0;
        let kept = !self.stunned(i)
            && !self.disarmed(i)
            && self.strike_ready(i, b)
            && (ordered || self.strike_target[i] == b as i32);
        if !kept {
            self.cancel_swing(i);
            return;
        }
        self.swing.point[i] -= 1;
        self.swing_left[i] = self.swing.point[i];
        if self.swing.point[i] == 0 {
            self.release_swing(i);
        }
    }

    fn fly_missile(&mut self, i: usize) {
        let Some(b) = self.swing_victim(i) else {
            self.cancel_swing(i);
            return;
        };
        let homing = self.cat.unit_ext.get(self.type_of[i] as usize).is_some_and(|e| e.missile_homing);
        let speed = self.cat.unit_ext.get(self.type_of[i] as usize).map_or(0, |e| e.missile_speed);
        if homing {
            if self.alive[b] == 0 {
                self.cancel_swing(i);
                return;
            }
            self.swing.tx[i] = self.pos_x[b];
            self.swing.ty[i] = self.pos_y[b];
        }
        let step = (i64::from(speed) * 1024 / TICKS_PER_SECOND as i64).max(1);
        let (x, y, tx, ty) = (self.swing.mx[i], self.swing.my[i], self.swing.tx[i], self.swing.ty[i]);
        let gap2 = dist2(x, y, tx, ty);
        let reach = if homing { step + i64::from(self.radius[b]) } else { step };
        if gap2 > reach * reach {
            let gap = isqrt(gap2).max(1);
            self.swing.mx[i] = (i64::from(x) + (i64::from(tx) - i64::from(x)) * step / gap) as i32;
            self.swing.my[i] = (i64::from(y) + (i64::from(ty) - i64::from(y)) * step / gap) as i32;
            return;
        }
        self.swing_left[i] = 0;
        let near = i64::from(self.radius[b]) + i64::from(ONE / 2);
        if homing || dist2(tx, ty, self.pos_x[b], self.pos_y[b]) <= near * near {
            self.land_swing(i);
        } else {
            self.swing_target[i] = -1;
        }
    }

    fn land_swing(&mut self, i: usize) {
        let Some(b) = self.swing_victim(i) else {
            self.swing_target[i] = -1;
            return;
        };
        self.swing_target[i] = -1;
        if self.alive[i] == 0 || self.alive[b] == 0 {
            return;
        }
        let missile = self.cat.unit_ext.get(self.type_of[i] as usize).is_some_and(|e| e.missile_speed > 0);
        if !missile && (self.stunned(i) || self.disarmed(i) || !self.strike_ready(i, b)) {
            return;
        }
        self.strike(i, b);
    }
}
