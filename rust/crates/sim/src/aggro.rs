//! Unit aggro and the per-unit protection and healing state a mod sets. Roles come from unit filters,
//! never from names: a structure is a unit carrying the structure filter, a unit is `Sim::is_attributed`,
//! anything else is a plain unit.
//!
//! The target-priority order, the aggro redirect an armed structure takes when an allied unit is hit, and the
//! wave-creep acquire and chase are pack rules (scenarios/moba/targeting.lua and lanes.lua). The sim keeps only
//! their state: the class override, the aggro target and its hold, the chase storage and the fired behaviours.
//!
//! Damage reduction, invulnerability and healing are per-unit state a mod sets (`set_damage_reduction`,
//! `set_invulnerable`, `set_healing`). The engine applies none of them by default. Any rule that decides
//! when a unit is protected or healed (a structure's guard, a timed shield, a team's team aura) is mod code
//! that reads `units_near` or `filter_bits` and sets this state.
//!
//! Aggro and this per-unit state are sim state: hashed and saved with the rest.

use crate::arrays::{dist2, Sim};
use crate::loader::FILTER_STRUCTURE;
use crate::TICKS_PER_SECOND;

#[derive(Clone, Debug)]
pub(crate) struct AggroState {
    pub(crate) target: Vec<i32>,
    pub(crate) until: Vec<u32>,
    /// Invulnerable (takes no damage) while the tick is below this, set by a mod.
    pub(crate) invuln_until: Vec<u32>,
    /// Percent of incoming damage removed per unit (0..=100), set by a mod.
    pub(crate) reduce_pct: Vec<u8>,
    /// Healing per second per unit (fixed-point life), set by a mod. Applied every tick while the unit lives.
    pub(crate) heal: Vec<i32>,
    /// Behaviours fired since the host last drained them (`take_fired`). Transient: neither hashed nor saved.
    /// Capped so a run that never drains it stays bounded.
    pub(crate) fired: Vec<Fired>,
}

/// Most fired behaviours the queue holds between drains.
pub(crate) const FIRED_CAP: usize = 1024;

/// One behaviour that fired on a unit, for the pack to answer. `slot` is the ability slot that holds `ability`
/// (-1 for none), and `target_kind` is the ability's target kind code (`TARGET_*`), 0 when there is no ability.
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
    /// The trigger the behaviour fired on, by the name a mod declares it with.
    pub fn trigger_name(&self) -> &'static str {
        crate::loader::Trigger::from_code(self.trigger).map_or("", |t| t.name())
    }

    /// The action the behaviour takes, by the name a mod declares it with.
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
    /// Resets the per-unit aggro state of a freshly spawned slot `i`.
    pub(crate) fn aggro_clear(&mut self, i: usize) {
        self.aggro.target[i] = -1;
        self.aggro.until[i] = 0;
        self.aggro.invuln_until[i] = 0;
        self.aggro.reduce_pct[i] = 0;
        self.aggro.heal[i] = 0;
    }

    /// The unit that last drew aggro from unit `i` and has not yet lapsed, -1 for none.
    pub fn aggro_target(&self, i: usize) -> i32 {
        if i < self.n && self.ticks < self.aggro.until[i] { self.aggro.target[i] } else { -1 }
    }

    pub fn is_structure(&self, i: usize) -> bool {
        self.filter[i] & FILTER_STRUCTURE != 0
    }


    /// Damage `dmg` to unit `j` after the mod's invulnerability and the mod's damage reduction.
    /// With no mod state set this returns `dmg`.
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

    /// Adds the healing a mod set on unit `i` for one tick: its per-second amount spread over the ticks of a
    /// second, capped at its maximum life. A dead unit heals nothing; a unit with no healing is left alone.
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
    /// Sets the healing of unit `i` in whole life per second (0 stops it). A mod's command; the amount is applied
    /// each tick by `tick_healing`, so a mod that wants a regeneration rule sets it while the rule holds.
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

    /// Makes unit `i` invulnerable for `ticks` ticks from now (a mod's command). Refused for a unit past the cap.
    pub fn set_invulnerable(&mut self, i: usize, ticks: u32) -> Result<(), &'static str> {
        if i >= self.n {
            return Err("unknown unit");
        }
        self.aggro.invuln_until[i] = self.ticks.wrapping_add(ticks);
        Ok(())
    }

    /// Sets the percent of damage unit `i` takes removed (0 clears it, 100 takes none). Above 100 is refused.
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

    /// The filter bits of unit `i` (a mask over FILTERS indices). A mod reads them to tell structures from buildings.
    pub fn filter_bits(&self, i: usize) -> u32 {
        self.filter.get(i).copied().unwrap_or(0)
    }

    /// The living units (any owner) whose position lies within `radius` of (`x`, `y`), all in fixed point.
    pub fn units_near(&self, x: i32, y: i32, radius: i32) -> Vec<usize> {
        let r2 = (radius.max(0) as i64) * (radius.max(0) as i64);
        (0..self.n).filter(|&k| self.alive[k] != 0 && dist2(self.pos_x[k], self.pos_y[k], x, y) <= r2).collect()
    }

    /// Makes unit `target` the unit `i` draws aggro toward for `hold` ticks from now (a mod's aggro redirect).
    pub fn set_aggro(&mut self, i: usize, target: usize, hold: u32) {
        self.aggro.target[i] = target as i32;
        self.aggro.until[i] = self.ticks.saturating_add(hold);
    }

    /// Takes the behaviours fired since the last call, oldest first. The host hands each one to the pack's
    /// `on_behaviour` hook, which chooses the action; a sim nobody drains only keeps the first FIRED_CAP.
    pub fn take_fired(&mut self) -> Vec<Fired> {
        std::mem::take(&mut self.aggro.fired)
    }

    /// Sets the target class attacker `i` uses for every candidate (0..=3, the classes `target_class` ranks), or
    /// clears it with `None` so the pack's own choice applies again. Applied by a queued mod command at the next tick.
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

    // A mod's class override is stored per attacker, set and cleared, and out-of-range classes are refused.
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
    /// Sets attacker `i`'s attack-move hold: `Some(true)` holds it in place while it is on an attack-move (the pack
    /// sets it while a foe it can strike is in reach), `Some(false)` lets it walk on, `None` clears it (no hold).
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

    /// The class override set on attacker `i`, or `None` when none is set. Read-only, for debug views.
    pub fn class_override_of(&self, i: usize) -> Option<u8> {
        match self.class_override.get(i) {
            Some(&c) if c >= 0 => Some(c as u8),
            _ => None,
        }
    }

    /// The attack-move hold set on attacker `i`, or `None` when none is set. Read-only, for debug views.
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

    // The Rust default takes no protection; a mod's reduction and invulnerability change the damage, and clearing it restores the default.
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
