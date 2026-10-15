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

struct ModFile {
    path: PathBuf,
    mtime: SystemTime,
    on_tick: Function,
}

pub struct ModHost {
    lua: Lua,
    world: Shared,
    state: Table,
    regions: Regions,
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
fn compile(lua: &Lua, world: &Shared, regions: &Regions, path: &Path) -> mlua::Result<Function> {
    let src = fs::read_to_string(path).map_err(io_err)?;
    let env = lua.create_table()?;
    bind_host(lua, &env, world, regions)?;
    lua.load(src.as_str())
        .set_name(format!("@{}", path.display()))
        .set_environment(env.clone())
        .exec()?;
    env.get::<Option<Function>>("on_tick")?
        .ok_or_else(|| mlua::Error::RuntimeError(format!("{}: on_tick is not defined", path.display())))
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
        Ok(ModHost { lua, world, state, regions, mod_file: None })
    }

    /// Compiles the mod at `path` and makes it the active mod.
    pub fn load(&mut self, path: &Path) -> mlua::Result<()> {
        let on_tick = compile(&self.lua, &self.world, &self.regions, path)?;
        self.mod_file = Some(ModFile { path: path.to_path_buf(), mtime: mtime(path).map_err(io_err)?, on_tick });
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
            Ok(f) => {
                m.on_tick = f;
                println!("reload: ok ({})", m.path.display());
            }
            Err(e) => println!("reload: FAILED, old on_tick kept: {e}"),
        }
    }

    /// Runs the active mod's on_tick once. A runtime error is returned with file and line.
    pub fn tick(&self, state: &Table) -> mlua::Result<()> {
        match &self.mod_file {
            Some(m) => m.on_tick.call::<()>(state.clone()),
            None => Ok(()),
        }
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
