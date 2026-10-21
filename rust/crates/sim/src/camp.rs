//! Neutral camps: a neutral unit the map places keeps its spawn point as its camp home. A camp
//! unit that strays past its leash walks back home. A dead camp unit respawns at home once its
//! timer runs out. Home, leash and timer are sim state: hashed and saved with the rest. Camp units
//! respawn only on a whole-minute clock mark and only when no living unit stands within
//! CAMP_CLEAR_MILLI of their home (reference stacking rule). Roshan waits a random 8 to 11 minutes and
//! bounty runes reappear on a 3 minute clock.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::{DAY_TICKS, ORDER_MOVE, TICKS_PER_SECOND};

/// Distance a camp unit may roam from home, milli-units (about 700 reference units at 15.625 milli per
/// unit). UNVERIFIED: the leash radius is not in the dump.
pub const CAMP_LEASH_MILLI: i32 = 10938;
/// Radius around a camp unit's home that must hold no living unit for it to respawn, milli-units
/// (about 300 reference units). UNVERIFIED: the camp box size is not in the dump.
pub const CAMP_CLEAR_MILLI: i32 = 4688;
/// Seconds into the game at which the first regular neutral camps appear (reference: the first camp set
/// spawns at 1:00, reference wiki citing reference Dueling Fates update). Roshan and
/// runes are present from the start.
pub const CAMP_FIRST_SPAWN_S: u32 = 60;
/// Seconds between camp respawn marks: neutral camps refill on every whole minute of the game clock
/// (reference wiki, Neutral Creeps).
pub const CAMP_RESPAWN_S: u32 = 60;
/// Roshan respawns a random 8 to 11 minutes after death (reference wiki, Roshan): minimum and spread.
pub const ROSHAN_RESPAWN_MIN_S: u32 = 480;
pub const ROSHAN_RESPAWN_SPREAD_S: u32 = 180;
/// Unit name of the large neutral boss, and item name of its reward. Both are catalog names.
pub const ROSHAN_NAME: &str = "roshan";
/// Pit markers: (boss unit name, night marker unit name, day marker unit name). A placed marker unit
/// only gives a position: the loader reads it and removes it. A boss uses its day pit before 15:00 and
/// then the pit of the current phase, night or day (Roshan and the Tormentor swap sides with the
/// day/night cycle from 15:00, reference patch 7.38). Roshan walks to the new pit; the Tormentor cannot
/// move, so it respawns at the pit of the phase it dies in.
pub const PIT_MARKERS: [(&str, &str, &str); 2] = [
    (ROSHAN_NAME, "roshan_pit_night", "roshan_pit_day"),
    ("tormentor", "tormentor_spawn_night", "tormentor_spawn_day"),
];
/// Game second at which the pit swap begins.
pub const PIT_SWAP_S: u32 = 900;

/// Timed boss kinds as (unit name, first spawn seconds, respawn minimum seconds, respawn spread
/// seconds). Roshan is present from the start and returns after 8 to 11 minutes (reference wiki). The
/// Tormentor first spawns at 15:00 and respawns 10 minutes after its death (patches 7.33 and 7.38).
pub const TIMED_BOSSES: [(&str, u32, u32, u32); 2] = [(ROSHAN_NAME, 0, ROSHAN_RESPAWN_MIN_S, ROSHAN_RESPAWN_SPREAD_S), ("tormentor", 900, 600, 0)];
pub const AEGIS_NAME: &str = "aegis";
/// Bounty rune: a 1-hp neutral pickup at a river point. A hero within the pickup range takes its
/// gold and the rune dies, then respawns at its point on the next RUNE_RESPAWN_S clock mark
/// (every 3 minutes, reference wiki, Runes). UNVERIFIED: pickup range and gold.
pub const RUNE_NAME: &str = "bounty_rune";
pub const RUNE_RESPAWN_S: u32 = 180;
pub const RUNE_PICKUP_MILLI: i32 = 1500;
pub const RUNE_GOLD: i32 = 50;

/// Per-unit camp state, sized to the unit cap. `boss_kind` and `reward_kind` come from the
/// catalog at load, not from the save, so they are neither hashed nor saved.
/// Camp, rune and pit timing and reach, in the units each value is read in. Defaults are the reference values;
/// a load-time mod sets any of them through `Sim::set_camp_rule`, so the rules are data, not engine constants.
#[derive(Clone, Debug)]
pub(crate) struct CampRules {
    pub(crate) leash_milli: i32,
    pub(crate) clear_milli: i32,
    pub(crate) first_spawn_s: u32,
    pub(crate) respawn_s: u32,
    pub(crate) rune_respawn_s: u32,
    pub(crate) rune_pickup_milli: i32,
    pub(crate) rune_gold: i32,
    pub(crate) pit_swap_s: u32,
}

impl Default for CampRules {
    fn default() -> CampRules {
        CampRules {
            leash_milli: CAMP_LEASH_MILLI,
            clear_milli: CAMP_CLEAR_MILLI,
            first_spawn_s: CAMP_FIRST_SPAWN_S,
            respawn_s: CAMP_RESPAWN_S,
            rune_respawn_s: RUNE_RESPAWN_S,
            rune_pickup_milli: RUNE_PICKUP_MILLI,
            rune_gold: RUNE_GOLD,
            pit_swap_s: PIT_SWAP_S,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CampState {
    pub(crate) is_camp: Vec<u8>,
    pub(crate) home_x: Vec<i32>,
    pub(crate) home_y: Vec<i32>,
    pub(crate) waiting: Vec<u8>,
    pub(crate) respawn_at: Vec<u32>,
    pub(crate) boss_kind: Option<u32>,
    /// (unit kind, first spawn s, respawn minimum s, respawn spread s) for each timed boss in the catalog.
    pub(crate) timed: Vec<(u32, u32, u32, u32)>,
    /// (boss unit kind, night pit, day pit) in Q16.16 sim coordinates, read from the placed markers.
    pub(crate) pits: Vec<(u32, (i32, i32), (i32, i32))>,
    pub(crate) reward_kind: u32,
    pub(crate) rune_kind: Option<u32>,
    pub(crate) rules: CampRules,
}

impl CampState {
    pub(crate) fn new(cap: usize) -> CampState {
        CampState {
            is_camp: vec![0; cap],
            home_x: vec![0; cap],
            home_y: vec![0; cap],
            waiting: vec![0; cap],
            respawn_at: vec![0; cap],
            boss_kind: None,
            timed: Vec::new(),
            pits: Vec::new(),
            reward_kind: 0,
            rune_kind: None,
            rules: CampRules::default(),
        }
    }

    /// Forgets any camp role of slot `i`; a slot reused by another spawn is not a camp unit.
    pub(crate) fn clear(&mut self, i: usize) {
        if let Some(v) = self.is_camp.get_mut(i) {
            *v = 0;
            self.waiting[i] = 0;
        }
    }
}

impl Sim {
    /// Marks live unit `i` as a camp unit whose home is its current point.
    pub(crate) fn mark_camp(&mut self, i: usize) {
        let c = &mut self.camps;
        c.is_camp[i] = 1;
        c.home_x[i] = self.pos_x[i];
        c.home_y[i] = self.pos_y[i];
    }

    /// Reads the pit marker units placed by the map into `camps.pits` and removes them.
    pub(crate) fn collect_pit_markers(&mut self) {
        let mut found: Vec<(usize, &str, bool, (i32, i32))> = Vec::new();
        for i in 0..self.n {
            if self.alive[i] == 0 {
                continue;
            }
            let Some(name) = self.kind_name(i) else { continue };
            for &(boss, night, day) in PIT_MARKERS.iter() {
                if name == night || name == day {
                    found.push((i, boss, name == night, (self.pos_x[i], self.pos_y[i])));
                }
            }
        }
        let mut pits: Vec<(u32, (i32, i32), (i32, i32))> = Vec::new();
        for &(_, boss, is_night, pos) in &found {
            let Some(kind) = self.cat.unit_index(boss) else { continue };
            let kind = kind as u32;
            let at = match pits.iter().position(|p| p.0 == kind) {
                Some(at) => at,
                None => {
                    pits.push((kind, pos, pos));
                    pits.len() - 1
                }
            };
            if is_night {
                pits[at].1 = pos;
            } else {
                pits[at].2 = pos;
            }
        }
        let markers: Vec<usize> = found.iter().map(|f| f.0).collect();
        for i in markers {
            self.remove_unit(i);
            self.camps.clear(i);
        }
        self.camps.pits = pits;
    }

    /// The pit unit kind `kind` belongs in now, if the map placed markers for it.
    fn pit_now(&self, kind: u32) -> Option<(i32, i32)> {
        let (_, night, day) = *self.camps.pits.iter().find(|p| p.0 == kind)?;
        let swapping = self.ticks >= self.camps.rules.pit_swap_s * TICKS_PER_SECOND as u32;
        Some(if swapping && self.is_night() { night } else { day })
    }

    /// Takes every placed regular camp unit off the map until the first camp mark, as reference's camps
    /// only appear at 1:00. Roshan and runes stay. Call once at load, after the boss and rune kinds are set.
    pub(crate) fn hold_first_camp_spawn(&mut self) {
        let first = self.camps.rules.first_spawn_s * TICKS_PER_SECOND as u32;
        for i in 0..self.n {
            if self.camps.is_camp[i] == 0 || self.camps.rune_kind == Some(self.type_of[i] as u32) {
                continue;
            }
            let timed_first = self.camps.timed.iter().find(|t| t.0 == self.type_of[i] as u32).map(|t| t.1);
            if timed_first == Some(0) || !self.remove_unit(i) {
                continue;
            }
            self.camps.waiting[i] = 1;
            self.camps.respawn_at[i] = timed_first.map_or(first, |s| s * TICKS_PER_SECOND as u32);
        }
    }

    /// Books the respawn timer of camp unit `j` when it dies. Roshan waits longer. A Roshan killed
    /// by an attacker unit grants that unit the Aegis item.
    pub(crate) fn camp_died(&mut self, j: usize, attacker: i32) {
        if self.camps.is_camp[j] == 0 {
            return;
        }
        let kind = self.type_of[j] as u32;
        let boss = self.camps.boss_kind == Some(kind);
        let timed = self.camps.timed.iter().find(|t| t.0 == kind).copied();
        let tps = TICKS_PER_SECOND as u32;
        self.camps.waiting[j] = 1;
        self.camps.respawn_at[j] = if let Some((_, _, min, spread)) = timed {
            self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let extra = ((self.rng >> 33) as u32) % (spread * tps + 1);
            self.ticks.wrapping_add(min * tps + extra)
        } else if self.camps.rune_kind == Some(kind) {
            next_mark(self.ticks, self.camps.rules.rune_respawn_s * tps)
        } else {
            next_mark(self.ticks, self.camps.rules.respawn_s * tps)
        };
        if boss && self.camps.reward_kind != 0 && attacker >= 0 && (attacker as usize) < self.n {
            let _ = self.inventory_add(attacker as usize, self.camps.reward_kind, 1);
        }
    }

    /// Per tick: each live camp unit past its leash is ordered home, and each dead camp unit whose
    /// timer has run out respawns at home. A respawn blocked by a full home cell retries next tick.
    pub(crate) fn tick_camps(&mut self) {
        let leash = fixed::from_milli(self.camps.rules.leash_milli) as i64;
        self.tick_rune_pickups();
        let mut due: Vec<usize> = Vec::new();
        for i in 0..self.n {
            if self.camps.is_camp[i] == 0 {
                continue;
            }
            if self.camps.waiting[i] != 0 {
                if self.ticks >= self.camps.respawn_at[i] {
                    due.push(i);
                }
                continue;
            }
            if self.alive[i] == 0 {
                continue;
            }
            if self.speed[i] > 0 && self.ticks >= self.camps.rules.pit_swap_s * TICKS_PER_SECOND as u32 && (self.day_clock == 0 || self.day_clock == DAY_TICKS) {
                if let Some((px, py)) = self.pit_now(self.type_of[i] as u32) {
                    self.camps.home_x[i] = px;
                    self.camps.home_y[i] = py;
                }
            }
            let (hx, hy) = (self.camps.home_x[i], self.camps.home_y[i]);
            if dist2(self.pos_x[i], self.pos_y[i], hx, hy) > leash * leash {
                let _ = self.issue_order_kind(i, ORDER_MOVE, hx, hy);
            }
        }
        let clear: Vec<bool> = due.iter().map(|&i| self.respawns_free(i) || self.camp_home_clear(i)).collect();
        for (&i, ok) in due.iter().zip(clear) {
            if ok {
                self.respawn_camp(i);
            } else {
                self.camps.respawn_at[i] = next_mark(self.ticks, self.camps.rules.respawn_s * TICKS_PER_SECOND as u32);
            }
        }
    }

    /// Each live bounty rune within pickup range of a living hero pays that hero RUNE_GOLD, then
    /// dies and goes on its respawn timer. The first hero in range takes it.
    fn tick_rune_pickups(&mut self) {
        let Some(rune) = self.camps.rune_kind else {
            return;
        };
        let reach = fixed::from_milli(self.camps.rules.rune_pickup_milli) as i64;
        for r in 0..self.n {
            if self.camps.is_camp[r] == 0 || self.alive[r] == 0 || self.type_of[r] as u32 != rune {
                continue;
            }
            let taker = (0..self.n).find(|&h| {
                self.is_hero(h)
                    && self.alive[h] != 0
                    && dist2(self.pos_x[h], self.pos_y[h], self.pos_x[r], self.pos_y[r]) <= reach * reach
            });
            if let Some(h) = taker {
                self.heroes.gold[h] = self.heroes.gold[h].saturating_add(self.camps.rules.rune_gold);
                self.hurt(r, fixed::from_int(10000));
            }
        }
    }

    /// Whether camp unit `i` is a boss or a rune, which respawn without a clear-home check.
    fn respawns_free(&self, i: usize) -> bool {
        let kind = self.type_of[i] as u32;
        self.camps.boss_kind == Some(kind) || self.camps.rune_kind == Some(kind) || self.camps.timed.iter().any(|t| t.0 == kind)
    }

    /// True when no living unit stands within CAMP_CLEAR_MILLI of camp unit `i`'s home.
    fn camp_home_clear(&self, i: usize) -> bool {
        let reach = fixed::from_milli(self.camps.rules.clear_milli) as i64;
        let (hx, hy) = (self.camps.home_x[i], self.camps.home_y[i]);
        !(0..self.n).any(|k| k != i && self.alive[k] != 0 && dist2(self.pos_x[k], self.pos_y[k], hx, hy) <= reach * reach)
    }

    fn respawn_camp(&mut self, i: usize) {
        let kind = self.type_of[i] as u32;
        let player = self.player[i];
        if let Some((px, py)) = self.pit_now(kind) {
            self.camps.home_x[i] = px;
            self.camps.home_y[i] = py;
        }
        let (x, y) = (self.camps.home_x[i], self.camps.home_y[i]);
        if !self.spawn_at(i, kind, player, x, y) {
            return;
        }
        self.camps.is_camp[i] = 1;
        self.camps.waiting[i] = 0;
    }
}

/// The first multiple of `interval` ticks strictly after `now`.
fn next_mark(now: u32, interval: u32) -> u32 {
    (now / interval + 1).wrapping_mul(interval)
}

impl Sim {
    /// Whether slot `i` is a spawn-group unit (a placed unit the map marked as a camp member).
    pub fn is_spawn_group_unit(&self, i: usize) -> bool {
        self.camps.is_camp.get(i).is_some_and(|&v| v != 0)
    }

    /// Sets the respawn timer of dead spawn-group unit `i`, applied by a queued mod command: `Some(s)` respawns
    /// it `s` seconds from now (the usual home-clear check still applies), `None` leaves it dead. A live unit or a
    /// unit outside a spawn group is refused.
    pub fn set_respawn(&mut self, i: usize, delay_s: Option<u32>) -> Result<(), &'static str> {
        if self.is_hero(i) {
            return self.set_hero_respawn(i, delay_s);
        }
        if !self.is_spawn_group_unit(i) || self.camps.waiting[i] == 0 {
            return Err("not a dead spawn-group unit");
        }
        self.camps.respawn_at[i] = match delay_s {
            Some(s) => self.ticks.wrapping_add(s.saturating_mul(TICKS_PER_SECOND as u32)),
            None => u32::MAX,
        };
        Ok(())
    }

    /// Sets one camp rule by name at load (a mod's `camp_rule(name, value)`). The names are the CampRules fields.
    pub fn set_camp_rule(&mut self, name: &str, value: u32) -> Result<(), &'static str> {
        let r = &mut self.camps.rules;
        match name {
            "leash_milli" => r.leash_milli = value as i32,
            "clear_milli" => r.clear_milli = value as i32,
            "first_spawn_s" => r.first_spawn_s = value,
            "respawn_s" => r.respawn_s = value,
            "rune_respawn_s" => r.rune_respawn_s = value,
            "rune_pickup_milli" => r.rune_pickup_milli = value as i32,
            "rune_gold" => r.rune_gold = value as i32,
            "pit_swap_s" => r.pit_swap_s = value,
            _ => return Err("unknown camp rule"),
        }
        Ok(())
    }
}

impl Sim {
    /// Sets the timed boss `name` (its first spawn, respawn minimum and respawn spread, in seconds): a load-time
    /// mod call. A boss not yet in the table is added; an unknown unit name is refused.
    pub fn set_timed_boss(&mut self, name: &str, first_s: u32, min_s: u32, spread_s: u32) -> Result<(), &'static str> {
        let k = self.cat.unit_index(name).ok_or("unknown unit")? as u32;
        match self.camps.timed.iter_mut().find(|t| t.0 == k) {
            Some(t) => *t = (k, first_s, min_s, spread_s),
            None => self.camps.timed.push((k, first_s, min_s, spread_s)),
        }
        Ok(())
    }
}

#[cfg(test)]
mod respawn_decision_tests {
    use crate::fixed;

    // A queued respawn decision sets the dead spawn-group unit's timer; None leaves it dead; a live or
    // non-group unit is refused and its timer is untouched.
    #[test]
    fn set_respawn_sets_the_timer_and_refuses_units_outside_a_dead_group() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let image = osdt_data::generate(&scenario).expect("duel-routes image");
        let mut sim = crate::load(&image, 64).expect("duel-routes sim");
        let kind = sim.catalog().unit_index("creep").expect("creep") as u32;
        let u = sim.spawn_for(0, kind, fixed::from_int(5), fixed::from_int(5)).expect("unit");
        assert!(sim.set_respawn(u, Some(3)).is_err(), "a unit outside a spawn group is refused");
        sim.camps.is_camp[u] = 1;
        assert!(sim.set_respawn(u, Some(3)).is_err(), "a live spawn-group unit is refused");
        sim.camps.waiting[u] = 1;
        sim.set_respawn(u, Some(3)).expect("dead group unit");
        assert_eq!(sim.camps.respawn_at[u], sim.ticks().wrapping_add(3 * crate::TICKS_PER_SECOND as u32));
        sim.set_respawn(u, None).expect("leave dead");
        assert_eq!(sim.camps.respawn_at[u], u32::MAX);
    }
}
