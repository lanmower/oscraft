use crate::arrays::Sim;

#[derive(Clone, Debug)]
pub(crate) struct ExpiryState {
    pub(crate) expires: Vec<u32>,
    pub(crate) casting_slot: i32,
}

impl ExpiryState {
    pub(crate) fn new(cap: usize) -> ExpiryState {
        ExpiryState { expires: vec![0; cap], casting_slot: -1 }
    }
}

pub(crate) const SPAWN_UNIT_PER_RANK: i32 = 1 << 30;
pub(crate) const SPAWN_RING: i32 = 1 << 29;
const RING_DIRECTIONS: [(i32, i32); 8] = [(5, 0), (4, 4), (0, 5), (-4, 4), (-5, 0), (-4, -4), (0, -5), (4, -4)];

impl Sim {
    pub fn timed_left(&self, i: usize) -> u32 {
        if i >= self.n || self.alive[i] == 0 {
            return 0;
        }
        self.expiry.expires[i].saturating_sub(self.ticks)
    }

    pub fn is_summoned(&self, i: usize) -> bool {
        i < self.n && (self.expiry.expires[i] != 0 || self.is_illusion(i))
    }

    pub fn timed_spawn_effect(&mut self, caster: usize, kind: i32, life: i32, x: i32, y: i32) {
        self.timed_spawn_group(caster, kind, life, 1, 0, x, y);
    }

    pub(crate) fn timed_spawn_group(&mut self, caster: usize, kind: i32, life: i32, count: i32, spacing_milli: i32, x: i32, y: i32) {
        let p = self.player[caster];
        let ring = spacing_milli & SPAWN_RING != 0;
        let spacing_milli = spacing_milli & !SPAWN_RING;
        let count = count.clamp(1, if ring { 8 } else { 32 });
        let cols = (1..=count).find(|c| c * c >= count).unwrap_or(1);
        let rows = (count + cols - 1) / cols;
        let step = crate::fixed::from_milli(spacing_milli);
        for k in 0..count {
            let (col, row) = (k % cols, k / cols);
            let (dx, dy) = if ring {
                let (ux, uy) = RING_DIRECTIONS[(k * 8 / count) as usize];
                ((step as i64 * ux as i64 / 5) as i32, (step as i64 * uy as i64 / 5) as i32)
            } else {
                (((2 * col - (cols - 1)) as i64 * step as i64 / 2) as i32, ((2 * row - (rows - 1)) as i64 * step as i64 / 2) as i32)
            };
            let Some(j) = self.spawn_for(p, kind as u32, x.wrapping_add(dx), y.wrapping_add(dy)) else {
                continue;
            };
            self.stats.summoned += 1;
            self.expiry.expires[j] = self.ticks.saturating_add(life.max(1) as u32);
        }
    }

    pub(crate) fn tick_timed_life(&mut self) {
        self.expire_shields();
        for i in 0..self.n {
            let at = self.expiry.expires[i];
            if at != 0 && self.alive[i] != 0 && self.ticks >= at {
                self.hp[i] = 0;
                self.alive[i] = 0;
                self.touch(i);
            }
        }
    }
}
