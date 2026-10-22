//! Per-player fog on a grid of VIS_SCALE-wide cells, rebuilt every VIS_PERIOD
//! ticks. Night sight is the unit's sight divided by the pack's `night_sight_divisor`.

use crate::arrays::{cell_of, Sim};
use crate::fixed;
use crate::*;
use std::sync::atomic::{AtomicU32, Ordering};

// Process-wide, so a replaced Sim never reuses a generation a client already saw.
static VISION_GENERATION: AtomicU32 = AtomicU32::new(0);

/// Marks every player's visible plane as possibly changed. Called wherever `vis`, `vision` or `fog` change.
pub(crate) fn bump_vision_generation() {
    VISION_GENERATION.fetch_add(1, Ordering::Relaxed);
}

impl Sim {
    /// Changes whenever any visible plane may have changed. Equal values mean `vision_plane` would return the same bytes.
    pub fn vision_generation(&self) -> u32 {
        VISION_GENERATION.load(Ordering::Relaxed)
    }

    /// The divisor of a unit's sight at night, from the pack's rules (`night_sight_divisor`); 1, no reduction, when unset.
    fn night_sight_divisor(&self) -> i32 {
        self.rule("night_sight_divisor").unwrap_or(1).max(1)
    }

    pub(crate) fn refresh_vision(&mut self) {
        self.stats.vision_refreshes += 1;
        bump_vision_generation();
        let vc = self.vcells;
        let night = self.is_night();
        let night_div = self.night_sight_divisor();
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
            let r = if night { self.sight_cells[i] / night_div } else { self.sight_cells[i] };
            if r <= 0 {
                continue;
            }
            let rc = ((r + VIS_SCALE - 1) / VIS_SCALE).max(1);
            let ccx = cell_of(self.pos_x[i]) / VIS_SCALE;
            let ccy = cell_of(self.pos_y[i]) / VIS_SCALE;
            // Elevation: a unit sees a cell only at or below its own height. Higher units see
            // over elevated cells; a unit on lower ground is denied the cells above it.
            let unit_elev = self.grid.elevation_milli(cell_of(self.pos_x[i]), cell_of(self.pos_y[i]));
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
                    if self.grid.elevation_milli(x * VIS_SCALE, y * VIS_SCALE) <= unit_elev {
                        self.vis[row + x as usize] = 1;
                    }
                }
            }
        }
        self.remember_seen();
    }

    /// Records, for each seeing player, the position of every enemy unit it can see now. A
    /// remembered position stays until that unit is seen again. Fixed arrays, no allocation.
    fn remember_seen(&mut self) {
        let cap = self.cap;
        for i in 0..self.n {
            if self.alive[i] == 0 {
                continue;
            }
            for p in 0..MAX_PLAYERS {
                if !self.has_seer[p] || !self.hostile(p as u8, self.player[i]) || !self.sees(p as u8, i) {
                    continue;
                }
                let k = p * cap + i;
                self.seen[k] = 1;
                self.seen_x[k] = self.pos_x[i];
                self.seen_y[k] = self.pos_y[i];
            }
        }
    }

    /// The position player `p` last saw unit `i` at, or None if it never has.
    pub fn last_seen(&self, p: u8, i: usize) -> Option<(i32, i32)> {
        if (p as usize) >= MAX_PLAYERS || i >= self.cap {
            return None;
        }
        let k = p as usize * self.cap + i;
        (self.seen[k] != 0).then(|| (self.seen_x[k], self.seen_y[k]))
    }

    /// Player `p`'s visible cells (own plus shared allies) as one byte per cell, row-major over
    /// `vision_dims()`. With the fog off every cell reads visible.
    pub fn vision_plane(&self, p: u8) -> Vec<u8> {
        if !self.fog || (p as usize) >= MAX_PLAYERS {
            return vec![1; self.vcells];
        }
        let shared = self.vision[p as usize];
        (0..self.vcells)
            .map(|at| {
                let on = self.vis[p as usize * self.vcells + at] != 0
                    || (0..MAX_PLAYERS).any(|q| shared & (1 << q) != 0 && self.vis[q * self.vcells + at] != 0);
                on as u8
            })
            .collect()
    }

    /// Vision grid width, height and cell size in tiles.
    pub fn vision_dims(&self) -> (i32, i32, i32) {
        (self.vw, self.vh, VIS_SCALE)
    }

    /// True when the player can see unit `i`. A player always sees its own units.
    pub fn visible(&self, p: u8, i: usize) -> bool {
        if i >= self.n {
            return false;
        }
        if self.player[i] == p {
            return true;
        }
        if self.hidden_by_invisibility(p, i) {
            return false;
        }
        !self.fog || self.sees(p, i)
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
        let at = (cy * self.vw + cx) as usize;
        let shared = self.vision[p as usize];
        self.vis[p as usize * self.vcells + at] != 0
            || (0..MAX_PLAYERS).any(|q| shared & (1 << q) != 0 && self.vis[q * self.vcells + at] != 0)
    }
}
