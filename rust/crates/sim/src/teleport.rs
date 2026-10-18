//! The `teleport` effect: the caster lands beside a friendly receiver. The receiver is the cast target,
//! so a channelled ability with a unit target and this effect is a town portal scroll. p0 is the landing
//! distance from the receiver in milli-units; the caster takes the nearest walkable spot on a ring of that
//! radius, widening the ring while every spot is blocked.

use crate::arrays::Sim;
use crate::fixed;
use crate::loader::Effect;
use crate::rules::Refusal;

pub(crate) const K_TELEPORT: u16 = 31;
const RING_POINTS: i64 = 8;
const RING_STEPS: i64 = 6;
/// Unit vectors of the eight landing directions, scaled by 1000.
const DIRECTIONS: [(i64, i64); 8] = [(1000, 0), (707, 707), (0, 1000), (-707, 707), (-1000, 0), (-707, -707), (0, -1000), (707, -707)];

impl Sim {
    pub(crate) fn teleport_effect(&mut self, caster: usize, eff: Effect, receiver: Option<usize>) {
        let Some(j) = receiver.filter(|&j| self.hostile(self.player[caster], self.player[j]) == false) else {
            self.refuse(Refusal::NoTarget);
            return;
        };
        if self.rooted(caster) {
            self.refuse(Refusal::Rooted);
            return;
        }
        let (jx, jy) = (self.pos_x[j] as i64, self.pos_y[j] as i64);
        let base = i64::from(fixed::from_milli(eff.p0.max(1)));
        for step in 1..=RING_STEPS {
            for (dx, dy) in DIRECTIONS.iter().take(RING_POINTS as usize) {
                let reach = base * step;
                let (x, y) = (jx + dx * reach / 1000, jy + dy * reach / 1000);
                if self.teleport(caster, x as i32, y as i32) {
                    return;
                }
            }
        }
        self.refuse(Refusal::NoTeleport);
    }
}
