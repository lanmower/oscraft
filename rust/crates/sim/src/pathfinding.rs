//! Flow-field pathfinding: a Dijkstra distance field per destination, solved by a
//! bucket queue of three rotating frontiers, with a pool that caps live fields
//! and spends a fixed cell budget per tick. Port of internal/pathfind.

use crate::fixed::ONE;

pub const COST_STRAIGHT: i32 = ONE;
pub const COST_DIAGONAL: i32 = 92682;
pub const UNIT_DIAGONAL: i32 = 46341;
pub const NO_DIR: u8 = 255;
pub const DEFAULT_MAX_FIELDS: usize = 16;
pub const DEFAULT_TICK_BUDGET: usize = 4096;
/// Cells a tick spends on one destination that has no served route yet.
pub const FIRST_BUILD_BUDGET: usize = 16384;
pub const MOVE_THRESHOLD: i32 = 4;
pub const REBUILD_CADENCE: u32 = 512;
/// Cell bit set on cells that refuse pathing and placement (`bit 0 = blocked`).
pub const BLOCKED: u8 = 1;

pub(crate) const NB_DX: [i32; 8] = [1, 1, 0, -1, -1, -1, 0, 1];
pub(crate) const NB_DY: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
pub(crate) const NB_COST: [i32; 8] = [
    COST_STRAIGHT, COST_DIAGONAL, COST_STRAIGHT, COST_DIAGONAL,
    COST_STRAIGHT, COST_DIAGONAL, COST_STRAIGHT, COST_DIAGONAL,
];
const NB_FX: [i32; 8] = [ONE, UNIT_DIAGONAL, 0, -UNIT_DIAGONAL, -ONE, -UNIT_DIAGONAL, 0, UNIT_DIAGONAL];
const NB_FY: [i32; 8] = [0, UNIT_DIAGONAL, ONE, UNIT_DIAGONAL, 0, -UNIT_DIAGONAL, -ONE, -UNIT_DIAGONAL];

/// Walkability: one byte per cell, nonzero is walkable, row-major, `w` wide.
#[derive(Clone, Debug, Default)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub flags: Vec<u8>,
    /// Height level per cell, same row-major indexing as flags.
    pub heights: Vec<u8>,
    /// Milli-units per height level.
    pub height_step: i32,
    /// Tile id per cell, same row-major indexing as flags. Author-defined; the sim maps ids to terrain.
    pub tiles: Vec<u8>,
}

impl Grid {
    pub fn open(w: usize, h: usize) -> Grid {
        Grid { w, h, flags: vec![0; w * h], heights: vec![0; w * h], height_step: 1000, tiles: vec![0; w * h] }
    }

    /// Elevation of cell (x, y) in milli-units: its height level times the height step.
    /// Zero outside the grid. Nothing in movement, vision or combat reads it yet.
    pub fn elevation_milli(&self, x: i32, y: i32) -> i32 {
        let (w, h) = (self.w as i32, self.h as i32);
        if x < 0 || y < 0 || x >= w || y >= h {
            return 0;
        }
        let level = self.heights.get((y * w + x) as usize).copied().unwrap_or(0) as i32;
        level * self.height_step
    }

    pub fn walkable(&self, x: i32, y: i32) -> bool {
        let (w, h) = (self.w as i32, self.h as i32);
        if x < 0 || y < 0 || x >= w || y >= h {
            return false;
        }
        self.flags[(y * w + x) as usize] & BLOCKED == 0
    }

    /// Sets or clears the blocked bit of one cell and leaves its other bits alone. Returns
    /// false for a cell outside the grid, which is left unchanged. Callers that reshape pathing
    /// must also invalidate the flow fields (Pool::invalidate).
    pub fn set_blocked(&mut self, x: i32, y: i32, blocked: bool) -> bool {
        let (w, h) = (self.w as i32, self.h as i32);
        if x < 0 || y < 0 || x >= w || y >= h {
            return false;
        }
        let cell = &mut self.flags[(y * w + x) as usize];
        if blocked {
            *cell |= BLOCKED;
        } else {
            *cell &= !BLOCKED;
        }
        true
    }

    /// Sets the tile id of cell (x, y). Returns false outside the grid.
    pub fn set_tile(&mut self, x: i32, y: i32, tile: u8) -> bool {
        let (w, h) = (self.w as i32, self.h as i32);
        if x < 0 || y < 0 || x >= w || y >= h {
            return false;
        }
        self.tiles[(y * w + x) as usize] = tile;
        true
    }

    /// Sets or clears the cliff bit (bit 1) of cell (x, y). Returns false outside the grid.
    pub fn set_cliff(&mut self, x: i32, y: i32, cliff: bool) -> bool {
        let (w, h) = (self.w as i32, self.h as i32);
        if x < 0 || y < 0 || x >= w || y >= h {
            return false;
        }
        let cell = &mut self.flags[(y * w + x) as usize];
        if cliff {
            *cell |= 0b10;
        } else {
            *cell &= !0b10;
        }
        true
    }

    pub fn nearest_walkable(&self, x: i32, y: i32, max_r: i32) -> Option<(i32, i32)> {
        if self.walkable(x, y) {
            return Some((x, y));
        }
        for r in 1..=max_r {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx != -r && dx != r && dy != -r && dy != r {
                        continue;
                    }
                    if self.walkable(x + dx, y + dy) {
                        return Some((x + dx, y + dy));
                    }
                }
            }
        }
        None
    }
}

/// One destination's solved route. `dist` is the cost to the goal; `dir` is the
/// index of the first step from each cell.
#[derive(Clone)]
pub struct Field {
    pub(crate) w: i32,
    pub(crate) h: i32,
    pub(crate) cells: usize,
    pub(crate) dist: Vec<i32>,
    pub(crate) dir: Vec<u8>,
    pub(crate) dir_next: Vec<u8>,
    pub(crate) settled: Vec<u8>,
    pub(crate) cur: Vec<u32>,
    pub(crate) n1: Vec<u32>,
    pub(crate) n2: Vec<u32>,
    pub(crate) b: i32,
    pub(crate) building: bool,
    pub(crate) ready: bool,
    pub(crate) goal_x: i32,
    pub(crate) goal_y: i32,
    pub(crate) want_x: i32,
    pub(crate) want_y: i32,
}

impl Field {
    pub fn new(w: usize, h: usize) -> Field {
        let cells = (w * h).max(1);
        Field {
            w: w as i32,
            h: h as i32,
            cells,
            dist: vec![-1; cells],
            dir: vec![0; cells],
            dir_next: vec![NO_DIR; cells],
            settled: vec![0; cells],
            cur: Vec::new(),
            n1: Vec::new(),
            n2: Vec::new(),
            b: 0,
            building: false,
            ready: false,
            goal_x: 0,
            goal_y: 0,
            want_x: 0,
            want_y: 0,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn is_building(&self) -> bool {
        self.building
    }

    pub fn goal(&self) -> (i32, i32) {
        (self.goal_x, self.goal_y)
    }

    fn dir_index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h || !self.ready {
            return None;
        }
        let k = self.dir[(y * self.w + x) as usize];
        if k > 7 { None } else { Some(k as usize) }
    }

    /// Unit step (each component -1, 0 or 1) from a cell, when the route is served.
    pub fn dir(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let k = self.dir_index(x, y)?;
        Some((NB_DX[k], NB_DY[k]))
    }

    /// The same step as a Q16.16 unit vector, so a speed multiplies it directly.
    pub fn dir_fixed(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let k = self.dir_index(x, y)?;
        Some((NB_FX[k], NB_FY[k]))
    }

    pub fn discard(&mut self) {
        self.building = false;
        self.ready = false;
    }

    /// Solves the field in one call (load time and tests). A tick uses reset + step.
    pub fn build(&mut self, g: &Grid, gx: i32, gy: i32) -> bool {
        if !self.reset(g, gx, gy) {
            return false;
        }
        let budget = self.cells;
        while !self.step(g, budget) {}
        true
    }

    pub fn reset(&mut self, g: &Grid, gx: i32, gy: i32) -> bool {
        if g.w != self.w as usize || g.h != self.h as usize || g.flags.len() < self.cells {
            return false;
        }
        let (mut gx, mut gy) = (gx, gy);
        if !g.walkable(gx, gy) {
            match g.nearest_walkable(gx, gy, 8) {
                Some((nx, ny)) => {
                    gx = nx;
                    gy = ny;
                }
                None => return false,
            }
        }
        self.dist.fill(-1);
        self.settled.fill(0);
        self.dir_next.fill(NO_DIR);
        self.cur.clear();
        self.n1.clear();
        self.n2.clear();
        self.b = 0;
        let goal = (gy * self.w + gx) as usize;
        self.dist[goal] = 0;
        self.want_x = gx;
        self.want_y = gy;
        self.building = true;
        self.cur.push(goal as u32);
        true
    }

    /// Settles at most `budget` cells. Returns true when the build is complete.
    pub fn step(&mut self, g: &Grid, budget: usize) -> bool {
        if !self.building {
            return true;
        }
        let (w, h) = (self.w, self.h);
        let nb_off: [i32; 8] = std::array::from_fn(|k| NB_DY[k] * w + NB_DX[k]);
        let mut n = 0usize;
        while n < budget {
            while self.cur.is_empty() {
                if self.n1.is_empty() && self.n2.is_empty() {
                    self.finish();
                    return true;
                }
                let spent = std::mem::take(&mut self.cur);
                self.cur = std::mem::take(&mut self.n1);
                self.n1 = std::mem::take(&mut self.n2);
                self.n2 = spent;
                self.b += 1;
            }
            let c = match self.cur.pop() {
                Some(c) => c as usize,
                None => break,
            };
            if self.settled[c] != 0 {
                continue;
            }
            self.settled[c] = 1;
            n += 1;
            let cy = c as i32 / w;
            let cx = c as i32 - cy * w;
            let dc = self.dist[c];
            let interior = cx > 0 && cy > 0 && cx < w - 1 && cy < h - 1;
            for k in 0..8usize {
                let nn = if interior {
                    (c as i32 + nb_off[k]) as usize
                } else {
                    let nx = cx + NB_DX[k];
                    let ny = cy + NB_DY[k];
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    (ny * w + nx) as usize
                };
                // A settled cell holds a distance no larger than this cell's plus any step cost,
                // so the distance comparison alone rejects it.
                let nd = dc.wrapping_add(NB_COST[k]);
                let known = self.dist[nn];
                if known >= 0 && nd >= known {
                    continue;
                }
                if g.flags[nn] & BLOCKED != 0 {
                    continue;
                }
                // No corner cutting: a diagonal needs both orthogonal neighbours walkable.
                if NB_COST[k] == COST_DIAGONAL {
                    let side_x = (c as i32 + NB_DX[k]) as usize;
                    let side_y = (c as i32 + NB_DY[k] * w) as usize;
                    if g.flags[side_x] & BLOCKED != 0 || g.flags[side_y] & BLOCKED != 0 {
                        continue;
                    }
                }
                self.dist[nn] = nd;
                self.dir_next[nn] = (k as u8) ^ 4;
                match (nd >> 16) - self.b {
                    0 => self.cur.push(nn as u32),
                    1 => self.n1.push(nn as u32),
                    _ => self.n2.push(nn as u32),
                }
            }
        }
        if self.cur.is_empty() && self.n1.is_empty() && self.n2.is_empty() {
            self.finish();
            return true;
        }
        false
    }

    fn finish(&mut self) {
        std::mem::swap(&mut self.dir, &mut self.dir_next);
        self.goal_x = self.want_x;
        self.goal_y = self.want_y;
        self.building = false;
        self.ready = true;
    }
}

/// The destinations that hold a field at once. Past the cap, the least recently
/// used slot is evicted and its units steer straight until a field is ready.
#[derive(Clone)]
pub struct Pool {
    pub(crate) w: usize,
    pub(crate) h: usize,
    pub(crate) fields: Vec<Option<Field>>,
    pub(crate) owner: Vec<i32>,
    pub(crate) want_x: Vec<i32>,
    pub(crate) want_y: Vec<i32>,
    pub(crate) dirty: Vec<bool>,
    pub(crate) age: Vec<u32>,
    pub(crate) seen: Vec<u32>,
    pub(crate) round: usize,
    pub(crate) tick_no: u32,
    pub budget: usize,
}

impl Pool {
    pub fn new(w: usize, h: usize, max: usize) -> Pool {
        let max = max.max(1);
        Pool {
            w,
            h,
            fields: (0..max).map(|_| None).collect(),
            owner: vec![-1; max],
            want_x: vec![0; max],
            want_y: vec![0; max],
            dirty: vec![false; max],
            age: vec![0; max],
            seen: vec![0; max],
            round: 0,
            tick_no: 0,
            budget: DEFAULT_TICK_BUDGET,
        }
    }

    /// Returns the slot routing to (gx, gy). Requests within `MOVE_THRESHOLD` cells of a held
    /// destination share its field, whoever asks; otherwise a slot is claimed for the destination.
    /// `id` only marks the slot as owned.
    pub fn request(&mut self, id: i32, gx: i32, gy: i32) -> usize {
        let held = (0..self.owner.len()).find(|&i| self.owner[i] >= 0 && !drifted(self.want_x[i], self.want_y[i], gx, gy));
        if let Some(i) = held {
            self.seen[i] = self.tick_no;
            let stale = match &self.fields[i] {
                Some(f) => !f.is_ready() || self.age[i] >= REBUILD_CADENCE,
                None => true,
            };
            if stale {
                self.dirty[i] = true;
            }
            return i;
        }
        let (i, evicted) = self.claim(id);
        self.want_x[i] = gx;
        self.want_y[i] = gy;
        self.seen[i] = self.tick_no;
        self.age[i] = 0;
        self.dirty[i] = true;
        if self.fields[i].is_none() {
            self.fields[i] = Some(Field::new(self.w, self.h));
        } else if evicted {
            if let Some(f) = self.fields[i].as_mut() {
                f.discard();
            }
        }
        i
    }

    pub fn field(&self, slot: usize) -> &Field {
        self.fields[slot].as_ref().expect("a claimed slot always holds a field")
    }

    fn claim(&mut self, id: i32) -> (usize, bool) {
        if let Some(i) = self.owner.iter().position(|&o| o < 0) {
            self.owner[i] = id;
            return (i, false);
        }
        let mut oldest = 0;
        for i in 1..self.seen.len() {
            if self.seen[i] < self.seen[oldest] {
                oldest = i;
            }
        }
        self.owner[oldest] = id;
        (oldest, true)
    }

    /// Ages every slot and advances one rebuild by `budget` cells. Call once per tick.
    pub fn tick(&mut self, g: &Grid) -> usize {
        self.tick_no = self.tick_no.wrapping_add(1);
        let n = self.owner.len();
        for i in 0..n {
            if self.owner[i] < 0 {
                continue;
            }
            self.age[i] = self.age[i].wrapping_add(1);
            if self.age[i] >= REBUILD_CADENCE {
                self.dirty[i] = true;
            }
        }
        let mut spent = 0;
        for fresh in [true, false] {
            for k in 0..n {
                let i = (self.round + k) % n;
                if self.owner[i] < 0 || !self.dirty[i] {
                    continue;
                }
                let unserved = self.fields[i].as_ref().map_or(true, |f| !f.is_ready());
                if unserved != fresh {
                    continue;
                }
                self.round = i + 1;
                let (wx, wy) = (self.want_x[i], self.want_y[i]);
                let budget = if fresh { FIRST_BUILD_BUDGET } else { self.budget };
                let f = match self.fields[i].as_mut() {
                    Some(f) => f,
                    None => {
                        self.dirty[i] = false;
                        continue;
                    }
                };
                if !f.is_building() && !f.reset(g, wx, wy) {
                    self.dirty[i] = false;
                    continue;
                }
                if f.step(g, budget) {
                    self.dirty[i] = false;
                    self.age[i] = 0;
                }
                spent += budget;
                break;
            }
        }
        spent
    }

    /// Drops every solved field so each destination is re-solved against the current grid.
    /// Units routed on a dropped field steer straight until its rebuild lands.
    pub fn invalidate(&mut self) {
        for i in 0..self.owner.len() {
            if self.owner[i] >= 0 {
                self.dirty[i] = true;
                if let Some(f) = self.fields[i].as_mut() {
                    f.discard();
                }
            }
        }
    }

    pub fn live(&self) -> usize {
        self.owner.iter().filter(|&&o| o >= 0).count()
    }

    pub fn ready_count(&self) -> usize {
        self.fields.iter().flatten().filter(|f| f.is_ready()).count()
    }

    pub fn cap(&self) -> usize {
        self.owner.len()
    }
}

fn drifted(ox: i32, oy: i32, nx: i32, ny: i32) -> bool {
    (ox - nx).abs() > MOVE_THRESHOLD || (oy - ny).abs() > MOVE_THRESHOLD
}
