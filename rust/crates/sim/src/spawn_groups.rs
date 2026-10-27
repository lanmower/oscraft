use crate::arrays::Sim;

#[derive(Clone, Debug)]
pub(crate) struct GroupState {
    pub(crate) member: Vec<u8>,
    pub(crate) anchor_x: Vec<i32>,
    pub(crate) anchor_y: Vec<i32>,
    pub(crate) waiting: Vec<u8>,
    pub(crate) revive_at: Vec<u32>,
    pub(crate) heading: Vec<i32>,
}

impl GroupState {
    pub(crate) fn new(cap: usize) -> GroupState {
        GroupState {
            member: vec![0; cap],
            anchor_x: vec![0; cap],
            anchor_y: vec![0; cap],
            waiting: vec![0; cap],
            revive_at: vec![0; cap],
            heading: vec![-1; cap],
        }
    }

    pub(crate) fn clear(&mut self, i: usize) {
        if let Some(v) = self.member.get_mut(i) {
            *v = 0;
            self.waiting[i] = 0;
            self.heading[i] = -1;
        }
    }
}

impl Sim {
    pub(crate) fn mark_group(&mut self, i: usize) {
        let c = &mut self.groups;
        c.member[i] = 1;
        c.anchor_x[i] = self.pos_x[i];
        c.anchor_y[i] = self.pos_y[i];
    }

    pub(crate) fn group_died(&mut self, j: usize) {
        if self.groups.member[j] == 0 {
            return;
        }
        self.groups.waiting[j] = 1;
        self.groups.revive_at[j] = u32::MAX;
    }

    pub(crate) fn tick_groups(&mut self) {
        for i in 0..self.n {
            if self.groups.member[i] == 0 || self.groups.waiting[i] == 0 || self.ticks < self.groups.revive_at[i] || self.alive(i) {
                continue;
            }
            let kind = self.type_of[i];
            if kind < 0 {
                continue;
            }
            let (player, x, y) = (self.player[i], self.groups.anchor_x[i], self.groups.anchor_y[i]);
            let facing = self.groups.heading[i];
            if self.spawn_at(i, kind as u32, player, x, y) {
                self.groups.member[i] = 1;
                self.groups.waiting[i] = 0;
                self.groups.heading[i] = facing;
                if facing >= 0 {
                    self.set_heading(i, facing);
                }
            }
        }
    }
}

impl Sim {
    pub fn is_spawn_group_unit(&self, i: usize) -> bool {
        self.groups.member.get(i).is_some_and(|&v| v != 0)
    }

    pub fn set_respawn(&mut self, i: usize, delay_s: Option<u32>) -> Result<(), &'static str> {
        if self.is_attributed(i) {
            return self.set_attributed_respawn(i, delay_s);
        }
        if !self.is_spawn_group_unit(i) || self.alive(i) {
            return Err("not a dead spawn-group unit");
        }
        self.groups.waiting[i] = 1;
        self.groups.revive_at[i] = match delay_s {
            Some(s) => self.ticks.saturating_add(s.saturating_mul(crate::TICKS_PER_SECOND as u32)),
            None => u32::MAX,
        };
        Ok(())
    }

    pub fn set_respawn_kind(&mut self, i: usize, kind: u32) -> Result<(), &'static str> {
        if !self.is_spawn_group_unit(i) || self.alive(i) {
            return Err("not a dead spawn-group unit");
        }
        if kind as usize >= self.cat.units.len() {
            return Err("unknown unit kind");
        }
        self.type_of[i] = kind as i32;
        Ok(())
    }

    pub fn set_respawn_heading(&mut self, i: usize, heading: Option<i32>) -> Result<(), &'static str> {
        if !self.is_spawn_group_unit(i) {
            return Err("not a spawn-group unit");
        }
        self.groups.heading[i] = heading.map_or(-1, |h| h.rem_euclid(crate::facing::FULL_TURN));
        Ok(())
    }

    pub fn join_spawn_group(&mut self, i: usize) -> Result<(), &'static str> {
        if !self.alive(i) {
            return Err("not a live unit");
        }
        self.mark_group(i);
        Ok(())
    }

    pub fn set_group_home(&mut self, i: usize, x: i32, y: i32) -> Result<(), &'static str> {
        if !self.is_spawn_group_unit(i) {
            return Err("not a spawn-group unit");
        }
        self.groups.anchor_x[i] = x;
        self.groups.anchor_y[i] = y;
        Ok(())
    }
}

#[cfg(test)]
mod respawn_decision_tests {
    use crate::fixed;

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
        assert_eq!(sim.groups.revive_at[u], sim.ticks().saturating_add(3 * crate::TICKS_PER_SECOND as u32));
        sim.set_respawn(u, None).expect("leave dead");
        assert_eq!(sim.groups.revive_at[u], u32::MAX);
    }

    #[test]
    fn a_due_member_returns_at_its_home_and_a_held_one_stays_dead() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let image = osdt_data::generate(&scenario).expect("duel-routes image");
        let mut sim = crate::load(&image, 64).expect("duel-routes sim");
        let kind = sim.catalog().unit_index("creep").expect("creep") as u32;
        let u = sim.spawn_for(0, kind, fixed::from_int(5), fixed::from_int(5)).expect("unit");
        sim.mark_group(u);
        let home = sim.position_fixed(u);
        sim.remove_unit(u);
        sim.set_respawn(u, None).expect("held");
        sim.tick_groups();
        assert!(!sim.alive(u), "a held member stays dead");
        sim.set_respawn(u, Some(0)).expect("due");
        sim.tick_groups();
        assert!(sim.alive(u), "a due member respawns");
        assert_eq!(sim.position_fixed(u), home, "the member respawns at its home");
        assert!(sim.is_spawn_group_unit(u), "the respawned unit is a member again");
    }

    #[test]
    fn a_member_respawns_facing_the_heading_set_for_it_and_keeps_it_for_later_lives() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let image = osdt_data::generate(&scenario).expect("duel-routes image");
        let mut sim = crate::load(&image, 64).expect("duel-routes sim");
        let kind = sim.catalog().unit_index("creep").expect("creep") as u32;
        let u = sim.spawn_for(0, kind, fixed::from_int(5), fixed::from_int(5)).expect("unit");
        assert!(sim.set_respawn_heading(u, Some(1)).is_err(), "a unit outside a spawn group has no respawn heading");
        sim.mark_group(u);
        sim.set_respawn_heading(u, Some(crate::FULL_TURN / 8 + crate::FULL_TURN)).expect("heading");
        for life in 0..2 {
            sim.remove_unit(u);
            sim.set_respawn(u, Some(0)).expect("due");
            sim.tick_groups();
            assert!(sim.alive(u), "life {life}: the member respawns");
            assert_eq!(sim.heading_of(u), crate::FULL_TURN / 8, "life {life}: the member faces the set heading, wrapped into one turn");
        }
        sim.set_respawn_heading(u, None).expect("clear");
        sim.remove_unit(u);
        sim.set_respawn(u, Some(0)).expect("due");
        sim.tick_groups();
        assert_ne!(sim.heading_of(u), crate::FULL_TURN / 8, "a cleared heading leaves the default facing");
    }
}
