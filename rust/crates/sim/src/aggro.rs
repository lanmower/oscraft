use crate::arrays::{dist2, Sim};
use crate::loader::FILTER_STRUCTURE;
use crate::TICKS_PER_SECOND;

#[derive(Clone, Debug)]
pub(crate) struct AggroState {
    pub(crate) target: Vec<i32>,
    pub(crate) until: Vec<u32>,
    pub(crate) invuln_until: Vec<u32>,
    pub(crate) reduce_pct: Vec<u8>,
    pub(crate) heal: Vec<i32>,
    pub(crate) fired: Vec<Fired>,
}

pub(crate) const FIRED_CAP: usize = 1024;

#[derive(Clone, Copy, Debug)]
pub struct Fired {
    pub unit: u32,
    pub trigger: u8,
    pub action: u8,
    pub ability: i32,
    pub slot: i32,
    pub target_kind: u8,
}

impl Fired {
    pub fn trigger_name(&self) -> &'static str {
        crate::loader::Trigger::from_code(self.trigger).map_or("", |t| t.name())
    }

    pub fn action_name(&self) -> &'static str {
        match self.action {
            crate::ACTION_FLEE => "flee",
            crate::ACTION_USE_ABILITY => "use_ability",
            _ => "",
        }
    }
}

impl AggroState {
    pub(crate) fn new(cap: usize) -> AggroState {
        AggroState {
            target: vec![-1; cap],
            until: vec![0; cap],
            invuln_until: vec![0; cap],
            reduce_pct: vec![0; cap],
            heal: vec![0; cap],
            fired: Vec::new(),
        }
    }
}

impl Sim {
    pub(crate) fn aggro_clear(&mut self, i: usize) {
        self.aggro.target[i] = -1;
        self.aggro.until[i] = 0;
        self.aggro.invuln_until[i] = 0;
        self.aggro.reduce_pct[i] = 0;
        self.aggro.heal[i] = 0;
    }

    pub fn aggro_target(&self, i: usize) -> i32 {
        if i < self.n && self.ticks < self.aggro.until[i] { self.aggro.target[i] } else { -1 }
    }

    pub fn is_structure(&self, i: usize) -> bool {
        self.filter[i] & FILTER_STRUCTURE != 0
    }

    pub(crate) fn mitigate_mech(&self, j: usize, dmg: i32) -> i32 {
        if dmg <= 0 {
            return dmg;
        }
        if self.ticks < self.aggro.invuln_until.get(j).copied().unwrap_or(0) {
            return 0;
        }
        let pct = self.aggro.reduce_pct.get(j).copied().unwrap_or(0) as i64;
        if pct > 0 {
            return (dmg as i64 * (100 - pct) / 100) as i32;
        }
        dmg
    }

    pub(crate) fn tick_healing(&mut self, i: usize) {
        let per_s = self.aggro.heal[i];
        if per_s <= 0 || self.alive[i] == 0 || self.hp[i] >= self.hp_max[i] {
            return;
        }
        let per_tick = per_s / TICKS_PER_SECOND as i32;
        self.hp[i] = self.hp[i].saturating_add(per_tick).min(self.hp_max[i]);
    }
}

impl crate::arrays::Sim {
    pub fn set_healing(&mut self, i: usize, hp_per_s: i32) -> Result<(), &'static str> {
        if i >= self.n {
            return Err("unknown unit");
        }
        if hp_per_s < 0 {
            return Err("healing out of range");
        }
        self.aggro.heal[i] = crate::fixed::from_int(hp_per_s);
        Ok(())
    }

    pub fn set_invulnerable(&mut self, i: usize, ticks: u32) -> Result<(), &'static str> {
        if i >= self.n {
            return Err("unknown unit");
        }
        self.aggro.invuln_until[i] = self.ticks.wrapping_add(ticks);
        Ok(())
    }

    pub fn set_damage_reduction(&mut self, i: usize, pct: u8) -> Result<(), &'static str> {
        if i >= self.n {
            return Err("unknown unit");
        }
        if pct > 100 {
            return Err("reduction out of range");
        }
        self.aggro.reduce_pct[i] = pct;
        Ok(())
    }

    pub fn filter_bits(&self, i: usize) -> u32 {
        self.filter.get(i).copied().unwrap_or(0)
    }

    pub fn units_near(&self, x: i32, y: i32, radius: i32) -> Vec<usize> {
        let r2 = (radius.max(0) as i64) * (radius.max(0) as i64);
        (0..self.n).filter(|&k| self.alive[k] != 0 && dist2(self.pos_x[k], self.pos_y[k], x, y) <= r2).collect()
    }

    pub fn set_aggro(&mut self, i: usize, target: usize, hold: u32) {
        self.aggro.target[i] = target as i32;
        self.aggro.until[i] = self.ticks.saturating_add(hold);
    }

    pub fn take_fired(&mut self) -> Vec<Fired> {
        std::mem::take(&mut self.aggro.fired)
    }

    pub fn set_class_override(&mut self, i: usize, class: Option<u8>) -> Result<(), &'static str> {
        if i >= self.n {
            return Err("unknown unit");
        }
        self.class_override[i] = match class {
            None => -1,
            Some(c) if c <= 3 => c as i8,
            Some(_) => return Err("class out of range"),
        };
        Ok(())
    }
}

#[cfg(test)]
mod override_tests {
    use crate::fixed;

    #[test]
    fn class_override_is_stored_and_clears() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let image = osdt_data::generate(&scenario).expect("duel-routes image");
        let mut sim = crate::load(&image, 64).expect("duel-routes sim");
        sim.set_fog(false);
        let kind = sim.catalog().unit_index("creep").expect("creep") as u32;
        let a = sim.spawn_for(0, kind, fixed::from_int(5), fixed::from_int(5)).expect("attacker");
        assert_eq!(sim.class_override_of(a), None);
        sim.set_class_override(a, Some(3)).expect("override");
        assert_eq!(sim.class_override_of(a), Some(3));
        sim.set_class_override(a, None).expect("clear");
        assert_eq!(sim.class_override_of(a), None);
        assert!(sim.set_class_override(a, Some(9)).is_err());
    }
}

impl crate::arrays::Sim {
    pub fn set_hold_override(&mut self, i: usize, hold: Option<bool>) -> Result<(), &'static str> {
        if i >= self.n {
            return Err("unknown unit");
        }
        self.hold_override[i] = match hold {
            None => -1,
            Some(true) => 1,
            Some(false) => 0,
        };
        Ok(())
    }

    pub fn class_override_of(&self, i: usize) -> Option<u8> {
        match self.class_override.get(i) {
            Some(&c) if c >= 0 => Some(c as u8),
            _ => None,
        }
    }

    pub fn hold_override_of(&self, i: usize) -> Option<bool> {
        match self.hold_override.get(i) {
            Some(&h) if h >= 0 => Some(h == 1),
            _ => None,
        }
    }
}

#[cfg(test)]
mod protection_state_tests {
    use crate::fixed;

    #[test]
    fn default_takes_full_damage_and_mod_state_changes_it() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let image = osdt_data::generate(&scenario).expect("duel-routes image");
        let mut sim = crate::load(&image, 64).expect("duel-routes sim");
        let kind = sim.catalog().unit_index("creep").expect("creep") as u32;
        let u = sim.spawn_for(0, kind, fixed::from_int(5), fixed::from_int(5)).expect("unit");
        assert_eq!(sim.mitigate_mech(u, 100), 100, "no mod state: full damage");
        sim.set_damage_reduction(u, 50).expect("reduction");
        assert_eq!(sim.mitigate_mech(u, 100), 50, "a 50 percent reduction halves the damage");
        sim.set_damage_reduction(u, 0).expect("clear");
        assert_eq!(sim.mitigate_mech(u, 100), 100, "clearing the reduction restores full damage");
        assert!(sim.set_damage_reduction(u, 101).is_err());
        sim.set_invulnerable(u, 5).expect("invulnerable");
        assert_eq!(sim.mitigate_mech(u, 100), 0, "invulnerable takes no damage");
        assert_eq!(sim.units_near(fixed::from_int(5), fixed::from_int(5), fixed::from_int(1)), vec![u]);
    }
}
