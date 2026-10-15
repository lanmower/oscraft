//! Steering, integration, teleport and the route hand-off to the flow-field pool.
//! Velocities are Q16.16 world units per tick.

use crate::arrays::{cell_of, Sim};
use crate::fixed::{self, ONE, SHIFT};
use crate::*;

/// Integer square root, rounded down. Integer only, so no float is read.
fn isqrt(n: i64) -> i64 {
    let mut x = n;
    let mut r = 0i64;
    let mut bit = 1i64 << 62;
    while bit > x {
        bit >>= 2;
    }
    while bit != 0 {
        if x >= r + bit {
            x -= r + bit;
            r = (r >> 1) + bit;
        } else {
            r >>= 1;
        }
        bit >>= 2;
    }
    r
}

impl Sim {
    /// Keeps a unit on its flow field while it is blocked or routed, and drops the
    /// route once the unit is within one world unit of its order.
    pub(crate) fn route_for(&mut self, i: usize) {
        self.cur_field = None;
        if self.flying[i] != 0 {
            return;
        }
        if self.routed[i] != 0 {
            let dx = self.tgt_x[i].wrapping_sub(self.pos_x[i]) as i64;
            let dy = self.tgt_y[i].wrapping_sub(self.pos_y[i]) as i64;
            if dx * dx + dy * dy < (ONE as i64) * (ONE as i64) {
                self.routed[i] = 0;
            } else {
                self.cur_field = Some(self.route_slot(i));
            }
        } else if self.blocked[i] != 0 {
            self.routed[i] = 1;
            self.cur_field = Some(self.route_slot(i));
        }
    }

    fn route_slot(&mut self, i: usize) -> usize {
        let gx = fixed::round(self.tgt_x[i]);
        let gy = fixed::round(self.tgt_y[i]);
        self.paths.request(self.player[i] as i32, gx, gy)
    }

    pub(crate) fn steer(&mut self, i: usize) {
        let dx = self.tgt_x[i].wrapping_sub(self.pos_x[i]);
        let dy = self.tgt_y[i].wrapping_sub(self.pos_y[i]);
        let cx = cell_of(self.pos_x[i]);
        let cy = cell_of(self.pos_y[i]);
        let speed = self.speed[i];

        let mut vx = 0i32;
        let mut vy = 0i32;
        let mut routed = false;
        if let Some(slot) = self.cur_field {
            if let Some((fx, fy)) = self.paths.field(slot).dir_fixed(cx, cy) {
                vx = fixed::mul(fx, speed);
                vy = fixed::mul(fy, speed);
                routed = true;
            }
        }
        if !routed {
            let d2 = dx as i64 * dx as i64 + dy as i64 * dy as i64;
            if d2 > (ONE as i64 * ONE as i64) / 64 {
                let dist = fixed::sqrt((d2 >> SHIFT) as i32) as i64;
                if dist > 0 {
                    vx = ((dx as i64 * speed as i64) / dist) as i32;
                    vy = ((dy as i64 * speed as i64) / dist) as i32;
                }
            }
        }

        // Separation from same-layer neighbours closer than the sum of radii.
        let (gw, gh) = (self.grid.w as i32, self.grid.h as i32);
        for oy in -1..=1i32 {
            for ox in -1..=1i32 {
                let nx = cx + ox;
                let ny = cy + oy;
                if nx < 0 || ny < 0 || nx >= gw || ny >= gh {
                    continue;
                }
                let cell = (ny * gw + nx) as usize;
                let cnt = self.bucket_n[cell] as usize;
                for k in 0..cnt {
                    let j = self.bucket_idx[cell * BUCKET_CAP + k] as usize;
                    if j == i || self.alive[j] == 0 || self.flying[j] != self.flying[i] {
                        continue;
                    }
                    let mut sx = self.pos_x[i].wrapping_sub(self.pos_x[j]);
                    let mut sy = self.pos_y[i].wrapping_sub(self.pos_y[j]);
                    let sd2 = sx as i64 * sx as i64 + sy as i64 * sy as i64;
                    let min_d = self.radius[i].wrapping_add(self.radius[j]) as i64;
                    let md2 = min_d * min_d;
                    if sd2 > 0 && sd2 < md2 {
                        let push = ((md2 - sd2) >> (SHIFT + 6)) as i32;
                        sx = sx.wrapping_abs();
                        sy = sy.wrapping_abs();
                        if sx > sy {
                            vx = vx.wrapping_add(push);
                        } else {
                            vy = vy.wrapping_add(push);
                        }
                    }
                }
            }
        }

        // Cap the combined velocity at the unit's speed.
        let vmag2 = vx as i64 * vx as i64 + vy as i64 * vy as i64;
        let sp = speed as i64;
        if vmag2 > sp * sp {
            let mut denom = fixed::sqrt((vmag2 >> SHIFT) as i32) as i64;
            if denom <= 0 {
                denom = 1;
            }
            let scale = (sp * ONE as i64 / denom) as i32;
            vx = ((vx as i64 * scale as i64) >> SHIFT) as i32;
            vy = ((vy as i64 * scale as i64) >> SHIFT) as i32;
        }
        self.vel_x[i] = vx;
        self.vel_y[i] = vy;
    }

    /// Moves a unit one tick. Ground units are refused unwalkable cells per axis;
    /// air units ignore terrain.
    pub(crate) fn integrate(&mut self, i: usize) {
        let mut nx = self.pos_x[i].wrapping_add(self.vel_x[i]);
        let mut ny = self.pos_y[i].wrapping_add(self.vel_y[i]);
        let max_x = fixed::from_int(self.grid.w as i32 - 1);
        let max_y = fixed::from_int(self.grid.h as i32 - 1);
        nx = nx.clamp(0, max_x);
        ny = ny.clamp(0, max_y);
        if self.flying[i] != 0 {
            self.pos_x[i] = nx;
            self.pos_y[i] = ny;
            self.blocked[i] = 0;
            return;
        }
        let mut refused = 0u8;
        if self.grid.walkable(cell_of(nx), cell_of(self.pos_y[i])) {
            self.pos_x[i] = nx;
        } else {
            self.vel_x[i] = 0;
            refused = 1;
        }
        if self.grid.walkable(cell_of(self.pos_x[i]), cell_of(ny)) {
            self.pos_y[i] = ny;
        } else {
            self.vel_y[i] = 0;
            refused = 1;
        }
        self.blocked[i] = refused;
    }

    /// Places unit `i` at a Q16.16 point, clearing its movement. Dead or unknown units are not moved.
    pub fn teleport(&mut self, i: usize, x: i32, y: i32) -> bool {
        if i >= self.n || self.alive[i] == 0 {
            return false;
        }
        let (x, y) = self.clamp_to_grid(x, y);
        if self.flying[i] == 0 && !self.grid.walkable(cell_of(x), cell_of(y)) {
            return false;
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
        true
    }

    /// Issues an ability's issue_order effect. The receiver is the cast target unit, or the
    /// caster when the cast names no unit. Order 1 (attack_move) sends the receiver to the
    /// point: the caster's own position when `point` is 0 (caster_relative), the cast point
    /// when `point` is 1. Orders 0 (stop) and 2 (hold) set the receiver's destination to its
    /// current position; this sim never chases, so hold and stop are the same here.
    pub(crate) fn issue_order(&mut self, caster: usize, order: i32, point: i32, tx: i32, ty: i32, target: i32) {
        let r = if target >= 0 && (target as usize) < self.n { target as usize } else { caster };
        if self.alive[r] == 0 {
            self.refuse(Refusal::NoTarget);
            return;
        }
        let (x, y) = match order {
            1 if point == 1 => (tx, ty),
            1 => (self.pos_x[caster], self.pos_y[caster]),
            _ => (self.pos_x[r], self.pos_y[r]),
        };
        self.move_fixed(r, x, y);
    }

    /// Moves the caster toward a cast point by at most `cap_milli` milli-units (format v6).
    /// A point within the cap is reached exactly; a farther point is clamped to the cap along
    /// the same line from the caster. A cap of 0, or one whose square exceeds the grid
    /// diagonal's square, refuses as move-range. Integer arithmetic only.
    pub(crate) fn move_caster(&mut self, caster: usize, cap_milli: i32, tx: i32, ty: i32) {
        let diagonal2 = (self.grid.w as i64 * 1000).pow(2) + (self.grid.h as i64 * 1000).pow(2);
        if cap_milli <= 0 || (cap_milli as i64).pow(2) > diagonal2 {
            self.refuse(Refusal::MoveRange);
            return;
        }
        let cap = fixed::from_milli(cap_milli) as i64;
        let (cx, cy) = (self.pos_x[caster] as i64, self.pos_y[caster] as i64);
        let (dx, dy) = (tx as i64 - cx, ty as i64 - cy);
        let d2 = dx * dx + dy * dy;
        let (nx, ny) = if d2 <= cap * cap {
            (tx, ty)
        } else {
            let d = isqrt(d2);
            ((cx + dx * cap / d) as i32, (cy + dy * cap / d) as i32)
        };
        if !self.teleport(caster, nx, ny) {
            self.refuse(Refusal::NoTeleport);
        }
    }

    /// Issues an order of kind `kind` (ORDER_*) to unit `i`. Move, attack-move and patrol
    /// take the destination (x, y); stop and hold take the unit's own position and ignore it.
    /// Patrol remembers the unit's current position as its return point. Refusals: an unknown
    /// kind is InvalidOrderKind, a dead or unknown unit is NoTarget, and a ground unit's
    /// destination on an unwalkable cell is InvalidOrderTarget. A refused order changes nothing.
    pub fn issue_order_kind(&mut self, i: usize, kind: u8, x: i32, y: i32) -> Result<(), Refusal> {
        if kind > ORDER_HOLD {
            return Err(self.reject(Refusal::InvalidOrderKind));
        }
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        match kind {
            ORDER_STOP | ORDER_HOLD => {
                self.tgt_x[i] = self.pos_x[i];
                self.tgt_y[i] = self.pos_y[i];
            }
            _ => {
                let (x, y) = self.clamp_to_grid(x, y);
                if self.flying[i] == 0 && !self.grid.walkable(cell_of(x), cell_of(y)) {
                    return Err(self.reject(Refusal::InvalidOrderTarget));
                }
                if kind == ORDER_PATROL {
                    self.patrol_x[i] = self.pos_x[i];
                    self.patrol_y[i] = self.pos_y[i];
                }
                self.tgt_x[i] = x;
                self.tgt_y[i] = y;
            }
        }
        self.order_kind[i] = kind;
        self.routed[i] = 0;
        self.touch(i);
        Ok(())
    }

    /// A patrolling unit that comes within one world unit of its far point turns back toward
    /// its return point: the two points swap, and the route is dropped for the next tick.
    pub(crate) fn advance_patrol(&mut self, i: usize) {
        if self.order_kind[i] != ORDER_PATROL {
            return;
        }
        let dx = self.tgt_x[i].wrapping_sub(self.pos_x[i]) as i64;
        let dy = self.tgt_y[i].wrapping_sub(self.pos_y[i]) as i64;
        if dx * dx + dy * dy >= (ONE as i64) * (ONE as i64) {
            return;
        }
        std::mem::swap(&mut self.tgt_x[i], &mut self.patrol_x[i]);
        std::mem::swap(&mut self.tgt_y[i], &mut self.patrol_y[i]);
        self.routed[i] = 0;
    }

    /// Sets a movement destination without moving the unit.
    pub fn move_fixed(&mut self, i: usize, x: i32, y: i32) -> bool {
        if i >= self.n || self.alive[i] == 0 {
            return false;
        }
        let (x, y) = self.clamp_to_grid(x, y);
        self.tgt_x[i] = x;
        self.tgt_y[i] = y;
        self.touch(i);
        true
    }

    /// The destination unit `i` was last ordered to, after the grid clamp. None for an unknown index.
    pub fn order_target(&self, i: usize) -> Option<(i32, i32)> {
        if i >= self.n {
            return None;
        }
        Some((self.tgt_x[i], self.tgt_y[i]))
    }
}
