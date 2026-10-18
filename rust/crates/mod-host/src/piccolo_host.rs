//! The mod host on piccolo, a pure Rust Lua VM, for the wasip3 guest (no C objects). It is the
//! reduced surface the guest drives: load a mod, read its version, spawn and trigger units. A mod
//! call runs under a step budget so a loop cannot stall the guest.

use piccolo::fuel::Fuel;
use piccolo::{Closure, Executor, Lua, StashedExecutor};
use sim::{LoadError, Sim, MAX_PLAYERS};

use crate::MAX_UNITS;

const FUEL_PER_STEP: i32 = 4096;
const MAX_STEPS: u32 = 2_500;

pub struct ModHost {
    lua: Lua,
    sim: Sim,
    version: u64,
    joined: Vec<usize>,
}

impl ModHost {
    /// A host over the sim loaded from `image`, an OSDT v3 image (rust/crates/data/FORMAT.md).
    pub fn new(image: &[u8]) -> Result<Self, LoadError> {
        Ok(Self { lua: Lua::core(), sim: sim::load(image, MAX_UNITS)?, version: 0, joined: Vec::new() })
    }

    /// Compiles the mod in `src`, named `name` in error messages, runs its top level, and makes
    /// it the active version.
    pub fn load_source(&mut self, name: &str, src: &str) -> Result<(), String> {
        let executor = self
            .lua
            .try_enter(|ctx| {
                let closure = Closure::load(ctx, Some(name), src.as_bytes())?;
                Ok(ctx.stash(Executor::start(ctx, closure.into(), ())))
            })
            .map_err(|e| e.to_string())?;
        self.run(&executor)?;
        self.version += 1;
        Ok(())
    }

    /// The count of mods this host has loaded.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Spawns a unit of `kind` for player 0 at Q16.16 `x`, `y`. The unit's index, or None when
    /// the sim refuses the spawn.
    pub fn cmd_spawn(&mut self, kind: u32, x: i32, y: i32) -> Option<usize> {
        self.sim.spawn(kind, x, y)
    }

    /// Fires the trigger `name` on `unit`; false when the unit or the trigger is unknown.
    pub fn fire_trigger(&mut self, unit: usize, name: &str) -> bool {
        self.sim.fire_trigger(unit, name)
    }

    /// The event names a mod can handle or trigger on, the same list the mlua backend serves.
    pub fn events(&self) -> &'static [(&'static str, &'static str)] {
        &crate::EVENTS
    }

    /// Records a player joining; the joined player id is kept for the event step. False when
    /// `player` is outside 0..MAX_PLAYERS.
    pub fn player_joined(&mut self, player: usize) -> bool {
        if player >= MAX_PLAYERS {
            return false;
        }
        self.joined.push(player);
        true
    }

    /// The players recorded since the last take, in join order: the event step hands them to on_player_joined.
    pub fn take_joined(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.joined)
    }

    /// The living unit handles, optionally only those `player` owns, in index order (game.units).
    pub fn units(&self, player: Option<u8>) -> Vec<usize> {
        (0..self.sim.count()).filter(|&i| self.sim.alive(i) && player.map_or(true, |p| self.sim.player(i) == p)).collect()
    }

    /// The catalog name of unit `unit`, None for an unknown handle (game.unit_kind).
    pub fn unit_kind(&self, unit: usize) -> Option<&str> {
        self.sim.kind_name(unit)
    }

    /// The living unit count per player id, indexed 0..MAX_PLAYERS (game.players).
    pub fn players(&self) -> Vec<usize> {
        let mut counts = vec![0usize; MAX_PLAYERS];
        for i in (0..self.sim.count()).filter(|&i| self.sim.alive(i)) {
            counts[self.sim.player(i) as usize % MAX_PLAYERS] += 1;
        }
        counts
    }

    fn run(&mut self, executor: &StashedExecutor) -> Result<(), String> {
        for _ in 0..MAX_STEPS {
            let mut fuel = Fuel::with(FUEL_PER_STEP);
            if self.lua.enter(|ctx| ctx.fetch(executor).step(ctx, &mut fuel)) {
                return self
                    .lua
                    .try_enter(|ctx| ctx.fetch(executor).take_result::<()>(ctx)?)
                    .map_err(|e| e.to_string());
            }
        }
        Err(format!("instruction_limit: over {MAX_STEPS} steps in one call"))
    }
}
