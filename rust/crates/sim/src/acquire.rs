//! Acquire data and chase storage for wave creeps. Each unit record carries its acquire range (the `acquire=` key
//! of `unit_extra` in the scenario data, read when the catalog is built). Values are in game units as the 6.88g
//! UnitWeapons acquire column gives them; `acquire_milli` converts them at 125/8 milli-units per game unit (the same
//! scale units.osd uses). Role mapping, coordinator decision: creep = Ghoul (500), creep_ranged = Necromancer (600),
//! creep_siege = Meat Wagon (800); each super and mega tier takes the value of its role here: creep_super Treant
//! (500), creep_ranged_super Druid of the Talon (600), creep_siege_super Glaive Thrower (800). The event spawns
//! n003, n00E and n00D keep their own values (500, 300, 500); no unit of those names is in the catalog yet, so they
//! match nothing. Towers and barracks have no entry and keep only their weapon reach.
//!
//! The acquire scan, the chase and the return to the saved order are pack rules (scenarios/moba/lanes.lua,
//! wave_creep_acquire). The sim keeps only the chase storage: target and the order to restore, hashed and saved
//! with the rest.

use crate::arrays::Sim;
use crate::loader::Catalog;

/// Game units to milli-units: 125 / 8 milli-units per game unit.
const MILLI_PER_GAME_NUM: i64 = 125;
const MILLI_PER_GAME_DEN: i64 = 8;

/// Milli-units for an acquire value in game units.
pub fn acquire_milli(game: i32) -> i32 {
    (game as i64 * MILLI_PER_GAME_NUM / MILLI_PER_GAME_DEN) as i32
}

/// One acquire range per unit kind, milli-units; 0 for kinds with no acquire value in their unit record.
pub(crate) fn per_kind(cat: &Catalog) -> Vec<i32> {
    cat.unit_ext.iter().map(|ext| acquire_milli(ext.acquire as i32)).collect()
}

/// Chase state per unit: target slot (-1 none) and the order to restore when the chase ends.
#[derive(Clone, Debug)]
pub(crate) struct ChaseState {
    pub(crate) target: Vec<i32>,
    pub(crate) ret_kind: Vec<u8>,
    pub(crate) ret_x: Vec<i32>,
    pub(crate) ret_y: Vec<i32>,
}

impl ChaseState {
    pub(crate) fn new(cap: usize) -> ChaseState {
        ChaseState {
            target: vec![-1; cap],
            ret_kind: vec![0; cap],
            ret_x: vec![0; cap],
            ret_y: vec![0; cap],
        }
    }
}

impl Sim {
    pub(crate) fn chase_clear(&mut self, i: usize) {
        self.chase.target[i] = -1;
    }

    pub(crate) fn chasing(&self, i: usize) -> bool {
        self.chase.target[i] >= 0
    }

    /// Acquire range of unit `i` in milli-units, 0 for kinds with no acquire value.
    pub fn acquire_of(&self, i: usize) -> i32 {
        self.acquire.get(self.type_of[i] as usize).copied().unwrap_or(0)
    }

    /// The chase state of unit `i`: its target (-1 none) and the order it restores when the chase ends.
    pub fn chase_get(&self, i: usize) -> (i32, u8, i32, i32) {
        (self.chase.target[i], self.chase.ret_kind[i], self.chase.ret_x[i], self.chase.ret_y[i])
    }

    /// Sets the chase state of unit `i`; a target of -1 ends the chase.
    pub fn chase_set(&mut self, i: usize, target: i32, kind: u8, x: i32, y: i32) {
        self.chase.target[i] = target;
        self.chase.ret_kind[i] = kind;
        self.chase.ret_x[i] = x;
        self.chase.ret_y[i] = y;
    }
}

impl crate::arrays::Sim {
    /// Sets the acquire range of unit `name` in game units. A load-time mod call: the unit record, the
    /// per-kind value and the catalog digest all change, so the witnesses see the new range.
    pub fn set_acquire(&mut self, name: &str, game: u32) -> Result<(), &'static str> {
        let k = self.cat.unit_index(name).ok_or("unknown unit")?;
        self.cat.unit_ext[k].acquire = game;
        self.acquire[k] = acquire_milli(game as i32);
        self.cat_digest = crate::hash::catalog_digest(&self.cat, &self.grid);
        Ok(())
    }
}
