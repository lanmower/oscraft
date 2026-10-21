//! Wave upgrades. A spawn effect whose p3 carries a wave spec (bits 0..30; bits 30..32 are the route id of route.rs) names alternative unit kinds that replace the
//! base kind once enemy structures carrying a filter tag are gone. Bits of p3: 0..10 upgraded kind
//! plus one, 10..20 final kind plus one, 20..25 counterpart tag filter bit plus one, 25..30 global tag
//! filter bit plus one. The upgraded kind applies when the enemy structure with the counterpart tag that
//! mirrors the spawn point (a point reflection of the map) is dead; the final kind applies when
//! every enemy structure with the global tag is dead. Dead units keep their slot and position, so
//! the mirror lookup also sees destroyed structures.

use crate::arrays::{dist2, Sim};
use crate::fixed;

/// Bits of a spawn p3 that the wave spec uses; the top two bits belong to the route id.
pub const SPEC_MASK: i32 = 0x3FFF_FFFF;
pub const SPEC_KIND_BITS: u32 = 10;
pub const SPEC_KIND_MASK: i32 = (1 << SPEC_KIND_BITS) - 1;
pub const SPEC_TAG_BITS: u32 = 5;
pub const SPEC_TAG_MASK: i32 = (1 << SPEC_TAG_BITS) - 1;

impl Sim {
    /// The kind a wave unit of `base` spawns as for `player` at (x, y) under spawn spec `spec`.
    pub(crate) fn wave_kind(&self, player: u8, base: u32, spec: i32, x: i32, y: i32) -> u32 {
        let spec = spec & SPEC_MASK;
        if spec == 0 {
            return base;
        }
        let upgraded = spec & SPEC_KIND_MASK;
        let finalk = (spec >> SPEC_KIND_BITS) & SPEC_KIND_MASK;
        let counter_tag = (spec >> (2 * SPEC_KIND_BITS)) & SPEC_TAG_MASK;
        let global_tag = (spec >> (2 * SPEC_KIND_BITS + SPEC_TAG_BITS)) & SPEC_TAG_MASK;
        if finalk != 0 && global_tag != 0 && self.side_cleared(player, 1u32 << (global_tag - 1)) {
            return (finalk - 1) as u32;
        }
        if upgraded != 0 && counter_tag != 0 && self.counterpart_dead(player, 1u32 << (counter_tag - 1), x, y) {
            return (upgraded - 1) as u32;
        }
        base
    }

    /// True when at least one hostile structure carries `bit` and none of them lives.
    fn side_cleared(&self, player: u8, bit: u32) -> bool {
        let mut any = false;
        for k in 0..self.n {
            if self.filter[k] & bit == 0 || !self.hostile(player, self.player[k]) {
                continue;
            }
            if self.alive[k] != 0 {
                return false;
            }
            any = true;
        }
        any
    }

    /// True when the hostile unit carrying `bit` nearest the point reflection of (x, y) is dead.
    fn counterpart_dead(&self, player: u8, bit: u32, x: i32, y: i32) -> bool {
        let mx = fixed::from_int(self.grid.w as i32).wrapping_sub(x);
        let my = fixed::from_int(self.grid.h as i32).wrapping_sub(y);
        let mut best: Option<(usize, i64)> = None;
        for k in 0..self.n {
            if self.filter[k] & bit == 0 || !self.hostile(player, self.player[k]) {
                continue;
            }
            let d = dist2(mx, my, self.pos_x[k], self.pos_y[k]);
            if best.map_or(true, |(_, b)| d < b) {
                best = Some((k, d));
            }
        }
        best.map_or(false, |(k, _)| self.alive[k] == 0)
    }
}
