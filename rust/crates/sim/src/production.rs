//! Production: a building trains its product. The train cost and build time are
//! the product's own (its minerals, gas and build ticks), paid up front when the
//! train starts, and the finished unit appears beside the building. A train the
//! player cannot pay for does not start and counts as CannotAfford in the stats.

use crate::arrays::Sim;
use crate::fixed;
use crate::loader::FILTER_WORKER;
use crate::rules::Refusal;
use crate::*;

impl Sim {
    /// Gives player `p` the opening resources and workers of its start at (x, y), from ECON.
    /// The workers are the first catalog unit with the worker filter; with none, only the
    /// resources are given. A worker the slots cannot hold is skipped.
    pub(crate) fn open_player(&mut self, p: u8, x: i32, y: i32) {
        let pi = p as usize;
        if pi >= MAX_PLAYERS {
            return;
        }
        let econ = self.cat.econ;
        self.minerals[pi] += econ.opening_minerals;
        self.vespene[pi] += econ.opening_gas;
        if let Some(kind) = self.cat.units.iter().position(|u| u.filter & FILTER_WORKER != 0) {
            for _ in 0..econ.opening_workers {
                if self.spawn_for(p, kind as u32, x, y).is_none() {
                    break;
                }
            }
        }
    }

    /// Supply the player's living units take up, each unit at its UNIT supply cost, plus the
    /// supply reserved by each train in progress (queued production holds its supply until the
    /// unit appears or the train is cancelled).
    pub(crate) fn supply_used(&self, p: usize) -> i32 {
        let living: i32 = (0..self.n)
            .filter(|&j| self.alive[j] != 0 && self.player[j] as usize == p)
            .map(|j| self.cat.units.get(self.type_of[j] as usize).map_or(0, |d| d.supply as i32))
            .sum();
        let reserved: i32 = (0..self.n)
            .filter(|&j| self.alive[j] != 0 && self.player[j] as usize == p && self.train_left[j] > 0 && self.trainee[j] >= 0)
            .map(|j| self.cat.units.get(self.trainee[j] as usize).map_or(0, |d| d.supply as i32))
            .sum();
        living + reserved
    }

    /// Supply the player may take up: the ECON base plus the supply every living unit of the
    /// player provides (UEXT), bounded by the ECON per-race maximum.
    pub(crate) fn supply_cap(&self, p: usize) -> i32 {
        let provided: i32 = (0..self.n)
            .filter(|&j| self.alive[j] != 0 && self.player[j] as usize == p)
            .map(|j| self.cat.unit_ext.get(self.type_of[j] as usize).map_or(0, |e| e.supply_provided as i32))
            .sum();
        (self.cat.econ.supply_base as i32 + provided).min(self.cat.econ.supply_max as i32)
    }

    /// Whether player `p` holds every prerequisite of target `target` of kind `kind` (0 unit,
    /// 1 research). A unit requirement needs a living unit of that type owned by `p`; a research
    /// requirement needs that research completed by `p`.
    /// The research index building `i` is running now, or None when it is idle.
    pub fn research_running(&self, i: usize) -> Option<usize> {
        if i >= self.n || self.research_left[i] <= 0 || self.research_rec[i] < 0 {
            return None;
        }
        Some(self.research_rec[i] as usize)
    }

    pub fn prereqs_met(&self, kind: u8, target: usize, p: usize) -> bool {
        let research_count = self.cat.research.len();
        self.cat
            .prereqs
            .iter()
            .filter(|q| q.target_kind == kind && q.target as usize == target)
            .all(|q| {
                if q.requirement_kind == 0 {
                    (0..self.n).any(|j| self.alive[j] != 0 && self.player[j] as usize == p && self.type_of[j] == q.requirement as i32)
                } else {
                    self.researched.get(p * research_count + q.requirement as usize).map_or(false, |&v| v != 0)
                }
            })
    }

    /// Starts research `r` at building `i` for its owner. The building must be the research's
    /// building and the research not yet complete (else InvalidOrderTarget); a building already
    /// running one is ResearchBusy; missing prerequisites are Prerequisite; the cost is paid up
    /// front, and a short purse is CannotAfford. A refused research changes nothing.
    pub fn start_research(&mut self, i: usize, r: usize) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        let rec = match self.cat.research.get(r).copied() {
            Some(rec) => rec,
            None => return Err(self.reject(Refusal::InvalidOrderTarget)),
        };
        let p = self.player[i] as usize;
        if p >= MAX_PLAYERS || rec.building as i32 != self.type_of[i] {
            return Err(self.reject(Refusal::InvalidOrderTarget));
        }
        if self.researched[p * self.cat.research.len() + r] != 0 {
            return Err(self.reject(Refusal::InvalidOrderTarget));
        }
        if self.research_left[i] > 0 {
            return Err(self.reject(Refusal::ResearchBusy));
        }
        if !self.prereqs_met(1, r, p) {
            return Err(self.reject(Refusal::Prerequisite));
        }
        if self.minerals[p] < rec.minerals || self.vespene[p] < rec.gas {
            return Err(self.reject(Refusal::CannotAfford));
        }
        self.minerals[p] -= rec.minerals;
        self.vespene[p] -= rec.gas;
        self.research_rec[i] = r as i32;
        self.research_left[i] = rec.time.max(1);
        Ok(())
    }

    /// Counts down the research running at building `i`. On completion the research is
    /// recorded as complete for the building's owner and its effect applied to every matching unit.
    pub(crate) fn research_step(&mut self, i: usize) {
        if self.research_left[i] <= 0 {
            return;
        }
        self.research_left[i] -= 1;
        if self.research_left[i] > 0 {
            return;
        }
        let r = self.research_rec[i];
        self.research_rec[i] = -1;
        let p = self.player[i] as usize;
        let rc = self.cat.research.len();
        if r >= 0 && (r as usize) < rc && p < MAX_PLAYERS {
            self.researched[p * rc + r as usize] = 1;
            let rec = self.cat.research[r as usize];
            if let Some(eff) = self.cat.effects.get(rec.effect as usize).copied() {
                for j in 0..self.n {
                    if self.alive[j] != 0 && self.player[j] as usize == p && self.meets_filter(j, rec.applies_mask) {
                        self.apply_effect(i, 0, eff, self.pos_x[j], self.pos_y[j], j as i32, true);
                    }
                }
            }
        }
    }

    /// Places the base's mineral patches and geysers from ECON: `patches` patches in a row east
    /// of the start and `geysers` geysers in a column south of it, one world unit apart. Each
    /// holds the ECON amount and belongs to no player. A spot the catalog cannot place is skipped.
    pub(crate) fn base_fields(&mut self, x: i32, y: i32) {
        let econ = self.cat.econ;
        const NEUTRAL: u8 = 0xFF;
        if let Some(kind) = self.cat.units.iter().position(|u| u.resource_kind == 1) {
            for k in 0..econ.patches_per_base as i32 {
                let at = (x + fixed::from_int(2 + k), y - fixed::from_int(2));
                if let Some(i) = self.spawn_for(NEUTRAL, kind as u32, at.0, at.1) {
                    self.amount[i] = econ.minerals_per_patch;
                }
            }
        }
        if let Some(kind) = self.cat.units.iter().position(|u| u.resource_kind == 2) {
            for k in 0..econ.extractors_per_base as i32 {
                let at = (x - fixed::from_int(2), y + fixed::from_int(2 + k));
                if let Some(i) = self.spawn_for(NEUTRAL, kind as u32, at.0, at.1) {
                    self.amount[i] = econ.gas_per_geyser;
                }
            }
        }
    }

    /// The refusal a train at building `i` meets now, or None when it can start. Records nothing.
    /// No trainee is NoTrain; a train in progress or held after a cancel is TrainBusy; missing
    /// prerequisites are Prerequisite; supply past the cap is SupplyBlocked; a short purse is CannotAfford.
    pub fn train_block(&self, i: usize) -> Option<Refusal> {
        if i >= self.n || self.alive[i] == 0 {
            return Some(Refusal::NoTarget);
        }
        let p = self.player[i] as usize;
        if p >= MAX_PLAYERS || self.trainee[i] < 0 {
            return Some(Refusal::NoTrain);
        }
        if self.train_left[i] > 0 || self.train_hold[i] != 0 {
            return Some(Refusal::TrainBusy);
        }
        let tr = self.trainee[i] as usize;
        let def = match self.cat.units.get(tr) {
            Some(d) => *d,
            None => return Some(Refusal::NoTrain),
        };
        if !self.prereqs_met(0, tr, p) {
            return Some(Refusal::Prerequisite);
        }
        if self.supply_used(p) + def.supply as i32 > self.supply_cap(p) {
            return Some(Refusal::SupplyBlocked);
        }
        if self.minerals[p] < def.minerals || self.vespene[p] < def.gas {
            return Some(Refusal::CannotAfford);
        }
        None
    }

    /// Starts the train of building `i`: the trainee's minerals and gas are deducted before the
    /// unit starts, and it appears when its build time has run out. Every train path starts here.
    /// A refused train changes nothing and is recorded as this call's refusal.
    pub fn start_train(&mut self, i: usize) -> Result<(), Refusal> {
        if let Some(r) = self.train_block(i) {
            return Err(self.reject(r));
        }
        let p = self.player[i] as usize;
        let def = self.cat.units.get(self.trainee[i] as usize).copied().unwrap_or_default();
        self.minerals[p] -= def.minerals;
        self.vespene[p] -= def.gas;
        self.train_left[i] = def.build_ticks.max(1);
        self.touch(i);
        Ok(())
    }

    /// Automatic train step: counts down a train in progress, and starts the next one when it
    /// can. A train that cannot start is counted in the stats under its refusal.
    /// The opening resources every start gets (ECON): minerals, gas and worker count.
    pub fn opening_resources(&self) -> (i32, i32, i32) {
        let e = self.cat.econ;
        (e.opening_minerals, e.opening_gas, e.opening_workers as i32)
    }

    /// Mineral patches and geysers per base from the map's ECON record: (patches, geysers).
    pub fn base_node_counts(&self) -> (i32, i32) {
        (self.cat.econ.patches_per_base as i32, self.cat.econ.extractors_per_base as i32)
    }

    /// The supply cap of player `p`: the ECON base plus provided supply, bounded by the maximum.
    /// Zero for an invalid player.
    pub fn supply_cap_of(&self, p: u8) -> i32 {
        if (p as usize) >= MAX_PLAYERS {
            return 0;
        }
        self.supply_cap(p as usize)
    }

    /// Supply the player's living units provide (UEXT supply provided), before the ECON base and
    /// bound. Zero for an invalid player.
    pub fn supply_providers(&self, p: u8) -> i32 {
        if (p as usize) >= MAX_PLAYERS {
            return 0;
        }
        (0..self.n)
            .filter(|&j| self.alive[j] != 0 && self.player[j] == p)
            .map(|j| self.cat.unit_ext.get(self.type_of[j] as usize).map_or(0, |e| e.supply_provided as i32))
            .sum()
    }

    pub(crate) fn produce_step(&mut self, i: usize) {
        let p = self.player[i] as usize;
        if self.trainee[i] < 0 || p >= MAX_PLAYERS || self.train_hold[i] != 0 {
            return;
        }
        if self.train_left[i] > 0 {
            self.train_left[i] -= 1;
            if self.train_left[i] == 0 {
                let tr = self.trainee[i] as usize;
                self.finish_train(i, p, tr);
            }
            return;
        }
        match self.train_block(i) {
            None => {
                let _ = self.start_train(i);
            }
            Some(r) => {
                self.stats.trains_refused += 1;
                self.stats.last_train_refusal = r;
            }
        }
    }

    /// Sends a unit that finished at building `i` to the building's rally point: the UEXT offset
    /// from the building. A zero offset is no rally point, so the unit stays where it appeared.
    fn rally(&mut self, i: usize, unit: usize) {
        let ext = self.cat.unit_ext.get(self.type_of[i] as usize).copied().unwrap_or_default();
        if ext.rally_x == 0 && ext.rally_y == 0 {
            return;
        }
        let x = self.pos_x[i].wrapping_add(fixed::from_milli(ext.rally_x));
        let y = self.pos_y[i].wrapping_add(fixed::from_milli(ext.rally_y));
        self.move_fixed(unit, x, y);
    }

    fn finish_train(&mut self, i: usize, p: usize, tr: usize) {
        let max_x = fixed::from_int(self.grid.w as i32 - 1);
        let beside = fixed::from_milli(self.cat.units[tr].radius);
        let x = self.pos_x[i].wrapping_add(beside).wrapping_add(self.radius[i]).min(max_x);
        let y = self.pos_y[i];
        if let Some(unit) = self.spawn_for(p as u8, tr as u32, x, y) {
            self.stats.trained += 1;
            self.rally(i, unit);
        }
    }
}

impl Sim {
    /// Cancels the train in progress at building `i` and refunds its full cost. The building
    /// stays idle until `resume_train`, so the automatic restart does not pay again at once.
    /// Refusals: an unknown or dead building is NoTarget, and a building with no train in
    /// progress is NoTrain. A refused cancel changes nothing.
    pub fn cancel_train(&mut self, i: usize) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        if self.train_left[i] <= 0 || self.trainee[i] < 0 {
            return Err(self.reject(Refusal::NoTrain));
        }
        let p = self.player[i] as usize;
        if let Some(def) = self.cat.units.get(self.trainee[i] as usize).copied() {
            if p < MAX_PLAYERS {
                self.minerals[p] += def.minerals;
                self.vespene[p] += def.gas;
            }
        }
        self.train_left[i] = 0;
        self.train_hold[i] = 1;
        Ok(())
    }

    /// Lets building `i` start its next train again after a cancel. Returns false for an
    /// index past the live units.
    pub fn resume_train(&mut self, i: usize) -> bool {
        if i >= self.n {
            return false;
        }
        self.train_hold[i] = 0;
        true
    }
}

