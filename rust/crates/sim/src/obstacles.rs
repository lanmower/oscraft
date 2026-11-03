use crate::arrays::Sim;
use crate::fixed;
use crate::rules::Refusal;
use crate::TICKS_PER_SECOND;

pub const MAX_OBSTACLES: usize = 16384;
pub const MAX_OBSTACLE_SIDE: i32 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Obstacle {
    pub x0: i32,
    pub y0: i32,
    pub w: i32,
    pub h: i32,
    pub alive: bool,
    pub until: u32,
    pub temp: bool,
    pub prop: i32,
}

impl Obstacle {
    fn centre(&self) -> (i32, i32) {
        (fixed::from_int(self.x0) + fixed::from_int(self.w) / 2, fixed::from_int(self.y0) + fixed::from_int(self.h) / 2)
    }
}

impl Sim {
    fn obstacle_in_grid(&self, x0: i32, y0: i32, w: i32, h: i32) -> bool {
        w >= 1
            && h >= 1
            && w <= MAX_OBSTACLE_SIDE
            && h <= MAX_OBSTACLE_SIDE
            && x0 >= 0
            && y0 >= 0
            && x0 as i64 + w as i64 <= self.grid.w as i64
            && y0 as i64 + h as i64 <= self.grid.h as i64
    }

    fn cover_index(&self, x: i32, y: i32) -> usize {
        (y * self.grid.w as i32 + x) as usize
    }

    fn cover_cells(&mut self, k: usize) {
        let o = self.obstacles[k];
        if self.obstacle_cover.len() != self.grid.flags.len() {
            self.obstacle_cover = vec![0; self.grid.flags.len()];
        }
        for y in o.y0..o.y0 + o.h {
            for x in o.x0..o.x0 + o.w {
                let at = self.cover_index(x, y);
                self.obstacle_cover[at] = self.obstacle_cover[at].saturating_add(1);
                self.grid.flags[at] |= crate::pathfinding::TREE;
            }
        }
    }

    fn uncover_cells(&mut self, k: usize) {
        let o = self.obstacles[k];
        for y in o.y0..o.y0 + o.h {
            for x in o.x0..o.x0 + o.w {
                let at = self.cover_index(x, y);
                self.obstacle_cover[at] = self.obstacle_cover[at].saturating_sub(1);
                if self.obstacle_cover[at] == 0 {
                    self.grid.flags[at] &= !crate::pathfinding::TREE;
                }
            }
        }
    }

    fn push_obstacle(&mut self, x0: i32, y0: i32, w: i32, h: i32, until: u32, temp: bool, prop: i32) -> Result<u32, Refusal> {
        if !self.obstacle_in_grid(x0, y0, w, h) {
            return Err(self.reject(Refusal::InvalidCell));
        }
        let spent = if temp { self.obstacles.iter().position(|o| o.temp && !o.alive) } else { None };
        let k = match spent {
            Some(k) => {
                self.obstacles[k] = Obstacle { x0, y0, w, h, alive: true, until, temp, prop };
                k
            }
            None => {
                if self.obstacles.len() >= MAX_OBSTACLES {
                    return Err(self.reject(Refusal::NoSpawnSlot));
                }
                self.obstacles.push(Obstacle { x0, y0, w, h, alive: true, until, temp, prop });
                self.obstacles.len() - 1
            }
        };
        self.cover_cells(k);
        self.paths.invalidate();
        Ok(k as u32)
    }

    pub fn add_tree(&mut self, x0: i32, y0: i32, w: i32, h: i32, prop: i32) -> Result<u32, Refusal> {
        self.push_obstacle(x0, y0, w, h, 0, false, prop)
    }

    pub fn add_blocker(&mut self, x0: i32, y0: i32, w: i32, h: i32, ticks: u32) -> Result<u32, Refusal> {
        let until = self.ticks.wrapping_add(ticks.max(1)).max(1);
        self.push_obstacle(x0, y0, w, h, until, true, -1)
    }

    pub fn kill_tree(&mut self, id: u32, regrow_ticks: u32) -> Result<(), Refusal> {
        let k = id as usize;
        if k >= self.obstacles.len() || !self.obstacles[k].alive || self.obstacles[k].temp {
            return Err(self.reject(Refusal::NoItem));
        }
        self.uncover_cells(k);
        let until = if regrow_ticks == 0 { 0 } else { self.ticks.wrapping_add(regrow_ticks).max(1) };
        self.obstacles[k].alive = false;
        self.obstacles[k].until = until;
        self.paths.invalidate();
        Ok(())
    }

    pub fn restore_tree(&mut self, id: u32) -> Result<(), Refusal> {
        let k = id as usize;
        if k >= self.obstacles.len() || self.obstacles[k].alive || self.obstacles[k].temp {
            return Err(self.reject(Refusal::NoItem));
        }
        self.obstacles[k].alive = true;
        self.obstacles[k].until = 0;
        self.cover_cells(k);
        self.paths.invalidate();
        Ok(())
    }

    pub fn remove_blocker(&mut self, id: u32) -> Result<(), Refusal> {
        let k = id as usize;
        if k >= self.obstacles.len() || !self.obstacles[k].alive || !self.obstacles[k].temp {
            return Err(self.reject(Refusal::NoItem));
        }
        self.expire_blocker(k);
        Ok(())
    }

    fn expire_blocker(&mut self, k: usize) {
        self.uncover_cells(k);
        self.obstacles[k].alive = false;
        self.obstacles[k].until = 0;
        self.paths.invalidate();
    }

    pub fn obstacles(&self) -> &[Obstacle] {
        &self.obstacles
    }

    pub fn dead_props(&self) -> Vec<u32> {
        self.obstacles.iter().filter(|o| !o.alive && o.prop >= 0).map(|o| o.prop as u32).collect()
    }

    pub fn set_move_tree_rule(&mut self, half: i32, regrow_ticks: u32) {
        self.move_tree_half = half.max(-1);
        self.move_tree_regrow = regrow_ticks;
    }

    pub fn move_tree_rule(&self) -> (i32, u32) {
        (self.move_tree_half, self.move_tree_regrow)
    }

    pub fn clear_trees_square(&mut self, x: i32, y: i32, half: i32, regrow_ticks: u32) -> usize {
        let (x, y, half) = (i64::from(x), i64::from(y), i64::from(half.max(0)));
        let mut felled = Vec::new();
        for (k, o) in self.obstacles.iter().enumerate() {
            if !o.alive || o.temp {
                continue;
            }
            let (lo_x, hi_x) = (i64::from(fixed::from_int(o.x0)), i64::from(fixed::from_int(o.x0 + o.w)));
            let (lo_y, hi_y) = (i64::from(fixed::from_int(o.y0)), i64::from(fixed::from_int(o.y0 + o.h)));
            if lo_x <= x + half && hi_x >= x - half && lo_y <= y + half && hi_y >= y - half {
                felled.push(k as u32);
            }
        }
        for &id in &felled {
            let _ = self.kill_tree(id, regrow_ticks);
        }
        felled.len()
    }

    pub fn tree_guard(&self) -> i32 {
        self.tree_guard
    }

    pub fn set_tree_guard(&mut self, radius: i32) {
        self.tree_guard = radius.max(0);
    }

    pub fn trees_near(&self, x: i32, y: i32, r: i32) -> Vec<u32> {
        let r = i64::from(r.max(0));
        let mut out = Vec::new();
        for (k, o) in self.obstacles.iter().enumerate() {
            if !o.alive || o.temp {
                continue;
            }
            let (lo_x, hi_x) = (i64::from(fixed::from_int(o.x0)), i64::from(fixed::from_int(o.x0 + o.w)));
            let (lo_y, hi_y) = (i64::from(fixed::from_int(o.y0)), i64::from(fixed::from_int(o.y0 + o.h)));
            let dx = (i64::from(x).clamp(lo_x, hi_x) - i64::from(x)).abs();
            let dy = (i64::from(y).clamp(lo_y, hi_y) - i64::from(y)).abs();
            if dx * dx + dy * dy <= r * r {
                out.push(k as u32);
            }
        }
        out
    }

    fn hero_within(&self, x: i32, y: i32, radius: i32) -> bool {
        let r2 = i64::from(radius) * i64::from(radius);
        (0..self.n).any(|i| {
            if self.alive[i] == 0 || !self.is_attributed(i) {
                return false;
            }
            let (dx, dy) = (i64::from(self.pos_x[i]) - i64::from(x), i64::from(self.pos_y[i]) - i64::from(y));
            dx * dx + dy * dy <= r2
        })
    }

    pub(crate) fn tick_obstacles(&mut self) {
        let now = self.ticks;
        for k in 0..self.obstacles.len() {
            let o = self.obstacles[k];
            if o.until == 0 || o.until > now {
                continue;
            }
            if o.temp {
                if o.alive {
                    self.expire_blocker(k);
                }
                continue;
            }
            if o.alive {
                continue;
            }
            let (cx, cy) = o.centre();
            if self.tree_guard > 0 && self.hero_within(cx, cy, self.tree_guard) {
                self.obstacles[k].until = now.wrapping_add(TICKS_PER_SECOND as u32).max(1);
                continue;
            }
            let _ = self.restore_tree(k as u32);
        }
    }

    pub(crate) fn adopt_obstacles(&mut self, old: &Sim) {
        let fits: Vec<Obstacle> = old.obstacles.iter().copied().filter(|o| self.obstacle_in_grid(o.x0, o.y0, o.w, o.h)).collect();
        if fits.len() != old.obstacles.len() {
            return;
        }
        self.set_obstacles(fits, old.tree_guard);
        for k in 0..self.obstacles.len() {
            if self.obstacles[k].alive {
                let o = self.obstacles[k];
                for y in o.y0..o.y0 + o.h {
                    for x in o.x0..o.x0 + o.w {
                        let at = self.cover_index(x, y);
                        self.grid.flags[at] |= crate::pathfinding::TREE;
                    }
                }
            }
        }
        self.paths.invalidate();
    }

    pub(crate) fn set_obstacles(&mut self, items: Vec<Obstacle>, guard: i32) {
        self.obstacles = items;
        self.tree_guard = guard;
        self.obstacle_cover = vec![0; self.grid.flags.len()];
        for k in 0..self.obstacles.len() {
            if !self.obstacles[k].alive {
                continue;
            }
            let o = self.obstacles[k];
            for y in o.y0..o.y0 + o.h {
                for x in o.x0..o.x0 + o.w {
                    let at = self.cover_index(x, y);
                    self.obstacle_cover[at] = self.obstacle_cover[at].saturating_add(1);
                }
            }
        }
    }
}
