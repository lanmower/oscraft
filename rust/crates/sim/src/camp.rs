//! Spawn groups: a generic primitive. A unit the map places for the neutral player is a group member
//! whose spawn point is its home. A member past its leash radius walks home. A dead member waits until
//! its respawn timer is set (`set_respawn`, a `cmd_set_respawn` from a mod), then respawns at its home.
//! Which units form a group, when each one is due, what a death pays and the pit swap are mod code
//! (scenarios/moba/camp.lua). Home and timer are sim state: hashed and saved. The leash radius and the
//! reincarnation item are load-time mod settings and are neither hashed nor saved.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::ORDER_MOVE;

/// Per-unit spawn group state, sized to the unit cap.
#[derive(Clone, Debug)]
pub(crate) struct CampState {
    /// 1 for a spawn-group member (set at load for neutral placements).
    pub(crate) member: Vec<u8>,
    /// Home point of each member, Q16.16 sim coordinates. A member walks back here past its leash.
    pub(crate) anchor_x: Vec<i32>,
    pub(crate) anchor_y: Vec<i32>,
    /// 1 while a member is dead and held for a respawn decision; the unit respawns once `revive_at` passes.
    pub(crate) waiting: Vec<u8>,
    pub(crate) revive_at: Vec<u32>,
    /// Leash radius per member, milli-units. 0 means no leash. Load-time setting, not hashed or saved.
    pub(crate) leash_milli: Vec<i32>,
    /// Item that lets a dying hero reincarnate: (item kind, seconds the hero is gone). Load-time setting.
    pub(crate) reincarnate: Option<(u32, u32)>,
}

impl CampState {
    pub(crate) fn new(cap: usize) -> CampState {
        CampState {
            member: vec![0; cap],
            anchor_x: vec![0; cap],
            anchor_y: vec![0; cap],
            waiting: vec![0; cap],
            revive_at: vec![0; cap],
            leash_milli: vec![0; cap],
            reincarnate: None,
        }
    }

    /// Forgets any group role of slot `i`; a slot reused by another spawn is not a member.
    pub(crate) fn clear(&mut self, i: usize) {
        if let Some(v) = self.member.get_mut(i) {
            *v = 0;
            self.waiting[i] = 0;
        }
    }
}

impl Sim {
    /// Marks live unit `i` as a group member whose home is its current point.
    pub(crate) fn mark_camp(&mut self, i: usize) {
        let c = &mut self.camps;
        c.member[i] = 1;
        c.anchor_x[i] = self.pos_x[i];
        c.anchor_y[i] = self.pos_y[i];
    }

    /// Books a member's death: it waits for a respawn decision and does not respawn by itself.
    pub(crate) fn group_died(&mut self, j: usize) {
        if self.camps.member[j] == 0 {
            return;
        }
        self.camps.waiting[j] = 1;
        self.camps.revive_at[j] = u32::MAX;
    }

    /// Per tick: each live member past its leash is ordered home, and each waiting member whose timer has run
    /// out respawns at its home. A respawn blocked by a full home cell retries next tick.
    pub(crate) fn tick_camps(&mut self) {
        let mut due: Vec<usize> = Vec::new();
        for i in 0..self.n {
            if self.camps.member[i] == 0 {
                continue;
            }
            if self.camps.waiting[i] != 0 {
                if self.ticks >= self.camps.revive_at[i] {
                    due.push(i);
                }
                continue;
            }
            if self.alive[i] == 0 || self.camps.leash_milli[i] <= 0 {
                continue;
            }
            let leash = fixed::from_milli(self.camps.leash_milli[i]) as i64;
            let (hx, hy) = (self.camps.anchor_x[i], self.camps.anchor_y[i]);
            if dist2(self.pos_x[i], self.pos_y[i], hx, hy) > leash * leash {
                let _ = self.issue_order_kind(i, ORDER_MOVE, hx, hy);
            }
        }
        for i in due {
            self.respawn_group(i);
        }
    }

    fn respawn_group(&mut self, i: usize) {
        let kind = self.type_of[i] as u32;
        let player = self.player[i];
        let (x, y) = (self.camps.anchor_x[i], self.camps.anchor_y[i]);
        if !self.spawn_at(i, kind, player, x, y) {
            return;
        }
        self.camps.member[i] = 1;
        self.camps.waiting[i] = 0;
    }
}

impl Sim {
    /// Whether slot `i` is a spawn-group member (a placed unit the map marked, or a mod marked).
    pub fn is_spawn_group_unit(&self, i: usize) -> bool {
        self.camps.member.get(i).is_some_and(|&v| v != 0)
    }

    /// Sets the respawn timer of dead spawn-group member `i`: `Some(s)` respawns it `s` seconds from now, `None`
    /// leaves it dead (`Some(0)` respawns it at the next tick). A member that is not dead, or a unit outside a
    /// spawn group, is refused. A hero goes to the hero respawn.
    pub fn set_respawn(&mut self, i: usize, delay_s: Option<u32>) -> Result<(), &'static str> {
        if self.is_hero(i) {
            return self.set_hero_respawn(i, delay_s);
        }
        if !self.is_spawn_group_unit(i) || self.alive(i) {
            return Err("not a dead spawn-group unit");
        }
        self.camps.waiting[i] = 1;
        self.camps.revive_at[i] = match delay_s {
            Some(s) => self.ticks.wrapping_add(s.saturating_mul(crate::TICKS_PER_SECOND as u32)),
            None => u32::MAX,
        };
        Ok(())
    }

    /// Sets the home point of spawn-group member `i` (Q16.16 sim coordinates). A dead member takes it at its respawn.
    pub fn set_group_home(&mut self, i: usize, x: i32, y: i32) -> Result<(), &'static str> {
        if !self.is_spawn_group_unit(i) {
            return Err("not a spawn-group unit");
        }
        self.camps.anchor_x[i] = x;
        self.camps.anchor_y[i] = y;
        Ok(())
    }

    /// Sets the leash radius (milli-units, 0 for none) of every spawn-group member. A load-time mod setting.
    pub fn set_group_leash(&mut self, milli: u32) {
        for i in 0..self.n {
            if self.camps.member[i] != 0 {
                self.camps.leash_milli[i] = milli as i32;
            }
        }
    }

    /// Names the item that lets a dying hero reincarnate and how long the hero is gone. A load-time mod setting;
    /// an unknown item is refused.
    pub fn set_reincarnate_item(&mut self, name: &str, seconds: u32) -> Result<(), &'static str> {
        let k = self.cat.items.iter().position(|it| self.cat.name(it.name) == name).ok_or("unknown item")?;
        self.camps.reincarnate = Some((crate::inventory::item_kind(k), seconds));
        Ok(())
    }
}

#[cfg(test)]
mod respawn_decision_tests {
    use crate::fixed;

    // A respawn decision sets the dead spawn-group unit's timer; None leaves it dead; a live or
    // non-group unit is refused and its timer is untouched.
    #[test]
    fn set_respawn_sets_the_timer_and_refuses_units_outside_a_dead_group() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let image = osdt_data::generate(&scenario).expect("duel-routes image");
        let mut sim = crate::load(&image, 64).expect("duel-routes sim");
        let kind = sim.catalog().unit_index("creep").expect("creep") as u32;
        let u = sim.spawn_for(0, kind, fixed::from_int(5), fixed::from_int(5)).expect("unit");
        assert!(sim.set_respawn(u, Some(3)).is_err(), "a unit outside a spawn group is refused");
        sim.camps.member[u] = 1;
        assert!(sim.set_respawn(u, Some(3)).is_err(), "a live spawn-group unit is refused");
        sim.remove_unit(u);
        sim.set_respawn(u, Some(3)).expect("dead group unit");
        assert_eq!(sim.camps.revive_at[u], sim.ticks().wrapping_add(3 * crate::TICKS_PER_SECOND as u32));
        sim.set_respawn(u, None).expect("leave dead");
        assert_eq!(sim.camps.revive_at[u], u32::MAX);
    }
}
