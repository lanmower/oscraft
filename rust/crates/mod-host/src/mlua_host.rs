//! Lua 5.4 mod host. A mod is a chunk that defines `on_tick(state)`. The host owns
//! `state` and the sim; a mod reload never resets them.

use mlua::{Function, HookTriggers, Lua, RegistryKey, Table, VmState};
use sim::{controller_code, AbilitySpec, Catalog, Doodad, Effect, ItemSpec, LoadError, Refusal, Sim, UnitSpec, UpgradeSpec, ABILITY_SLOTS, ITEM_SLOTS, MAX_PLAYERS, MOD_ALLIANCES, MOD_CONTROLLERS, TICKS_PER_SECOND};
use std::cell::{Cell, RefCell, RefMut};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use crate::shared::{self, PRELUDE_SRC};
use crate::MAX_UNITS;
const MAX_REGIONS: usize = 64;
const MAX_TIMERS: usize = 256;
// The instruction hook fires once per HOOK_STRIDE instructions; a budget is counted in hook firings.
const HOOK_STRIDE: u32 = 1000;
const DEFAULT_INSTRUCTIONS: u64 = 10_000_000;
const DEFAULT_MEMORY: usize = 16 << 20;

type Shared = Rc<RefCell<Sim>>;
type Regions = Rc<RefCell<Vec<Region>>>;
type Timers = Rc<RefCell<Vec<Timer>>>;
type Clock = Rc<Cell<u64>>;
type Cmds = Rc<RefCell<Vec<Cmd>>>;
type SystemSink = Rc<RefCell<Vec<System>>>;

// A component a system or each query can ask for. Units carry these; they are the only ones
// a script can see.
#[derive(Clone, Copy, PartialEq)]
enum Comp {
    Position,
    Owner,
    Hp,
}

impl Comp {
    fn parse(name: &str) -> mlua::Result<Comp> {
        match name {
            "position" => Ok(Comp::Position),
            "owner" => Ok(Comp::Owner),
            "hp" => Ok(Comp::Hp),
            other => Err(mlua::Error::RuntimeError(format!("unknown_component: {other} (known: position, owner, hp)"))),
        }
    }
}

// A living unit as it stood when a system or query began.
struct Row {
    id: usize,
    x: i32,
    y: i32,
    owner: u8,
    hp: i32,
}

// A named system: the components it asked for and the Lua function run per living unit.
struct System {
    name: String,
    comps: Vec<Comp>,
    func: Function,
}

// A command queued during a step; applied after every system, in queue order.
#[derive(Clone)]
pub enum Cmd {
    Spawn { handle: u32, index: usize, x: i32, y: i32 },
    Order { unit: usize, x: i32, y: i32 },
    Damage { caster: usize, target: usize, dmg: i32 },
    Buff { unit: usize, kind: i32, ticks: i32 },
    OrderHandle { handle: u32, x: i32, y: i32 },
    DamageHandle { caster: u32, target: u32, dmg: i32 },
    BuffHandle { handle: u32, kind: i32, ticks: i32 },
    Cast { unit: usize, slot: usize, target: i32, x: i32, y: i32 },
    UseItem { unit: usize, slot: usize, target: i32, x: i32, y: i32 },
    Buy { unit: usize, item: usize },
    SkillUp { unit: usize, slot: usize },
    Sell { unit: usize, slot: usize },
    Unstash { unit: usize, slot: usize },
    Attack { unit: usize, target: usize },
    SetOwner { unit: usize, player: u8 },
    SetFog { on: bool },
    SetTeam { player: u8, team: u8 },
    SetAlliance { a: u8, b: u8, kind: usize },
    AddPlayer { player: u8, controller: u8 },
    RemovePlayer { player: u8 },
    DefineAbility { name: String, cooldown: i32, energy: i32, target: u8, filter: u16, effects: Vec<Effect> },
    DefineUnit { spec: UnitSpec },
    DefineItem { spec: ItemSpec },
    DefineUpgrade { spec: UpgradeSpec },
    GrantUpgrade { player: u8, name: String },
    GiveAbility { unit: usize, slot: usize, name: String },
    AddGold { player: u8, amount: i32 },
    AddHeroGold { unit: usize, amount: i32 },
    Victory { player: u8 },
    Defeat { player: u8 },
    // A client-facing UI event as a JSON object body; the server stamps id and tick and streams it.
    Ui { body: String },
}

// Q16.16 corners, normalized so x0 <= x1 and y0 <= y1.
#[derive(Clone)]
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

// The action is a Lua function held in the registry. A timer is due when the host
// clock reaches its due tick; the clock advances once per ModHost::advance.
#[derive(Clone)]
struct Timer {
    name: String,
    due: u64,
    action: Rc<RegistryKey>,
}

// The live state a reload may touch before it is validated: the sim, regions, timers and
// queued commands. A rejected reload puts all four back.
struct Saved {
    sim: Vec<u8>,
    regions: Vec<Region>,
    timers: Vec<Timer>,
    cmds: Vec<Cmd>,
}

struct Hooks {
    on_tick: Function,
    on_spawned: Option<Function>,
    on_attacked: Option<Function>,
    on_died: Option<Function>,
    on_enter: Option<Function>,
    on_leave: Option<Function>,
    on_player_joined: Option<Function>,
    // Runs every trigger registered on the event name it is called with (game.trigger).
    fire: Function,
    systems: Vec<System>,
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
    killer: Option<usize>,
}

#[derive(Clone, Copy)]
enum Pending {
    Spawned { unit: usize },
    Attacked { unit: usize },
    Died { unit: usize, killer: Option<usize> },
    Enter { unit: usize, region: usize },
    Leave { unit: usize, region: usize },
    PlayerJoined { player: usize },
}

/// A failed reload: the mod file, the Lua line when one is known, and the message.
#[derive(Debug)]
pub struct ReloadError {
    pub file: PathBuf,
    pub line: Option<u32>,
    pub message: String,
}

impl ReloadError {
    fn locate(path: &Path, e: &mlua::Error) -> Self {
        let text = e.to_string();
        // mlua shortens long chunk names to "...<tail>", so match the file name, which survives.
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let prefix = format!("{name}:");
        if let Some(at) = text.find(&prefix) {
            let rest = &text[at + prefix.len()..];
            let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
            if let Ok(line) = rest[..digits].parse::<u32>() {
                let message = rest[digits..].trim_start_matches(':').trim().to_string();
                return ReloadError { file: path.to_path_buf(), line: Some(line), message };
            }
        }
        ReloadError { file: path.to_path_buf(), line: None, message: text }
    }
}

impl fmt::Display for ReloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{}: {}", self.file.display(), line, self.message),
            None => write!(f, "{}: {}", self.file.display(), self.message),
        }
    }
}

pub struct ModHost {
    lua: Lua,
    world: Shared,
    state: Table,
    regions: Regions,
    timers: Timers,
    clock: Clock,
    spent: Rc<Cell<u64>>,
    budget: Rc<Cell<u64>>,
    inside: Vec<u64>,
    snap: Vec<UnitSnap>,
    prev: Vec<UnitSnap>,
    pending: Vec<Pending>,
    mod_file: Option<ModFile>,
    staged: Option<Hooks>,
    version: u64,
    cmds: Cmds,
    joined: Vec<usize>,
}

// The events a mod handles, each with the Lua arguments it receives. A mod defines a global
// function of the same name, and game.trigger takes the same names. Editors list them.
// on_tick is the event the host calls once per step with the state table (see run_on_tick).
use crate::EVENTS;

// Every global bind_host and bind_game register on a mod environment. compile_src checks this
// list against the environment both ways (check_host_globals), so a global cannot be registered
// without being listed, or listed without being registered. Editors read it for completion.
pub const HOST_GLOBALS: &[&str] = &[
    "spawn_unit",
    "get_unit_count",
    "define_region",
    "in_region",
    "schedule",
    "cancel",
    "order_unit",
    "order_player",
    "unit_position",
    "unit_hp",
    "unit_player",
    "unit_alive",
    "edit_terrain",
    "damage_unit",
    "order_attack",
    "list_abilities",
    "ability_info",
    "list_events",
    "set_tile",
    "list_doodads",
    "remove_doodad",
    "place_doodad",
    "buff_unit",
    "game",
];

// The Lua base names compile_src copies into every mod environment; they are not host globals.
const BASE_ENV: [&str; 11] = ["pcall", "xpcall", "error", "assert", "type", "tostring", "tonumber", "select", "pairs", "ipairs", "next"];

fn io_err(e: std::io::Error) -> mlua::Error {
    mlua::Error::RuntimeError(e.to_string())
}

fn fixed(v: i64) -> mlua::Result<i32> {
    shared::fixed(v).map_err(mlua::Error::RuntimeError)
}

fn invalid_unit(i: i64) -> mlua::Error {
    mlua::Error::RuntimeError(format!("invalid_unit: {i}"))
}

// A handle cmd_spawn returned. The room maps it to the unit it spawned; the mod never names a room index.
fn handle_arg(h: i64) -> mlua::Result<u32> {
    u32::try_from(h).map_err(|_| mlua::Error::RuntimeError(format!("invalid_handle: {h}")))
}

fn out_of_grid(x0: i64, y0: i64, x1: i64, y1: i64, w: i64, h: i64) -> mlua::Error {
    mlua::Error::RuntimeError(format!("out_of_grid: rect ({x0},{y0})-({x1},{y1}) exceeds grid {w}x{h}"))
}

fn player_arg(p: i64) -> mlua::Result<u8> {
    shared::player_arg(p).map_err(mlua::Error::RuntimeError)
}

fn effect_from(sim: &Sim, entry: &Table) -> mlua::Result<Effect> {
    let kind: String = entry.get("kind")?;
    let field = |key: &str| -> Result<i32, String> {
        let v = entry.get::<Option<i64>>(key).map_err(|e| e.to_string())?.unwrap_or(0);
        shared::fixed(v)
    };
    let text = |key: &str| -> Result<String, String> { entry.get::<String>(key).map_err(|e| e.to_string()) };
    shared::effect_from(sim, &kind, &field, &text).map_err(mlua::Error::RuntimeError)
}

// A unit index the sim has spawned, dead or alive.
fn known_unit(sim: &Sim, i: i64) -> mlua::Result<usize> {
    usize::try_from(i).ok().filter(|&u| u < sim.count()).ok_or_else(|| invalid_unit(i))
}

// A unit index the sim reports alive; the only units an order may address.
fn live_unit(sim: &Sim, i: i64) -> mlua::Result<usize> {
    usize::try_from(i).ok().filter(|&u| sim.alive(u)).ok_or_else(|| invalid_unit(i))
}

// Maps a Lua heap overflow to the named limit error; other errors pass through.
fn named(e: mlua::Error) -> mlua::Error {
    match e {
        mlua::Error::MemoryError(_) => mlua::Error::RuntimeError("memory_limit: the mod exceeded its Lua heap limit".into()),
        other => other,
    }
}

fn unknown_region(name: &str) -> mlua::Error {
    mlua::Error::RuntimeError(format!("unknown_region: {name}"))
}

// One ability as a Lua table: its catalog name, timing, target rules and effects.
fn ability_row(lua: &Lua, cat: &Catalog, index: usize) -> mlua::Result<Table> {
    let a = &cat.abilities[index];
    let row = lua.create_table()?;
    row.set("name", cat.name(a.name))?;
    row.set("cooldown", a.cooldown as i64)?;
    row.set("energy", a.energy as i64)?;
    row.set("target", a.target as i64)?;
    row.set("filter", a.filter as i64)?;
    row.set("projectile", a.is_projectile())?;
    row.set("projectile_speed", a.projectile_speed as i64)?;
    let first = a.effect_first as usize;
    let span = cat.effects.get(first..first + a.effect_count as usize).unwrap_or(&[]);
    let effects = lua.create_table()?;
    for (i, e) in span.iter().enumerate() {
        let et = lua.create_table()?;
        et.set("kind", e.kind as i64)?;
        et.set("p0", e.p0 as i64)?;
        et.set("p1", e.p1 as i64)?;
        et.set("p2", e.p2 as i64)?;
        et.set("p3", e.p3 as i64)?;
        effects.raw_set((i + 1) as i64, et)?;
    }
    row.set("effects", effects)?;
    Ok(row)
}

// `spawn_unit` takes the unit's catalog name. `Sim::spawn` takes the catalog index,
// which is the unit's position in the OSDT UNIT section: `Sim::catalog().unit_index`
// (sim/src/loader.rs) maps the STRS name to that position. A name the catalog does
// not know spawns nothing.
fn bind_host(lua: &Lua, env: &Table, world: &Shared, regions: &Regions, timers: &Timers, clock: &Clock) -> mlua::Result<()> {
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
    let t = timers.clone();
    let c = clock.clone();
    env.set(
        "schedule",
        lua.create_function(move |lua, (name, after, action): (String, i64, Function)| {
            let after = u64::try_from(after).map_err(|_| mlua::Error::RuntimeError(format!("schedule_delay: {after} is negative")))?;
            let due = c.get() + after.max(1);
            let action = Rc::new(lua.create_registry_value(action)?);
            let mut timers = t.borrow_mut();
            if let Some(slot) = timers.iter_mut().find(|x| x.name == name) {
                *slot = Timer { name, due, action };
                return Ok(());
            }
            if timers.len() >= MAX_TIMERS {
                return Err(mlua::Error::RuntimeError(format!("timer_limit: at most {MAX_TIMERS} timers")));
            }
            timers.push(Timer { name, due, action });
            Ok(())
        })?,
    )?;
    let t = timers.clone();
    env.set(
        "cancel",
        lua.create_function(move |_, name: String| {
            let mut timers = t.borrow_mut();
            let Some(i) = timers.iter().position(|x| x.name == name) else { return Ok(false) };
            timers.remove(i);
            Ok(true)
        })?,
    )?;
    let w = world.clone();
    env.set(
        "order_unit",
        lua.create_function(move |_, (i, x, y): (i64, i64, i64)| {
            let (x, y) = (fixed(x)?, fixed(y)?);
            let mut sim = w.borrow_mut();
            let u = live_unit(&sim, i)?;
            sim.order(u, x, y);
            Ok(())
        })?,
    )?;
    let w = world.clone();
    env.set(
        "order_player",
        lua.create_function(move |_, (p, x, y): (i64, i64, i64)| {
            let player = u8::try_from(p).map_err(|_| mlua::Error::RuntimeError(format!("invalid_player: {p}")))?;
            let (x, y) = (fixed(x)?, fixed(y)?);
            w.borrow_mut().order_all(player, x, y);
            Ok(())
        })?,
    )?;
    let w = world.clone();
    env.set(
        "unit_position",
        lua.create_function(move |_, i: i64| {
            let sim = w.borrow();
            let u = known_unit(&sim, i)?;
            let (x, y) = sim.position_fixed(u);
            Ok((x as i64, y as i64))
        })?,
    )?;
    let w = world.clone();
    env.set(
        "unit_hp",
        lua.create_function(move |_, i: i64| {
            let sim = w.borrow();
            let u = known_unit(&sim, i)?;
            Ok(sim.hp(u) as i64)
        })?,
    )?;
    let w = world.clone();
    env.set(
        "unit_player",
        lua.create_function(move |_, i: i64| {
            let sim = w.borrow();
            let u = known_unit(&sim, i)?;
            Ok(sim.player(u) as i64)
        })?,
    )?;
    let w = world.clone();
    env.set(
        "unit_alive",
        lua.create_function(move |_, i: i64| {
            let sim = w.borrow();
            let u = known_unit(&sim, i)?;
            Ok(sim.alive(u))
        })?,
    )?;
    // Edits the inclusive rectangle of cells between two corners. walkable=true opens the
    // cells, false blocks them. The whole rectangle is checked before any cell changes, so
    // an out-of-grid rectangle leaves the terrain as it was.
    let w = world.clone();
    env.set(
        "edit_terrain",
        lua.create_function(move |_, (x0, y0, x1, y1, walkable): (i64, i64, i64, i64, bool)| {
            let mut sim = w.borrow_mut();
            let (grid_w, grid_h) = (sim.grid().w as i64, sim.grid().h as i64);
            let (lo_x, hi_x) = (x0.min(x1), x0.max(x1));
            let (lo_y, hi_y) = (y0.min(y1), y0.max(y1));
            if lo_x < 0 || lo_y < 0 || hi_x >= grid_w || hi_y >= grid_h {
                return Err(out_of_grid(x0, y0, x1, y1, grid_w, grid_h));
            }
            for y in lo_y..=hi_y {
                for x in lo_x..=hi_x {
                    sim.set_cell_blocked(x as i32, y as i32, !walkable)
                        .map_err(|_| mlua::Error::RuntimeError("terrain_edit: sim refused an in-grid cell".into()))?;
                }
            }
            Ok(())
        })?,
    )?;
    // damage_unit(caster, target, dmg): the caster's player is the attacker. Both units must
    // be alive. The sim spares friendly and dead targets and the caster itself.
    let w = world.clone();
    env.set(
        "damage_unit",
        lua.create_function(move |_, (caster, target, dmg): (i64, i64, i64)| {
            let dmg = i32::try_from(dmg).map_err(|_| mlua::Error::RuntimeError(format!("invalid_damage: {dmg} out of i32 range")))?;
            let mut sim = w.borrow_mut();
            let c = live_unit(&sim, caster)?;
            let t = live_unit(&sim, target)?;
            let player = sim.player(c);
            sim.damage_target(c as i32, player, dmg, t as i32);
            Ok(())
        })?,
    )?;
    // order_attack(attacker, target) orders a live unit to attack a live target through the sim's
    // attack path. Its damage fires on_attacked and, when lethal, on_died. A target that is not
    // a live unit, or one the attacker cannot hit, raises invalid_target.
    let w = world.clone();
    env.set(
        "order_attack",
        lua.create_function(move |_, (attacker, target): (i64, i64)| {
            let mut sim = w.borrow_mut();
            let a = live_unit(&sim, attacker)?;
            let t = usize::try_from(target)
                .ok()
                .filter(|&u| sim.alive(u))
                .ok_or_else(|| mlua::Error::RuntimeError(format!("invalid_target: {target} is not a live unit")))?;
            sim.order_attack(a, t).map_err(|r| match r {
                Refusal::InvalidOrderTarget | Refusal::NoTarget => {
                    mlua::Error::RuntimeError(format!("invalid_target: {attacker} cannot attack {target}"))
                }
                other => mlua::Error::RuntimeError(format!("order_refused: {}", other.name())),
            })
        })?,
    )?;
    // list_abilities() returns every catalog ability as a table; ability_info(name) returns one,
    // or raises unknown_ability. Editors read these rows to author abilities.
    let w = world.clone();
    env.set(
        "list_abilities",
        lua.create_function(move |lua, ()| {
            let sim = w.borrow();
            let cat = sim.catalog();
            let out = lua.create_table()?;
            for i in 0..cat.abilities.len() {
                out.raw_set((i + 1) as i64, ability_row(lua, cat, i)?)?;
            }
            Ok(out)
        })?,
    )?;
    let w = world.clone();
    env.set(
        "ability_info",
        lua.create_function(move |lua, name: String| {
            let sim = w.borrow();
            let cat = sim.catalog();
            let index = cat
                .abilities
                .iter()
                .position(|a| cat.name(a.name) == name)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("unknown_ability: {name}")))?;
            ability_row(lua, cat, index)
        })?,
    )?;
    // list_events() returns each event a mod can define, with its Lua arguments.
    env.set(
        "list_events",
        lua.create_function(move |lua, ()| {
            let out = lua.create_table()?;
            for (i, (name, args)) in EVENTS.iter().enumerate() {
                let row = lua.create_table()?;
                row.set("name", *name)?;
                row.set("args", *args)?;
                out.raw_set((i + 1) as i64, row)?;
            }
            Ok(out)
        })?,
    )?;
    // set_tile(x, y, tile) sets the author-defined tile id (0..=255) of one cell. A cell outside
    // the grid raises out_of_grid; a tile outside the byte range raises invalid_tile.
    let w = world.clone();
    env.set(
        "set_tile",
        lua.create_function(move |_, (x, y, tile): (i64, i64, i64)| {
            let mut sim = w.borrow_mut();
            let (grid_w, grid_h) = (sim.grid().w as i64, sim.grid().h as i64);
            if x < 0 || y < 0 || x >= grid_w || y >= grid_h {
                return Err(out_of_grid(x, y, x, y, grid_w, grid_h));
            }
            let tile = u8::try_from(tile).map_err(|_| mlua::Error::RuntimeError(format!("invalid_tile: {tile} is outside 0..=255")))?;
            sim.set_cell_tile(x as i32, y as i32, tile)
                .map_err(|_| mlua::Error::RuntimeError("tile_edit: sim refused an in-grid cell".into()))
        })?,
    )?;
    // list_doodads() returns the map's doodad records (DODA) so an editor can read what it may place or remove.
    let w = world.clone();
    env.set(
        "list_doodads",
        lua.create_function(move |lua, ()| {
            let sim = w.borrow();
            let out = lua.create_table()?;
            for (i, d) in sim.catalog().doodads.iter().enumerate() {
                let row = lua.create_table()?;
                row.set("id", d.id as i64)?;
                row.set("kind", d.kind as i64)?;
                row.set("x", d.x as i64)?;
                row.set("y", d.y as i64)?;
                row.set("rotation", d.rotation as i64)?;
                out.raw_set((i + 1) as i64, row)?;
            }
            Ok(out)
        })?,
    )?;
    // remove_doodad(id) removes a placed doodad and returns its record. An id that is not placed
    // raises no_doodad; a sim refusal of any other kind is named by its refusal.
    let w = world.clone();
    env.set(
        "remove_doodad",
        lua.create_function(move |lua, id: i64| {
            let id = u32::try_from(id).map_err(|_| mlua::Error::RuntimeError(format!("no_doodad: {id} is not a doodad id")))?;
            let mut sim = w.borrow_mut();
            let d = sim.remove_doodad(id).map_err(|r| match r {
                Refusal::NoDoodad => mlua::Error::RuntimeError(format!("no_doodad: {id} is not placed")),
                other => mlua::Error::RuntimeError(format!("doodad_refused: {}", other.name())),
            })?;
            let row = lua.create_table()?;
            row.set("id", d.id as i64)?;
            row.set("kind", d.kind as i64)?;
            row.set("x", d.x as i64)?;
            row.set("y", d.y as i64)?;
            row.set("rotation", d.rotation as i64)?;
            Ok(row)
        })?,
    )?;
    // place_doodad(id, kind, x, y, rotation) places a doodad at map milli-units. Named refusals:
    // off_grid (position outside the map, kind above 3, rotation outside 0..1000, or a value outside
    // the record's range) and duplicate_doodad (id already placed).
    let w = world.clone();
    env.set(
        "place_doodad",
        lua.create_function(move |_, (id, kind, x, y, rotation): (i64, i64, i64, i64, i64)| {
            let off = |v: i64| mlua::Error::RuntimeError(format!("off_grid: {v} is outside the doodad record's range"));
            let d = Doodad {
                id: u32::try_from(id).map_err(|_| mlua::Error::RuntimeError(format!("invalid_doodad: id {id} is outside u32")))?,
                kind: u16::try_from(kind).map_err(|_| off(kind))?,
                x: i32::try_from(x).map_err(|_| off(x))?,
                y: i32::try_from(y).map_err(|_| off(y))?,
                rotation: i32::try_from(rotation).map_err(|_| off(rotation))?,
            };
            let mut sim = w.borrow_mut();
            sim.place_doodad(d).map_err(|r| match r {
                Refusal::OffGrid => mlua::Error::RuntimeError(format!("off_grid: doodad {id} lies outside the map or has a bad kind or rotation")),
                Refusal::DuplicateDoodad => mlua::Error::RuntimeError(format!("duplicate_doodad: {id} is already placed")),
                other => mlua::Error::RuntimeError(format!("doodad_refused: {}", other.name())),
            })
        })?,
    )?;
    // buff_unit(unit, kind, ticks) gives a live unit a behaviour for `ticks` ticks. Returns
    // false when the unit has no free buff slot, and the sim counts that drop.
    let w = world.clone();
    env.set(
        "buff_unit",
        lua.create_function(move |_, (unit, kind, ticks): (i64, i64, i64)| {
            let mut sim = w.borrow_mut();
            let u = live_unit(&sim, unit)?;
            let kind = i32::try_from(kind).map_err(|_| mlua::Error::RuntimeError(format!("invalid_buff: kind {kind} out of i32 range")))?;
            let ticks = i32::try_from(ticks)
                .ok()
                .filter(|&t| t > 0)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("invalid_buff: ticks {ticks} must be a positive i32")))?;
            Ok(sim.apply_buff(u, kind, ticks))
        })?,
    )?;
    Ok(())
}

fn parse_comps(list: Table) -> mlua::Result<Vec<Comp>> {
    list.sequence_values::<String>().map(|name| Comp::parse(&name?)).collect()
}

// Every living unit, as a snapshot taken before any Lua runs against it.
fn snapshot(world: &Shared) -> Vec<Row> {
    let sim = world.borrow();
    (0..sim.count())
        .filter(|&i| sim.alive(i))
        .map(|i| {
            let (x, y) = sim.position_fixed(i);
            Row { id: i, x, y, owner: sim.player(i), hp: sim.hp(i) }
        })
        .collect()
}

// The table a script sees for one unit: its id, plus only the components it asked for.
fn view(lua: &Lua, row: &Row, comps: &[Comp]) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("id", row.id as i64)?;
    for c in comps {
        match c {
            Comp::Position => {
                let p = lua.create_table()?;
                p.set("x", row.x as i64)?;
                p.set("y", row.y as i64)?;
                t.set("position", p)?;
            }
            Comp::Owner => t.set("owner", row.owner as i64)?,
            Comp::Hp => t.set("hp", row.hp as i64)?,
        }
    }
    Ok(t)
}

// The game table: game.system registers a per-tick system, game.each runs a query now, and the
// cmd_ functions queue commands for the command step. Unit arguments are validated when queued.
fn bind_game(lua: &Lua, env: &Table, world: &Shared, cmds: &Cmds, systems: &SystemSink) -> mlua::Result<()> {
    let game = lua.create_table()?;
    let sink = systems.clone();
    game.set(
        "system",
        lua.create_function(move |_, (name, comps, func): (String, Table, Function)| {
            let comps = parse_comps(comps)?;
            let mut list = sink.borrow_mut();
            if list.iter().any(|s| s.name == name) {
                return Err(mlua::Error::RuntimeError(format!("duplicate_system: {name} is already registered")));
            }
            list.push(System { name, comps, func });
            Ok(())
        })?,
    )?;
    let w = world.clone();
    game.set(
        "each",
        lua.create_function(move |lua, (comps, func): (Table, Function)| {
            let comps = parse_comps(comps)?;
            for row in snapshot(&w) {
                func.call::<()>(view(lua, &row, &comps)?)?;
            }
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    let next_handle = Rc::new(std::cell::Cell::new(0u32));
    game.set(
        "cmd_spawn",
        lua.create_function(move |_, (kind, x, y): (String, i64, i64)| {
            let index = w
                .borrow()
                .catalog()
                .unit_index(&kind)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("unknown_unit: {kind}")))?;
            let handle = next_handle.get() + 1;
            next_handle.set(handle);
            buf.borrow_mut().push(Cmd::Spawn { handle, index, x: fixed(x)?, y: fixed(y)? });
            Ok(handle)
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "cmd_order_handle",
        lua.create_function(move |_, (handle, x, y): (i64, i64, i64)| {
            buf.borrow_mut().push(Cmd::OrderHandle { handle: handle_arg(handle)?, x: fixed(x)?, y: fixed(y)? });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "cmd_damage_handle",
        lua.create_function(move |_, (caster, target, dmg): (i64, i64, i64)| {
            let dmg = i32::try_from(dmg).map_err(|_| mlua::Error::RuntimeError(format!("invalid_damage: {dmg} out of i32 range")))?;
            buf.borrow_mut().push(Cmd::DamageHandle { caster: handle_arg(caster)?, target: handle_arg(target)?, dmg });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "cmd_buff_handle",
        lua.create_function(move |_, (handle, kind, ticks): (i64, i64, i64)| {
            let kind = i32::try_from(kind).map_err(|_| mlua::Error::RuntimeError(format!("invalid_buff: kind {kind} out of i32 range")))?;
            let ticks = i32::try_from(ticks)
                .ok()
                .filter(|&t| t > 0)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("invalid_buff: ticks {ticks} must be a positive i32")))?;
            buf.borrow_mut().push(Cmd::BuffHandle { handle: handle_arg(handle)?, kind, ticks });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_order",
        lua.create_function(move |_, (unit, x, y): (i64, i64, i64)| {
            let u = live_unit(&w.borrow(), unit)?;
            buf.borrow_mut().push(Cmd::Order { unit: u, x: fixed(x)?, y: fixed(y)? });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_damage",
        lua.create_function(move |_, (caster, target, dmg): (i64, i64, i64)| {
            let (c, t) = {
                let sim = w.borrow();
                (live_unit(&sim, caster)?, live_unit(&sim, target)?)
            };
            let dmg = i32::try_from(dmg).map_err(|_| mlua::Error::RuntimeError(format!("invalid_damage: {dmg} out of i32 range")))?;
            buf.borrow_mut().push(Cmd::Damage { caster: c, target: t, dmg });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_buff",
        lua.create_function(move |_, (unit, kind, ticks): (i64, i64, i64)| {
            let u = live_unit(&w.borrow(), unit)?;
            let kind = i32::try_from(kind).map_err(|_| mlua::Error::RuntimeError(format!("invalid_buff: kind {kind} out of i32 range")))?;
            let ticks = i32::try_from(ticks)
                .ok()
                .filter(|&t| t > 0)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("invalid_buff: ticks {ticks} must be a positive i32")))?;
            buf.borrow_mut().push(Cmd::Buff { unit: u, kind, ticks });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_cast",
        lua.create_function(move |_, (unit, slot, target, x, y): (i64, i64, i64, i64, i64)| {
            let u = live_unit(&w.borrow(), unit)?;
            let slot = usize::try_from(slot).ok().filter(|&s| s < ABILITY_SLOTS).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_slot: {slot} is outside 0..{ABILITY_SLOTS}"))
            })?;
            // A target of -1 casts at the point; any other target must be a live unit.
            let target = if target == -1 { -1 } else { live_unit(&w.borrow(), target)? as i32 };
            buf.borrow_mut().push(Cmd::Cast { unit: u, slot, target, x: fixed(x)?, y: fixed(y)? });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_use_item",
        lua.create_function(move |_, (unit, slot, target, x, y): (i64, i64, i64, i64, i64)| {
            let u = live_unit(&w.borrow(), unit)?;
            let slot = usize::try_from(slot).ok().filter(|&s| s < ITEM_SLOTS).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_slot: {slot} is outside 0..{ITEM_SLOTS}"))
            })?;
            let target = if target == -1 { -1 } else { live_unit(&w.borrow(), target)? as i32 };
            buf.borrow_mut().push(Cmd::UseItem { unit: u, slot, target, x: fixed(x)?, y: fixed(y)? });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_buy",
        lua.create_function(move |_, (unit, item): (i64, i64)| {
            let u = live_unit(&w.borrow(), unit)?;
            let item = usize::try_from(item).map_err(|_| mlua::Error::RuntimeError(format!("invalid_item: {item}")))?;
            buf.borrow_mut().push(Cmd::Buy { unit: u, item });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_skill_up",
        lua.create_function(move |_, (unit, slot): (i64, i64)| {
            let u = live_unit(&w.borrow(), unit)?;
            let slot = usize::try_from(slot).ok().filter(|&s| s < ABILITY_SLOTS).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_slot: {slot} is outside 0..{ABILITY_SLOTS}"))
            })?;
            buf.borrow_mut().push(Cmd::SkillUp { unit: u, slot });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_sell",
        lua.create_function(move |_, (unit, slot): (i64, i64)| {
            let u = live_unit(&w.borrow(), unit)?;
            let slot = usize::try_from(slot).ok().filter(|&s| s < ITEM_SLOTS).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_slot: {slot} is outside 0..{ITEM_SLOTS}"))
            })?;
            buf.borrow_mut().push(Cmd::Sell { unit: u, slot });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_unstash",
        lua.create_function(move |_, (unit, slot): (i64, i64)| {
            let u = live_unit(&w.borrow(), unit)?;
            let slot = usize::try_from(slot).ok().filter(|&s| s < ITEM_SLOTS).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_slot: {slot} is outside 0..{ITEM_SLOTS}"))
            })?;
            buf.borrow_mut().push(Cmd::Unstash { unit: u, slot });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_attack",
        lua.create_function(move |_, (attacker, target): (i64, i64)| {
            let (a, t) = {
                let sim = w.borrow();
                (live_unit(&sim, attacker)?, live_unit(&sim, target)?)
            };
            buf.borrow_mut().push(Cmd::Attack { unit: a, target: t });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_set_owner",
        lua.create_function(move |_, (unit, player): (i64, i64)| {
            let u = live_unit(&w.borrow(), unit)?;
            let player = usize::try_from(player).ok().filter(|&p| p < MAX_PLAYERS).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_player: {player} is outside 0..{MAX_PLAYERS}"))
            })?;
            buf.borrow_mut().push(Cmd::SetOwner { unit: u, player: player as u8 });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "cmd_set_fog",
        lua.create_function(move |_, on: bool| {
            buf.borrow_mut().push(Cmd::SetFog { on });
            Ok(())
        })?,
    )?;
    let w = world.clone();
    game.set(
        "team",
        lua.create_function(move |_, player: i64| {
            let p = player_arg(player)?;
            Ok(w.borrow().team_of(p).map(i64::from))
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "set_team",
        lua.create_function(move |_, (player, team): (i64, i64)| {
            let player = player_arg(player)?;
            let team = usize::try_from(team).ok().filter(|&t| t < MAX_PLAYERS).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_team: {team} is outside 0..{MAX_PLAYERS}"))
            })?;
            buf.borrow_mut().push(Cmd::SetTeam { player, team: team as u8 });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "alliance",
        lua.create_function(move |_, (a, b, kind): (i64, i64, Option<String>)| {
            let (a, b) = (player_arg(a)?, player_arg(b)?);
            let Some(kind) = kind else { return Ok(w.borrow().relation(a, b).map(str::to_owned)) };
            let kind = MOD_ALLIANCES.iter().position(|k| *k == kind).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_alliance: {kind} is not {}", MOD_ALLIANCES.join(", ")))
            })?;
            buf.borrow_mut().push(Cmd::SetAlliance { a, b, kind });
            Ok(None)
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "add_player",
        lua.create_function(move |_, (player, controller): (i64, String)| {
            let player = player_arg(player)?;
            let controller = controller_code(&controller).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_controller: {controller} is not {}", MOD_CONTROLLERS.join(", ")))
            })?;
            buf.borrow_mut().push(Cmd::AddPlayer { player, controller });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "remove_player",
        lua.create_function(move |_, player: i64| {
            buf.borrow_mut().push(Cmd::RemovePlayer { player: player_arg(player)? });
            Ok(())
        })?,
    )?;
    let w = world.clone();
    game.set(
        "units",
        lua.create_function(move |lua, player: Option<i64>| {
            let sim = w.borrow();
            let list = lua.create_table()?;
            let mut n = 0i64;
            for i in 0..sim.count() {
                if !sim.alive(i) || player.is_some_and(|p| sim.player(i) as i64 != p) {
                    continue;
                }
                n += 1;
                list.raw_set(n, i as i64)?;
            }
            Ok(list)
        })?,
    )?;
    let w = world.clone();
    game.set(
        "players",
        lua.create_function(move |lua, ()| {
            let sim = w.borrow();
            let counts = living_per_player(&sim);
            let list = lua.create_table()?;
            for (p, alive) in counts.iter().enumerate() {
                let row = lua.create_table()?;
                row.set("id", p as i64)?;
                row.set("units", *alive)?;
                row.set("team", sim.team_of(p as u8).map(i64::from))?;
                row.set("controller", MOD_CONTROLLERS[sim.controller(p as u8) as usize])?;
                list.raw_set(p as i64 + 1, row)?;
            }
            Ok(list)
        })?,
    )?;
    let w = world.clone();
    game.set(
        "unit_kind",
        lua.create_function(move |_, unit: i64| {
            let sim = w.borrow();
            // known_unit, not live_unit: on_died runs after the unit is dead and its kind is still asked.
            let u = known_unit(&sim, unit)?;
            Ok(sim.kind_name(u).map(str::to_owned))
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "ability",
        lua.create_function(move |_, (name, spec): (String, Table)| {
            let mut sim = w.borrow_mut();
            let mut effects = Vec::new();
            if let Some(list) = spec.get::<Option<Table>>("effects")? {
                for entry in list.sequence_values::<Table>() {
                    effects.push(effect_from(&sim, &entry?)?);
                }
            }
            let int = |key: &str| -> mlua::Result<i32> { fixed(spec.get::<Option<i64>>(key)?.unwrap_or(0)) };
            let target = spec.get::<Option<String>>("target")?;
            let (target, cooldown, energy, filter) =
                shared::ability_args(target.as_deref(), int("cooldown")?, int("energy")?, int("filter")?).map_err(mlua::Error::RuntimeError)?;
            sim.define_ability(AbilitySpec { name: &name, cooldown, energy, target, filter, effects: effects.clone() })
                .map_err(|why| mlua::Error::RuntimeError(format!("invalid_ability: {name}: {why}")))?;
            buf.borrow_mut().push(Cmd::DefineAbility { name: name.clone(), cooldown, energy, target, filter, effects });
            Ok(name)
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "unit",
        lua.create_function(move |_, (name, spec): (String, Table)| {
            let int = |key: &str, default: i64| -> mlua::Result<i32> { fixed(spec.get::<Option<i64>>(key)?.unwrap_or(default)) };
            let weapon = match spec.get::<Option<Table>>("weapon")? {
                None => None,
                Some(t) => {
                    let get = |key: &str, default: i64| -> mlua::Result<i32> { fixed(t.get::<Option<i64>>(key)?.unwrap_or(default)) };
                    Some((get("damage", 0)?, get("range", 1000)?, get("period", 16)?))
                }
            };
            let abilities = match spec.get::<Option<Table>>("abilities")? {
                None => Vec::new(),
                Some(list) => list.sequence_values::<String>().collect::<mlua::Result<Vec<_>>>()?,
            };
            let def = UnitSpec {
                name: name.clone(),
                model: spec.get::<Option<String>>("model")?.unwrap_or_default(),
                health: int("health", 0)?,
                speed: int("speed", 0)?,
                radius: int("radius", 500)?,
                armor: int("armor", 0)?,
                shield: int("shield", 0)?,
                sight: int("sight", 8000)?,
                filter: u32::try_from(int("filter", 1)?).map_err(|_| mlua::Error::RuntimeError("invalid_unit: filter must not be negative".into()))?,
                cost: int("cost", 0)?,
                bounty: int("bounty", 0)?,
                weapon,
                abilities,
            };
            w.borrow_mut().define_unit(&def).map_err(|why| mlua::Error::RuntimeError(format!("invalid_unit: {name}: {why}")))?;
            buf.borrow_mut().push(Cmd::DefineUnit { spec: def });
            Ok(name)
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "item",
        lua.create_function(move |_, (name, spec): (String, Table)| {
            let int = |key: &str| -> mlua::Result<i32> { fixed(spec.get::<Option<i64>>(key)?.unwrap_or(0)) };
            let def = ItemSpec {
                name: name.clone(),
                cost: int("cost")?,
                armor: int("armor")?,
                damage: int("damage")?,
                health: int("health")?,
                ability: spec.get::<Option<String>>("ability")?,
            };
            let index = w.borrow_mut().define_item(&def).map_err(|why| mlua::Error::RuntimeError(format!("invalid_item: {name}: {why}")))?;
            buf.borrow_mut().push(Cmd::DefineItem { spec: def });
            Ok(index as i64)
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "upgrade",
        lua.create_function(move |_, (name, spec): (String, Table)| {
            let int = |key: &str, default: i64| -> mlua::Result<i32> { fixed(spec.get::<Option<i64>>(key)?.unwrap_or(default)) };
            let names = |key: &str| -> mlua::Result<Vec<String>> {
                match spec.get::<Option<Table>>(key)? {
                    None => Ok(Vec::new()),
                    Some(list) => list.sequence_values::<String>().collect(),
                }
            };
            let entry: Table = spec.get("effect")?;
            let effect = effect_from(&w.borrow(), &entry)?;
            let def = UpgradeSpec {
                name: name.clone(),
                building: spec.get("building")?,
                time: int("time", 1)?,
                cost: int("cost", 0)?,
                gas: int("gas", 0)?,
                effect,
                filter: u32::try_from(int("filter", 0)?).map_err(|_| mlua::Error::RuntimeError("invalid_upgrade: filter must not be negative".into()))?,
                requires: names("requires")?,
                unlocks: names("unlocks")?,
            };
            w.borrow_mut().define_upgrade(&def).map_err(|why| mlua::Error::RuntimeError(format!("invalid_upgrade: {name}: {why}")))?;
            buf.borrow_mut().push(Cmd::DefineUpgrade { spec: def });
            Ok(name)
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_grant_upgrade",
        lua.create_function(move |_, (player, name): (i64, String)| {
            let player = player_arg(player)?;
            if w.borrow().research_index(&name).is_none() {
                return Err(mlua::Error::RuntimeError(format!("unknown_upgrade: {name}")));
            }
            buf.borrow_mut().push(Cmd::GrantUpgrade { player, name });
            Ok(())
        })?,
    )?;
    let w = world.clone();
    game.set(
        "time",
        lua.create_function(move |_, ()| {
            let ticks = w.borrow().ticks();
            Ok((i64::from(ticks), f64::from(ticks) / f64::from(TICKS_PER_SECOND)))
        })?,
    )?;
    let w = world.clone();
    game.set(
        "unit_cooldown",
        lua.create_function(move |_, (unit, slot): (i64, i64)| {
            let sim = w.borrow();
            let u = known_unit(&sim, unit)?;
            let slot = usize::try_from(slot).ok().filter(|&s| s < ABILITY_SLOTS).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_slot: {slot} is outside 0..{ABILITY_SLOTS}"))
            })?;
            Ok(i64::from(sim.ability_cooldown(u, slot)))
        })?,
    )?;
    let w = world.clone();
    game.set(
        "unit_abilities",
        lua.create_function(move |lua, unit: i64| {
            let sim = w.borrow();
            let u = known_unit(&sim, unit)?;
            let cat = sim.catalog();
            let list = lua.create_table()?;
            let mut n = 0i64;
            for slot in 0..ABILITY_SLOTS {
                let Some(ai) = sim.ability_in_slot(u, slot) else { continue };
                let Some(a) = cat.abilities.get(ai) else { continue };
                let row = lua.create_table()?;
                row.set("slot", slot as i64)?;
                row.set("name", cat.name(a.name))?;
                row.set("cooldown", i64::from(sim.ability_cooldown(u, slot)))?;
                n += 1;
                list.raw_set(n, row)?;
            }
            Ok(list)
        })?,
    )?;
    let w = world.clone();
    game.set(
        "unit_items",
        lua.create_function(move |lua, unit: i64| {
            let sim = w.borrow();
            let u = known_unit(&sim, unit)?;
            let cat = sim.catalog();
            let list = lua.create_table()?;
            let mut n = 0i64;
            for slot in 0..ITEM_SLOTS {
                let (kind, count) = sim.inventory_slot(u, slot);
                let Some(item) = (kind as usize).checked_sub(1).and_then(|i| cat.items.get(i)) else { continue };
                let row = lua.create_table()?;
                row.set("slot", slot as i64)?;
                row.set("name", cat.name(item.name))?;
                row.set("count", i64::from(count))?;
                row.set("cooldown", i64::from(sim.item_cooldown(u, slot)))?;
                n += 1;
                list.raw_set(n, row)?;
            }
            Ok(list)
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_give_ability",
        lua.create_function(move |_, (unit, slot, name): (i64, i64, String)| {
            let sim = w.borrow();
            let u = live_unit(&sim, unit)?;
            let slot = usize::try_from(slot).ok().filter(|&s| s < ABILITY_SLOTS).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_slot: {slot} is outside 0..{ABILITY_SLOTS}"))
            })?;
            let cat = sim.catalog();
            if !cat.abilities.iter().any(|a| cat.name(a.name) == name) {
                return Err(mlua::Error::RuntimeError(format!("unknown_ability: {name}")));
            }
            buf.borrow_mut().push(Cmd::GiveAbility { unit: u, slot, name });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "cmd_add_gold",
        lua.create_function(move |_, (player, amount): (i64, i64)| {
            let player = player_arg(player)?;
            let amount = i32::try_from(amount).map_err(|_| mlua::Error::RuntimeError(format!("invalid_amount: {amount} out of i32 range")))?;
            buf.borrow_mut().push(Cmd::AddGold { player, amount });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_add_hero_gold",
        lua.create_function(move |_, (unit, amount): (i64, i64)| {
            let u = live_unit(&w.borrow(), unit)?;
            let amount = i32::try_from(amount).map_err(|_| mlua::Error::RuntimeError(format!("invalid_amount: {amount} out of i32 range")))?;
            buf.borrow_mut().push(Cmd::AddHeroGold { unit: u, amount });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "message",
        lua.create_function(move |_, (player, text, seconds): (i64, String, Option<f64>)| {
            let body = shared::ui_message(player, &text, seconds).map_err(mlua::Error::RuntimeError)?;
            buf.borrow_mut().push(Cmd::Ui { body });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "banner",
        lua.create_function(move |_, (text, seconds): (String, Option<f64>)| {
            let body = shared::ui_banner(&text, seconds).map_err(mlua::Error::RuntimeError)?;
            buf.borrow_mut().push(Cmd::Ui { body });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "panel",
        lua.create_function(move |_, (id, spec): (String, Option<Table>)| {
            let spec = match spec {
                None => None,
                Some(t) => Some((t.get::<Option<String>>("title")?, t.get::<Option<String>>("text")?, t.get::<Option<i64>>("player")?)),
            };
            let body = shared::ui_panel(&id, spec).map_err(mlua::Error::RuntimeError)?;
            buf.borrow_mut().push(Cmd::Ui { body });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "camera",
        lua.create_function(move |_, (player, x, y, zoom, seconds): (i64, f64, f64, Option<f64>, Option<f64>)| {
            let body = shared::ui_camera(player, x, y, zoom, seconds).map_err(mlua::Error::RuntimeError)?;
            buf.borrow_mut().push(Cmd::Ui { body });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "sound",
        lua.create_function(move |_, name: String| {
            buf.borrow_mut().push(Cmd::Ui { body: shared::ui_sound(&name) });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "victory",
        lua.create_function(move |_, player: i64| {
            buf.borrow_mut().push(Cmd::Victory { player: player_arg(player)? });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "defeat",
        lua.create_function(move |_, player: i64| {
            buf.borrow_mut().push(Cmd::Defeat { player: player_arg(player)? });
            Ok(())
        })?,
    )?;
    let w = world.clone();
    game.set("winner", lua.create_function(move |_, ()| Ok(w.borrow().winner().map(i64::from)))?)?;
    let w = world.clone();
    game.set("gold", lua.create_function(move |_, player: i64| Ok(w.borrow().resource_a(player_arg(player)?) as i64))?)?;
    let w = world.clone();
    game.set(
        "hero_gold",
        lua.create_function(move |_, unit: i64| {
            let sim = w.borrow();
            Ok(sim.hero_gold(known_unit(&sim, unit)?) as i64)
        })?,
    )?;
    env.set("game", game)
}

// The living units each player owns, indexed by player id (0..MAX_PLAYERS). game.players reads it.
fn living_per_player(sim: &Sim) -> Vec<i64> {
    let mut counts = vec![0i64; MAX_PLAYERS];
    for i in 0..sim.count() {
        if sim.alive(i) {
            counts[sim.player(i) as usize % MAX_PLAYERS] += 1;
        }
    }
    counts
}

// game.trigger and the event runner. The chunk returns a function over the event-name set: it
// yields (trigger, fire). trigger(event, {conditions = {...}, actions = {...}, action = f})
// registers a trigger; fire(event, ...) runs each trigger of that event: every condition must
// return truthy for the actions to run, and each action gets the same arguments as the event.
const TRIGGER_SRC: &str = r#"
return function(known)
  local triggers = {}
  local function trigger(event, spec)
    if not known[event] then error("unknown_event: " .. tostring(event), 2) end
    if type(spec) ~= "table" then error("invalid_trigger: " .. event .. " needs a table of conditions and actions", 2) end
    local entry = { conditions = spec.conditions or {}, actions = spec.actions or {} }
    if spec.action then entry.actions[#entry.actions + 1] = spec.action end
    if #entry.actions == 0 then error("invalid_trigger: " .. event .. " has no action", 2) end
    triggers[event] = triggers[event] or {}
    table.insert(triggers[event], entry)
    return #triggers[event]
  end
  local function fire(event, ...)
    local list = triggers[event]
    if not list then return end
    for _, entry in ipairs(list) do
      local pass = true
      for _, cond in ipairs(entry.conditions) do
        if not cond(...) then pass = false break end
      end
      if pass then
        for _, act in ipairs(entry.actions) do act(...) end
      end
    end
  end
  return trigger, fire
end
"#;


const VARS_KEY: &str = "oscraft_vars";

// pcall and xpcall for mod scripts. They catch script errors, but a limit error (instruction,
// memory) is rethrown, so a script cannot keep spinning past its budget by catching the limit.
const PCALL_SRC: &str = r#"
local native_pcall, native_xpcall = pcall, xpcall
local function limit_error(e)
  return type(e) == "string" and (string.find(e, "instruction_limit", 1, true) ~= nil
    or string.find(e, "memory_limit", 1, true) ~= nil or string.find(e, "not enough memory", 1, true) ~= nil)
end
local function pass(ok, ...)
  if not ok and limit_error((...)) then error((...), 0) end
  return ok, ...
end
return function(f, ...) return pass(native_pcall(f, ...)) end,
  function(f, handler) return pass(native_xpcall(f, handler)) end
"#;

// Builds a fresh environment, runs the chunk in it, and returns its on_tick and systems.
// Errors carry "<path>:<line>: <message>" because the chunk is named "@<path>".
fn compile(lua: &Lua, world: &Shared, regions: &Regions, timers: &Timers, clock: &Clock, cmds: &Cmds, path: &Path) -> mlua::Result<Hooks> {
    let src = fs::read_to_string(path).map_err(io_err)?;
    compile_src(lua, world, regions, timers, clock, cmds, &path.display().to_string(), &src)
}

fn compile_src(lua: &Lua, world: &Shared, regions: &Regions, timers: &Timers, clock: &Clock, cmds: &Cmds, name: &str, src: &str) -> mlua::Result<Hooks> {
    let env = lua.create_table()?;
    let (pcall, xpcall) = lua.load(PCALL_SRC).eval::<(Function, Function)>()?;
    env.set("pcall", pcall)?;
    env.set("xpcall", xpcall)?;
    let globals = lua.globals();
    for name in ["error", "assert", "type", "tostring", "tonumber", "select", "pairs", "ipairs", "next"] {
        env.set(name, globals.get::<mlua::Value>(name)?)?;
    }
    let systems: SystemSink = Rc::new(RefCell::new(Vec::new()));
    bind_host(lua, &env, world, regions, timers, clock)?;
    bind_game(lua, &env, world, cmds, &systems)?;
    let known = lua.create_table()?;
    for (event, _) in EVENTS {
        known.set(event, true)?;
    }
    let (trigger, fire) = lua.load(TRIGGER_SRC).eval::<Function>()?.call::<(Function, Function)>(known)?;
    let game: Table = env.get("game")?;
    game.set("trigger", trigger)?;
    let vars: Table = lua.named_registry_value(VARS_KEY)?;
    lua.load(PRELUDE_SRC).eval::<Function>()?.call::<()>((game, vars, env.clone(), MAX_PLAYERS as i64))?;
    check_host_globals(&env)?;
    lua.load(src)
        .set_name(format!("@{name}"))
        .set_environment(env.clone())
        .exec()?;
    let on_tick = env
        .get::<Option<Function>>("on_tick")?
        .ok_or_else(|| mlua::Error::RuntimeError(format!("{name}: on_tick is not defined")))?;
    let registered = std::mem::take(&mut *systems.borrow_mut());
    Ok(Hooks {
        on_tick,
        on_spawned: env.get("on_spawned")?,
        on_attacked: env.get("on_attacked")?,
        on_died: env.get("on_died")?,
        on_enter: env.get("on_enter")?,
        on_leave: env.get("on_leave")?,
        on_player_joined: env.get("on_player_joined")?,
        fire,
        systems: registered,
    })
}

// Refuses a mod environment whose host globals and HOST_GLOBALS disagree, in either direction.
fn check_host_globals(env: &Table) -> mlua::Result<()> {
    for name in HOST_GLOBALS {
        if !env.contains_key(*name)? {
            return Err(mlua::Error::RuntimeError(format!("host_globals: {name} is listed but not registered")));
        }
    }
    for pair in env.pairs::<String, mlua::Value>() {
        let (name, _) = pair?;
        if !HOST_GLOBALS.contains(&name.as_str()) && !BASE_ENV.contains(&name.as_str()) {
            return Err(mlua::Error::RuntimeError(format!("host_globals: {name} is registered but not listed")));
        }
    }
    Ok(())
}

fn mtime(path: &Path) -> std::io::Result<SystemTime> {
    fs::metadata(path)?.modified()
}

impl ModHost {
    /// A host over the sim loaded from `image`, an OSDT v3 image (rust/crates/data/FORMAT.md),
    /// on the image's own map. `state.ticks` starts at 0.
    pub fn new(image: &[u8]) -> Result<Self, LoadError> {
        let lua = Lua::new();
        let spent = Rc::new(Cell::new(0u64));
        let budget = Rc::new(Cell::new(DEFAULT_INSTRUCTIONS / HOOK_STRIDE as u64));
        let (s, b) = (spent.clone(), budget.clone());
        lua.set_hook(HookTriggers::new().every_nth_instruction(HOOK_STRIDE), move |_, _| {
            let n = s.get() + 1;
            s.set(n);
            if n > b.get() {
                return Err(mlua::Error::RuntimeError(format!(
                    "instruction_limit: over {} instructions in one call",
                    b.get() * HOOK_STRIDE as u64
                )));
            }
            Ok(VmState::Continue)
        });
        lua.set_memory_limit(DEFAULT_MEMORY).expect("set memory limit");
        let state = lua.create_table().expect("create state table");
        state.set("ticks", 0i64).expect("seed state.ticks");
        let vars = lua.create_table().expect("create vars table");
        for scope in ["global", "player", "unit", "score"] {
            vars.set(scope, lua.create_table().expect("create vars scope")).expect("seed vars scope");
        }
        lua.set_named_registry_value(VARS_KEY, vars).expect("store vars table");
        let world = Rc::new(RefCell::new(sim::load(image, MAX_UNITS)?));
        let regions: Regions = Rc::new(RefCell::new(Vec::with_capacity(MAX_REGIONS)));
        Ok(ModHost {
            lua,
            world,
            state,
            regions,
            timers: Rc::new(RefCell::new(Vec::with_capacity(MAX_TIMERS))),
            clock: Rc::new(Cell::new(0)),
            spent,
            budget,
            inside: vec![0; MAX_UNITS],
            snap: Vec::with_capacity(MAX_UNITS),
            prev: Vec::with_capacity(MAX_UNITS),
            pending: Vec::with_capacity(MAX_UNITS * MAX_REGIONS),
            mod_file: None,
            staged: None,
            version: 0,
            cmds: Rc::new(RefCell::new(Vec::new())),
            joined: Vec::new(),
        })
    }

    /// Queues a player-joined event for the next step. False when `player` is outside 0..MAX_PLAYERS.
    pub fn player_joined(&mut self, player: usize) -> bool {
        if player >= MAX_PLAYERS {
            return false;
        }
        self.joined.push(player);
        true
    }

    /// Compiles the mod at `path` and makes it the active mod.
    pub fn load(&mut self, path: &Path) -> mlua::Result<()> {
        self.spent.set(0);
        let hooks = compile(&self.lua, &self.world, &self.regions, &self.timers, &self.clock, &self.cmds, path).map_err(named)?;
        self.mod_file = Some(ModFile { path: path.to_path_buf(), mtime: mtime(path).map_err(io_err)?, hooks });
        self.staged = None;
        self.version += 1;
        Ok(())
    }

    /// Compiles the mod in `src`, named `name` in error messages, and makes it the active mod.
    /// The source has no file, so `reload` finds no file under `name` and reports it.
    pub fn load_source(&mut self, name: &str, src: &str) -> mlua::Result<()> {
        self.spent.set(0);
        let hooks = compile_src(&self.lua, &self.world, &self.regions, &self.timers, &self.clock, &self.cmds, name, src).map_err(named)?;
        self.mod_file = Some(ModFile { path: PathBuf::from(name), mtime: SystemTime::UNIX_EPOCH, hooks });
        self.staged = None;
        self.version += 1;
        Ok(())
    }

    /// Replaces the sim with one built from `image`, an OSDT image as `new` takes. The mod's next
    /// read of the catalog sees the new values; the units of the old sim are gone.
    pub fn replace_image(&mut self, image: &[u8]) -> Result<(), LoadError> {
        let next = sim::load(image, MAX_UNITS)?;
        *self.world.borrow_mut() = next;
        Ok(())
    }

    /// The sim this host advances. Mutable so a caller can command or trigger units.
    pub fn sim(&self) -> RefMut<'_, Sim> {
        self.world.borrow_mut()
    }

    /// The reload entry point. Compiles the mod file when its mtime moved, off the tick path.
    /// A success is staged and goes live at the next advance; a failure clears any staged
    /// version, keeps the live one in effect, and returns the error with file and line.
    /// Returns Ok(true) when a new version was staged.
    pub fn reload(&mut self) -> Result<bool, ReloadError> {
        let Some(m) = self.mod_file.as_mut() else { return Ok(false) };
        let t = match mtime(&m.path) {
            Ok(t) => t,
            Err(e) => return Err(ReloadError { file: m.path.clone(), line: None, message: e.to_string() }),
        };
        if t == m.mtime {
            return Ok(false);
        }
        m.mtime = t;
        let path = m.path.clone();
        self.spent.set(0);
        let saved = self.save_live();
        match compile(&self.lua, &self.world, &self.regions, &self.timers, &self.clock, &self.cmds, &path) {
            Ok(hooks) => {
                self.staged = Some(hooks);
                Ok(true)
            }
            Err(e) => {
                self.staged = None;
                if let Err(r) = self.restore_live(saved) {
                    return Err(ReloadError { file: path, line: None, message: format!("rollback failed: {r}") });
                }
                Err(ReloadError::locate(&path, &e))
            }
        }
    }

    // The top-level code of a new chunk runs against the live world, so a reload that fails
    // after it ran must put these back. Taken only when a reload is attempted.
    fn save_live(&self) -> Saved {
        Saved {
            sim: self.world.borrow().save_state(),
            regions: self.regions.borrow().clone(),
            timers: self.timers.borrow().clone(),
            cmds: self.cmds.borrow().clone(),
        }
    }

    fn restore_live(&self, saved: Saved) -> mlua::Result<()> {
        self.world
            .borrow_mut()
            .load_state(&saved.sim)
            .map_err(|_| mlua::Error::RuntimeError("the saved sim state did not load back".to_string()))?;
        *self.regions.borrow_mut() = saved.regions;
        *self.timers.borrow_mut() = saved.timers;
        *self.cmds.borrow_mut() = saved.cmds;
        Ok(())
    }

    /// The live mod version: 1 after the first load, +1 for each reload that went live.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Calls reload and prints the outcome.
    pub fn reload_if_changed(&mut self) {
        match self.reload() {
            Ok(true) => println!("ev=mod_reload_staged path={:?}", self.mod_file.as_ref().map(|m| m.path.display().to_string()).unwrap_or_default()),
            Ok(false) => {}
            Err(e) => println!("ev=mod_reload_failed error={:?}", e.to_string()),
        }
    }

    /// Runs the active mod's on_tick once. A runtime error is returned with file and line.
    /// A staged reload goes live in advance, not here.
    pub fn tick(&self, state: &Table) -> mlua::Result<()> {
        self.spent.set(0);
        self.run_on_tick(state).map_err(named)
    }

    fn run_on_tick(&self, state: &Table) -> mlua::Result<()> {
        match &self.mod_file {
            Some(m) => {
                m.hooks.on_tick.call::<()>(state.clone())?;
                m.hooks.fire.call::<()>(("on_tick", state.clone()))
            }
            None => Ok(()),
        }
    }

    /// Caps each step (one advance, or one load or reload) at `instructions` Lua
    /// instructions and the Lua heap at `memory_bytes`. A step over either stops with a
    /// named error; the next step starts with a fresh budget.
    pub fn set_limits(&mut self, instructions: u64, memory_bytes: usize) -> mlua::Result<()> {
        self.budget.set(instructions / HOOK_STRIDE as u64);
        self.lua.set_memory_limit(memory_bytes)?;
        Ok(())
    }

    /// One sim tick, then trigger zone transitions for the new unit positions, then
    /// on_tick. Transitions are emitted in unit index order, then region index order.
    pub fn advance(&mut self, state: &Table) -> mlua::Result<()> {
        self.spent.set(0);
        self.step(state).map_err(named)
    }

    // One step in fixed order: the sim tick, zone and unit events, every system in
    // registration order, the queued commands, due timers, then on_tick. A command queued by
    // on_tick or a timer waits for the command step of the next advance.
    // Swaps a staged reload in at the tick boundary, before the step's sim tick.
    fn apply_staged(&mut self) {
        if let Some(hooks) = self.staged.take() {
            if let Some(m) = self.mod_file.as_mut() {
                m.hooks = hooks;
                self.version += 1;
            }
        }
    }

    fn step(&mut self, state: &Table) -> mlua::Result<()> {
        self.apply_staged();
        self.clock.set(self.clock.get() + 1);
        self.world.borrow_mut().tick();
        self.collect();
        self.dispatch()?;
        self.run_systems()?;
        self.apply_commands();
        self.fire_timers()?;
        self.run_on_tick(state)
    }

    // Each system runs over every living unit, with the snapshot taken as it begins.
    fn run_systems(&self) -> mlua::Result<()> {
        let Some(m) = self.mod_file.as_ref() else { return Ok(()) };
        for sys in m.hooks.systems.iter() {
            for row in snapshot(&self.world) {
                sys.func.call::<()>(view(&self.lua, &row, &sys.comps)?)?;
            }
        }
        Ok(())
    }

    // Applies the queued commands in the order they were queued. A command aimed at a unit
    // that died since it was queued does nothing.
    fn apply_commands(&self) {
        let list = std::mem::take(&mut *self.cmds.borrow_mut());
        let mut sim = self.world.borrow_mut();
        for cmd in list {
            match cmd {
                Cmd::Spawn { index, x, y, .. } => {
                    let _ = sim.spawn(index as u32, x, y);
                }
                // Handles resolve in the server room, which holds the handle map; this local world has none.
                Cmd::OrderHandle { .. } | Cmd::DamageHandle { .. } | Cmd::BuffHandle { .. } => {}
                Cmd::Order { unit, x, y } => {
                    if sim.alive(unit) {
                        sim.order(unit, x, y);
                    }
                }
                Cmd::Damage { caster, target, dmg } => {
                    if sim.alive(caster) {
                        let player = sim.player(caster);
                        sim.damage_target(caster as i32, player, dmg, target as i32);
                    }
                }
                Cmd::Buff { unit, kind, ticks } => {
                    if sim.alive(unit) {
                        let _ = sim.apply_buff(unit, kind, ticks);
                    }
                }
                // A refused cast or attack does nothing, as a refused order does.
                Cmd::Cast { unit, slot, target, x, y } => {
                    if sim.alive(unit) {
                        let _ = sim.cast_ability(unit, slot, target, x, y);
                    }
                }
                Cmd::Attack { unit, target } => {
                    if sim.alive(unit) {
                        let _ = sim.order_attack_or_approach(unit, target);
                    }
                }
                Cmd::UseItem { unit, slot, target, x, y } => {
                    if sim.alive(unit) {
                        let _ = sim.use_item(unit, slot, target, x, y);
                    }
                }
                Cmd::Buy { unit, item } => {
                    if sim.alive(unit) {
                        let _ = sim.buy_item(unit, item);
                    }
                }
                Cmd::SkillUp { unit, slot } => {
                    if sim.alive(unit) {
                        let _ = sim.skill_up(unit, slot);
                    }
                }
                Cmd::Sell { unit, slot } => {
                    if sim.alive(unit) {
                        let _ = sim.sell_item(unit, slot);
                    }
                }
                Cmd::Unstash { unit, slot } => {
                    if sim.alive(unit) {
                        let _ = sim.unstash(unit, slot);
                    }
                }
                Cmd::SetOwner { unit, player } => {
                    let _ = sim.set_owner(unit, player);
                }
                Cmd::SetFog { on } => sim.set_fog(on),
                Cmd::SetTeam { player, team } => {
                    let _ = sim.set_team(player, team);
                }
                Cmd::SetAlliance { a, b, kind } => {
                    let _ = sim.set_relation(a, b, kind);
                }
                Cmd::AddPlayer { player, controller } => {
                    let _ = sim.add_player(player, controller);
                }
                Cmd::RemovePlayer { player } => {
                    let _ = sim.remove_player(player);
                }
                // game.ability, unit, item and upgrade already defined theirs in this world when the mod called them.
                Cmd::DefineAbility { .. } | Cmd::DefineUnit { .. } | Cmd::DefineItem { .. } | Cmd::DefineUpgrade { .. } => {}
                Cmd::GrantUpgrade { player, name } => {
                    if let Some(r) = sim.research_index(&name) {
                        sim.grant_research(player, r);
                    }
                }
                Cmd::GiveAbility { unit, slot, name } => {
                    let _ = sim.give_ability(unit, slot, &name);
                }
                Cmd::AddGold { player, amount } => sim.add_resources(player, amount, 0),
                Cmd::AddHeroGold { unit, amount } => {
                    let _ = sim.add_hero_gold(unit, amount);
                }
                Cmd::Victory { player } => sim.declare_victory(player),
                Cmd::Defeat { player } => sim.declare_defeat(player),
                // UI events live in the server's event stream; the host world has nothing to change.
                Cmd::Ui { .. } => {}
            }
        }
    }

    // Fires every due timer in insertion order. A timer is removed before its action
    // runs, so an action that schedules again cannot fire twice in one advance.
    fn fire_timers(&self) -> mlua::Result<()> {
        let now = self.clock.get();
        loop {
            let due = self.timers.borrow().iter().position(|t| t.due <= now);
            let Some(i) = due else { return Ok(()) };
            let timer = self.timers.borrow_mut().remove(i);
            let action: Function = self.lua.registry_value(&*timer.action)?;
            action.call::<()>(())?;
        }
    }

    fn collect(&mut self) {
        self.snap.clear();
        self.pending.clear();
        self.pending.extend(self.joined.drain(..).map(|player| Pending::PlayerJoined { player }));
        {
            let sim = self.world.borrow();
            for i in 0..sim.count() {
                let (x, y) = sim.position_fixed(i);
                self.snap.push(UnitSnap { x, y, hp: sim.hp(i), alive: sim.alive(i), killer: sim.killer_of(i) });
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
                        self.pending.push(Pending::Died { unit, killer: s.killer });
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
                    hooks.fire.call::<()>(("on_spawned", unit as i64))?;
                }
                Pending::Attacked { unit } => {
                    if let Some(h) = &hooks.on_attacked {
                        h.call::<()>(unit as i64)?;
                    }
                    hooks.fire.call::<()>(("on_attacked", unit as i64))?;
                }
                Pending::Died { unit, killer } => {
                    let killer_player = killer.map(|k| i64::from(self.world.borrow().player(k)));
                    let killer = killer.map(|k| k as i64);
                    if let Some(h) = &hooks.on_died {
                        h.call::<()>((unit as i64, killer, killer_player))?;
                    }
                    hooks.fire.call::<()>(("on_died", unit as i64, killer, killer_player))?;
                }
                Pending::Enter { unit, region } => {
                    if let Some(name) = self.region_name(region) {
                        if let Some(h) = &hooks.on_enter {
                            h.call::<()>((name.clone(), unit as i64))?;
                        }
                        hooks.fire.call::<()>(("on_enter", name, unit as i64))?;
                    }
                }
                Pending::Leave { unit, region } => {
                    if let Some(name) = self.region_name(region) {
                        if let Some(h) = &hooks.on_leave {
                            h.call::<()>((name.clone(), unit as i64))?;
                        }
                        hooks.fire.call::<()>(("on_leave", name, unit as i64))?;
                    }
                }
                Pending::PlayerJoined { player } => {
                    if let Some(h) = &hooks.on_player_joined {
                        h.call::<()>(player as i64)?;
                    }
                    hooks.fire.call::<()>(("on_player_joined", player as i64))?;
                }
            }
        }
        Ok(())
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// The commands queued since the last take, in queue order. `advance` applies its own queue;
    /// a caller that forwards commands takes them here after advance.
    pub fn take_commands(&mut self) -> Vec<Cmd> {
        std::mem::take(&mut *self.cmds.borrow_mut())
    }

    pub fn state(&self) -> &Table {
        &self.state
    }

    /// Units the sim has spawned, including dead ones (`Sim::count`).
    pub fn unit_count(&self) -> usize {
        self.world.borrow().count()
    }

    /// The named regions the live mod has defined, in definition order.
    pub fn region_names(&self) -> Vec<String> {
        self.regions.borrow().iter().map(|g| g.name.clone()).collect()
    }

    /// The live timers the mod has scheduled and not yet fired or cancelled.
    pub fn timer_count(&self) -> usize {
        self.timers.borrow().len()
    }

    /// The sim's FNV-1a state hash (sim/src/hash.rs). Two runs of the same mod from
    /// identical state agree on it.
    pub fn state_hash(&self) -> u64 {
        self.world.borrow().state_hash()
    }
}
#[cfg(all(test, feature = "build"))]
mod tests {
    use super::*;
    use crate::build_image;

    fn moba_image() -> Vec<u8> {
        let scenario = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("moba");
        build_image(&scenario).expect("moba image")
    }

    // Two creeps on the middle lane (y = 30), one per side and 22 tiles apart, past each other's sight, so only a command makes them fight.
    fn host_with_creeps() -> ModHost {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        let (a, b) = {
            let mut sim = host.sim();
            let creep = sim.catalog().unit_index("creep").expect("moba has a creep") as u32;
            let a = sim.spawn_for(0, creep, sim::fixed::from_int(14), sim::fixed::from_int(30)).expect("creep a");
            let b = sim.spawn_for(1, creep, sim::fixed::from_int(36), sim::fixed::from_int(30)).expect("creep b");
            (a, b)
        };
        host.state().set("attacker", a as i64).expect("state");
        host.state().set("target", b as i64).expect("state");
        host
    }

    // A command queued from on_tick applies in the next advance, so the tests advance once to queue and once more to apply.
    // The sim never chases: a commanded attack sends the attacker one A* leg toward its target, so the mod
    // re-issues it every tick until the target is in weapon reach.
    #[test]
    fn cmd_attack_moves_the_attacker_into_weapon_reach_of_the_target() {
        let mut host = host_with_creeps();
        host.sim().set_fog(false);
        host.load_source("attack", r#"
            function on_tick(state)
              game.cmd_attack(state.attacker, state.target)
            end
        "#).expect("mod loads");
        let (a, b) = (host.state().get::<i64>("attacker").expect("attacker") as usize, host.state().get::<i64>("target").expect("target") as usize);
        let gap = |sim: &Sim| {
            let ((ax, ay), (bx, by)) = (sim.position_fixed(a), sim.position_fixed(b));
            let (dx, dy) = (i64::from(ax) - i64::from(bx), i64::from(ay) - i64::from(by));
            ((dx * dx + dy * dy) as f64).sqrt() / 65536.0
        };
        let before = gap(&host.sim());
        for _ in 0..120 {
            host.advance(&host.state().clone()).expect("step");
        }
        let after = gap(&host.sim());
        assert!(before > 15.0 && after < 5.0, "the attacker did not close on the target: {before:.2} -> {after:.2} tiles");
    }

    // With fog on, the side 0 creep cannot see the side 1 creep 22 tiles away, so the sim refuses the attack.
    #[test]
    fn cmd_attack_on_an_unseen_target_is_refused_by_the_sim() {
        let mut host = host_with_creeps();
        host.load_source("attack", r#"
            function on_tick(state)
              game.cmd_attack(state.attacker, state.target)
            end
        "#).expect("mod loads");
        host.advance(&host.state().clone()).expect("queue");
        host.advance(&host.state().clone()).expect("apply");
        assert!(matches!(host.sim().stats.last_refusal, Refusal::InvalidOrderTarget), "wrong refusal: {:?}", host.sim().stats.last_refusal);
    }

    // The moba scenario's answer_attack behaviour runs when a unit is hit. The engine runs its attack_nearest action,
    // so no behaviour is refused as unsupported.
    #[test]
    fn answer_attack_runs_and_is_not_refused_as_unsupported() {
        let mut host = host_with_creeps();
        let a = host.state().get::<i64>("attacker").expect("attacker") as usize;
        {
            let mut sim = host.sim();
            sim.set_fog(false);
            sim.hurt(a, 1);
        }
        host.load_source("idle", "function on_tick(state) end\n").expect("mod loads");
        host.advance(&host.state().clone()).expect("step");
        let unsupported = host.sim().stats.refused_by[sim::Refusal::ActionUnsupported as usize];
        assert_eq!(unsupported, 0, "a behaviour was refused as unsupported");
    }

    #[test]
    fn cmd_set_owner_moves_the_unit_to_the_named_player() {
        let mut host = host_with_creeps();
        let a = host.state().get::<i64>("attacker").expect("attacker") as usize;
        host.load_source("owner", r#"
            function on_tick(state)
              game.cmd_set_owner(state.attacker, 1)
            end
        "#).expect("mod loads");
        host.advance(&host.state().clone()).expect("queue");
        host.advance(&host.state().clone()).expect("apply");
        assert_eq!(host.sim().player(a), 1);
    }

    #[test]
    fn cmd_set_owner_refuses_a_player_past_the_limit_by_name() {
        let mut host = host_with_creeps();
        host.load_source("owner", r#"
            function on_tick(state)
              game.cmd_set_owner(state.attacker, 99)
            end
        "#).expect("mod loads");
        let err = host.advance(&host.state().clone()).expect_err("player 99 must be refused");
        assert!(err.to_string().contains("invalid_player"), "wrong error: {err}");
    }

    #[test]
    fn cmd_set_fog_turns_fog_off_in_the_sim() {
        let mut host = host_with_creeps();
        assert!(host.sim().fog_on());
        host.load_source("fog", r#"
            function on_tick(state)
              game.cmd_set_fog(false)
            end
        "#).expect("mod loads");
        host.advance(&host.state().clone()).expect("queue");
        host.advance(&host.state().clone()).expect("apply");
        assert!(!host.sim().fog_on());
    }

    #[test]
    fn cmd_cast_refuses_a_slot_outside_the_ability_slots_by_name() {
        let mut host = host_with_creeps();
        host.load_source("bad_slot", r#"
            function on_tick(state)
              game.cmd_cast(state.attacker, 99, -1, 0, 0)
            end
        "#).expect("mod loads");
        let err = host.advance(&host.state().clone()).expect_err("slot 99 must be refused");
        assert!(err.to_string().contains("invalid_slot"), "wrong error: {err}");
    }

    // A creep has no ability in slot 0, so the sim refuses the cast as NoSlot. Other units idling on the map are refused
    // for their own unlearned skills, so only the NoSlot count shows the queued command reached the sim.
    #[test]
    fn cmd_cast_reaches_the_sim_and_is_refused_when_the_unit_has_no_such_ability() {
        let mut host = host_with_creeps();
        host.load_source("cast", r#"
            function on_tick(state)
              game.cmd_cast(state.attacker, 0, -1, 11 * 65536, 12 * 65536)
            end
        "#).expect("mod loads");
        let no_slot = |host: &ModHost| host.sim().stats.refused_by[Refusal::NoSlot as usize];
        let before = no_slot(&host);
        host.advance(&host.state().clone()).expect("queue");
        host.advance(&host.state().clone()).expect("apply");
        assert_eq!(no_slot(&host), before + 1);
    }

    // Player 2 shares team 0 with player 0 but sees what player 0 cannot: its creep stands beside the target.
    // Shared vision lets player 0 see through it; a plain alliance does not.
    #[test]
    fn shared_vision_shows_player_zero_what_its_ally_sees() {
        let seen_after = |kind: &str| {
            let mut host = host_with_creeps();
            let b = host.state().get::<i64>("target").expect("target") as usize;
            {
                let mut sim = host.sim();
                let creep = sim.catalog().unit_index("creep").expect("creep") as u32;
                sim.spawn_for(2, creep, sim::fixed::from_int(34), sim::fixed::from_int(30)).expect("scout");
            }
            host.load_source("vision", &format!("game.alliance(0, 2, \"{kind}\")\nfunction on_tick(state) end\n")).expect("mod loads");
            for _ in 0..40 {
                host.advance(&host.state().clone()).expect("step");
            }
            let sim = host.sim();
            sim.visible(0, b)
        };
        assert!(!seen_after("ally"), "an alliance alone shared sight");
        assert!(seen_after("vision"), "shared vision did not show the target");
    }

    #[test]
    fn team_and_slot_commands_reach_the_sim_and_refuse_bad_arguments_by_name() {
        let mut host = host_with_creeps();
        host.load_source("teams", r#"
            function on_tick(state)
              game.add_player(5, "computer")
              game.set_team(5, 0)
              state.team5 = game.team(5)
              state.rel = game.alliance(0, 5)
            end
        "#).expect("mod loads");
        host.advance(&host.state().clone()).expect("queue");
        host.advance(&host.state().clone()).expect("apply");
        assert_eq!(host.sim().controller(5), sim::CONTROLLER_COMPUTER);
        assert_eq!(host.state().get::<i64>("team5").expect("team5"), 0);
        assert_eq!(host.state().get::<String>("rel").expect("rel"), "ally");
        for (call, why) in [
            ("game.set_team(0, 99)", "invalid_team"),
            ("game.alliance(0, 1, \"friend\")", "invalid_alliance"),
            ("game.add_player(0, \"robot\")", "invalid_controller"),
            ("game.remove_player(99)", "invalid_player"),
        ] {
            let mut host = host_with_creeps();
            host.load_source("bad", &format!("function on_tick(state) {call} end")).expect("mod loads");
            let err = host.advance(&host.state().clone()).expect_err("bad argument must be refused");
            assert!(err.to_string().contains(why), "{call}: wrong error: {err}");
        }
    }
}
