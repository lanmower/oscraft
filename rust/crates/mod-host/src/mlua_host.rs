//! Lua 5.4 mod host. A mod is a chunk that defines `on_tick(state)`. The host owns
//! `state` and the sim; a mod reload never resets them.

use mlua::{Function, HookTriggers, Lua, RegistryKey, Table, Value, VmState};
use sim::{controller_code, AbilitySpec, Catalog, Doodad, Effect, ItemSpec, LoadError, Payout, Refusal, Request, Sim, UnitSpec, UpgradeSpec, Verdict, ABILITY_SLOTS, ITEM_SLOTS, MAX_PLAYERS, MOD_ALLIANCES, MOD_CONTROLLERS, NO_COMPONENT, NO_TAKE, TICKS_PER_SECOND, VERDICT_TAKES};
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
    SetClassOverride { unit: usize, class: Option<u8> },
    SetHoldOverride { unit: usize, hold: Option<bool> },
    // A spawn-group unit's respawn timer: Some(s) respawns it s seconds from now, None leaves it dead.
    SetRespawn { unit: usize, delay_s: Option<u32> },
    // A unit's invulnerability for `ticks` ticks from the apply, and its damage reduction in percent.
    SetInvulnerable { unit: usize, ticks: u32 },
    SetReduction { unit: usize, pct: u8 },
    // A unit's healing in whole life per second (0 stops it), the side a mod's on_check_win declares the winner,
    // and a move-to-point order that attacks what it meets on the way.
    SetHealing { unit: usize, hp_per_s: i32 },
    DeclareSide { side: u8 },
    AttackMove { unit: usize, x: i32, y: i32 },
    Spawn { handle: u32, index: usize, x: i32, y: i32, owner: u8, route: i32 },
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
    // A mod's answer to a trade request, applied as the sim applies it (store.rs).
    Trade { verdict: Verdict },
    // Moves every stash item of a unit into its free slots, with no trade check (a delivery).
    PackStash { unit: usize },
    // Saves a unit's order point, and orders the unit back to its saved point.
    SaveOrder { unit: usize, x: i32, y: i32 },
    RestoreOrder { unit: usize },
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
    // Takes a living unit off the map (the sim's remove_unit); a dead or unknown unit changes nothing.
    RemoveUnit { unit: usize },
    // Puts `count` of catalog item `item` into a living unit's inventory (the sim's grant_item).
    GiveItem { unit: usize, item: usize, count: u32 },
    AddGold { player: u8, amount: i32 },
    AddUnitGold { unit: usize, amount: i32 },
    Victory { player: u8 },
    Defeat { player: u8 },
    // A client-facing UI event as a JSON object body; the server stamps id and tick and streams it.
    Ui { body: String },
}

impl Cmd {
    // A trade request is answered by the world that applies it; only its verdict travels to the room.
    fn is_trade_request(&self) -> bool {
        matches!(self, Cmd::Buy { .. } | Cmd::Sell { .. } | Cmd::Unstash { .. })
    }
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
    // on_unit_died(unit, state) returns the respawn decision for a dead unit: nil keeps the Rust timer,
    // false leaves it dead, a number of seconds respawns it after that long.
    on_unit_died: Option<Function>,
    // target_priority(unit, candidates, state) returns the target class (0..3) for a unit that drew aggro;
    // nil keeps the Rust rule, false clears an earlier class.
    target_priority: Option<Function>,
    // on_march(unit, player, state) returns the x, y point a route-ended or summoned unit moves to (nil keeps it
    // where it stands). on_cast(unit, ability, state) runs when a cast takes hold. on_check_win(state) returns the
    // winning side, or nil while no side has won. A mod without these callbacks has no march, no cast reaction
    // and no win.
    on_march: Option<Function>,
    on_cast: Option<Function>,
    on_check_win: Option<Function>,
    on_enter: Option<Function>,
    on_leave: Option<Function>,
    on_player_joined: Option<Function>,
    // The store rule. on_buy(unit, item, state) returns true, price, takes or false, reason;
    // on_sell(unit, slot, state) returns true, refund or false, reason; on_unstash(unit, slot, state)
    // returns true or false, reason. A reason is "no-item", "no-shop", "cannot-afford", "inventory-full"
    // or "missing-component". A mod without a rule refuses that trade.
    on_buy: Option<Function>,
    on_sell: Option<Function>,
    on_unstash: Option<Function>,
    // on_deliver(caster, target, reach_milli, state) runs a courier's own logic. It returns nothing.
    on_deliver: Option<Function>,
    // Runs every trigger registered on the event name it is called with (game.trigger).
    fire: Function,
    systems: Vec<System>,
    // Set by the mod's top-level `GOLD_BY_MOD = true`: the sim's kill payout belongs to the mod.
    gold_by_mod: bool,
}

struct ModFile {
    path: PathBuf,
    mtime: SystemTime,
    hooks: Hooks,
}

// One death as on_died receives it: a unit id, the killer's and the victim's players, the bounty.
// Built from the mod's own world, or from a room's payable death (see feed_payout).
struct Death {
    victim: u32,
    victim_player: u8,
    killer: Option<u32>,
    killer_player: Option<u8>,
    bounty: i32,
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
    forward: RefCell<Vec<Cmd>>,
    // Commands an on_died of this world's own deaths queued. They apply to this world but are not
    // forwarded: their unit indices belong to this world, not to the room.
    local: RefCell<Vec<Cmd>>,
    joined: Vec<usize>,
    gold_by_mod: bool,
}

// The largest respawn delay on_unit_died may return, in seconds (one day).
const MAX_RESPAWN_S: i64 = 86_400;

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
    "region_bounds",
    "map_size",
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

// A point coordinate a mod returned: an integer in i32 range, or a whole-number float.
fn point_coord(v: &Value) -> Option<i32> {
    match v {
        Value::Integer(n) => i32::try_from(*n).ok(),
        Value::Number(n) if n.fract() == 0.0 => i32::try_from(*n as i64).ok(),
        _ => None,
    }
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
    let r = regions.clone();
    env.set(
        "region_bounds",
        lua.create_function(move |_, name: String| {
            let regions = r.borrow();
            let g = regions.iter().find(|g| g.name == name).ok_or_else(|| unknown_region(&name))?;
            Ok((i64::from(g.x0), i64::from(g.y0), i64::from(g.x1), i64::from(g.y1)))
        })?,
    )?;
    let mw = world.clone();
    env.set(
        "map_size",
        lua.create_function(move |_, ()| {
            let sim = mw.borrow();
            Ok(((sim.grid().w as i64) << 16, (sim.grid().h as i64) << 16))
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
        lua.create_function(move |_, (kind, x, y, owner, route): (String, i64, i64, Option<i64>, Option<i64>)| {
            let index = w
                .borrow()
                .catalog()
                .unit_index(&kind)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("unknown_unit: {kind}")))?;
            let owner = player_arg(owner.unwrap_or(0))?;
            let route = route.unwrap_or(0);
            let route = i32::try_from(route).map_err(|_| mlua::Error::RuntimeError(format!("invalid_route: {route}")))?;
            let handle = next_handle.get() + 1;
            next_handle.set(handle);
            buf.borrow_mut().push(Cmd::Spawn { handle, index, x: fixed(x)?, y: fixed(y)?, owner, route });
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
    let (w, buf) = (world.clone(), cmds.clone());
    // set_target_class(unit, class) / clear_target_class(unit): queued, applied at the next tick start.
    game.set(
        "set_target_class",
        lua.create_function(move |_, (unit, class): (i64, i64)| {
            if !(0..=3).contains(&class) {
                return Err(mlua::Error::RuntimeError(format!("invalid_target_class: {class} is outside 0..=3")));
            }
            let u = live_unit(&w.borrow(), unit)?;
            buf.borrow_mut().push(Cmd::SetClassOverride { unit: u, class: Some(class as u8) });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    // set_attack_hold(unit, bool) / clear_attack_hold(unit): queued, applied at the next tick start.
    game.set(
        "set_attack_hold",
        lua.create_function(move |_, (unit, hold): (i64, bool)| {
            buf.borrow_mut().push(Cmd::SetHoldOverride { unit: unit as usize, hold: Some(hold) });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "clear_attack_hold",
        lua.create_function(move |_, unit: i64| {
            buf.borrow_mut().push(Cmd::SetHoldOverride { unit: unit as usize, hold: None });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "clear_target_class",
        lua.create_function(move |_, unit: i64| {
            buf.borrow_mut().push(Cmd::SetClassOverride { unit: unit as usize, class: None });
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
    let w = world.clone();
    game.set(
        "unit_bounty",
        lua.create_function(move |_, unit: i64| {
            let sim = w.borrow();
            let u = known_unit(&sim, unit)?;
            Ok(i64::from(sim.unit_bounty(u)))
        })?,
    )?;
    let w = world.clone();
    game.set(
        "rival_of",
        lua.create_function(move |_, player: i64| {
            let side = player_arg(player)?;
            let sim = w.borrow();
            Ok(sim.rival_of(side).map(i64::from))
        })?,
    )?;
    // Protection queries and commands for a mod's own rules (a structure's backdoor, a timed shield).
    // Positions and radii are fixed point, as unit_position returns them.
    let w = world.clone();
    game.set(
        "units_near",
        lua.create_function(move |lua, (x, y, radius): (i64, i64, i64)| {
            let (x, y, radius) = (fixed(x)?, fixed(y)?, fixed(radius)?);
            let ids: Vec<i64> = w.borrow().units_near(x, y, radius).into_iter().map(|u| u as i64).collect();
            lua.create_sequence_from(ids)
        })?,
    )?;
    let w = world.clone();
    game.set(
        "is_structure",
        lua.create_function(move |_, i: i64| {
            let sim = w.borrow();
            Ok(sim.is_structure(known_unit(&sim, i)?))
        })?,
    )?;
    let w = world.clone();
    game.set(
        "is_hero",
        lua.create_function(move |_, i: i64| {
            let sim = w.borrow();
            Ok(sim.is_hero(known_unit(&sim, i)?))
        })?,
    )?;
    let w = world.clone();
    game.set(
        "hostile",
        lua.create_function(move |_, (a, b): (i64, i64)| {
            let (a, b) = (player_arg(a)?, player_arg(b)?);
            Ok(w.borrow().hostile(a, b))
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "set_damage_reduction",
        lua.create_function(move |_, (unit, pct): (i64, i64)| {
            let unit = usize::try_from(unit).map_err(|_| invalid_unit(unit))?;
            let pct = u8::try_from(pct).ok().filter(|&p| p <= 100).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_reduction: {pct} is outside 0..=100"))
            })?;
            buf.borrow_mut().push(Cmd::SetReduction { unit, pct });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "set_invulnerable",
        lua.create_function(move |_, (unit, ticks): (i64, i64)| {
            let unit = usize::try_from(unit).map_err(|_| invalid_unit(unit))?;
            let ticks = u32::try_from(ticks).map_err(|_| mlua::Error::RuntimeError(format!("invalid_ticks: {ticks}")))?;
            buf.borrow_mut().push(Cmd::SetInvulnerable { unit, ticks });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "set_healing",
        lua.create_function(move |_, (unit, hp_per_s): (i64, i64)| {
            let unit = usize::try_from(unit).map_err(|_| invalid_unit(unit))?;
            let hp_per_s = i32::try_from(hp_per_s).ok().filter(|&h| h >= 0).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_healing: {hp_per_s} is outside 0..=2147483647"))
            })?;
            buf.borrow_mut().push(Cmd::SetHealing { unit, hp_per_s });
            Ok(())
        })?,
    )?;
    let buf = cmds.clone();
    game.set(
        "declare_side",
        lua.create_function(move |_, side: i64| {
            let side = u8::try_from(side).ok().filter(|&s| (s as usize) < MAX_PLAYERS).ok_or_else(|| {
                mlua::Error::RuntimeError(format!("invalid_side: {side} is outside 0..{MAX_PLAYERS}"))
            })?;
            buf.borrow_mut().push(Cmd::DeclareSide { side });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "attack_move",
        lua.create_function(move |_, (unit, x, y): (i64, i64, i64)| {
            let unit = live_unit(&w.borrow(), unit)?;
            buf.borrow_mut().push(Cmd::AttackMove { unit, x: fixed(x)?, y: fixed(y)? });
            Ok(())
        })?,
    )?;
    let w = world.clone();
    game.set(
        "filter_bits",
        lua.create_function(move |_, unit: i64| {
            let sim = w.borrow();
            Ok(sim.filter_bits(known_unit(&sim, unit)?) as i64)
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
    // timed_boss(name, first_s, min_s, spread_s): a load-time setting of one timed boss's spawn timing (see camp.rs).
    let w = world.clone();
    game.set(
        "timed_boss",
        lua.create_function(move |_, (name, first, min, spread): (String, i64, i64, i64)| {
            let to_u32 = |v: i64| u32::try_from(v).map_err(|_| mlua::Error::RuntimeError("invalid_timed_boss: times must not be negative".into()));
            w.borrow_mut().set_timed_boss(&name, to_u32(first)?, to_u32(min)?, to_u32(spread)?).map_err(|why| mlua::Error::RuntimeError(format!("invalid_timed_boss: {name}: {why}")))
        })?,
    )?;
    // camp_rule(name, value): a load-time setting of one camp, rune or pit timing or reach (see CampRules in camp.rs).
    let w = world.clone();
    game.set(
        "camp_rule",
        lua.create_function(move |_, (name, value): (String, i64)| {
            let value = u32::try_from(value).map_err(|_| mlua::Error::RuntimeError("invalid_camp_rule: value must not be negative".into()))?;
            w.borrow_mut().set_camp_rule(&name, value).map_err(|why| mlua::Error::RuntimeError(format!("invalid_camp_rule: {name}: {why}")))
        })?,
    )?;
    // acquire(name, game_units): a load-time setting of one unit's acquire range (see acquire.rs in the sim).
    let w = world.clone();
    game.set(
        "acquire",
        lua.create_function(move |_, (name, value): (String, i64)| {
            let game_units = u32::try_from(value).map_err(|_| mlua::Error::RuntimeError("invalid_unit: acquire must not be negative".into()))?;
            w.borrow_mut().set_acquire(&name, game_units).map_err(|why| mlua::Error::RuntimeError(format!("invalid_unit: {name}: {why}")))
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
        "unit_level",
        lua.create_function(move |_, unit: i64| {
            let sim = w.borrow();
            let u = known_unit(&sim, unit)?;
            Ok(i64::from(sim.level(u)))
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
                row.set("item", kind as i64 - 1)?;
                row.set("name", cat.name(item.name))?;
                row.set("count", i64::from(count))?;
                row.set("cooldown", i64::from(sim.item_cooldown(u, slot)))?;
                n += 1;
                list.raw_set(n, row)?;
            }
            Ok(list)
        })?,
    )?;
    // The store and courier surface (docs/MODDING.md, Stores and carriers). Reads report the sim's
    // state; the commands queue like the others and take effect in the next command step.
    let w = world.clone();
    game.set(
        "item_info",
        lua.create_function(move |lua, item: i64| {
            let sim = w.borrow();
            let cat = sim.catalog();
            let Some(it) = usize::try_from(item).ok().and_then(|i| cat.items.get(i)) else {
                return Ok(Value::Nil);
            };
            let t = lua.create_table()?;
            t.set("name", cat.name(it.name))?;
            t.set("cost", it.cost)?;
            t.set("charges", i64::from(it.charges))?;
            t.set("stack_max", i64::from(it.stack_max))?;
            t.set("shops", i64::from(it.shops))?;
            t.set("stock_max", i64::from(it.stock_max))?;
            t.set("stock_initial", i64::from(it.stock_initial))?;
            t.set("stock_time_s", i64::from(it.stock_time_s))?;
            t.set("stock_start_s", i64::from(it.stock_start_s))?;
            let comps = lua.create_table()?;
            let mut n = 0i64;
            for &c in it.components.iter().filter(|&&c| c != NO_COMPONENT) {
                n += 1;
                comps.raw_set(n, i64::from(c))?;
            }
            t.set("components", comps)?;
            Ok(Value::Table(t))
        })?,
    )?;
    let w = world.clone();
    game.set(
        "unit_stash",
        lua.create_function(move |lua, unit: i64| {
            let sim = w.borrow();
            let u = known_unit(&sim, unit)?;
            let cat = sim.catalog();
            let list = lua.create_table()?;
            let mut n = 0i64;
            for slot in 0..sim.inventory_slots() {
                let (kind, count) = sim.stash_slot(u, slot);
                let Some(item) = (kind as usize).checked_sub(1).and_then(|i| cat.items.get(i)) else { continue };
                let row = lua.create_table()?;
                row.set("slot", slot as i64)?;
                row.set("item", kind as i64 - 1)?;
                row.set("name", cat.name(item.name))?;
                row.set("count", i64::from(count))?;
                n += 1;
                list.raw_set(n, row)?;
            }
            Ok(list)
        })?,
    )?;
    let w = world.clone();
    game.set(
        "doodads_near",
        lua.create_function(move |lua, (unit, reach): (i64, i64)| {
            let sim = w.borrow();
            let u = known_unit(&sim, unit)?;
            let reach = i32::try_from(reach).map_err(|_| mlua::Error::RuntimeError(format!("invalid_reach: {reach}")))?;
            let list = lua.create_table()?;
            for (n, kind) in sim.doodads_near(u, reach).into_iter().enumerate() {
                list.raw_set(n as i64 + 1, i64::from(kind))?;
            }
            Ok(list)
        })?,
    )?;
    let w = world.clone();
    game.set(
        "within",
        lua.create_function(move |_, (unit, x, y, reach): (i64, i64, i64, i64)| {
            let sim = w.borrow();
            let u = known_unit(&sim, unit)?;
            let reach = i32::try_from(reach).map_err(|_| mlua::Error::RuntimeError(format!("invalid_reach: {reach}")))?;
            Ok(sim.within_reach(u, fixed(x)?, fixed(y)?, reach))
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_pack_stash",
        lua.create_function(move |_, unit: i64| {
            let u = live_unit(&w.borrow(), unit)?;
            buf.borrow_mut().push(Cmd::PackStash { unit: u });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_save_order",
        lua.create_function(move |_, (unit, x, y): (i64, i64, i64)| {
            let u = live_unit(&w.borrow(), unit)?;
            buf.borrow_mut().push(Cmd::SaveOrder { unit: u, x: fixed(x)?, y: fixed(y)? });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_restore_order",
        lua.create_function(move |_, unit: i64| {
            let u = live_unit(&w.borrow(), unit)?;
            buf.borrow_mut().push(Cmd::RestoreOrder { unit: u });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_remove",
        lua.create_function(move |_, unit: i64| {
            let u = live_unit(&w.borrow(), unit)?;
            buf.borrow_mut().push(Cmd::RemoveUnit { unit: u });
            Ok(())
        })?,
    )?;
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_give_item",
        lua.create_function(move |_, (unit, name, count): (i64, String, i64)| {
            let sim = w.borrow();
            let u = live_unit(&sim, unit)?;
            let item = sim.catalog().item_index(&name).ok_or_else(|| mlua::Error::RuntimeError(format!("unknown_item: {name}")))?;
            let count = u32::try_from(count).ok().filter(|&c| c > 0).ok_or_else(|| mlua::Error::RuntimeError(format!("invalid_count: {count} must be 1 or more")))?;
            buf.borrow_mut().push(Cmd::GiveItem { unit: u, item, count });
            Ok(())
        })?,
    )?;
    // game.cmd_set_respawn(unit, seconds | false): the respawn decision for a dead unit (false leaves it dead).
    // The same command on_unit_died returns, so a buyback is a respawn now (0) plus an add_unit_gold.
    let (w, buf) = (world.clone(), cmds.clone());
    game.set(
        "cmd_set_respawn",
        lua.create_function(move |_, (unit, delay): (i64, Value)| {
            let u = known_unit(&w.borrow(), unit)?;
            let delay_s = match delay {
                Value::Boolean(false) => None,
                Value::Integer(n) if (0..=MAX_RESPAWN_S).contains(&n) => Some(n as u32),
                other => return Err(mlua::Error::RuntimeError(format!("invalid_respawn: {}", other.type_name()))),
            };
            buf.borrow_mut().push(Cmd::SetRespawn { unit: u, delay_s });
            Ok(())
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
        "cmd_add_unit_gold",
        lua.create_function(move |_, (unit, amount): (i64, i64)| {
            let u = known_unit(&w.borrow(), unit)?;
            let amount = i32::try_from(amount).map_err(|_| mlua::Error::RuntimeError(format!("invalid_amount: {amount} out of i32 range")))?;
            buf.borrow_mut().push(Cmd::AddUnitGold { unit: u, amount });
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
        "unit_gold",
        lua.create_function(move |_, unit: i64| {
            let sim = w.borrow();
            Ok(sim.unit_gold(known_unit(&sim, unit)?) as i64)
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
        on_unit_died: env.get("on_unit_died")?,
        target_priority: env.get("target_priority")?,
        on_march: env.get("on_march")?,
        on_cast: env.get("on_cast")?,
        on_check_win: env.get("on_check_win")?,
        on_enter: env.get("on_enter")?,
        on_leave: env.get("on_leave")?,
        on_player_joined: env.get("on_player_joined")?,
        on_buy: env.get("on_buy")?,
        on_sell: env.get("on_sell")?,
        on_unstash: env.get("on_unstash")?,
        on_deliver: env.get("on_deliver")?,
        fire,
        systems: registered,
        gold_by_mod: env.get::<Option<bool>>("GOLD_BY_MOD")?.unwrap_or(false),
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
            forward: RefCell::new(Vec::new()),
            local: RefCell::new(Vec::new()),
            joined: Vec::new(),
            gold_by_mod: false,
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
        self.gold_by_mod = hooks.gold_by_mod;
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
        self.gold_by_mod = hooks.gold_by_mod;
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
    // that died since it was queued does nothing. Each applied command is also kept for the
    // caller to forward (take_commands), so the room's sim receives what this world applied.
    fn apply_commands(&self) {
        let local = std::mem::take(&mut *self.local.borrow_mut());
        let list = std::mem::take(&mut *self.cmds.borrow_mut());
        // A trade request is answered in this world and its verdict forwarded to the room, so the
        // request itself is not forwarded: the room would answer it a second time.
        self.forward.borrow_mut().extend(list.iter().filter(|c| !c.is_trade_request()).cloned());
        for (cmd, from_mod) in local.into_iter().map(|c| (c, false)).chain(list.into_iter().map(|c| (c, true))) {
            self.apply_one(cmd, from_mod);
        }
    }

    // Applies one command to this world, then answers the trade requests it queued. A verdict is
    // applied here, and forwarded when the request came from the mod's own commands.
    fn apply_one(&self, cmd: Cmd, from_mod: bool) {
        let requests = {
            let mut sim = self.world.borrow_mut();
            match cmd {
                Cmd::Spawn { index, x, y, owner, route, .. } => {
                    let _ = sim.spawn_mod_unit(owner, index as u32, x, y, route);
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
                Cmd::SetClassOverride { unit, class } => {
                    let _ = sim.set_class_override(unit, class);
                }
                Cmd::SetHoldOverride { unit, hold } => {
                    let _ = sim.set_hold_override(unit, hold);
                }
                Cmd::SetRespawn { unit, delay_s } => {
                    let _ = sim.set_respawn(unit, delay_s);
                }
                Cmd::SetInvulnerable { unit, ticks } => {
                    let _ = sim.set_invulnerable(unit, ticks);
                }
                Cmd::SetReduction { unit, pct } => {
                    let _ = sim.set_damage_reduction(unit, pct);
                }
                Cmd::SetHealing { unit, hp_per_s } => {
                    let _ = sim.set_healing(unit, hp_per_s);
                }
                Cmd::DeclareSide { side } => sim.declare_side(side),
                Cmd::AttackMove { unit, x, y } => {
                    if sim.alive(unit) {
                        let _ = sim.issue_order_kind(unit, sim::ORDER_ATTACK_MOVE, x, y);
                    }
                }
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
                Cmd::RemoveUnit { unit } => {
                    sim.remove_unit(unit);
                }
                Cmd::GiveItem { unit, item, count } => {
                    let _ = sim.grant_item(unit, item, count);
                }
                Cmd::AddGold { player, amount } => sim.add_resources(player, amount, 0),
                Cmd::AddUnitGold { unit, amount } => {
                    let _ = sim.add_unit_gold(unit, amount);
                }
                Cmd::Victory { player } => sim.declare_victory(player),
                Cmd::Defeat { player } => sim.declare_defeat(player),
                // UI events live in the server's event stream; the host world has nothing to change.
                Cmd::Trade { verdict } => {
                    let _ = sim.apply_verdict(&verdict);
                }
                Cmd::PackStash { unit } => {
                    if sim.alive(unit) {
                        sim.pack_stash(unit);
                    }
                }
                Cmd::SaveOrder { unit, x, y } => {
                    let _ = sim.save_order(unit, x, y);
                }
                Cmd::RestoreOrder { unit } => {
                    let _ = sim.restore_order(unit);
                }
                // UI events live in the server's event stream; the host world has nothing to change.
                Cmd::Ui { .. } => {}
            }
            sim.drain_requests()
        };
        for req in requests {
            if let Some(verdict) = self.answer(&req) {
                if from_mod {
                    self.forward.borrow_mut().push(verdict.clone());
                }
                self.apply_one(verdict, false);
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
        // Taken every step, mod or not, so the sim's acquisition log never grows past its cap unread.
        let acquired = self.world.borrow_mut().take_acquisitions();
        // The destination requests and the casts are taken every step for the same reason.
        let requests = self.world.borrow_mut().take_destination_requests();
        let casts = self.world.borrow_mut().take_casts();
        let Some(m) = self.mod_file.as_ref() else { return Ok(()) };
        let hooks = &m.hooks;
        for (unit, target) in acquired {
            self.call_target_priority(hooks, unit, target);
        }
        for (unit, player) in requests {
            self.call_on_march(hooks, unit, player);
        }
        for (caster, ability) in casts {
            self.call_on_cast(hooks, caster, ability);
        }
        if self.world.borrow().winner().is_none() {
            self.call_on_check_win(hooks);
        }
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
                    let death = {
                        let w = self.world.borrow();
                        Death {
                            victim: unit as u32,
                            victim_player: w.player(unit),
                            killer: killer.map(|k| k as u32),
                            killer_player: killer.map(|k| w.player(k)),
                            bounty: w.unit_bounty(unit),
                        }
                    };
                    let before = self.cmds.borrow().len();
                    self.call_died(hooks, &death)?;
                    self.call_unit_died(hooks, unit);
                    let own = self.cmds.borrow_mut().split_off(before);
                    self.local.borrow_mut().extend(own);
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

    /// Feeds a trade or courier request from the room to the mod (the room's sim queued it). Returns the
    /// commands the answer queued, for the room to apply. The same answer is applied to this world, so
    /// its mirror of the room stays in step; no request is re-answered.
    pub fn feed_request(&self, req: &Request) -> Vec<Cmd> {
        let before = self.cmds.borrow().len();
        let answer = self.answer(req);
        let mut fed = self.cmds.borrow_mut().split_off(before);
        fed.extend(answer);
        for cmd in fed.iter().cloned() {
            self.apply_one(cmd, false);
        }
        fed
    }

    // Answers one trade or courier request with the mod's rule. A trade's answer is its verdict. A mod
    // with no rule for a trade refuses it, so nothing is bought by default. A deliver request has no
    // verdict: its handler queues the courier's commands, which the caller collects.
    fn answer(&self, req: &Request) -> Option<Cmd> {
        let hooks = self.mod_file.as_ref().map(|m| &m.hooks);
        match *req {
            Request::Buy { unit, item } => {
                let verdict = match hooks.and_then(|h| h.on_buy.clone()) {
                    Some(f) => self.buy_verdict(&f, unit, item),
                    None => Verdict::Buy { unit, item, allow: false, deny: Refusal::NoItem, price: 0, takes: [NO_TAKE; VERDICT_TAKES] },
                };
                Some(Cmd::Trade { verdict })
            }
            Request::Sell { unit, slot } => {
                let verdict = match hooks.and_then(|h| h.on_sell.clone()) {
                    Some(f) => self.sell_verdict(&f, unit, slot),
                    None => Verdict::Sell { unit, slot, allow: false, deny: Refusal::NoItem, refund: 0 },
                };
                Some(Cmd::Trade { verdict })
            }
            Request::Unstash { unit, slot } => {
                let verdict = match hooks.and_then(|h| h.on_unstash.clone()) {
                    Some(f) => self.unstash_verdict(&f, unit, slot),
                    None => Verdict::Unstash { unit, slot, allow: false, deny: Refusal::NoItem },
                };
                Some(Cmd::Trade { verdict })
            }
            Request::Deliver { caster, target, reach_milli } => {
                if let Some(f) = hooks.and_then(|h| h.on_deliver.clone()) {
                    if let Err(e) = f.call::<()>((caster as i64, target as i64, i64::from(reach_milli), self.state.clone())) {
                        println!("ev=mod_hook_failed hook=on_deliver error={:?}", e.to_string());
                    }
                }
                None
            }
        }
    }

    fn buy_verdict(&self, f: &Function, unit: usize, item: usize) -> Verdict {
        let refuse = |deny| Verdict::Buy { unit, item, allow: false, deny, price: 0, takes: [NO_TAKE; VERDICT_TAKES] };
        let items = self.world.borrow().catalog().items.len();
        match f.call::<(Value, Value, Value)>((unit as i64, item as i64, self.state.clone())) {
            Ok((Value::Boolean(true), price, takes)) => match (whole(&price, i64::from(i32::MAX)), parse_takes(&takes, items)) {
                (Some(price), Some(takes)) => Verdict::Buy { unit, item, allow: true, deny: Refusal::NoItem, price: price as i32, takes },
                _ => refuse(Refusal::NoItem),
            },
            Ok((_, reason, _)) => refuse(denial(&reason)),
            Err(e) => {
                println!("ev=mod_hook_failed hook=on_buy error={:?}", e.to_string());
                refuse(Refusal::NoItem)
            }
        }
    }

    fn sell_verdict(&self, f: &Function, unit: usize, slot: usize) -> Verdict {
        match f.call::<(Value, Value)>((unit as i64, slot as i64, self.state.clone())) {
            Ok((Value::Boolean(true), refund)) => match whole(&refund, i64::from(i32::MAX)) {
                Some(refund) => Verdict::Sell { unit, slot, allow: true, deny: Refusal::NoItem, refund: refund as i32 },
                None => Verdict::Sell { unit, slot, allow: false, deny: Refusal::NoItem, refund: 0 },
            },
            Ok((_, reason)) => Verdict::Sell { unit, slot, allow: false, deny: denial(&reason), refund: 0 },
            Err(e) => {
                println!("ev=mod_hook_failed hook=on_sell error={:?}", e.to_string());
                Verdict::Sell { unit, slot, allow: false, deny: Refusal::NoItem, refund: 0 }
            }
        }
    }

    fn unstash_verdict(&self, f: &Function, unit: usize, slot: usize) -> Verdict {
        match f.call::<(Value, Value)>((unit as i64, slot as i64, self.state.clone())) {
            Ok((Value::Boolean(true), _)) => Verdict::Unstash { unit, slot, allow: true, deny: Refusal::NoItem },
            Ok((_, reason)) => Verdict::Unstash { unit, slot, allow: false, deny: denial(&reason) },
            Err(e) => {
                println!("ev=mod_hook_failed hook=on_unstash error={:?}", e.to_string());
                Verdict::Unstash { unit, slot, allow: false, deny: Refusal::NoItem }
            }
        }
    }

    /// Feeds one payable death of the room's sim to the mod's on_died, with the room's unit ids and
    /// the bounty (docs/MODDING.md, on_died). The commands the handlers queued are returned, not
    /// applied: their ids are the room's, so the room applies them to its own sim and this world
    /// is untouched.
    pub fn feed_payout(&self, p: &Payout) -> mlua::Result<Vec<Cmd>> {
        let Some(m) = self.mod_file.as_ref() else { return Ok(Vec::new()) };
        let death = Death {
            victim: p.victim,
            victim_player: p.victim_player,
            killer: p.killer,
            killer_player: p.killer_player,
            bounty: p.bounty,
        };
        let before = self.cmds.borrow().len();
        let result = self.call_died(&m.hooks, &death);
        self.call_unit_died(&m.hooks, p.victim as usize);
        let fed = self.cmds.borrow_mut().split_off(before);
        result.map(|()| fed)
    }

    // Calls on_unit_died for a dead unit and queues the respawn decision it returns. A hook that errors
    // or returns a value outside the contract queues nothing (the Rust timer stands) and logs an event.
    // The command goes through the same path as the other mod commands, so a replay reproduces it.
    fn call_unit_died(&self, hooks: &Hooks, unit: usize) {
        let Some(h) = &hooks.on_unit_died else { return };
        let delay_s = match h.call::<Value>((unit as i64, self.state.clone())) {
            Ok(Value::Nil) | Ok(Value::Boolean(true)) => return,
            Ok(Value::Boolean(false)) => None,
            Ok(Value::Integer(n)) if (0..=MAX_RESPAWN_S).contains(&n) => Some(n as u32),
            Ok(Value::Number(n)) if (0.0..=MAX_RESPAWN_S as f64).contains(&n) => Some(n as u32),
            Ok(other) => {
                println!("ev=mod_hook_refused hook=on_unit_died value={:?}", other.type_name());
                return;
            }
            Err(e) => {
                println!("ev=mod_hook_failed hook=on_unit_died error={:?}", e.to_string());
                return;
            }
        };
        self.cmds.borrow_mut().push(Cmd::SetRespawn { unit, delay_s });
    }

    // Calls target_priority for one aggro acquisition (unit drew aggro from target). A class 0..3 is queued
    // as a class override, which applies to the unit's candidates; false clears it. Errors fall back to the Rust rule.
    fn call_target_priority(&self, hooks: &Hooks, unit: usize, target: usize) {
        let Some(h) = &hooks.target_priority else { return };
        let candidates = match self.lua.create_sequence_from([target as i64]) {
            Ok(t) => t,
            Err(_) => return,
        };
        let class = match h.call::<Value>((unit as i64, candidates, self.state.clone())) {
            Ok(Value::Nil) | Ok(Value::Boolean(true)) => return,
            Ok(Value::Boolean(false)) => None,
            Ok(Value::Integer(c)) if (0..=3).contains(&c) => Some(c as u8),
            Ok(other) => {
                println!("ev=mod_hook_refused hook=target_priority value={:?}", other.type_name());
                return;
            }
            Err(e) => {
                println!("ev=mod_hook_failed hook=target_priority error={:?}", e.to_string());
                return;
            }
        };
        self.cmds.borrow_mut().push(Cmd::SetClassOverride { unit, class });
    }

    // Calls on_march for a unit that needs a destination and queues the attack-move it returns: on_march(unit,
    // player, state) returns the x and y of the point, in fixed point. nil keeps the unit where it stands. An error
    // or a malformed answer queues nothing.
    fn call_on_march(&self, hooks: &Hooks, unit: usize, player: u8) {
        let Some(h) = &hooks.on_march else { return };
        match h.call::<(Value, Value)>((unit as i64, player as i64, self.state.clone())) {
            Ok((x, y)) => match (point_coord(&x), point_coord(&y)) {
                (Some(x), Some(y)) => self.cmds.borrow_mut().push(Cmd::AttackMove { unit, x, y }),
                _ if matches!(x, Value::Nil | Value::Boolean(false)) => {}
                _ => println!("ev=mod_hook_refused hook=on_march value={:?}", x.type_name()),
            },
            Err(e) => println!("ev=mod_hook_failed hook=on_march error={:?}", e.to_string()),
        }
    }

    // Calls on_cast when a cast takes hold, with the caster and the ability's name.
    fn call_on_cast(&self, hooks: &Hooks, caster: usize, ability: usize) {
        let Some(h) = &hooks.on_cast else { return };
        let name = {
            let sim = self.world.borrow();
            let cat = sim.catalog();
            match cat.abilities.get(ability) {
                Some(a) => cat.name(a.name).to_string(),
                None => return,
            }
        };
        if let Err(e) = h.call::<()>((caster as i64, name, self.state.clone())) {
            println!("ev=mod_hook_failed hook=on_cast error={:?}", e.to_string());
        }
    }

    // Calls on_check_win and queues the side it returns as the winner. nil or false means no side has won yet.
    fn call_on_check_win(&self, hooks: &Hooks) {
        let Some(h) = &hooks.on_check_win else { return };
        match h.call::<Value>(self.state.clone()) {
            Ok(Value::Nil) | Ok(Value::Boolean(false)) => {}
            Ok(Value::Integer(s)) if (0..MAX_PLAYERS as i64).contains(&s) => {
                self.cmds.borrow_mut().push(Cmd::DeclareSide { side: s as u8 });
            }
            Ok(other) => println!("ev=mod_hook_refused hook=on_check_win value={:?}", other.type_name()),
            Err(e) => println!("ev=mod_hook_failed hook=on_check_win error={:?}", e.to_string()),
        }
    }

    // Calls the on_died hook and every on_died trigger with the same arguments.
    fn call_died(&self, hooks: &Hooks, d: &Death) -> mlua::Result<()> {
        let killer = d.killer.map(i64::from);
        let killer_player = d.killer_player.map(i64::from);
        let (victim, victim_player, bounty) = (i64::from(d.victim), i64::from(d.victim_player), i64::from(d.bounty));
        if let Some(h) = &hooks.on_died {
            h.call::<()>((victim, killer, killer_player, bounty, victim_player))?;
        }
        hooks.fire.call::<()>(("on_died", victim, killer, killer_player, bounty, victim_player))
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Whether the loaded mod declared `GOLD_BY_MOD = true`: its kill payout replaces the sim's.
    pub fn gold_by_mod(&self) -> bool {
        self.gold_by_mod
    }

    /// The commands `advance` applied to this world since the last take, in apply order, so a
    /// caller that forwards them (the server room) applies the same changes to its own sim.
    /// Commands queued by on_tick or a timer are applied, and so taken, by the next advance.
    pub fn take_commands(&mut self) -> Vec<Cmd> {
        std::mem::take(&mut *self.forward.borrow_mut())
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
// The mod's answer values, read from the Lua results. A count must be a whole number in range.
fn whole(v: &Value, max: i64) -> Option<i64> {
    let n = match v {
        Value::Integer(n) => *n,
        Value::Number(f) if f.fract() == 0.0 && f.abs() < 9.2e18 => *f as i64,
        _ => return None,
    };
    (0..=max).contains(&n).then_some(n)
}

// The catalog items a purchase takes: a list of at most VERDICT_TAKES valid item indices, or none.
fn parse_takes(v: &Value, items: usize) -> Option<[u32; VERDICT_TAKES]> {
    let mut out = [NO_TAKE; VERDICT_TAKES];
    match v {
        Value::Nil => Some(out),
        Value::Table(t) => {
            let mut n = 0;
            for x in t.clone().sequence_values::<i64>() {
                let x = x.ok()?;
                if n >= VERDICT_TAKES || x < 0 || x as usize >= items {
                    return None;
                }
                out[n] = x as u32;
                n += 1;
            }
            Some(out)
        }
        _ => None,
    }
}

// The refusal a mod names with a denial. An unknown or missing reason is the item being unavailable.
fn denial(v: &Value) -> Refusal {
    let Value::String(s) = v else { return Refusal::NoItem };
    match s.to_str().ok().as_deref() {
        Some("no-shop") => Refusal::OutOfReach,
        Some("cannot-afford") => Refusal::CannotAfford,
        Some("inventory-full") => Refusal::InventoryFull,
        Some("missing-component") => Refusal::MissingComponent,
        _ => Refusal::NoItem,
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

    // Two creeps on a walkable open row (y = 34, tiles 40 and 62; the map's reference GridNav leaves x 39..119 open there), one per side and 22 tiles apart, past each other's sight, so only a command makes them fight.
    fn host_with_creeps() -> ModHost {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        let (a, b) = {
            let mut sim = host.sim();
            let creep = sim.catalog().unit_index("creep").expect("moba has a creep") as u32;
            // The first open tile at or after x = 40 for side 0, and the first at or before x = 62 for side 1, on the row.
            // Walkable tiles of the current map: the first that takes a creep for side 0 from the left, and the first
            // for side 1 from the right, at least 20 tiles apart.
            let spawn_row = |player: u8, xs: Vec<i32>, sim: &mut sim::Sim| -> Option<(usize, i32)> {
                for y in 20..=60 {
                    for &x in &xs {
                        if !sim.grid().walkable(x, y) {
                            continue;
                        }
                        if let Some(i) = sim.spawn_for(player, creep, sim::fixed::from_int(x), sim::fixed::from_int(y)) {
                            return Some((i, x));
                        }
                    }
                }
                None
            };
            let (a, ax) = spawn_row(0, (40..=119).collect(), &mut sim).expect("creep a");
            let (b, _) = spawn_row(1, (ax + 20..=119).rev().collect(), &mut sim).expect("creep b");
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

    // A mod's game.cmd_remove takes the unit off the map through the command path; without the mod it stays alive.
    #[test]
    fn cmd_remove_takes_the_unit_off_the_map_and_no_mod_leaves_it() {
        let mut host = host_with_creeps();
        let b = host.state().get::<i64>("target").expect("target") as usize;
        host.advance(&host.state().clone()).expect("step without a mod");
        assert!(host.sim().alive(b), "a unit with no mod loaded must stay alive");
        host.load_source("remove", r#"
            function on_tick(state)
              if unit_alive(state.target) then game.cmd_remove(state.target) end
            end
        "#).expect("mod loads");
        host.advance(&host.state().clone()).expect("queue");
        host.advance(&host.state().clone()).expect("apply");
        assert!(!host.sim().alive(b), "game.cmd_remove did not remove the unit");
    }

    // A moba-style buyback built only from generic primitives: on_unit_died leaves the hero dead, and a
    // mod timer-style check on the tick pays the gold (game.cmd_add_unit_gold) and respawns it now
    // (game.cmd_set_respawn with 0). Two runs of the same script reproduce the same state.
    #[test]
    fn a_buyback_built_from_generic_primitives_reproduces() {
        const BUYBACK_SRC: &str = r#"
            function on_tick(state)
              if state.buy and not unit_alive(state.hero) and game.unit_gold(state.hero) >= 230 then
                game.cmd_add_unit_gold(state.hero, -230)
                game.cmd_set_respawn(state.hero, 0)
                state.buy = false
              end
            end
            function on_unit_died(unit, state) return false end
        "#;
        let run = || -> (u64, bool, i32) {
            let mut host = ModHost::new(&moba_image()).expect("moba image loads");
            let hero = {
                let mut sim = host.sim();
                let kind = sim.catalog().unit_index("axe").expect("moba has axe") as u32;
                let mut found = None;
                'search: for y in 20..=60 {
                    for x in 40..=100 {
                        if !sim.grid().walkable(x, y) {
                            continue;
                        }
                        if let Some(i) = sim.spawn_attributed(0, kind, sim::fixed::from_int(x), sim::fixed::from_int(y)) {
                            found = Some(i);
                            break 'search;
                        }
                    }
                }
                let h = found.expect("a walkable tile takes the hero");
                sim.add_unit_gold(h, 500);
                sim.hurt(h, sim::fixed::from_int(30_000));
                h
            };
            host.state().set("hero", hero as i64).expect("state");
            host.state().set("buy", false).expect("state");
            host.load_source("buyback", BUYBACK_SRC).expect("mod loads");
            // The death reaches on_unit_died and its decision (leave dead) applies first.
            for _ in 0..3 {
                host.advance(&host.state().clone()).expect("step");
            }
            assert!(!host.sim().alive(hero), "the hero should be dead before the buyback");
            host.state().set("buy", true).expect("state");
            for _ in 0..3 {
                host.advance(&host.state().clone()).expect("step");
            }
            let hash = host.state_hash();
            let sim = host.sim();
            (hash, sim.alive(hero), sim.unit_gold(hero))
        };
        let (hash_a, alive_a, gold_a) = run();
        let (hash_b, alive_b, gold_b) = run();
        assert!(alive_a, "the buyback did not respawn the hero");
        assert!(gold_a <= 500 - 230 + 1 && gold_a >= 500 - 230, "gold after buyback {gold_a}");
        assert_eq!((alive_a, gold_a), (alive_b, gold_b));
        assert_eq!(hash_a, hash_b, "the same buyback script did not reproduce the same state");
    }

    // game.cmd_give_item puts a catalog item into a living unit's inventory through the sim's slot rules.
    #[test]
    fn cmd_give_item_adds_the_item_to_the_unit_and_rejects_an_unknown_one() {
        let mut host = host_with_creeps();
        let a = host.state().get::<i64>("attacker").expect("attacker") as usize;
        let kind = host.sim().catalog().item_index("blink").expect("moba has blink") as u32 + 1;
        host.load_source("give", r#"
            function on_tick(state)
              game.cmd_give_item(state.attacker, "blink", 1)
            end
        "#).expect("mod loads");
        host.advance(&host.state().clone()).expect("queue");
        host.advance(&host.state().clone()).expect("apply");
        assert_eq!(host.sim().inventory_count(a, kind), 1, "the item was not granted");
        let err = host.load_source("bad", r#"
            function on_tick(state)
              game.cmd_give_item(state.attacker, "no_such_item", 1)
            end
        "#).and_then(|_| host.advance(&host.state().clone()));
        assert!(err.is_err(), "an unknown item name must be a named error");
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

    // set_target_class accepts only the four classes 0..=3 and only a live unit; anything else is a Lua error
    // raised from on_tick, and nothing is queued for the sim.
    #[test]
    fn set_target_class_refuses_classes_outside_0_to_3_and_dead_or_unknown_units() {
        let mut host = host_with_creeps();
        let a = host.state().get::<i64>("attacker").expect("attacker");
        for (unit, class, want) in [(a, 4, "invalid_target_class"), (a, -1, "invalid_target_class"), (9_999_999, 1, "invalid_unit")] {
            host.load_source("klass", &format!("function on_tick(state) game.set_target_class({unit}, {class}) end\n")).expect("mod loads");
            let err = host.advance(&host.state().clone()).expect_err("the call must be refused");
            assert!(err.to_string().contains(want), "wrong error for ({unit}, {class}): {err}");
        }
        host.advance(&host.state().clone()).ok();
        assert!(host.take_commands().iter().all(|c| !matches!(c, Cmd::SetClassOverride { .. })), "a refused call queued a class override");
    }

    #[test]
    fn set_target_class_accepts_0_to_3_on_a_live_unit() {
        let mut host = host_with_creeps();
        let a = host.state().get::<i64>("attacker").expect("attacker") as usize;
        host.load_source("klass", &format!("function on_tick(state) game.set_target_class({a}, 3) end\n")).expect("mod loads");
        host.advance(&host.state().clone()).expect("queue");
        host.advance(&host.state().clone()).expect("apply");
        let applied = host.take_commands().into_iter().any(|c| matches!(c, Cmd::SetClassOverride { unit, class: Some(3) } if unit == a));
        assert!(applied, "the class override was not applied to the live unit");
    }

    #[test]
    fn cmd_spawn_queues_the_owner_and_route_and_defaults_both_to_zero() {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        host.load_source("spawn", r#"
            function on_tick(state)
              if state.spawned then return end
              state.spawned = true
              game.cmd_spawn("creep", 50 * 65536, 34 * 65536, 1, 2)
              game.cmd_spawn("creep", 52 * 65536, 34 * 65536)
            end
        "#).expect("mod loads");
        host.advance(&host.state().clone()).expect("queue");
        host.advance(&host.state().clone()).expect("apply");
        let spawns: Vec<(u8, i32)> = host
            .take_commands()
            .into_iter()
            .filter_map(|c| match c {
                Cmd::Spawn { owner, route, .. } => Some((owner, route)),
                _ => None,
            })
            .collect();
        assert_eq!(spawns, vec![(1, 2), (0, 0)], "owner and route are queued; both default to 0");
    }

    #[test]
    fn applied_commands_are_forwarded() {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        host.load_source("gold", r#"
            function on_tick(state)
              if state.paid then return end
              state.paid = true
              game.cmd_add_gold(1, 25)
            end
        "#).expect("mod loads");
        host.advance(&host.state().clone()).expect("queue");
        host.advance(&host.state().clone()).expect("apply");
        let gold: Vec<(u8, i32)> = host
            .take_commands()
            .into_iter()
            .filter_map(|c| match c {
                Cmd::AddGold { player, amount } => Some((player, amount)),
                _ => None,
            })
            .collect();
        assert_eq!(gold, vec![(1, 25)], "a command the world applied reaches the caller");
        assert!(host.take_commands().is_empty(), "a take drains the forwarded commands");
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
                // A walkable tile beside the target (the scout must stand within sight of it).
                let (tx, ty) = sim.position_fixed(b);
                let (tx, ty) = (sim::fixed::to_int(tx), sim::fixed::to_int(ty));
                [(3, 0), (-3, 0), (0, 3), (0, -3), (4, 4), (-4, -4), (4, -4), (-4, 4)]
                    .iter()
                    .find_map(|&(dx, dy)| {
                        let (x, y) = (tx + dx, ty + dy);
                        if sim.grid().walkable(x, y) {
                            sim.spawn_for(2, creep, sim::fixed::from_int(x), sim::fixed::from_int(y))
                        } else {
                            None
                        }
                    })
                    .expect("scout");
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

    // on_unit_died and target_priority queue commands only when the mod defines them and they return a value
    // in the contract. A missing, erroring or out-of-contract hook queues nothing, so the Rust rule stands.
    // Answers the room-side requests the way the room does: each one goes to the mod, and the verdicts it
    // returns are applied (feed_request applies them to this world too).
    fn settle(host: &ModHost) {
        let reqs = host.sim().drain_requests();
        for r in reqs {
            host.feed_request(&r);
        }
    }

    // A moba-style store rule with no shops: every item costs 300 and takes nothing. It buys through the
    // generic primitives only: game.cmd_buy queues the purchase, the sim asks on_buy, and the answer debits.
    const FLAT_PRICE_STORE: &str = r#"
        local done = false
        function on_tick(state)
          if not done then
            done = true
            game.cmd_buy(state.attacker, state.item)
          end
        end
        function on_buy(unit, item, state)
          return true, 300, {}
        end
    "#;

    // A purchase is decided by the mod: its price is debited, the item is placed, and a listed component is
    // consumed. Without an on_buy the purchase is refused, so no mod means nothing is bought.
    #[test]
    fn a_moba_style_mod_reproduces_a_purchase_through_the_store_callback() {
        let mut host = host_with_creeps();
        let a = host.state().get::<i64>("attacker").expect("attacker") as usize;
        let blink = host.sim().catalog().item_index("blink").expect("moba has blink");
        host.state().set("item", blink as i64).expect("state");
        host.sim().add_resources(0, 1000, 0);
        host.load_source("store", FLAT_PRICE_STORE).expect("mod loads");
        for _ in 0..3 {
            host.advance(&host.state().clone()).expect("step");
        }
        assert_eq!(host.sim().inventory_count(a, blink as u32 + 1), 1, "the purchase did not place the item");
        assert_eq!(host.sim().resource_a(0), 700, "the price was not debited from the counter");
    }

    #[test]
    fn a_mod_without_an_on_buy_rule_leaves_the_item_unbought() {
        let mut host = host_with_creeps();
        let a = host.state().get::<i64>("attacker").expect("attacker") as usize;
        let blink = host.sim().catalog().item_index("blink").expect("moba has blink");
        host.state().set("item", blink as i64).expect("state");
        host.sim().add_resources(0, 1000, 0);
        host.load_source("nobuy", "local done = false\nfunction on_tick(state)\n  if not done then done = true game.cmd_buy(state.attacker, state.item) end\nend\n").expect("mod loads");
        for _ in 0..3 {
            host.advance(&host.state().clone()).expect("step");
        }
        assert_eq!(host.sim().inventory_count(a, blink as u32 + 1), 0, "a mod with no rule bought an item");
        assert_eq!(host.sim().resource_a(0), 1000, "a refused purchase took gold");
    }

    // A denial names its refusal and changes nothing; a listed component is consumed with the price.
    #[test]
    fn a_denial_names_its_refusal_and_a_listed_component_is_consumed() {
        let mut host = host_with_creeps();
        let a = host.state().get::<i64>("attacker").expect("attacker") as usize;
        let blink = host.sim().catalog().item_index("blink").expect("moba has blink");
        let comp = host.sim().catalog().item_index("tango").expect("moba has tango");
        host.state().set("item", blink as i64).expect("state");
        host.sim().add_resources(0, 1000, 0);
        assert!(host.sim().inventory_add(a, comp as u32 + 1, 1));
        host.load_source("deny", "local done = false\nfunction on_tick(state)\n  if not done then done = true game.cmd_buy(state.attacker, state.item) end\nend\nfunction on_buy(unit, item, state) return false, \"no-shop\" end\n").expect("mod loads");
        for _ in 0..3 {
            host.advance(&host.state().clone()).expect("step");
        }
        assert_eq!(host.sim().inventory_count(a, blink as u32 + 1), 0, "a denied purchase placed the item");
        assert_eq!(host.sim().stats.last_refusal, Refusal::OutOfReach, "the denial did not name its refusal");
        assert_eq!(host.sim().resource_a(0), 1000);
        // Now allowed, with the component listed: the component is consumed and the item placed.
        host.load_source("take", "local done = false\nfunction on_tick(state)\n  if not done then done = true game.cmd_buy(state.attacker, state.item) end\nend\nfunction on_buy(unit, item, state) return true, 250, {state.comp} end\n").expect("mod loads");
        host.state().set("comp", comp as i64).expect("state");
        for _ in 0..3 {
            host.advance(&host.state().clone()).expect("step");
        }
        assert_eq!(host.sim().inventory_count(a, comp as u32 + 1), 0, "the listed component was not consumed");
        assert_eq!(host.sim().inventory_count(a, blink as u32 + 1), 1, "the purchase with a component was not placed");
        assert_eq!(host.sim().resource_a(0), 750, "the net price was not debited");
    }

    // The deliver request reaches the mod, which walks the courier to the receiver, hands over the receiver's
    // stash, and walks it back to the point it left from. A mod with no deliver rule delivers nothing.
    #[test]
    fn a_moba_courier_delivers_the_stash_and_walks_home() {
        let mut host = host_with_creeps();
        let a = host.state().get::<i64>("attacker").expect("attacker") as usize;
        let b = host.state().get::<i64>("target").expect("target") as usize;
        let blink = host.sim().catalog().item_index("blink").expect("moba has blink");
        {
            let mut sim = host.sim();
            sim.set_fog(false);
            assert!(sim.set_owner(b, 0).is_ok(), "the receiver joins the caster's side");
            assert!(sim.give_ability(a, 0, "courier_deliver"), "the caster takes the courier ability");
            assert!(sim.stash_item(b, blink, 1), "the receiver's stash takes the item");
        }
        let start = host.sim().position_fixed(a);
        host.load_source("courier", include_str!("../../../../scenarios/moba/store.lua")).expect("the moba store mod loads");
        assert!(host.sim().cast_ability(a, 0, b as i32, 0, 0), "the courier cast was refused");
        settle(&host);
        for _ in 0..3000 {
            host.advance(&host.state().clone()).expect("step");
        }
        assert_eq!(host.sim().inventory_count(b, blink as u32 + 1), 1, "the receiver's stash was not delivered");
        assert_eq!(host.sim().stash_slot(b, 0).1, 0, "the delivered stash slot is still full");
        let (x, y) = host.sim().position_fixed(a);
        let (sx, sy) = start;
        let (dx, dy) = (i64::from(x) - i64::from(sx), i64::from(y) - i64::from(sy));
        assert!(dx * dx + dy * dy <= (2 * 65536i64).pow(2), "the courier did not walk back to its start");
    }

    #[test]
    fn a_mod_without_a_deliver_rule_delivers_nothing() {
        let mut host = host_with_creeps();
        let a = host.state().get::<i64>("attacker").expect("attacker") as usize;
        let b = host.state().get::<i64>("target").expect("target") as usize;
        let blink = host.sim().catalog().item_index("blink").expect("moba has blink");
        {
            let mut sim = host.sim();
            assert!(sim.set_owner(b, 0).is_ok());
            assert!(sim.give_ability(a, 0, "courier_deliver"));
            assert!(sim.stash_item(b, blink, 1));
        }
        host.load_source("idle", "function on_tick(state) end\n").expect("mod loads");
        assert!(host.sim().cast_ability(a, 0, b as i32, 0, 0));
        settle(&host);
        for _ in 0..200 {
            host.advance(&host.state().clone()).expect("step");
        }
        assert_eq!(host.sim().inventory_count(b, blink as u32 + 1), 0, "a mod with no courier rule delivered");
    }

    fn queued(host: &ModHost) -> Vec<Cmd> {
        host.cmds.borrow().clone()
    }

    #[test]
    fn on_unit_died_number_queues_a_respawn_and_false_leaves_the_unit_dead() {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        host.load_source("r.lua", "function on_tick(state) end function on_unit_died(unit, state) return 7 end").expect("loads");
        let m = host.mod_file.as_ref().expect("mod");
        host.call_unit_died(&m.hooks, 3);
        assert!(queued(&host).iter().any(|c| matches!(c, Cmd::SetRespawn { unit: 3, delay_s: Some(7) })));
        host.cmds.borrow_mut().clear();
        host.load_source("r.lua", "function on_tick(state) end function on_unit_died(unit, state) return false end").expect("loads");
        let m = host.mod_file.as_ref().expect("mod");
        host.call_unit_died(&m.hooks, 4);
        assert!(queued(&host).iter().any(|c| matches!(c, Cmd::SetRespawn { unit: 4, delay_s: None })));
    }

    #[test]
    fn removing_the_respawn_hook_or_an_erroring_one_restores_the_rust_timer() {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        host.load_source("r.lua", "function on_tick(state) end function on_unit_died() error('boom') end").expect("loads");
        let m = host.mod_file.as_ref().expect("mod");
        host.call_unit_died(&m.hooks, 3);
        assert!(queued(&host).is_empty(), "an erroring hook must queue nothing");
        host.load_source("r.lua", "function on_tick(state) end function on_unit_died(unit, state) return 'soon' end").expect("loads");
        let m = host.mod_file.as_ref().expect("mod");
        host.call_unit_died(&m.hooks, 3);
        assert!(queued(&host).is_empty(), "an out-of-contract value must queue nothing");
        host.load_source("r.lua", "function on_tick(state) end").expect("loads");
        let m = host.mod_file.as_ref().expect("mod");
        host.call_unit_died(&m.hooks, 3);
        assert!(queued(&host).is_empty(), "without the hook the Rust timer stands");
    }

    #[test]
    fn target_priority_queues_a_class_override_and_false_clears_it() {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        host.load_source("p.lua", "function on_tick(state) end function target_priority(unit, candidates, state) return 3 end").expect("loads");
        let m = host.mod_file.as_ref().expect("mod");
        host.call_target_priority(&m.hooks, 2, 5);
        assert!(queued(&host).iter().any(|c| matches!(c, Cmd::SetClassOverride { unit: 2, class: Some(3) })));
        host.cmds.borrow_mut().clear();
        host.load_source("p.lua", "function on_tick(state) end function target_priority(unit, candidates, state) return false end").expect("loads");
        let m = host.mod_file.as_ref().expect("mod");
        host.call_target_priority(&m.hooks, 2, 5);
        assert!(queued(&host).iter().any(|c| matches!(c, Cmd::SetClassOverride { unit: 2, class: None })));
        host.cmds.borrow_mut().clear();
        host.load_source("p.lua", "function on_tick(state) end function target_priority(unit, candidates, state) return 9 end").expect("loads");
        let m = host.mod_file.as_ref().expect("mod");
        host.call_target_priority(&m.hooks, 2, 5);
        assert!(queued(&host).is_empty(), "a class outside 0..3 must queue nothing");
    }

    // scenarios/moba/protection.lua sets a structure's damage reduction once no hostile plain unit has
    // been within reach for the activation time, and sends nothing when no mod is loaded.
    #[test]
    fn moba_protection_sends_the_reduction_after_the_activation_time() {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba/protection.lua")).expect("protection mod");
        host.load_source("protection", &src).expect("protection loads");
        for _ in 0..(16 * 16 + 8) {
            host.advance(&host.state().clone()).expect("step");
        }
        let cmds = host.take_commands();
        assert!(cmds.iter().any(|c| matches!(c, Cmd::SetReduction { pct: 50, .. })), "a structure with no threat for 15 seconds must be reduced");
    }

    #[test]
    fn no_mod_sends_no_reduction() {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        for _ in 0..20 {
            host.advance(&host.state().clone()).expect("step");
        }
        assert!(host.take_commands().iter().all(|c| !matches!(c, Cmd::SetReduction { .. })));
    }

    // The moba mod is protection.lua, march.lua and win.lua as one chunk: a host runs one mod file, and only
    // protection.lua defines on_tick.
    fn moba_mod_source() -> String {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
        ["protection.lua", "hero.lua", "march.lua", "win.lua"]
            .iter()
            .map(|f| std::fs::read_to_string(dir.join(f)).expect("moba mod file"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // Spawns a unit of `kind` for `player` on the first open tile, asking for a destination when `route` is nonzero.
    fn spawn_on_open_ground(host: &ModHost, kind: u32, player: u8, route: i32) -> usize {
        let mut sim = host.sim();
        for y in 20..=60 {
            for x in 40..=100 {
                if !sim.grid().walkable(x, y) {
                    continue;
                }
                if let Some(i) = sim.spawn_mod_unit(player, kind, sim::fixed::from_int(x), sim::fixed::from_int(y), route) {
                    return i;
                }
            }
        }
        panic!("no open ground for the unit");
    }

    // Every living building of `player`, by unit id.
    fn buildings_of(host: &ModHost, player: u8) -> Vec<usize> {
        let sim = host.sim();
        (0..sim.count()).filter(|&k| sim.alive(k) && sim.player(k) == player && (sim.filter_bits(k) & 2048) != 0).collect()
    }

    #[test]
    fn a_moba_mod_marches_a_summon_to_the_nearest_enemy_building() {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        host.load_source("moba", &moba_mod_source()).expect("moba mod loads");
        let creep = host.sim().catalog().unit_index("creep").expect("creep") as u32;
        let summon = spawn_on_open_ground(&host, creep, 0, 1);
        for _ in 0..3 {
            host.advance(&host.state().clone()).expect("step");
        }
        let cmds = host.take_commands();
        let ordered = cmds.iter().find_map(|c| match c {
            Cmd::AttackMove { unit, x, y } if *unit == summon => Some((*x, *y)),
            _ => None,
        });
        let (x, y) = ordered.expect("the summon was not marched");
        let enemy_buildings = buildings_of(&host, 1);
        assert!(!enemy_buildings.is_empty(), "the moba map has enemy buildings");
        assert!(
            enemy_buildings.iter().any(|&b| host.sim().position_fixed(b) == (x, y)),
            "the march must point at an enemy building, got ({x}, {y})"
        );
    }

    #[test]
    fn a_moba_mod_glyph_reduces_the_team_structures_its_filter_covers() {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        host.load_source("moba", &moba_mod_source()).expect("moba mod loads");
        // A structure that is not a building: filter bit 6 set, bit 11 clear.
        let kind = {
            let sim = host.sim();
            sim.catalog().units.iter().position(|u| u.filter & 64 != 0 && u.filter & 2048 == 0).expect("a non-building structure kind") as u32
        };
        let glyphed = spawn_on_open_ground(&host, kind, 0, 0);
        let name = {
            let sim = host.sim();
            let cat = sim.catalog();
            let index = cat.abilities.iter().position(|a| cat.name(a.name) == "glyph_of_fortification").expect("the moba map has the glyph ability");
            cat.name(cat.abilities[index].name).to_string()
        };
        assert!(host.sim().give_ability(glyphed, 0, &name), "the structure takes the glyph ability");
        let cast = host.sim().cast_ability(glyphed, 0, -1, 0, 0);
        assert!(cast, "the glyph cast takes hold");
        for _ in 0..3 {
            host.advance(&host.state().clone()).expect("step");
        }
        let cmds = host.take_commands();
        assert!(
            cmds.iter().any(|c| matches!(c, Cmd::SetReduction { unit, pct: 100 } if *unit == glyphed)),
            "the glyph must cover the team's non-building structure"
        );
    }

    #[test]
    fn a_moba_mod_declares_the_side_that_outlives_the_buildings_and_no_other() {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        host.load_source("moba", &moba_mod_source()).expect("moba mod loads");
        host.advance(&host.state().clone()).expect("step");
        assert_eq!(host.sim().winner(), None, "no side has won while both stand");
        for b in buildings_of(&host, 1) {
            host.sim().hurt(b, sim::fixed::from_int(30_000));
        }
        for _ in 0..3 {
            host.advance(&host.state().clone()).expect("step");
        }
        let side = host.sim().team_of(0);
        assert_eq!(host.sim().winner(), side, "the side of player 0 outlives player 1's buildings");
    }

    // A mod without the callbacks has no march, no protection and no win, whatever the sim does.
    #[test]
    fn a_mod_without_the_callbacks_has_no_march_no_protection_and_no_win() {
        let mut host = ModHost::new(&moba_image()).expect("moba image loads");
        host.load_source("bare", "function on_tick(state) end").expect("bare mod loads");
        let creep = host.sim().catalog().unit_index("creep").expect("creep") as u32;
        let summon = spawn_on_open_ground(&host, creep, 0, 1);
        for b in buildings_of(&host, 1) {
            host.sim().hurt(b, sim::fixed::from_int(30_000));
        }
        for _ in 0..20 {
            host.advance(&host.state().clone()).expect("step");
        }
        let cmds = host.take_commands();
        assert!(cmds.iter().all(|c| !matches!(c, Cmd::AttackMove { .. })), "no march without on_march");
        assert!(cmds.iter().all(|c| !matches!(c, Cmd::SetReduction { .. } | Cmd::SetHealing { .. })), "no protection without the callbacks");
        assert!(cmds.iter().all(|c| !matches!(c, Cmd::DeclareSide { .. })), "no win without on_check_win");
        assert_eq!(host.sim().winner(), None, "no side is declared without on_check_win");
        assert!(host.sim().alive(summon), "the summon stands where it was placed");
    }
}
