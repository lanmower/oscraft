//! Spawn groups: a generic primitive. A unit the map places for the neutral player is a group member
//! whose spawn point is its home. A member past its leash radius walks home. A dead member waits until
//! its respawn timer is set (`set_respawn`, a `cmd_set_respawn` from a mod), then respawns at its home.
//! Which units form a group, when each one is due, what a death pays and the pit swap are mod code
//! (the pack's spawn-group script). Home and timer are sim state: hashed and saved. The leash radius and the
//! reincarnation item are load-time mod settings and are neither hashed nor saved.

use crate::arrays::Sim;



/// Per-unit spawn group state, sized to the unit cap.
#[derive(Clone, Debug)]
pub(crate) struct GroupState {
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
    /// Item that lets a dying unit reincarnate: (item kind, seconds the unit is gone). Load-time setting.
    pub(crate) revive_item: Option<(u32, u32)>,
}

impl GroupState {
    pub(crate) fn new(cap: usize) -> GroupState {
        GroupState {
            member: vec![0; cap],
            anchor_x: vec![0; cap],
            anchor_y: vec![0; cap],
            waiting: vec![0; cap],
            revive_at: vec![0; cap],
            leash_milli: vec![0; cap],
            revive_item: None,
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
    pub(crate) fn mark_group(&mut self, i: usize) {
        let c = &mut self.groups;
        c.member[i] = 1;
        c.anchor_x[i] = self.pos_x[i];
        c.anchor_y[i] = self.pos_y[i];
    }

    /// Books a member's death: it waits for a respawn decision and does not respawn by itself.
    pub(crate) fn group_died(&mut self, j: usize) {
        if self.groups.member[j] == 0 {
            return;
        }
        self.groups.waiting[j] = 1;
        self.groups.revive_at[j] = u32::MAX;
    }

}

impl Sim {
    /// Whether slot `i` is a spawn-group member (a placed unit the map marked, or a mod marked).
    pub fn is_spawn_group_unit(&self, i: usize) -> bool {
        self.groups.member.get(i).is_some_and(|&v| v != 0)
    }

    /// Sets the respawn timer of dead spawn-group member `i`: `Some(s)` respawns it `s` seconds from now, `None`
    /// leaves it dead (`Some(0)` respawns it at the next tick). A member that is not dead, or a unit outside a
    /// spawn group, is refused. A unit goes to the unit respawn.
    pub fn set_respawn(&mut self, i: usize, delay_s: Option<u32>) -> Result<(), &'static str> {
        if self.is_attributed(i) {
            return self.set_attributed_respawn(i, delay_s);
        }
        if !self.is_spawn_group_unit(i) || self.alive(i) {
            return Err("not a dead spawn-group unit");
        }
        self.groups.waiting[i] = 1;
        self.groups.revive_at[i] = match delay_s {
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
        self.groups.anchor_x[i] = x;
        self.groups.anchor_y[i] = y;
        Ok(())
    }

    /// Sets the leash radius (milli-units, 0 for none) of every spawn-group member. A load-time mod setting.
    pub fn set_group_leash(&mut self, milli: u32) {
        for i in 0..self.n {
            if self.groups.member[i] != 0 {
                self.groups.leash_milli[i] = milli as i32;
            }
        }
    }

    /// Names the item that lets a dying unit reincarnate and how long the unit is gone. A load-time mod setting;
    /// an unknown item is refused.
    pub fn set_reincarnate_item(&mut self, name: &str, seconds: u32) -> Result<(), &'static str> {
        let k = self.cat.items.iter().position(|it| self.cat.name(it.name) == name).ok_or("unknown item")?;
        self.groups.revive_item = Some((crate::inventory::item_kind(k), seconds));
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
        sim.groups.member[u] = 1;
        assert!(sim.set_respawn(u, Some(3)).is_err(), "a live spawn-group unit is refused");
        sim.remove_unit(u);
        sim.set_respawn(u, Some(3)).expect("dead group unit");
        assert_eq!(sim.groups.revive_at[u], sim.ticks().wrapping_add(3 * crate::TICKS_PER_SECOND as u32));
        sim.set_respawn(u, None).expect("leave dead");
        assert_eq!(sim.groups.revive_at[u], u32::MAX);
    }
}
