//! The `teleport` effect: the caster lands beside a friendly receiver. The receiver is the cast target,
//! so a channelled ability with a unit target and this effect is a town portal scroll. p0 is the landing
//! distance from the receiver in milli-units; the caster takes the nearest walkable spot on a ring of that
//! radius, widening the ring while every spot is blocked.

use crate::arrays::Sim;
use crate::fixed;
use crate::loader::Effect;
use crate::rules::Refusal;

pub(crate) const K_TELEPORT: u16 = 31;
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
        let (points, steps) = (self.cat.tune.teleport_ring_points as usize, i64::from(self.cat.tune.teleport_ring_steps));
        for step in 1..=steps {
            for (dx, dy) in DIRECTIONS.iter().take(points) {
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

#[cfg(test)]
mod tests {
    use crate::load;

    /// The landing ring the moba scenario's TUNE section carries: 8 points, 6 widening steps, as the
    /// teleport effect had them as constants.
    #[test]
    fn teleport_ring_loads_from_data_and_matches_old_constants() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("moba");
        let image = osdt_data::generate(&scenario).expect("moba image");
        let s = load(&image, 1024).expect("moba sim");
        assert_eq!(s.catalog().tune.teleport_ring_points, 8);
        assert_eq!(s.catalog().tune.teleport_ring_steps, 6);
    }
}
