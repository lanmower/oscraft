//! Target priority, hero aggro, backdoor protection and the team glyph. Roles come from unit filters,
//! never from names: a structure is a unit carrying the structure filter, a hero is `Sim::is_hero`,
//! anything else is a plain unit. Every rule is off until `HeroRules` turns it on.
//!
//! Priority (rule `aggro_priority`): a hero attacker keeps the nearest target. A plain-unit attacker
//! prefers plain units over heroes over structures. An armed structure prefers the unit that hit an
//! allied hero in its reach (the aggro target, held `aggro_hold_s`), then plain units, heroes,
//! structures. Within one class the nearest target wins. UNVERIFIED: the class order and the hold
//! time are recalled reference ruleset behaviour, not in the dump (SteamDatabase tracking repo
//! @282fad29f0f74504fad465c52d197fb61f4d612b has no targeting keys).
//!
//! Backdoor protection (rule `backdoor_reduction_pct`): a structure with no hostile armed plain unit
//! within `backdoor_radius_milli` for `backdoor_activation_s` takes reduced damage and regenerates
//! `backdoor_regen_per_s`. Verified from npc_abilities.txt `backdoor_protection`: radius 900,
//! activation_time 15, regen_rate 90. UNVERIFIED: the damage reduction (not in the dump) and the
//! radius scale into sim milli-units (900 reference units x tower range 6000/700).
//!
//! Glyph: the `glyph` effect makes every friendly unit of the caster's team whose filter holds all
//! `need` bits and none of the `skip` bits take no damage for a duration, then locks the team out for
//! a cooldown. Duration, cooldown and filters are the effect's parameters.
//!
//! Aggro, backdoor stamps and glyph timers are sim state: hashed and saved with the rest.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::loader::FILTER_STRUCTURE;
use crate::rules::Refusal;
use crate::{MAX_PLAYERS, TICKS_PER_SECOND};

/// Ticks between backdoor scans.
const BACKDOOR_PERIOD: u32 = 4;

#[derive(Clone, Debug)]
pub(crate) struct AggroState {
    pub(crate) target: Vec<i32>,
    pub(crate) until: Vec<u32>,
    pub(crate) creep_seen: Vec<u32>,
    pub(crate) glyph_until: Vec<u32>,
    pub(crate) glyph_ready: Vec<u32>,
    pub(crate) glyph_need: Vec<u32>,
    pub(crate) glyph_skip: Vec<u32>,
    /// The enemy each unit last auto-attacked, -1 for none. Kept while it lives and stays in reach.
    pub(crate) strike: Vec<i32>,
    /// Spawn generation of each slot, bumped by every spawn into it. A sticky target is only that same
    /// spawn while its generation still matches, so a reused slot never hands the new unit its old target.
    pub(crate) spawn_gen: Vec<u32>,
    /// The spawn generation of `strike`'s target at the moment it was struck.
    pub(crate) strike_gen: Vec<u32>,
    /// (unit, target) pairs where `unit` drew aggro from `target` since the host last drained the log.
    /// Transient: neither hashed nor saved. Capped so a run that never drains it stays bounded.
    pub(crate) acquired: Vec<(u32, u32)>,
}

/// Most acquisitions the log holds between drains.
const ACQUIRED_CAP: usize = 1024;

impl AggroState {
    pub(crate) fn new(cap: usize) -> AggroState {
        AggroState {
            target: vec![-1; cap],
            until: vec![0; cap],
            creep_seen: vec![0; cap],
            glyph_until: vec![0; MAX_PLAYERS],
            glyph_ready: vec![0; MAX_PLAYERS],
            glyph_need: vec![0; MAX_PLAYERS],
            glyph_skip: vec![0; MAX_PLAYERS],
            strike: vec![-1; cap],
            spawn_gen: vec![0; cap],
            strike_gen: vec![0; cap],
            acquired: Vec::new(),
        }
    }
}

impl Sim {
    /// Resets the per-unit aggro state of a freshly spawned slot `i` and starts a new spawn generation for it.
    pub(crate) fn aggro_clear(&mut self, i: usize) {
        self.aggro.target[i] = -1;
        self.aggro.until[i] = 0;
        self.aggro.strike[i] = -1;
        self.aggro.spawn_gen[i] = self.aggro.spawn_gen[i].wrapping_add(1);
        self.aggro.creep_seen[i] = self.ticks;
    }

    /// The unit unit `i` last struck, -1 when none or when that slot has been spawned into since.
    pub(crate) fn sticky_strike(&self, i: usize) -> i32 {
        let s = self.aggro.strike[i];
        let same_spawn = s >= 0 && self.aggro.strike_gen[i] == self.aggro.spawn_gen[s as usize];
        if same_spawn { s } else { -1 }
    }

    /// The unit that last drew aggro from unit `i` and has not yet lapsed, -1 for none.
    pub fn aggro_target(&self, i: usize) -> i32 {
        if i < self.n && self.ticks < self.aggro.until[i] { self.aggro.target[i] } else { -1 }
    }

    pub(crate) fn is_structure(&self, i: usize) -> bool {
        self.filter[i] & FILTER_STRUCTURE != 0
    }

    /// Priority class of target `j` for attacker `i`; lower is preferred.
    pub(crate) fn target_class(&self, i: usize, j: usize) -> u8 {
        let forced = self.class_override[i];
        if forced >= 0 {
            return forced as u8;
        }
        if !self.heroes.rules.aggro_priority {
            return 0;
        }
        let hero_j = self.is_hero(j);
        let structure_j = self.is_structure(j);
        if self.is_hero(i) {
            return 0;
        }
        let plain = !hero_j && !structure_j;
        if self.is_structure(i) {
            if self.aggro.target[i] == j as i32 && self.ticks < self.aggro.until[i] {
                return 0;
            }
            return if plain { 1 } else if hero_j { 2 } else { 3 };
        }
        if plain {
            0
        } else if hero_j {
            1
        } else {
            2
        }
    }

    /// Called when `attacker` damages `victim`. A hero hitting an enemy hero makes every armed
    /// non-hero unit on the victim's side that reaches the victim target the attacker.
    pub(crate) fn aggro_on_hit(&mut self, victim: usize, attacker: usize) {
        if !self.heroes.rules.aggro_priority
            || victim == attacker
            || !self.is_hero(victim)
            || !self.is_hero(attacker)
            || !self.hostile(self.player[victim], self.player[attacker])
        {
            return;
        }
        let hold = (self.heroes.rules.aggro_hold_s * TICKS_PER_SECOND) as u32;
        let side = self.player[victim];
        for u in 0..self.n {
            if self.alive[u] == 0 || self.damage[u] <= 0 || self.is_hero(u) || self.player[u] != side {
                continue;
            }
            let reach = (self.range_sq[u] as i64) << fixed::SHIFT;
            if dist2(self.pos_x[u], self.pos_y[u], self.pos_x[victim], self.pos_y[victim]) <= reach {
                self.aggro.target[u] = attacker as i32;
                self.aggro.until[u] = self.ticks.wrapping_add(hold);
                if self.aggro.acquired.len() < ACQUIRED_CAP {
                    self.aggro.acquired.push((u as u32, attacker as u32));
                }
            }
        }
    }

    /// Whether structure `j` is under backdoor protection now.
    pub fn backdoor_protected(&self, j: usize) -> bool {
        let r = self.heroes.rules;
        if r.backdoor_reduction_pct <= 0 || j >= self.n || !self.is_structure(j) {
            return false;
        }
        let quiet = self.ticks.wrapping_sub(self.aggro.creep_seen[j]);
        quiet >= (r.backdoor_activation_s * TICKS_PER_SECOND) as u32
    }

    /// Damage `dmg` to unit `j` after backdoor reduction and the glyph.
    pub(crate) fn mitigate_mech(&self, j: usize, dmg: i32) -> i32 {
        if dmg <= 0 {
            return dmg;
        }
        if self.glyph_protects(j) {
            return 0;
        }
        if self.backdoor_protected(j) {
            let keep = 100 - self.heroes.rules.backdoor_reduction_pct as i64;
            return (dmg as i64 * keep / 100) as i32;
        }
        dmg
    }

    fn team_slot(&self, p: u8) -> usize {
        (self.team[(p as usize).min(MAX_PLAYERS - 1)] as usize).min(MAX_PLAYERS - 1)
    }

    /// Whether the glyph of unit `j`'s team is running and covers it.
    pub fn glyph_protects(&self, j: usize) -> bool {
        let t = self.team_slot(self.player[j]);
        let a = &self.aggro;
        self.ticks < a.glyph_until[t]
            && a.glyph_need[t] != 0
            && self.filter[j] & a.glyph_need[t] == a.glyph_need[t]
            && self.filter[j] & a.glyph_skip[t] == 0
    }

    /// Ticks until the team of `p` may glyph again; 0 when ready.
    pub fn glyph_cooldown_left(&self, p: u8) -> i32 {
        self.aggro.glyph_ready[self.team_slot(p)].saturating_sub(self.ticks) as i32
    }

    /// The `glyph` effect: p0 duration ticks, p1 team cooldown ticks, p2 filter bits a covered unit
    /// must hold, p3 filter bits it must not hold. A team still on cooldown is refused.
    pub(crate) fn glyph_effect(&mut self, caster: usize, duration: i32, cooldown: i32, need: i32, skip: i32) {
        let t = self.team_slot(self.player[caster]);
        if self.ticks < self.aggro.glyph_ready[t] {
            self.refuse(Refusal::Cooldown);
            return;
        }
        self.aggro.glyph_until[t] = self.ticks.wrapping_add(duration.max(0) as u32);
        self.aggro.glyph_ready[t] = self.ticks.wrapping_add(cooldown.max(0) as u32);
        self.aggro.glyph_need[t] = need as u32;
        self.aggro.glyph_skip[t] = skip as u32;
        for i in 0..self.n {
            if self.alive[i] != 0 && self.team_slot(self.player[i]) == t && self.glyph_protects(i) {
                self.touch(i);
            }
        }
    }

    /// Per tick: refresh backdoor stamps on a cadence and regenerate protected structures each second.
    pub(crate) fn tick_aggro(&mut self) {
        let r = self.heroes.rules;
        if r.backdoor_reduction_pct <= 0 {
            return;
        }
        let scan = self.ticks % BACKDOOR_PERIOD == 0;
        let regen = r.backdoor_regen_per_s > 0 && self.ticks % TICKS_PER_SECOND as u32 == 0;
        if !scan && !regen {
            return;
        }
        let rad = fixed::from_milli(r.backdoor_radius_milli);
        let mut buf = std::mem::take(&mut self.scratch);
        for j in 0..self.n {
            if self.alive[j] == 0 || !self.is_structure(j) {
                continue;
            }
            if scan {
                let count = self.gather(self.pos_x[j], self.pos_y[j], rad, &mut buf);
                let owner = self.player[j];
                let near = buf[..count].iter().any(|&k| {
                    let k = k as usize;
                    self.alive[k] != 0
                        && self.damage[k] > 0
                        && !self.is_hero(k)
                        && !self.is_structure(k)
                        && self.hostile(owner, self.player[k])
                });
                if near {
                    self.aggro.creep_seen[j] = self.ticks;
                }
            }
            if regen && self.backdoor_protected(j) && self.hp[j] < self.hp_max[j] {
                let heal = fixed::from_int(r.backdoor_regen_per_s);
                self.hp[j] = self.hp[j].saturating_add(heal).min(self.hp_max[j]);
                self.touch(j);
            }
        }
        self.scratch = buf;
    }
}

impl crate::arrays::Sim {
    /// Takes the (unit, target) aggro acquisitions since the last call, oldest first. Read by a host that
    /// runs a target-priority callback; a sim nobody drains only keeps the first ACQUIRED_CAP.
    pub fn take_acquisitions(&mut self) -> Vec<(usize, usize)> {
        std::mem::take(&mut self.aggro.acquired).into_iter().map(|(u, t)| (u as usize, t as usize)).collect()
    }

    /// Sets the target class attacker `i` uses for every candidate (0..=3, the classes `target_class` ranks), or
    /// clears it with `None` so the Rust rule applies again. Applied by a queued mod command at the next tick.
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

    // A mod's class override is read before the Rust rule and cleared back to it.
    #[test]
    fn class_override_takes_precedence_and_clears() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let image = osdt_data::generate(&scenario).expect("duel-routes image");
        let mut sim = crate::load(&image, 64).expect("duel-routes sim");
        sim.set_fog(false);
        let kind = sim.catalog().unit_index("creep").expect("creep") as u32;
        let a = sim.spawn_for(0, kind, fixed::from_int(5), fixed::from_int(5)).expect("attacker");
        let b = sim.spawn_for(1, kind, fixed::from_int(6), fixed::from_int(5)).expect("target");
        let rust = sim.target_class(a, b);
        assert_eq!(sim.class_override_of(a), None);
        sim.set_class_override(a, Some(3)).expect("override");
        assert_eq!(sim.class_override_of(a), Some(3));
        assert_eq!(sim.target_class(a, b), 3);
        sim.set_class_override(a, None).expect("clear");
        assert_eq!(sim.class_override_of(a), None);
        assert_eq!(sim.target_class(a, b), rust);
        assert!(sim.set_class_override(a, Some(9)).is_err());
    }
}

impl crate::arrays::Sim {
    /// Sets attacker `i`'s attack-move hold: `Some(true)` always holds on an attack-move, `Some(false)` never does,
    /// `None` restores the Rust rule (hold only while a strikeable foe is in reach).
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

    /// The class override set on attacker `i`, or `None` when the Rust rule applies. Read-only, for debug views.
    pub fn class_override_of(&self, i: usize) -> Option<u8> {
        match self.class_override.get(i) {
            Some(&c) if c >= 0 => Some(c as u8),
            _ => None,
        }
    }

    /// The attack-move hold override of attacker `i`, or `None` when the Rust rule applies. Read-only, for debug views.
    pub fn hold_override_of(&self, i: usize) -> Option<bool> {
        match self.hold_override.get(i) {
            Some(&h) if h >= 0 => Some(h == 1),
            _ => None,
        }
    }
}
