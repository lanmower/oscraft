use crate::arrays::{cell_of, Sim};
use crate::fixed;
use crate::loader::Effect;
use crate::rules::Refusal;
use crate::spell::isqrt64;

pub(crate) const K_WALL: u16 = 38;
pub(crate) const WALL_FROM_CASTER: i32 = 0;

impl Sim {
    pub(crate) fn wall_effect(&mut self, caster: usize, eff: Effect, tx: i32, ty: i32) {
        let (cx, cy) = (self.pos_x[caster] as i64, self.pos_y[caster] as i64);
        let (dx, dy) = (tx as i64 - cx, ty as i64 - cy);
        let len = isqrt64((dx * dx + dy * dy) as u64) as i64;
        if len == 0 {
            self.refuse(Refusal::NoTarget);
            return;
        }
        let total = fixed::from_milli(eff.p0) as i64;
        let (ux, uy, sx, sy) = if eff.p3 == WALL_FROM_CASTER {
            (dx, dy, cx, cy)
        } else {
            (-dy, dx, tx as i64 - (-dy) * total / len / 2, ty as i64 - dx * total / len / 2)
        };
        let side = eff.p2.max(1);
        let step = (fixed::ONE / 2) as i64;
        let mut last = (i32::MIN, i32::MIN);
        let first = if eff.p3 == WALL_FROM_CASTER { 4 } else { 0 };
        for k in first..=total / step {
            let along = k * step;
            let (px, py) = (sx + ux * along / len, sy + uy * along / len);
            let cell = (cell_of(px as i32), cell_of(py as i32));
            if cell == last {
                continue;
            }
            last = cell;
            let _ = self.add_blocker(cell.0, cell.1, side, side, eff.p1 as u32);
        }
    }
}
