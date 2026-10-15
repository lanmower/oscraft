//! Lua 5.4 mod host. A mod is a chunk that defines `on_tick(state)`. The host owns
//! `state` and the sim; a mod reload never resets them.

use mlua::{Function, HookTriggers, Lua, RegistryKey, Table, VmState};
use sim::{Catalog, Doodad, LoadError, Refusal, Sim};
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

const MAX_UNITS: usize = 1024;
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
enum Cmd {
    Spawn { index: usize, x: i32, y: i32 },
    Order { unit: usize, x: i32, y: i32 },
    Damage { caster: usize, target: usize, dmg: i32 },
    Buff { unit: usize, kind: i32, ticks: i32 },
}

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

// The action is a Lua function held in the registry. A timer is due when the host
// clock reaches its due tick; the clock advances once per ModHost::advance.
struct Timer {
    name: String,
    due: u64,
    action: RegistryKey,
}

struct Hooks {
    on_tick: Function,
    on_spawned: Option<Function>,
    on_attacked: Option<Function>,
    on_died: Option<Function>,
    on_enter: Option<Function>,
    on_leave: Option<Function>,
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
    timers: Timers,
    clock: Clock,
    spent: Rc<Cell<u64>>,
    budget: Rc<Cell<u64>>,
    inside: Vec<u64>,
    snap: Vec<UnitSnap>,
    prev: Vec<UnitSnap>,
    pending: Vec<Pending>,
    mod_file: Option<ModFile>,
    cmds: Cmds,
}

// The events a mod handles, each with the Lua arguments it receives. Editors list them.
const EVENTS: [(&str, &str); 5] = [
    ("on_spawned", "unit"),
    ("on_attacked", "unit"),
    ("on_died", "unit"),
    ("on_enter", "region, unit"),
    ("on_leave", "region, unit"),
];

fn io_err(e: std::io::Error) -> mlua::Error {
    mlua::Error::RuntimeError(e.to_string())
}

fn fixed(v: i64) -> mlua::Result<i32> {
    i32::try_from(v).map_err(|_| mlua::Error::RuntimeError(format!("Q16.16 value {v} out of i32 range")))
}

fn invalid_unit(i: i64) -> mlua::Error {
    mlua::Error::RuntimeError(format!("invalid_unit: {i}"))
}

fn out_of_grid(x0: i64, y0: i64, x1: i64, y1: i64, w: i64, h: i64) -> mlua::Error {
    mlua::Error::RuntimeError(format!("out_of_grid: rect ({x0},{y0})-({x1},{y1}) exceeds grid {w}x{h}"))
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
            let action = lua.create_registry_value(action)?;
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
    game.set(
        "cmd_spawn",
        lua.create_function(move |_, (kind, x, y): (String, i64, i64)| {
            let index = w
                .borrow()
                .catalog()
                .unit_index(&kind)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("unknown_unit: {kind}")))?;
            buf.borrow_mut().push(Cmd::Spawn { index, x: fixed(x)?, y: fixed(y)? });
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
    env.set("game", game)
}

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
    lua.load(src.as_str())
        .set_name(format!("@{}", path.display()))
        .set_environment(env.clone())
        .exec()?;
    let on_tick = env
        .get::<Option<Function>>("on_tick")?
        .ok_or_else(|| mlua::Error::RuntimeError(format!("{}: on_tick is not defined", path.display())))?;
    let registered = std::mem::take(&mut *systems.borrow_mut());
    Ok(Hooks {
        on_tick,
        on_spawned: env.get("on_spawned")?,
        on_attacked: env.get("on_attacked")?,
        on_died: env.get("on_died")?,
        on_enter: env.get("on_enter")?,
        on_leave: env.get("on_leave")?,
        systems: registered,
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
            cmds: Rc::new(RefCell::new(Vec::new())),
        })
    }

    /// Compiles the mod at `path` and makes it the active mod.
    pub fn load(&mut self, path: &Path) -> mlua::Result<()> {
        self.spent.set(0);
        let hooks = compile(&self.lua, &self.world, &self.regions, &self.timers, &self.clock, &self.cmds, path).map_err(named)?;
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
        self.spent.set(0);
        match compile(&self.lua, &self.world, &self.regions, &self.timers, &self.clock, &self.cmds, &m.path) {
            Ok(hooks) => {
                m.hooks = hooks;
                println!("reload: ok ({})", m.path.display());
            }
            Err(e) => println!("reload: FAILED, old on_tick kept: {e}"),
        }
    }

    /// Runs the active mod's on_tick once. A runtime error is returned with file and line.
    pub fn tick(&self, state: &Table) -> mlua::Result<()> {
        self.spent.set(0);
        self.run_on_tick(state).map_err(named)
    }

    fn run_on_tick(&self, state: &Table) -> mlua::Result<()> {
        match &self.mod_file {
            Some(m) => m.hooks.on_tick.call::<()>(state.clone()),
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
    fn step(&mut self, state: &Table) -> mlua::Result<()> {
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
                Cmd::Spawn { index, x, y } => {
                    let _ = sim.spawn(index as u32, x, y);
                }
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
            let action: Function = self.lua.registry_value(&timer.action)?;
            action.call::<()>(())?;
        }
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
