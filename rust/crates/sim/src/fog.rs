//! Per-player fog on a grid of VIS_SCALE-wide cells, rebuilt every VIS_PERIOD
//! ticks, plus script reveal rectangles stamped on top of every rebuild.

use crate::arrays::{cell_of, Reveal, Sim};
use crate::fixed;
use crate::*;

impl Sim {
    pub fn reveal_area(&mut self, p: u8, x0: i32, y0: i32, x1: i32, y1: i32, until: i32) {
        if (p as usize) >= MAX_PLAYERS {
            return;
        }
        let (x0, x1) = if x1 < x0 { (x1, x0) } else { (x0, x1) };
        let (y0, y1) = if y1 < y0 { (y1, y0) } else { (y0, y1) };
        for k in 0..REVEAL_CAP {
            if !self.reveals[k].live {
                self.reveals[k] = Reveal { p, x0, y0, x1, y1, until, live: true };
                self.has_seer[p as usize] = true;
                return;
            }
        }
        self.stats.reveals_dropped += 1;
    }

    pub(crate) fn refresh_vision(&mut self) {
        self.stats.vision_refreshes += 1;
        let vc = self.vcells;
        for p in 0..MAX_PLAYERS {
            if self.has_seer[p] {
                self.vis[p * vc..(p + 1) * vc].fill(0);
            }
        }
        for i in 0..self.n {
            if self.alive[i] == 0 {
                continue;
            }
            let p = self.player[i] as usize;
            if p >= MAX_PLAYERS {
                continue;
            }
            let r = self.sight_cells[i];
            if r <= 0 {
                continue;
            }
            let rc = ((r + VIS_SCALE - 1) / VIS_SCALE).max(1);
            let ccx = cell_of(self.pos_x[i]) / VIS_SCALE;
            let ccy = cell_of(self.pos_y[i]) / VIS_SCALE;
            let base = p * vc;
            for dy in -rc..=rc {
                let y = ccy + dy;
                if y < 0 || y >= self.vh {
                    continue;
                }
                let dx = fixed::isqrt(rc * rc - dy * dy);
                let x0 = (ccx - dx).max(0);
                let x1 = (ccx + dx).min(self.vw - 1);
                if x0 > x1 {
                    continue;
                }
                let row = base + (y * self.vw) as usize;
                for x in x0..=x1 {
                    self.vis[row + x as usize] = 1;
                }
            }
        }
        self.stamp_reveals();
    }

    fn stamp_reveals(&mut self) {
        for k in 0..REVEAL_CAP {
            let r = self.reveals[k];
            if !r.live {
                continue;
            }
            if r.until != 0 && self.ticks as i32 >= r.until {
                self.reveals[k].live = false;
                continue;
            }
            let p = r.p as usize;
            if p >= MAX_PLAYERS {
                self.reveals[k].live = false;
                continue;
            }
            let base = p * self.vcells;
            let cx0 = clamp_cell(fixed::to_int(r.x0) / VIS_SCALE, self.vw);
            let cx1 = clamp_cell(fixed::to_int(r.x1) / VIS_SCALE, self.vw);
            let cy0 = clamp_cell(fixed::to_int(r.y0) / VIS_SCALE, self.vh);
            let cy1 = clamp_cell(fixed::to_int(r.y1) / VIS_SCALE, self.vh);
            for y in cy0..=cy1 {
                let row = base + (y * self.vw) as usize;
                for x in cx0..=cx1 {
                    self.vis[row + x as usize] = 1;
                }
            }
        }
    }

    /// True when the player can see unit `i`. A player always sees its own units.
    pub fn visible(&self, p: u8, i: usize) -> bool {
        if i >= self.n {
            return false;
        }
        if !self.fog || self.player[i] == p {
            return true;
        }
        self.sees(p, i)
    }

    fn sees(&self, p: u8, i: usize) -> bool {
        if (p as usize) >= MAX_PLAYERS {
            return false;
        }
        let cx = cell_of(self.pos_x[i]) / VIS_SCALE;
        let cy = cell_of(self.pos_y[i]) / VIS_SCALE;
        if cx < 0 || cy < 0 || cx >= self.vw || cy >= self.vh {
            return false;
        }
        self.vis[p as usize * self.vcells + (cy * self.vw + cx) as usize] != 0
    }
}

fn clamp_cell(v: i32, n: i32) -> i32 {
    v.clamp(0, n - 1)
}
