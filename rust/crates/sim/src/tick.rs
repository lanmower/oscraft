//! One simulation tick. Order: buckets, fog cadence, flow fields, buffs, projectiles,
//! then per unit upkeep, economy, routing, steering, integration and attack.

use crate::arrays::Sim;
use crate::*;

impl Sim {
    pub fn tick(&mut self) {
        self.rebucket();
        self.scan_left = RETARGET_BUDGET;
        if self.fog && self.ticks % VIS_PERIOD == 0 {
            self.refresh_vision();
        }
        self.paths.tick(&self.grid);
        self.tick_buffs();
        self.tick_healers();
        self.tick_projectiles();
        for i in 0..self.n {
            if self.alive[i] == 0 {
                continue;
            }
            if self.abil_on {
                self.tick_cooldowns(i);
            }
            if self.harvest && self.kind[i] == KIND_WORKER {
                self.harvest_step(i);
            }
            if self.produce && self.kind[i] == KIND_PRODUCER {
                self.produce_step(i);
            }
            self.route_for(i);
            self.steer(i);
            self.integrate(i);
            self.attack(i);
        }
        self.ticks = self.ticks.wrapping_add(1);
    }

    /// Counts down every ability cooldown on unit `i`.
    pub(crate) fn tick_cooldowns(&mut self, i: usize) {
        for slot in 0..ABILITY_SLOTS {
            let c = &mut self.abil_cd[i * ABILITY_SLOTS + slot];
            if *c > 0 {
                *c -= 1;
            }
        }
    }
}
