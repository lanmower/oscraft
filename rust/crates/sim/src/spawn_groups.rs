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

