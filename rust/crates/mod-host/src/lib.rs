//! Lua 5.4 mod host. A mod is a chunk that defines `on_tick(state)`. The host owns
//! `state`, the unit log and the sim; a mod reload never resets them.

use mlua::{Function, Lua, Table};
use sim::Sim;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

const MAX_UNITS: usize = 1024;

#[derive(Clone)]
struct Unit {
    kind: String,
    x: i32,
    y: i32,
}

struct World {
    units: Vec<Unit>,
    sim: Sim,
}

type Shared = Rc<RefCell<World>>;

struct ModFile {
    path: PathBuf,
    mtime: SystemTime,
    on_tick: Function,
}

pub struct ModHost {
    lua: Lua,
    world: Shared,
    state: Table,
    mod_file: Option<ModFile>,
}

fn io_err(e: std::io::Error) -> mlua::Error {
    mlua::Error::RuntimeError(e.to_string())
}

fn fixed(v: i64) -> mlua::Result<i32> {
    i32::try_from(v).map_err(|_| mlua::Error::RuntimeError(format!("Q16.16 value {v} out of i32 range")))
}

fn bind_host(lua: &Lua, env: &Table, world: &Shared) -> mlua::Result<()> {
    let w = world.clone();
    env.set(
        "spawn_unit",
        lua.create_function(move |_, (kind, x, y): (String, i64, i64)| {
            let (x, y) = (fixed(x)?, fixed(y)?);
            let mut world = w.borrow_mut();
            if let Some(index) = world.sim.catalog().unit_index(&kind) {
                let _ = world.sim.spawn(index as u32, x, y);
            }
            world.units.push(Unit { kind, x, y });
            Ok(())
        })?,
    )?;
    let w = world.clone();
    env.set("get_unit_count", lua.create_function(move |_, ()| Ok(w.borrow().units.len() as i64))?)?;
    Ok(())
}

// Builds a fresh environment, runs the chunk in it, and returns its on_tick.
// Errors carry "<path>:<line>: <message>" because the chunk is named "@<path>".
fn compile(lua: &Lua, world: &Shared, path: &Path) -> mlua::Result<Function> {
    let src = fs::read_to_string(path).map_err(io_err)?;
    let env = lua.create_table()?;
    bind_host(lua, &env, world)?;
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

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ (*b as u64)).wrapping_mul(0x0100_0000_01b3))
}

impl ModHost {
    /// An empty host: no mod loaded, `state.ticks` starts at 0.
    pub fn new() -> Self {
        let lua = Lua::new();
        let state = lua.create_table().expect("create state table");
        state.set("ticks", 0i64).expect("seed state.ticks");
        let world = Rc::new(RefCell::new(World { units: Vec::new(), sim: Sim::new(MAX_UNITS) }));
        ModHost { lua, world, state, mod_file: None }
    }

    /// Compiles the mod at `path` and makes it the active mod.
    pub fn load(&mut self, path: &Path) -> mlua::Result<()> {
        let on_tick = compile(&self.lua, &self.world, path)?;
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
        match compile(&self.lua, &self.world, &m.path) {
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

    pub fn unit_count(&self) -> usize {
        self.world.borrow().units.len()
    }

    pub fn count_kind(&self, kind: &str) -> usize {
        self.world.borrow().units.iter().filter(|u| u.kind == kind).count()
    }

    /// FNV-1a over the unit log and `state.ticks`, in a fixed byte layout.
    pub fn state_hash(&self) -> u64 {
        let w = self.world.borrow();
        let mut bytes: Vec<u8> = (w.units.len() as u64).to_le_bytes().to_vec();
        for u in &w.units {
            bytes.extend(u.kind.as_bytes());
            bytes.push(0);
            bytes.extend(u.x.to_le_bytes());
            bytes.extend(u.y.to_le_bytes());
        }
        let ticks: i64 = self.state.get("ticks").unwrap_or(-1);
        bytes.extend(ticks.to_le_bytes());
        fnv1a(&bytes)
    }
}

impl Default for ModHost {
    fn default() -> Self {
        Self::new()
    }
}
