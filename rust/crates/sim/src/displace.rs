use crate::arrays::{cell_of, dist2, Sim};
use crate::fixed;
use crate::loader::Effect;
use crate::rules::Refusal;
use crate::spell::{isqrt64, AOE_CAP};

pub(crate) const K_DISPLACE: u16 = 34;
pub(crate) const FROM_POINT_AWAY: i32 = 0;
pub(crate) const FROM_POINT_TOWARD: i32 = 1;
pub(crate) const FROM_CASTER_TOWARD: i32 = 2;
pub(crate) const CASTER_TO_POINT: i32 = 4;
pub(crate) const FALLOFF: i32 = 8;
const FALLOFF_FLOOR_MILLI: i32 = 16;

pub(crate) fn params_valid(p0: i32, p1: i32, p2: i32, p3: i32) -> bool {
    p0 >= 0 && p2 >= 0 && p3 >= 0 && ((0..=4).contains(&p1) || ((FALLOFF..=FALLOFF + 4).contains(&p1) && p3 > 0))
}

impl Sim {
    pub(crate) fn displace_effect(&mut self, caster: usize, eff: Effect, tx: i32, ty: i32, target: i32) {
        let mode = eff.p1 & !FALLOFF;
        if mode == CASTER_TO_POINT {
            let origin = (self.pos_x[caster], self.pos_y[caster]);
            self.displace_unit(caster, origin, (tx, ty), eff, true);
            return;
        }
        let origin = if mode == FROM_POINT_AWAY || mode == FROM_POINT_TOWARD {
            self.spell.aoe_origin.unwrap_or((tx, ty))
        } else {
            (self.pos_x[caster], self.pos_y[caster])
        };
        if eff.p3 == 0 || eff.p1 & FALLOFF != 0 {
            match (target >= 0 && (target as usize) < self.n).then_some(target as usize) {
                Some(j) if j != caster && self.alive[j] != 0 && !self.displace_resisted(caster, j) => {
                    self.displace_unit(j, origin, origin, eff, false);
                }
                Some(_) => {}
                None => self.refuse(Refusal::NoTarget),
            }
            return;
        }
        let mut found = [0u32; AOE_CAP];
        let n = self.collect(tx, ty, fixed::from_milli(eff.p3), &mut found);
        let p = self.player[caster];
        for &u in &found[..n] {
            let j = u as usize;
            if j != caster && self.hostile(p, self.player[j]) && !self.displace_resisted(caster, j) {
                self.displace_unit(j, origin, origin, eff, false);
            }
        }
    }

    fn displace_resisted(&self, caster: usize, j: usize) -> bool {
        self.hostile(self.player[caster], self.player[j]) && self.turned_away(caster, j)
    }

    fn displace_unit(&mut self, j: usize, from: (i32, i32), to: (i32, i32), eff: Effect, caster_mode: bool) {
        let (ux, uy) = (self.pos_x[j], self.pos_y[j]);
        let mode = eff.p1 & !FALLOFF;
        let toward = caster_mode || mode == FROM_POINT_TOWARD || mode == FROM_CASTER_TOWARD;
        let (ax, ay) = if caster_mode { to } else { from };
        let (dx, dy) = if toward { (ax as i64 - ux as i64, ay as i64 - uy as i64) } else { (ux as i64 - ax as i64, uy as i64 - ay as i64) };
        let dist = isqrt64((dx * dx + dy * dy) as u64) as i64;
        if dist == 0 {
            return;
        }
        let mut wanted = fixed::from_milli(eff.p0) as i64;
        if eff.p1 & FALLOFF != 0 {
            let reach = fixed::from_milli(eff.p3) as i64;
            wanted = (wanted * (reach - dist).max(0) / reach).max(fixed::from_milli(FALLOFF_FLOOR_MILLI) as i64);
        }
        let total = if toward && (wanted == 0 || wanted > dist) { dist } else { wanted };
        if total <= 0 {
            return;
        }
        let ticks = eff.p2.max(1) as i64;
        self.spell.disp_vx[j] = (dx * total / dist / ticks) as i32;
        self.spell.disp_vy[j] = (dy * total / dist / ticks) as i32;
        self.spell.disp_left[j] = ticks as i32;
        if !caster_mode {
            self.interrupt_channel(j);
        }
        if eff.p2 == 0 {
            self.tick_displace(j);
        }
        self.touch(j);
    }

    pub(crate) fn tick_displace(&mut self, i: usize) {
        if self.spell.disp_left[i] <= 0 {
            return;
        }
        self.spell.disp_left[i] -= 1;
        let (x, y) = self.clamp_to_grid(self.pos_x[i].wrapping_add(self.spell.disp_vx[i]), self.pos_y[i].wrapping_add(self.spell.disp_vy[i]));
        if self.flying[i] == 0 && !self.grid.walkable(cell_of(x), cell_of(y)) {
            self.spell.disp_left[i] = 0;
            return;
        }
        if dist2(self.pos_x[i], self.pos_y[i], x, y) == 0 {
            self.spell.disp_left[i] = 0;
            return;
        }
        self.pos_x[i] = x;
        self.pos_y[i] = y;
        self.tgt_x[i] = x;
        self.tgt_y[i] = y;
        self.vel_x[i] = 0;
        self.vel_y[i] = 0;
        self.routed[i] = 0;
        self.blocked[i] = 0;
        self.touch(i);
    }

    pub fn displaced(&self, i: usize) -> bool {
        self.spell.disp_left.get(i).is_some_and(|&t| t > 0)
    }
}
