//! The mod host on piccolo, a pure Rust Lua VM, for the wasip3 guest (no C objects). It is the
//! reduced surface the guest drives: load a mod, read its version, spawn and trigger units. A mod
//! call runs under a step budget so a loop cannot stall the guest.

use piccolo::fuel::Fuel;
use piccolo::{Closure, Executor, Lua, StashedExecutor};
use sim::{LoadError, Sim};

use crate::MAX_UNITS;

const FUEL_PER_STEP: i32 = 4096;
const MAX_STEPS: u32 = 2_500;

pub struct ModHost {
    lua: Lua,
    sim: Sim,
    version: u64,
}

impl ModHost {
    /// A host over the sim loaded from `image`, an OSDT v3 image (rust/crates/data/FORMAT.md).
    pub fn new(image: &[u8]) -> Result<Self, LoadError> {
        Ok(Self { lua: Lua::core(), sim: sim::load(image, MAX_UNITS)?, version: 0 })
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
