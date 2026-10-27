use crate::arrays::{cell_of, dist2, Sim};
use crate::astar::{self, APPROACH_LOOKAHEAD};
use crate::fixed::{self, ONE, SHIFT};
use crate::*;

pub(crate) fn isqrt(n: i64) -> i64 {
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

    fn friendly_footprint_at(&self, i: usize, x: i32, y: i32) -> bool {
        let (gw, gh) = (self.grid.w as i32, self.grid.h as i32);
        let (cx, cy) = (cell_of(x), cell_of(y));
        for oy in -1..=1i32 {
            for ox in -1..=1i32 {
                let (nx, ny) = (cx + ox, cy + oy);
                if nx < 0 || ny < 0 || nx >= gw || ny >= gh {
                    continue;
                }
                let cell = (ny * gw + nx) as usize;
                for j in self.bucket_members(cell) {
                    if j == i || self.alive[j] == 0 || self.flying[j] != self.flying[i] || self.player[j] != self.player[i] {
                        continue;
                    }
                    let sx = x.wrapping_sub(self.pos_x[j]) as i64;
                    let sy = y.wrapping_sub(self.pos_y[j]) as i64;
                    let min_d = self.radius[i].wrapping_add(self.radius[j]) as i64;
                    if sx * sx + sy * sy < min_d * min_d {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn route_slot(&mut self, i: usize) -> usize {
        let gx = fixed::round(self.tgt_x[i]);
        let gy = fixed::round(self.tgt_y[i]);
        self.paths.request(self.player[i] as i32, gx, gy)
    }

    fn separation(&self, i: usize) -> (i32, i32) {
        let (gw, gh) = (self.grid.w as i32, self.grid.h as i32);
        let cx = cell_of(self.pos_x[i]);
        let cy = cell_of(self.pos_y[i]);
        let mut vx = 0i32;
        let mut vy = 0i32;
        for oy in -1..=1i32 {
            for ox in -1..=1i32 {
                let nx = cx + ox;
                let ny = cy + oy;
                if nx < 0 || ny < 0 || nx >= gw || ny >= gh {
                    continue;
                }
                let cell = (ny * gw + nx) as usize;
                for j in self.bucket_members(cell) {
                    if j == i || self.alive[j] == 0 || self.flying[j] != self.flying[i] {
                        continue;
                    }
                    let mut sx = self.pos_x[i].wrapping_sub(self.pos_x[j]);
                    let sy = self.pos_y[i].wrapping_sub(self.pos_y[j]);
                    if sx == 0 && sy == 0 {
                        sx = if i < j { -1 } else { 1 };
                    }
                    let sd2 = sx as i64 * sx as i64 + sy as i64 * sy as i64;
                    let min_d = self.radius[i].wrapping_add(self.radius[j]) as i64;
                    let md2 = min_d * min_d;
                    if sd2 < md2 {
                        let d = (fixed::sqrt((sd2 >> SHIFT) as i32) as i64).max(1);
                        let half = (min_d - d) / 8;
                        vx = vx.wrapping_add((sx as i64 * half / d) as i32);
                        vy = vy.wrapping_add((sy as i64 * half / d) as i32);
                    }
                }
            }
        }
        (vx, vy)
    }

    pub(crate) fn separate(&mut self, i: usize) {
        if self.rooted(i) {
            self.vel_x[i] = 0;
            self.vel_y[i] = 0;
            return;
        }
        let (vx, vy) = self.separation(i);
        self.vel_x[i] = vx;
        self.vel_y[i] = vy;
        self.integrate(i);
    }

    pub(crate) fn steer(&mut self, i: usize) {
        let dx = self.tgt_x[i].wrapping_sub(self.pos_x[i]);
        let dy = self.tgt_y[i].wrapping_sub(self.pos_y[i]);
        let cx = cell_of(self.pos_x[i]);
        let cy = cell_of(self.pos_y[i]);
        let speed = self.effective_speed(i);

        let mut vx = 0i32;
        let mut vy = 0i32;
        let mut routed = false;
        let remaining2 = dx as i64 * dx as i64 + dy as i64 * dy as i64;
        let hand_attack_move = self.order_kind[i] == ORDER_ATTACK_MOVE && self.dests.id[i] == 0;
        let field = self.cur_field.filter(|_| !(hand_attack_move && remaining2 <= speed as i64 * speed as i64));
        if let Some(slot) = field {
            if let Some((fx, fy)) = self.paths.field(slot).dir_fixed(cx, cy) {
                vx = fixed::mul(fx, speed);
                vy = fixed::mul(fy, speed);
                routed = true;
            }
        }
        if !routed {
            let d2 = dx as i64 * dx as i64 + dy as i64 * dy as i64;
            if d2 > (ONE as i64 * ONE as i64) / 64 {
                let dist = isqrt(d2);
                let (tx, ty) = (self.tgt_x[i], self.tgt_y[i]);
                let snap_clear = (self.flying[i] != 0
                    || (self.grid.walkable(cell_of(tx), cy) && self.grid.walkable(cx, cell_of(ty))))
                    && !self.friendly_footprint_at(i, tx, ty);
                if dist > 0 && dist <= speed as i64 {
                    if snap_clear {
                        vx = dx;
                        vy = dy;
                    } else {
                        self.tgt_x[i] = self.pos_x[i];
                        self.tgt_y[i] = self.pos_y[i];
                    }
                } else if dist > 0 {
                    vx = ((dx as i64 * speed as i64) / dist) as i32;
                    vy = ((dy as i64 * speed as i64) / dist) as i32;
                }
            }
        }

        let (sx, sy) = self.separation(i);
        vx = vx.wrapping_add(sx);
        vy = vy.wrapping_add(sy);

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

    fn blocked_gap2(&self, x: i32, y: i32, radius: i32) -> i64 {
        let reach = fixed::round(radius) + 1;
        let (cx, cy) = (cell_of(x), cell_of(y));
        let mut best = i64::MAX;
        for oy in -reach..=reach {
            for ox in -reach..=reach {
                let (nx, ny) = (cx + ox, cy + oy);
                if self.grid.walkable(nx, ny) {
                    continue;
                }
                let (x0, y0) = (nx.wrapping_mul(ONE), ny.wrapping_mul(ONE));
                best = best.min(dist2(x, y, x.clamp(x0, x0 + ONE), y.clamp(y0, y0 + ONE)));
            }
        }
        best
    }

    fn clearance_kept(&self, radius: i32, from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> bool {
        if radius <= 0 {
            return true;
        }
        let need = i64::from(radius) * i64::from(radius);
        let after = self.blocked_gap2(to_x, to_y, radius);
        after >= need || after >= self.blocked_gap2(from_x, from_y, radius)
    }

    pub(crate) fn integrate(&mut self, i: usize) {
        if self.rooted(i) {
            self.vel_x[i] = 0;
            self.vel_y[i] = 0;
            return;
        }
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
        let (ox, oy) = (self.pos_x[i], self.pos_y[i]);
        let radius = self.radius[i];
        if self.grid.walkable(cell_of(nx), cell_of(self.pos_y[i])) && self.clearance_kept(radius, ox, oy, nx, self.pos_y[i]) {
            self.pos_x[i] = nx;
        } else {
            self.vel_x[i] = 0;
            refused = 1;
        }
        if self.grid.walkable(cell_of(self.pos_x[i]), cell_of(ny)) && self.clearance_kept(radius, self.pos_x[i], oy, self.pos_x[i], ny) {
            self.pos_y[i] = ny;
        } else {
            self.vel_y[i] = 0;
            refused = 1;
        }
        let entered = (cell_of(self.pos_x[i]), cell_of(self.pos_y[i]));
        if entered != (cell_of(ox), cell_of(oy)) && !self.cell_has_room(self.pos_x[i], self.pos_y[i]) {
            self.pos_x[i] = ox;
            self.pos_y[i] = oy;
            self.vel_x[i] = 0;
            self.vel_y[i] = 0;
            self.refuse(Refusal::CellFull);
            refused = 1;
        }
        self.blocked[i] = refused;
    }

    pub fn teleport(&mut self, i: usize, x: i32, y: i32) -> bool {
        if i >= self.n || self.alive[i] == 0 {
            return false;
        }
        let (x, y) = self.clamp_to_grid(x, y);
        if self.flying[i] == 0 && self.move_tree_half >= 0 && !self.obstacles.is_empty() {
            self.clear_trees_square(x, y, self.move_tree_half, self.move_tree_regrow);
        }
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

    pub(crate) fn move_caster(&mut self, caster: usize, cap_milli: i32, path: [i32; 3], tx: i32, ty: i32) {
        if self.rooted(caster) {
            self.refuse(Refusal::Rooted);
            return;
        }
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
        let start = (self.pos_x[caster], self.pos_y[caster]);
        if !self.teleport(caster, nx, ny) {
            self.refuse(Refusal::NoTeleport);
            return;
        }
        if path[0] > 0 && path[1] != 0 {
            self.path_hits(caster, start, (self.pos_x[caster], self.pos_y[caster]), path);
        }
    }

    fn path_hits(&mut self, caster: usize, start: (i32, i32), end: (i32, i32), path: [i32; 3]) {
        const CARRY: i32 = 1;
        const DAMAGE: i32 = 2;
        let half = fixed::from_milli(path[0]) as i64;
        let (sx, sy) = (start.0 as i64, start.1 as i64);
        let (dx, dy) = (end.0 as i64 - sx, end.1 as i64 - sy);
        let len = isqrt(dx * dx + dy * dy);
        if len == 0 {
            return;
        }
        let owner = self.player[caster];
        for j in 0..self.n {
            if j == caster || self.alive[j] == 0 || !self.hostile(owner, self.player[j]) || self.is_structure(j) || self.turned_away(caster, j) {
                continue;
            }
            let (wx, wy) = (self.pos_x[j] as i64 - sx, self.pos_y[j] as i64 - sy);
            let along = (wx * dx + wy * dy) / len;
            let across = (wx * dy - wy * dx) / len;
            if along < -half || along > len + half || across.abs() > half + self.radius[j] as i64 {
                continue;
            }
            if path[1] & DAMAGE != 0 {
                self.spell_damage(caster as i32, j, path[2], 2);
            }
            if path[1] & CARRY != 0 && self.is_attributed(j) && self.alive[j] != 0 {
                let px = end.0 as i64 + dy * across / len;
                let py = end.1 as i64 - dx * across / len;
                self.carried_to(j, px as i32, py as i32);
            }
        }
    }

    fn carried_to(&mut self, j: usize, x: i32, y: i32) {
        let (x, y) = self.clamp_to_grid(x, y);
        if self.flying[j] == 0 && !self.grid.walkable(cell_of(x), cell_of(y)) {
            return;
        }
        self.pos_x[j] = x;
        self.pos_y[j] = y;
        self.tgt_x[j] = x;
        self.tgt_y[j] = y;
        self.vel_x[j] = 0;
        self.vel_y[j] = 0;
        self.routed[j] = 0;
        self.blocked[j] = 0;
        self.interrupt_channel(j);
        self.touch(j);
    }

    pub(crate) fn strike_measure(&self, i: usize, target: usize) -> ((i32, i32), i64) {
        let (tx, ty) = (self.pos_x[target], self.pos_y[target]);
        let (ux, uy) = (self.pos_x[i], self.pos_y[i]);
        if !self.is_structure(target) {
            return ((tx, ty), dist2(ux, uy, tx, ty));
        }
        let radius = self.radius[target];
        let limit = (radius + ONE) as i64;
        let span = fixed::round(radius) + 3;
        let (w, h) = (self.grid.w as i32, self.grid.h as i32);
        let (cx, cy) = (cell_of(tx), cell_of(ty));
        let mut edge: Option<((i32, i32), i64)> = None;
        for oy in -span..=span {
            for ox in -span..=span {
                let (nx, ny) = (cx + ox, cy + oy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h || self.grid.walkable(nx, ny) {
                    continue;
                }
                let (x0, y0) = (nx * ONE, ny * ONE);
                if dist2(x0 + ONE / 2, y0 + ONE / 2, tx, ty) > limit * limit {
                    continue;
                }
                let p = (ux.clamp(x0, x0 + ONE), uy.clamp(y0, y0 + ONE));
                let d2 = dist2(ux, uy, p.0, p.1);
                if edge.map_or(true, |(_, best)| d2 < best) {
                    edge = Some((p, d2));
                }
            }
        }
        if let Some(hit) = edge {
            return hit;
        }
        let d = isqrt(dist2(ux, uy, tx, ty));
        let r = radius as i64;
        if d <= r {
            return ((ux, uy), 0);
        }
        let ax = tx as i64 + (ux as i64 - tx as i64) * r / d;
        let ay = ty as i64 + (uy as i64 - ty as i64) * r / d;
        let anchor = (ax as i32, ay as i32);
        (anchor, dist2(ux, uy, anchor.0, anchor.1))
    }

    pub(crate) fn strike_stop(&self, i: usize, target: usize) -> (i32, i32) {
        let (anchor, d2) = self.strike_measure(i, target);
        let d = isqrt(d2).max(1);
        let r = self.range_q[i] as i64 * 3 / 4;
        let dx = self.pos_x[i] as i64 - anchor.0 as i64;
        let dy = self.pos_y[i] as i64 - anchor.1 as i64;
        ((anchor.0 as i64 + dx * r / d) as i32, (anchor.1 as i64 + dy * r / d) as i32)
    }

    pub fn order_attack_or_approach(&mut self, i: usize, target: usize) -> Result<bool, Refusal> {
        if i >= self.n || self.alive[i] == 0 || target >= self.n || self.alive[target] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        if self.stunned(i) {
            return Err(self.reject(Refusal::Stunned));
        }
        let legal = i != target
            && self.damage[i] > 0
            && (self.hostile(self.player[i], self.player[target]) || self.denies(i, target))
            && (self.targets[i] & self.layer_of(target)) != 0
            && self.visible(self.player[i], target);
        if !legal {
            return Err(self.reject(Refusal::InvalidOrderTarget));
        }
        let reach = (self.range_sq[i] as i64) << SHIFT;
        let (_, d2) = self.strike_measure(i, target);
        if d2 <= reach {
            self.order_attack(i, target)?;
            return Ok(true);
        }
        let (sx, sy) = self.strike_stop(i, target);
        self.approach_to(i, sx, sy);
        Ok(false)
    }

    fn approach_to(&mut self, i: usize, x: i32, y: i32) {
        let from = (cell_of(self.pos_x[i]), cell_of(self.pos_y[i]));
        let to = (cell_of(x), cell_of(y));
        if self.flying[i] != 0 || astar::line_clear(&self.grid, from, to) {
            self.move_fixed(i, x, y);
            return;
        }
        let near = astar::span(from, to) <= astar::ASTAR_MAX_SPAN;
        let goal = if near { self.grid.nearest_walkable(to.0, to.1, 8) } else { None };
        let route = goal.and_then(|g| astar::route(&self.grid, from, g, astar::ASTAR_NODE_BUDGET));
        let waypoint = route.and_then(|path| path.get(APPROACH_LOOKAHEAD.min(path.len()).max(1) - 1).copied());
        match waypoint {
            Some(c) => {
                let (wx, wy) = astar::cell_centre(c);
                self.move_fixed(i, wx, wy);
            }
            None => {
                let (ax, ay) = if self.flying[i] != 0 || self.grid.walkable(to.0, to.1) {
                    (x, y)
                } else {
                    self.grid.nearest_walkable(to.0, to.1, 8).map(astar::cell_centre).unwrap_or((x, y))
                };
                self.move_fixed(i, ax, ay);
                self.routed[i] = 1;
            }
        }
    }

    pub fn issue_order_kind(&mut self, i: usize, kind: u8, x: i32, y: i32) -> Result<(), Refusal> {
        if kind > ORDER_HOLD {
            return Err(self.reject(Refusal::InvalidOrderKind));
        }
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        if self.stunned(i) {
            return Err(self.reject(Refusal::Stunned));
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
        self.interrupt_channel(i);
        self.touch(i);
        self.record_order(i);
        Ok(())
    }

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

    pub fn order_of(&self, i: usize) -> (u8, i32, i32) {
        (self.order_kind[i], self.tgt_x[i], self.tgt_y[i])
    }
}
