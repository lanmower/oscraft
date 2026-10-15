//! Lua 5.4 mod host. A mod is a chunk that defines `on_tick(state)`. The host owns
//! `state` and the sim; a mod reload never resets them.

use mlua::{Function, Lua, Table};
use sim::{LoadError, Sim};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

const MAX_UNITS: usize = 1024;
const MAX_REGIONS: usize = 64;

type Shared = Rc<RefCell<Sim>>;
type Regions = Rc<RefCell<Vec<Region>>>;

// Q16.16 corners, normalized so x0 <= x1 and y0 <= y1.
struct Region {
    name: String,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl Region {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
}

struct Hooks {
    on_tick: Function,
    on_spawned: Option<Function>,
    on_attacked: Option<Function>,
    on_died: Option<Function>,
    on_enter: Option<Function>,
    on_leave: Option<Function>,
}

struct ModFile {
    path: PathBuf,
    mtime: SystemTime,
    hooks: Hooks,
}

#[derive(Clone, Copy)]
struct UnitSnap {
    x: i32,
    y: i32,
    hp: i32,
    alive: bool,
}

#[derive(Clone, Copy)]
enum Pending {
    Spawned { unit: usize },
    Attacked { unit: usize },
    Died { unit: usize },
    Enter { unit: usize, region: usize },
    Leave { unit: usize, region: usize },
}

pub struct ModHost {
    lua: Lua,
    world: Shared,
    state: Table,
    regions: Regions,
    inside: Vec<u64>,
    snap: Vec<UnitSnap>,
    prev: Vec<UnitSnap>,
    pending: Vec<Pending>,
    mod_file: Option<ModFile>,
}

fn io_err(e: std::io::Error) -> mlua::Error {
    mlua::Error::RuntimeError(e.to_string())
}

fn fixed(v: i64) -> mlua::Result<i32> {
    i32::try_from(v).map_err(|_| mlua::Error::RuntimeError(format!("Q16.16 value {v} out of i32 range")))
}

fn unknown_region(name: &str) -> mlua::Error {
    mlua::Error::RuntimeError(format!("unknown_region: {name}"))
}

// `spawn_unit` takes the unit's catalog name. `Sim::spawn` takes the catalog index,
// which is the unit's position in the OSDT UNIT section: `Sim::catalog().unit_index`
// (sim/src/loader.rs) maps the STRS name to that position. A name the catalog does
// not know spawns nothing.
fn bind_host(lua: &Lua, env: &Table, world: &Shared, regions: &Regions) -> mlua::Result<()> {
    let w = world.clone();
    env.set(
        "spawn_unit",
        lua.create_function(move |_, (kind, x, y): (String, i64, i64)| {
            let (x, y) = (fixed(x)?, fixed(y)?);
            let mut sim = w.borrow_mut();
            let index = sim.catalog().unit_index(&kind);
            if let Some(index) = index {
                sim.spawn(index as u32, x, y);
            }
            Ok(())
        })?,
    )?;
    let w = world.clone();
    env.set("get_unit_count", lua.create_function(move |_, ()| Ok(w.borrow().count() as i64))?)?;

    let r = regions.clone();
    env.set(
        "define_region",
        lua.create_function(move |_, (name, x0, y0, x1, y1): (String, i64, i64, i64, i64)| {
            let (x0, y0, x1, y1) = (fixed(x0)?, fixed(y0)?, fixed(x1)?, fixed(y1)?);
            let region = Region {
                name: name.clone(),
                x0: x0.min(x1),
                y0: y0.min(y1),
                x1: x0.max(x1),
                y1: y0.max(y1),
            };
            let mut regions = r.borrow_mut();
            if let Some(slot) = regions.iter_mut().find(|g| g.name == name) {
                *slot = region;
                return Ok(());
            }
            if regions.len() >= MAX_REGIONS {
                return Err(mlua::Error::RuntimeError(format!("region_limit: at most {MAX_REGIONS} regions")));
            }
            regions.push(region);
            Ok(())
        })?,
    )?;
    let r = regions.clone();
    env.set(
        "in_region",
        lua.create_function(move |_, (name, x, y): (String, i64, i64)| {
            let (x, y) = (fixed(x)?, fixed(y)?);
            let regions = r.borrow();
            let region = regions.iter().find(|g| g.name == name).ok_or_else(|| unknown_region(&name))?;
            Ok(region.contains(x, y))
        })?,
    )?;
    Ok(())
}

// Builds a fresh environment, runs the chunk in it, and returns its on_tick.
// Errors carry "<path>:<line>: <message>" because the chunk is named "@<path>".
fn compile(lua: &Lua, world: &Shared, regions: &Regions, path: &Path) -> mlua::Result<Hooks> {
    let src = fs::read_to_string(path).map_err(io_err)?;
    let env = lua.create_table()?;
    bind_host(lua, &env, world, regions)?;
    lua.load(src.as_str())
        .set_name(format!("@{}", path.display()))
        .set_environment(env.clone())
        .exec()?;
    let on_tick = env
        .get::<Option<Function>>("on_tick")?
        .ok_or_else(|| mlua::Error::RuntimeError(format!("{}: on_tick is not defined", path.display())))?;
    Ok(Hooks {
        on_tick,
        on_spawned: env.get("on_spawned")?,
        on_attacked: env.get("on_attacked")?,
        on_died: env.get("on_died")?,
        on_enter: env.get("on_enter")?,
        on_leave: env.get("on_leave")?,
    })
}

fn mtime(path: &Path) -> std::io::Result<SystemTime> {
    fs::metadata(path)?.modified()
}

impl ModHost {
    /// A host over the sim loaded from `image`, an OSDT v3 image (rust/crates/data/FORMAT.md),
    /// on the image's own map. `state.ticks` starts at 0.
    pub fn new(image: &[u8]) -> Result<Self, LoadError> {
        let lua = Lua::new();
        let state = lua.create_table().expect("create state table");
        state.set("ticks", 0i64).expect("seed state.ticks");
        let world = Rc::new(RefCell::new(sim::load(image, MAX_UNITS)?));
        let regions: Regions = Rc::new(RefCell::new(Vec::with_capacity(MAX_REGIONS)));
        Ok(ModHost {
            lua,
            world,
            state,
            regions,
            inside: vec![0; MAX_UNITS],
            snap: Vec::with_capacity(MAX_UNITS),
            prev: Vec::with_capacity(MAX_UNITS),
            pending: Vec::with_capacity(MAX_UNITS * MAX_REGIONS),
            mod_file: None,
        })
    }

    /// Compiles the mod at `path` and makes it the active mod.
    pub fn load(&mut self, path: &Path) -> mlua::Result<()> {
        let hooks = compile(&self.lua, &self.world, &self.regions, path)?;
        self.mod_file = Some(ModFile { path: path.to_path_buf(), mtime: mtime(path).map_err(io_err)?, hooks });
        Ok(())
    }

    /// Recompiles when the mod file's mtime moved. A failed compile keeps the old on_tick.
    pub fn reload_if_changed(&mut self) {
        let Some(m) = self.mod_file.as_mut() else { return };
        let Ok(t) = mtime(&m.path) else { return };
        if t == m.mtime {
            return;
        }
        m.mtime = t;
        match compile(&self.lua, &self.world, &self.regions, &m.path) {
            Ok(hooks) => {
                m.hooks = hooks;
                println!("reload: ok ({})", m.path.display());
            }
            Err(e) => println!("reload: FAILED, old on_tick kept: {e}"),
        }
    }

    /// Runs the active mod's on_tick once. A runtime error is returned with file and line.
    pub fn tick(&self, state: &Table) -> mlua::Result<()> {
        match &self.mod_file {
            Some(m) => m.hooks.on_tick.call::<()>(state.clone()),
            None => Ok(()),
        }
    }

    /// One sim tick, then trigger zone transitions for the new unit positions, then
    /// on_tick. Transitions are emitted in unit index order, then region index order.
    pub fn advance(&mut self, state: &Table) -> mlua::Result<()> {
        self.world.borrow_mut().tick();
        self.collect();
        self.dispatch()?;
        self.tick(state)
    }

    fn collect(&mut self) {
        self.snap.clear();
        self.pending.clear();
        {
            let sim = self.world.borrow();
            for i in 0..sim.count() {
                let (x, y) = sim.position_fixed(i);
                self.snap.push(UnitSnap { x, y, hp: sim.hp(i), alive: sim.alive(i) });
            }
        }
        let regions = self.regions.borrow();
        for (unit, s) in self.snap.iter().enumerate() {
            match self.prev.get(unit) {
                None => self.pending.push(Pending::Spawned { unit }),
                Some(p) => {
                    if s.hp < p.hp {
                        self.pending.push(Pending::Attacked { unit });
                    }
                    if p.alive && !s.alive {
                        self.pending.push(Pending::Died { unit });
                    }
                }
            }
            let mut after = 0u64;
            if s.alive {
                for (r, g) in regions.iter().enumerate() {
                    if g.contains(s.x, s.y) {
                        after |= 1 << r;
                    }
                }
            }
            let before = self.inside[unit];
            for r in 0..regions.len() {
                let bit = 1u64 << r;
                match (before & bit != 0, after & bit != 0) {
                    (false, true) => self.pending.push(Pending::Enter { unit, region: r }),
                    (true, false) => self.pending.push(Pending::Leave { unit, region: r }),
                    _ => {}
                }
            }
            self.inside[unit] = after;
        }
        std::mem::swap(&mut self.prev, &mut self.snap);
    }

    fn region_name(&self, region: usize) -> Option<String> {
        self.regions.borrow().get(region).map(|g| g.name.clone())
    }

    fn dispatch(&self) -> mlua::Result<()> {
        let Some(m) = self.mod_file.as_ref() else { return Ok(()) };
        let hooks = &m.hooks;
        for &event in self.pending.iter() {
            match event {
                Pending::Spawned { unit } => {
                    if let Some(h) = &hooks.on_spawned {
                        h.call::<()>(unit as i64)?;
                    }
                }
                Pending::Attacked { unit } => {
                    if let Some(h) = &hooks.on_attacked {
                        h.call::<()>(unit as i64)?;
                    }
                }
                Pending::Died { unit } => {
                    if let Some(h) = &hooks.on_died {
                        h.call::<()>(unit as i64)?;
                    }
                }
                Pending::Enter { unit, region } => {
                    if let (Some(h), Some(name)) = (&hooks.on_enter, self.region_name(region)) {
                        h.call::<()>((name, unit as i64))?;
                    }
                }
                Pending::Leave { unit, region } => {
                    if let (Some(h), Some(name)) = (&hooks.on_leave, self.region_name(region)) {
                        h.call::<()>((name, unit as i64))?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    pub fn state(&self) -> &Table {
        &self.state
    }

    /// Units the sim has spawned, including dead ones (`Sim::count`).
    pub fn unit_count(&self) -> usize {
        self.world.borrow().count()
    }

    /// The sim's FNV-1a state hash (sim/src/hash.rs). Two runs of the same mod from
    /// identical state agree on it.
    pub fn state_hash(&self) -> u64 {
        self.world.borrow().state_hash()
    }
}
