//! The mod host on piccolo, a pure Rust Lua VM, for the wasip3 guest (no C objects). A mod runs in
//! its own environment table with the same game table (system, each, trigger, cmd_*, units,
//! players, unit_kind, timer, region, var, score, ability, gold, victory and the UI calls), the same
//! unit queries and the same event dispatch as the mlua host. The Lua half of var, region and timer
//! is the source mlua runs (shared.rs). Not carried over: terrain, tile, doodad and unit, item and
//! upgrade editing globals. A mod call runs under a step budget so a loop cannot stall the guest.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use piccolo::fuel::Fuel;
use piccolo::{
    Callback, CallbackReturn, Closure, Context, Error, Executor, IntoValue, Lua, StashedExecutor, StashedFunction,
    Stack, StashedTable, Table, Value, Variadic,
};
use sim::{AbilitySpec, Catalog, LoadError, Payout, Sim, ABILITY_SLOTS, ITEM_SLOTS, MAX_PLAYERS, NO_COMPONENT, TICKS_PER_SECOND};

use crate::shared::{self, amount_arg, fixed, player_arg, PRELUDE_SRC};
use crate::{EVENTS, MAX_UNITS};

const FUEL_PER_STEP: i32 = 4096;
const MAX_STEPS: u32 = 2_500;
const MAX_REGIONS: usize = 64;

#[derive(Clone)]
pub enum Cmd {
    Spawn { index: usize, x: i32, y: i32, owner: u8, route: i32 },
    Order { unit: usize, x: i32, y: i32 },
    Damage { caster: usize, target: usize, dmg: i32 },
    Buff { unit: usize, kind: i32, ticks: i32 },
    Cast { unit: usize, slot: usize, target: i32, x: i32, y: i32 },
    UseItem { unit: usize, slot: usize, target: i32, x: i32, y: i32 },
    Buy { unit: usize, item: usize },
    Sell { unit: usize, slot: usize },
    Unstash { unit: usize, slot: usize },
    Attack { unit: usize, target: usize },
    SetOwner { unit: usize, player: u8 },
    SetFog { on: bool },
    GiveAbility { unit: usize, slot: usize, name: String },
    AddGold { player: u8, amount: i32 },
    AddUnitGold { unit: usize, amount: i32 },
    RemoveUnit { unit: usize },
    GiveItem { unit: usize, item: usize, count: u32 },
    SetClassOverride { unit: usize, class: Option<u8> },
    SetHoldOverride { unit: usize, hold: Option<bool> },
    SkillUp { unit: usize, slot: usize },
    RemovePlayer { player: u8 },
    // A spawn-group unit's respawn timer: Some(s) respawns it s seconds from now, None leaves it dead.
    SetRespawn { unit: usize, delay_s: Option<u32> },
    SetInvulnerable { unit: usize, ticks: u32 },
    SetReduction { unit: usize, pct: u8 },
    SetHealing { unit: usize, hp_per_s: i32 },
    DeclareSide { side: u8 },
    AttackMove { unit: usize, x: i32, y: i32 },
    SetTeam { player: u8, team: u8 },
    PackStash { unit: usize },
    SaveOrder { unit: usize, x: i32, y: i32 },
    RestoreOrder { unit: usize },
    Victory { player: u8 },
    Defeat { player: u8 },
    // A client-facing UI event as a JSON object body, kept for the caller to stream (take_ui).
    Ui { body: String },
}

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

#[derive(Clone, Copy)]
struct UnitSnap {
    x: i32,
    y: i32,
    hp: i32,
    alive: bool,
    killer: Option<usize>,
}

enum Pending {
    Spawned(usize),
    Attacked(usize),
    Died(usize, Option<usize>),
    Enter(usize, usize),
    Leave(usize, usize),
    PlayerJoined(usize),
}

// What the native functions and the host both touch: the sim, regions, the queued commands and
// the counters the witness reads.
struct Shared {
    sim: Sim,
    regions: Vec<Region>,
    // Commands from the mod's own handlers (on_died, on_unit_died), applied before `cmds` as mlua does.
    local: Vec<Cmd>,
    cmds: Vec<Cmd>,
    clock: u64,
    fired: u64,
    applied: u64,
    ui: Vec<String>,
}

type Sh = Rc<RefCell<Shared>>;

struct Hooks {
    emit: StashedFunction,
    run_systems: StashedFunction,
    fire_timers: StashedFunction,
    // The mod's named hooks, when it defines them (the same names mlua looks up).
    on_unit_died: Option<StashedFunction>,
    on_cast: Option<StashedFunction>,
    on_march: Option<StashedFunction>,
    on_check_win: Option<StashedFunction>,
    target_priority: Option<StashedFunction>,
}

// A hook's answer, reduced from a Lua value: nil, a boolean, an integer, or another type's name.
#[derive(Clone, Copy)]
enum Ret {
    Nil,
    Bool(bool),
    Int(i64),
    Other,
}

// The respawn decision on_unit_died returns, as cmd_set_respawn takes it: None when the hook leaves the unit
// alone or answers with a value the decision does not take; Some(None) to leave it dead; Some(Some(s)) to respawn.
fn respawn_of(r: Ret) -> Option<Option<u32>> {
    match r {
        Ret::Bool(false) => Some(None),
        Ret::Int(n) if (0..=MAX_RESPAWN_S).contains(&n) => Some(Some(n as u32)),
        _ => None,
    }
}

impl Ret {
    fn of(v: Value) -> Self {
        match v {
            Value::Nil => Ret::Nil,
            Value::Boolean(b) => Ret::Bool(b),
            Value::Integer(n) => Ret::Int(n),
            Value::Number(n) if n.fract() == 0.0 => Ret::Int(n as i64),
            _ => Ret::Other,
        }
    }
}

// The Lua half of the host, run once per mod load with (env, known events, rows, note_fired, clock,
// max_players). It adds game.system, game.each, game.trigger, schedule and cancel to the mod
// environment, installs the shared var, score, region and timer source over them, and returns emit (the global handler, then every trigger of the event), run_systems and fire_timers.
// A trigger runs its actions only when every condition returns truthy.
fn prelude() -> String {
    format!("local install = (function()\n{PRELUDE_SRC}\nend)()\n{HOST_PRELUDE}")
}

const HOST_PRELUDE: &str = r#"
local env, known, rows, note_fired, clock, max_players = ...
local game = env.game
local systems, triggers, timers = {}, {}, {}
local function remove(list, i)
  local item = list[i]
  for j = i, #list do list[j] = list[j + 1] end
  return item
end
function game.system(name, comps, func)
  for _, s in ipairs(systems) do
    if s.name == name then error("duplicate_system: " .. name .. " is already registered", 2) end
  end
  systems[#systems + 1] = { name = name, comps = comps, func = func }
end
function game.each(comps, func)
  for _, row in ipairs(rows(comps)) do func(row) end
end
function game.trigger(event, spec)
  if not known[event] then error("unknown_event: " .. tostring(event), 2) end
  if type(spec) ~= "table" then error("invalid_trigger: " .. event .. " needs a table of conditions and actions", 2) end
  local entry = { conditions = spec.conditions or {}, actions = spec.actions or {} }
  if spec.action then entry.actions[#entry.actions + 1] = spec.action end
  if #entry.actions == 0 then error("invalid_trigger: " .. event .. " has no action", 2) end
  triggers[event] = triggers[event] or {}
  triggers[event][#triggers[event] + 1] = entry
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
      note_fired()
      for _, act in ipairs(entry.actions) do act(...) end
    end
  end
end
function env.schedule(name, after, action)
  if after < 0 then error("schedule_delay: " .. after .. " is negative", 2) end
  local due = clock() + (after < 1 and 1 or after)
  for _, t in ipairs(timers) do
    if t.name == name then t.due, t.action = due, action return end
  end
  if #timers >= 256 then error("timer_limit: at most 256 timers", 2) end
  timers[#timers + 1] = { name = name, due = due, action = action }
end
function env.cancel(name)
  for i, t in ipairs(timers) do
    if t.name == name then remove(timers, i) return true end
  end
  return false
end
local host = { define_region = env.define_region, unit_position = env.unit_position, in_region = env.in_region, region_bounds = env.region_bounds, map_size = env.map_size, schedule = env.schedule, cancel = env.cancel }
install(game, { global = {}, player = {}, unit = {}, score = {} }, host, max_players)
local function emit(event, ...)
  local handler = env[event]
  if handler then handler(...) end
  fire(event, ...)
end
local function run_systems()
  for _, s in ipairs(systems) do
    for _, row in ipairs(rows(s.comps)) do s.func(row) end
  end
end
local function fire_timers()
  local now = clock()
  while true do
    local hit
    for i, t in ipairs(timers) do
      if t.due <= now then hit = i break end
    end
    if not hit then return end
    remove(timers, hit).action()
  end
end
return emit, run_systems, fire_timers
"#;

fn fail<'gc>(ctx: Context<'gc>, msg: String) -> Error<'gc> {
    Error::from_value(msg.into_value(ctx))
}

fn int<'gc>(ctx: Context<'gc>, v: Value<'gc>, i: usize) -> Result<i64, Error<'gc>> {
    v.to_integer().ok_or_else(|| fail(ctx, format!("bad_argument: argument {} must be an integer", i + 1)))
}

fn a<'gc>(ctx: Context<'gc>, s: &Stack<'gc, '_>, n: usize) -> Result<i64, Error<'gc>> {
    int(ctx, s.get(n), n)
}

fn text<'gc>(ctx: Context<'gc>, v: Value<'gc>, i: usize) -> Result<String, Error<'gc>> {
    match v {
        Value::String(s) => s.to_str().map(str::to_owned).map_err(|_| fail(ctx, format!("bad_argument: argument {} is not utf8", i + 1))),
        _ => Err(fail(ctx, format!("bad_argument: argument {} must be a string", i + 1))),
    }
}

fn num<'gc>(ctx: Context<'gc>, v: Value<'gc>, i: usize) -> Result<f64, Error<'gc>> {
    v.to_number().ok_or_else(|| fail(ctx, format!("bad_argument: argument {} must be a number", i + 1)))
}

fn num_opt<'gc>(ctx: Context<'gc>, v: Value<'gc>, i: usize) -> Result<Option<f64>, Error<'gc>> {
    if matches!(v, Value::Nil) {
        Ok(None)
    } else {
        num(ctx, v, i).map(Some)
    }
}

fn text_opt<'gc>(ctx: Context<'gc>, v: Value<'gc>, i: usize) -> Result<Option<String>, Error<'gc>> {
    if matches!(v, Value::Nil) {
        Ok(None)
    } else {
        text(ctx, v, i).map(Some)
    }
}

// A table-valued argument, the spec of game.ability or game.panel.
fn table_arg<'gc>(ctx: Context<'gc>, v: Value<'gc>, i: usize) -> Result<Table<'gc>, Error<'gc>> {
    match v {
        Value::Table(t) => Ok(t),
        _ => Err(fail(ctx, format!("bad_argument: argument {} must be a table", i + 1))),
    }
}

// An integer field of a spec table, `default` when absent.
fn field_int<'gc>(ctx: Context<'gc>, t: Table<'gc>, key: &str, default: i64) -> Result<i64, String> {
    match t.get(ctx, ctx.intern(key.as_bytes())) {
        Value::Nil => Ok(default),
        v => v.to_integer().ok_or_else(|| format!("bad_field: {key} must be an integer")),
    }
}

fn field_text<'gc>(ctx: Context<'gc>, t: Table<'gc>, key: &str) -> Result<String, String> {
    match t.get(ctx, ctx.intern(key.as_bytes())) {
        Value::String(s) => s.to_str().map(str::to_owned).map_err(|_| format!("bad_field: {key} is not utf8")),
        _ => Err(format!("bad_field: {key} must be a string")),
    }
}

fn field_text_opt<'gc>(ctx: Context<'gc>, t: Table<'gc>, key: &str) -> Result<Option<String>, String> {
    match t.get(ctx, ctx.intern(key.as_bytes())) {
        Value::Nil => Ok(None),
        _ => field_text(ctx, t, key).map(Some),
    }
}

// One ability as a Lua table: its catalog name, timing, target rules and effects.
fn ability_row<'gc>(ctx: Context<'gc>, cat: &Catalog, index: usize) -> Table<'gc> {
    let a = &cat.abilities[index];
    let row = Table::new(&ctx);
    row.set(ctx, "name", cat.name(a.name).to_owned()).expect("row");
    row.set(ctx, "cooldown", a.cooldown as i64).expect("row");
    row.set(ctx, "energy", a.energy as i64).expect("row");
    row.set(ctx, "target", a.target as i64).expect("row");
    row.set(ctx, "filter", a.filter as i64).expect("row");
    row.set(ctx, "projectile", a.is_projectile()).expect("row");
    row.set(ctx, "projectile_speed", a.projectile_speed as i64).expect("row");
    let first = a.effect_first as usize;
    let span = cat.effects.get(first..first + a.effect_count as usize).unwrap_or(&[]);
    let effects = Table::new(&ctx);
    for (i, e) in span.iter().enumerate() {
        let et = Table::new(&ctx);
        et.set(ctx, "kind", e.kind as i64).expect("row");
        et.set(ctx, "p0", e.p0 as i64).expect("row");
        et.set(ctx, "p1", e.p1 as i64).expect("row");
        et.set(ctx, "p2", e.p2 as i64).expect("row");
        et.set(ctx, "p3", e.p3 as i64).expect("row");
        effects.set(ctx, i as i64 + 1, et).expect("row");
    }
    row.set(ctx, "effects", effects).expect("row");
    row
}

fn known_unit(sim: &Sim, i: i64) -> Result<usize, String> {
    usize::try_from(i).ok().filter(|&u| u < sim.count()).ok_or_else(|| format!("invalid_unit: {i}"))
}

// The largest respawn delay a mod can ask for, in seconds (mlua's MAX_RESPAWN_S).
const MAX_RESPAWN_S: i64 = 86_400;

// A Lua sequence of `items`, indexed from 1.
fn list_of<'gc>(ctx: Context<'gc>, items: impl Iterator<Item = i64>) -> Result<Table<'gc>, Error<'gc>> {
    let out = Table::new(&ctx);
    for (n, v) in items.enumerate() {
        out.set(ctx, n as i64 + 1, v).expect("list");
    }
    Ok(out)
}

fn live_unit(sim: &Sim, i: i64) -> Result<usize, String> {
    usize::try_from(i).ok().filter(|&u| sim.alive(u)).ok_or_else(|| format!("invalid_unit: {i}"))
}

fn slot_of(slot: i64, limit: usize) -> Result<usize, String> {
    usize::try_from(slot).ok().filter(|&s| s < limit).ok_or_else(|| format!("invalid_slot: {slot} is outside 0..{limit}"))
}

fn buff_args(kind: i64, ticks: i64) -> Result<(i32, i32), String> {
    let kind = i32::try_from(kind).map_err(|_| format!("invalid_buff: kind {kind} out of i32 range"))?;
    let ticks = i32::try_from(ticks).ok().filter(|&t| t > 0).ok_or_else(|| format!("invalid_buff: ticks {ticks} must be a positive i32"))?;
    Ok((kind, ticks))
}

// A cast or item target: -1 is the point, anything else must be a live unit.
fn target_of(sim: &Sim, target: i64) -> Result<i32, String> {
    if target == -1 {
        Ok(-1)
    } else {
        live_unit(sim, target).map(|t| t as i32)
    }
}

// Declares a global or game-table function. `[names]` are the Rc handles the body captures; the
// body reads arguments with a(n) and returns through ret or Ok(CallbackReturn::Return).
macro_rules! native {
    ($ctx:ident, $table:ident, $name:literal, [$($cap:ident),*], |$c:ident, $s:ident| $body:block) => {{
        $(#[allow(unused_variables)] let $cap = $cap.clone();)*
        #[allow(unused_mut, unused_variables)]
        let callback = Callback::from_fn(&$ctx, move |$c, _, mut $s| $body);
        $table.set($ctx, $name, callback).expect("bind native");
    }};
}

fn bind<'gc>(ctx: Context<'gc>, env: Table<'gc>, sh: &Sh) {
    let game = Table::new(&ctx);

    native!(ctx, env, "spawn_unit", [sh], |c, s| {
        let kind = text(c, s.get(0), 0)?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        if let Some(index) = sh.sim.catalog().unit_index(&kind) {
            sh.sim.spawn(index as u32, x, y);
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "get_unit_count", [sh], |c, s| {
        s.replace(c, sh.borrow().sim.count() as i64);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "define_region", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let mut v = [0i32; 4];
        for (k, slot) in v.iter_mut().enumerate() {
            *slot = fixed(a(c, &s, k + 1)?).map_err(|m| fail(c, m))?;
        }
        let region = Region { name: name.clone(), x0: v[0].min(v[2]), y0: v[1].min(v[3]), x1: v[0].max(v[2]), y1: v[1].max(v[3]) };
        let mut sh = sh.borrow_mut();
        if let Some(slot) = sh.regions.iter_mut().find(|g| g.name == name) {
            *slot = region;
        } else if sh.regions.len() >= MAX_REGIONS {
            return Err(fail(c, format!("region_limit: at most {MAX_REGIONS} regions")));
        } else {
            sh.regions.push(region);
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "in_region", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let hit = sh.borrow().regions.iter().find(|g| g.name == name).map(|g| g.contains(x, y));
        s.replace(c, hit.ok_or_else(|| fail(c, format!("unknown_region: {name}")))?);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "region_bounds", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let hit = sh.borrow().regions.iter().find(|g| g.name == name).map(|g| (i64::from(g.x0), i64::from(g.y0), i64::from(g.x1), i64::from(g.y1)));
        s.replace(c, hit.ok_or_else(|| fail(c, format!("unknown_region: {name}")))?);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "map_size", [sh], |c, s| {
        let (w, h) = {
            let sh = sh.borrow();
            (sh.sim.grid().w as i64, sh.sim.grid().h as i64)
        };
        s.replace(c, (w << 16, h << 16));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "order_unit", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        sh.sim.order(u, x, y);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "order_player", [sh], |c, s| {
        let p = a(c, &s, 0)?;
        let player = u8::try_from(p).map_err(|_| fail(c, format!("invalid_player: {p}")))?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        sh.borrow_mut().sim.order_all(player, x, y);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "unit_position", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let (x, y) = sh.sim.position_fixed(u);
        s.replace(c, (x as i64, y as i64));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "unit_hp", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.hp(u) as i64);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "unit_player", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.player(u) as i64);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "unit_alive", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.alive(u));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "damage_unit", [sh], |c, s| {
        let (caster, target, dmg) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let dmg = i32::try_from(dmg).map_err(|_| fail(c, format!("invalid_damage: {dmg} out of i32 range")))?;
        let mut sh = sh.borrow_mut();
        let cu = live_unit(&sh.sim, caster).map_err(|m| fail(c, m))?;
        let t = live_unit(&sh.sim, target).map_err(|m| fail(c, m))?;
        let player = sh.sim.player(cu);
        sh.sim.damage_target(cu as i32, player, dmg, t as i32);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "buff_unit", [sh], |c, s| {
        let (unit, kind, ticks) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let (kind, ticks) = buff_args(kind, ticks).map_err(|m| fail(c, m))?;
        let ok = sh.sim.apply_buff(u, kind, ticks);
        s.replace(c, ok);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "list_abilities", [sh], |c, s| {
        let sh = sh.borrow();
        let cat = sh.sim.catalog();
        let out = Table::new(&c);
        for i in 0..cat.abilities.len() {
            out.set(c, i as i64 + 1, ability_row(c, cat, i)).expect("row");
        }
        s.replace(c, out);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "ability_info", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let sh = sh.borrow();
        let cat = sh.sim.catalog();
        let index = cat.abilities.iter().position(|a| cat.name(a.name) == name).ok_or_else(|| fail(c, format!("unknown_ability: {name}")))?;
        s.replace(c, ability_row(c, cat, index));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "list_events", [sh], |c, s| {
        let out = Table::new(&c);
        for (i, (name, args)) in EVENTS.iter().enumerate() {
            let row = Table::new(&c);
            row.set(c, "name", *name).expect("row");
            row.set(c, "args", *args).expect("row");
            out.set(c, i as i64 + 1, row).expect("row");
        }
        s.replace(c, out);
        Ok(CallbackReturn::Return)
    });

    native!(ctx, game, "cmd_spawn", [sh], |c, s| {
        let kind = text(c, s.get(0), 0)?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let owner = match s.get(3) {
            Value::Nil => 0,
            _ => player_arg(a(c, &s, 3)?).map_err(|m| fail(c, m))?,
        };
        let route = match s.get(4) {
            Value::Nil => 0,
            _ => i32::try_from(a(c, &s, 4)?).map_err(|_| fail(c, "invalid_route: route out of i32 range".to_string()))?,
        };
        let mut sh = sh.borrow_mut();
        let index = sh.sim.catalog().unit_index(&kind).ok_or_else(|| fail(c, format!("unknown_unit: {kind}")))?;
        sh.cmds.push(Cmd::Spawn { index, x, y, owner, route });
        let handle = sh.cmds.len() as i64;
        s.replace(c, handle);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_order", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::Order { unit, x, y });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_remove", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::RemoveUnit { unit: u });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_give_item", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let name = text(c, s.get(1), 1)?;
        let count = a(c, &s, 2)?;
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let item = sh.sim.catalog().item_index(&name).ok_or_else(|| fail(c, format!("unknown_item: {name}")))?;
        let count = u32::try_from(count).ok().filter(|&n| n > 0).ok_or_else(|| fail(c, format!("invalid_count: {count} must be 1 or more")))?;
        sh.cmds.push(Cmd::GiveItem { unit: u, item, count });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "units_near", [sh], |c, s| {
        let (x, y, r) = (
            fixed(a(c, &s, 0)?).map_err(|m| fail(c, m))?,
            fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?,
            fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?,
        );
        let ids = sh.borrow().sim.units_near(x, y, r);
        s.replace(c, list_of(c, ids.into_iter().map(|u| u as i64))?);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "is_structure", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.is_structure(u));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "is_hero", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.is_hero(u));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "hostile", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let q = player_arg(a(c, &s, 1)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.borrow().sim.hostile(p, q));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "filter_bits", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.filter_bits(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "team", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let team = sh.borrow().sim.team_of(p).map_or(Value::Nil, |t| Value::Integer(i64::from(t)));
        s.replace(c, team);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_team", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let t = a(c, &s, 1)?;
        let team = usize::try_from(t).ok().filter(|&t| t < MAX_PLAYERS).ok_or_else(|| fail(c, format!("invalid_team: {t} is outside 0..{MAX_PLAYERS}")))?;
        sh.borrow_mut().cmds.push(Cmd::SetTeam { player: p, team: team as u8 });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "within", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let reach = i32::try_from(a(c, &s, 3)?).map_err(|_| fail(c, "invalid_reach: out of i32 range".into()))?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.within_reach(u, x, y, reach));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "doodads_near", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let reach = i32::try_from(a(c, &s, 1)?).map_err(|_| fail(c, "invalid_reach: out of i32 range".into()))?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        s.replace(c, list_of(c, sh.sim.doodads_near(u, reach).into_iter().map(i64::from))?);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_stash", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let cat = sh.sim.catalog();
        let out = Table::new(&c);
        let mut n = 0i64;
        for slot in 0..sh.sim.inventory_slots() {
            let (kind, count) = sh.sim.stash_slot(u, slot);
            let Some(item) = (kind as usize).checked_sub(1).and_then(|i| cat.items.get(i)) else { continue };
            let row = Table::new(&c);
            row.set(c, "slot", slot as i64).expect("row");
            row.set(c, "item", i64::from(kind) - 1).expect("row");
            row.set(c, "name", cat.name(item.name).to_owned()).expect("row");
            row.set(c, "count", i64::from(count)).expect("row");
            n += 1;
            out.set(c, n, row).expect("row");
        }
        s.replace(c, out);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "item_info", [sh], |c, s| {
        let item = a(c, &s, 0)?;
        let sh = sh.borrow();
        let cat = sh.sim.catalog();
        let Some(it) = usize::try_from(item).ok().and_then(|i| cat.items.get(i)) else {
            s.replace(c, Value::Nil);
            return Ok(CallbackReturn::Return);
        };
        let t = Table::new(&c);
        t.set(c, "name", cat.name(it.name).to_owned()).expect("item");
        t.set(c, "cost", i64::from(it.cost)).expect("item");
        t.set(c, "charges", i64::from(it.charges)).expect("item");
        t.set(c, "stack_max", i64::from(it.stack_max)).expect("item");
        t.set(c, "shops", i64::from(it.shops)).expect("item");
        t.set(c, "stock_max", i64::from(it.stock_max)).expect("item");
        t.set(c, "stock_initial", i64::from(it.stock_initial)).expect("item");
        t.set(c, "stock_time_s", i64::from(it.stock_time_s)).expect("item");
        t.set(c, "stock_start_s", i64::from(it.stock_start_s)).expect("item");
        t.set(c, "components", list_of(c, it.components.iter().filter(|&&k| k != NO_COMPONENT).map(|&k| i64::from(k)))?).expect("item");
        s.replace(c, t);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_damage_reduction", [sh], |c, s| {
        let unit = usize::try_from(a(c, &s, 0)?).map_err(|_| fail(c, "invalid_unit".into()))?;
        let pct = a(c, &s, 1)?;
        let pct = u8::try_from(pct).ok().filter(|&p| p <= 100).ok_or_else(|| fail(c, format!("invalid_reduction: {pct} is outside 0..=100")))?;
        sh.borrow_mut().cmds.push(Cmd::SetReduction { unit, pct });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_invulnerable", [sh], |c, s| {
        let unit = usize::try_from(a(c, &s, 0)?).map_err(|_| fail(c, "invalid_unit".into()))?;
        let t = a(c, &s, 1)?;
        let ticks = u32::try_from(t).map_err(|_| fail(c, format!("invalid_ticks: {t}")))?;
        sh.borrow_mut().cmds.push(Cmd::SetInvulnerable { unit, ticks });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_healing", [sh], |c, s| {
        let unit = usize::try_from(a(c, &s, 0)?).map_err(|_| fail(c, "invalid_unit".into()))?;
        let h = a(c, &s, 1)?;
        let hp_per_s = i32::try_from(h).ok().filter(|&v| v >= 0).ok_or_else(|| fail(c, format!("invalid_healing: {h} is outside 0..=2147483647")))?;
        sh.borrow_mut().cmds.push(Cmd::SetHealing { unit, hp_per_s });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "declare_side", [sh], |c, s| {
        let side = a(c, &s, 0)?;
        let side = u8::try_from(side).ok().filter(|&v| (v as usize) < MAX_PLAYERS).ok_or_else(|| fail(c, format!("invalid_side: {side} is outside 0..{MAX_PLAYERS}")))?;
        sh.borrow_mut().cmds.push(Cmd::DeclareSide { side });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "attack_move", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::AttackMove { unit, x, y });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_pack_stash", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::PackStash { unit });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_save_order", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::SaveOrder { unit, x, y });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_restore_order", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::RestoreOrder { unit });
        Ok(CallbackReturn::Return)
    });
    // game.cmd_set_respawn(unit, seconds | false): the respawn decision for a dead unit (false leaves it dead).
    native!(ctx, game, "cmd_set_respawn", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let delay_s = match s.get(1) {
            Value::Boolean(false) => None,
            Value::Integer(n) if (0..=MAX_RESPAWN_S).contains(&n) => Some(n as u32),
            other => return Err(fail(c, format!("invalid_respawn: {}", other.type_name()))),
        };
        let mut sh = sh.borrow_mut();
        let unit = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::SetRespawn { unit, delay_s });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_target_class", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let class = a(c, &s, 1)?;
        if !(0..=3).contains(&class) {
            return Err(fail(c, format!("invalid_target_class: {class} is outside 0..=3")));
        }
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::SetClassOverride { unit, class: Some(class as u8) });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "clear_target_class", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::SetClassOverride { unit, class: None });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_attack_hold", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let hold = s.get(1).to_bool();
        let mut sh = sh.borrow_mut();
        let unit = usize::try_from(unit).map_err(|_| fail(c, format!("invalid_unit: {unit}")))?;
        sh.cmds.push(Cmd::SetHoldOverride { unit, hold: Some(hold) });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "clear_attack_hold", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let mut sh = sh.borrow_mut();
        let unit = usize::try_from(unit).map_err(|_| fail(c, format!("invalid_unit: {unit}")))?;
        sh.cmds.push(Cmd::SetHoldOverride { unit, hold: None });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_skill_up", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let slot = a(c, &s, 1)?;
        let slot = usize::try_from(slot).ok().filter(|&v| v < ABILITY_SLOTS).ok_or_else(|| fail(c, format!("invalid_slot: {slot} is outside 0..{ABILITY_SLOTS}")))?;
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::SkillUp { unit, slot });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "remove_player", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::RemovePlayer { player: p });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_damage", [sh], |c, s| {
        let (caster, target, dmg) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let mut sh = sh.borrow_mut();
        let (cu, t) = (live_unit(&sh.sim, caster).map_err(|m| fail(c, m))?, live_unit(&sh.sim, target).map_err(|m| fail(c, m))?);
        let dmg = i32::try_from(dmg).map_err(|_| fail(c, format!("invalid_damage: {dmg} out of i32 range")))?;
        sh.cmds.push(Cmd::Damage { caster: cu, target: t, dmg });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_buff", [sh], |c, s| {
        let (unit, kind, ticks) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let (kind, ticks) = buff_args(kind, ticks).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::Buff { unit: u, kind, ticks });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_cast", [sh], |c, s| {
        let (unit, slot, target) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let (x, y) = (fixed(a(c, &s, 3)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 4)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ABILITY_SLOTS).map_err(|m| fail(c, m))?;
        let target = target_of(&sh.sim, target).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::Cast { unit: u, slot, target, x, y });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_use_item", [sh], |c, s| {
        let (unit, slot, target) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let (x, y) = (fixed(a(c, &s, 3)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 4)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ITEM_SLOTS).map_err(|m| fail(c, m))?;
        let target = target_of(&sh.sim, target).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::UseItem { unit: u, slot, target, x, y });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_buy", [sh], |c, s| {
        let (unit, item) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let item = usize::try_from(item).map_err(|_| fail(c, format!("invalid_item: {item}")))?;
        sh.cmds.push(Cmd::Buy { unit: u, item });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_sell", [sh], |c, s| {
        let (unit, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ITEM_SLOTS).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::Sell { unit: u, slot });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_unstash", [sh], |c, s| {
        let (unit, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ITEM_SLOTS).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::Unstash { unit: u, slot });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_attack", [sh], |c, s| {
        let (attacker, target) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let (u, t) = (live_unit(&sh.sim, attacker).map_err(|m| fail(c, m))?, live_unit(&sh.sim, target).map_err(|m| fail(c, m))?);
        sh.cmds.push(Cmd::Attack { unit: u, target: t });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_set_owner", [sh], |c, s| {
        let (unit, player) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let player = usize::try_from(player)
            .ok()
            .filter(|&p| p < MAX_PLAYERS)
            .ok_or_else(|| fail(c, format!("invalid_player: {player} is outside 0..{MAX_PLAYERS}")))?;
        sh.cmds.push(Cmd::SetOwner { unit: u, player: player as u8 });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_set_fog", [sh], |c, s| {
        let on = s.get(0).to_bool();
        sh.borrow_mut().cmds.push(Cmd::SetFog { on });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "ability", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let spec = table_arg(c, s.get(1), 1)?;
        let mut sh = sh.borrow_mut();
        let int = |key: &str| field_int(c, spec, key, 0).and_then(fixed);
        let mut effects = Vec::new();
        if let Value::Table(list) = spec.get(c, "effects") {
            for k in 1..=list.length() {
                let entry = table_arg(c, list.get(c, k), k as usize - 1)?;
                let field = |key: &str| field_int(c, entry, key, 0).and_then(fixed);
                let kind = field_text(c, entry, "kind").map_err(|m| fail(c, m))?;
                let txt = |key: &str| field_text(c, entry, key);
                effects.push(shared::effect_from(&sh.sim, &kind, &field, &txt).map_err(|m| fail(c, m))?);
            }
        }
        let target = field_text_opt(c, spec, "target").map_err(|m| fail(c, m))?;
        let (target, cooldown, energy, filter) = (|| shared::ability_args(target.as_deref(), int("cooldown")?, int("energy")?, int("filter")?))().map_err(|m| fail(c, m))?;
        sh.sim
            .define_ability(AbilitySpec { name: &name, cooldown, energy, target, filter, effects })
            .map_err(|why| fail(c, format!("invalid_ability: {name}: {why}")))?;
        s.replace(c, name);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_give_ability", [sh], |c, s| {
        let (unit, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let name = text(c, s.get(2), 2)?;
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ABILITY_SLOTS).map_err(|m| fail(c, m))?;
        let cat = sh.sim.catalog();
        if !cat.abilities.iter().any(|a| cat.name(a.name) == name) {
            return Err(fail(c, format!("unknown_ability: {name}")));
        }
        sh.cmds.push(Cmd::GiveAbility { unit: u, slot, name });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_add_gold", [sh], |c, s| {
        let (player, amount) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let player = player_arg(player).map_err(|m| fail(c, m))?;
        let amount = amount_arg(amount).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::AddGold { player, amount });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_add_unit_gold", [sh], |c, s| {
        let (unit, amount) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let amount = amount_arg(amount).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::AddUnitGold { unit: u, amount });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "message", [sh], |c, s| {
        let (player, msg) = (a(c, &s, 0)?, text(c, s.get(1), 1)?);
        let seconds = num_opt(c, s.get(2), 2)?;
        let body = shared::ui_message(player, &msg, seconds).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::Ui { body });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "banner", [sh], |c, s| {
        let msg = text(c, s.get(0), 0)?;
        let seconds = num_opt(c, s.get(1), 1)?;
        let body = shared::ui_banner(&msg, seconds).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::Ui { body });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "panel", [sh], |c, s| {
        let id = text(c, s.get(0), 0)?;
        let spec = match s.get(1) {
            Value::Nil => None,
            v => {
                let t = table_arg(c, v, 1)?;
                let player = match t.get(c, "player") {
                    Value::Nil => None,
                    v => Some(int(c, v, 1)?),
                };
                Some((field_text_opt(c, t, "title").map_err(|m| fail(c, m))?, field_text_opt(c, t, "text").map_err(|m| fail(c, m))?, player))
            }
        };
        let body = shared::ui_panel(&id, spec).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::Ui { body });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "camera", [sh], |c, s| {
        let player = a(c, &s, 0)?;
        let (x, y) = (num(c, s.get(1), 1)?, num(c, s.get(2), 2)?);
        let (zoom, seconds) = (num_opt(c, s.get(3), 3)?, num_opt(c, s.get(4), 4)?);
        let body = shared::ui_camera(player, x, y, zoom, seconds).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::Ui { body });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "sound", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        sh.borrow_mut().cmds.push(Cmd::Ui { body: shared::ui_sound(&name) });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "victory", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::Victory { player });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "defeat", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::Defeat { player });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "winner", [sh], |c, s| {
        let winner = sh.borrow().sim.winner().map(i64::from);
        s.replace(c, winner);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "gold", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let gold = sh.borrow().sim.resource_a(player) as i64;
        s.replace(c, gold);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_gold", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.unit_gold(u) as i64);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "units", [sh], |c, s| {
        let player = s.get(0).to_integer();
        let sh = sh.borrow();
        let list: Vec<i64> = (0..sh.sim.count())
            .filter(|&i| sh.sim.alive(i) && player.map_or(true, |p| sh.sim.player(i) as i64 == p))
            .map(|i| i as i64)
            .collect();
        s.replace(c, list);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "players", [sh], |c, s| {
        let counts = living_per_player(&sh.borrow().sim);
        let list = Table::new(&c);
        for (p, alive) in counts.iter().enumerate() {
            let row = Table::new(&c);
            row.set(c, "id", p as i64).expect("row");
            row.set(c, "units", *alive as i64).expect("row");
            list.set(c, p as i64 + 1, row).expect("row");
        }
        s.replace(c, list);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_kind", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.kind_name(u).map(str::to_owned));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_bounty", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.unit_bounty(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "rival_of", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let sh = sh.borrow();
        s.replace(c, sh.sim.rival_of(player).map(i64::from));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "time", [sh], |c, s| {
        let ticks = sh.borrow().sim.ticks();
        s.replace(c, (i64::from(ticks), f64::from(ticks) / f64::from(TICKS_PER_SECOND)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_cooldown", [sh], |c, s| {
        let (unit, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ABILITY_SLOTS).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.ability_cooldown(u, slot)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_level", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.level(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_abilities", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let cat = sh.sim.catalog();
        let list = Table::new(&c);
        let mut n = 0i64;
        for slot in 0..ABILITY_SLOTS {
            let Some(ability) = sh.sim.ability_in_slot(u, slot).and_then(|ai| cat.abilities.get(ai)) else { continue };
            let row = Table::new(&c);
            row.set(c, "slot", slot as i64).expect("row");
            row.set(c, "name", cat.name(ability.name).to_owned()).expect("row");
            row.set(c, "cooldown", i64::from(sh.sim.ability_cooldown(u, slot))).expect("row");
            n += 1;
            list.set(c, n, row).expect("row");
        }
        s.replace(c, list);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_items", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let cat = sh.sim.catalog();
        let list = Table::new(&c);
        let mut n = 0i64;
        for slot in 0..ITEM_SLOTS {
            let (kind, count) = sh.sim.inventory_slot(u, slot);
            let Some(item) = (kind as usize).checked_sub(1).and_then(|i| cat.items.get(i)) else { continue };
            let row = Table::new(&c);
            row.set(c, "slot", slot as i64).expect("row");
            row.set(c, "name", cat.name(item.name).to_owned()).expect("row");
            row.set(c, "count", i64::from(count)).expect("row");
            row.set(c, "cooldown", i64::from(sh.sim.item_cooldown(u, slot))).expect("row");
            n += 1;
            list.set(c, n, row).expect("row");
        }
        s.replace(c, list);
        Ok(CallbackReturn::Return)
    });
    env.set(ctx, "game", game).expect("bind game");
}

// The living units each player owns, indexed by player id (0..MAX_PLAYERS).
fn living_per_player(sim: &Sim) -> Vec<usize> {
    let mut counts = vec![0usize; MAX_PLAYERS];
    for i in (0..sim.count()).filter(|&i| sim.alive(i)) {
        counts[sim.player(i) as usize % MAX_PLAYERS] += 1;
    }
    counts
}

// The components a system or game.each can ask for, and the table a script sees for one unit.
fn rows<'gc>(ctx: Context<'gc>, sim: &Sim, comps: Table<'gc>) -> Result<Table<'gc>, Error<'gc>> {
    let mut wanted = Vec::new();
    for k in 1..=comps.length() {
        let name = text(ctx, comps.get(ctx, k), k as usize - 1)?;
        match name.as_str() {
            "position" | "owner" | "hp" => wanted.push(name),
            other => return Err(fail(ctx, format!("unknown_component: {other} (known: position, owner, hp)"))),
        }
    }
    let out = Table::new(&ctx);
    let mut n = 0i64;
    for i in (0..sim.count()).filter(|&i| sim.alive(i)) {
        let row = Table::new(&ctx);
        row.set(ctx, "id", i as i64).expect("row");
        for name in &wanted {
            match name.as_str() {
                "position" => {
                    let (x, y) = sim.position_fixed(i);
                    let p = Table::new(&ctx);
                    p.set(ctx, "x", x as i64).expect("row");
                    p.set(ctx, "y", y as i64).expect("row");
                    row.set(ctx, "position", p).expect("row");
                }
                "owner" => {
                    row.set(ctx, "owner", sim.player(i) as i64).expect("row");
                }
                _ => {
                    row.set(ctx, "hp", sim.hp(i) as i64).expect("row");
                }
            }
        }
        n += 1;
        out.set(ctx, n, row).expect("row");
    }
    Ok(out)
}

fn mtime(path: &Path) -> Result<SystemTime, String> {
    std::fs::metadata(path).and_then(|m| m.modified()).map_err(|e| format!("stat {}: {e}", path.display()))
}

fn mod_name(path: &Path) -> String {
    path.file_stem().map_or_else(|| "mod".to_string(), |s| s.to_string_lossy().into_owned())
}

pub struct ModHost {
    lua: Lua,
    sh: Sh,
    state: StashedTable,
    hooks: Option<Hooks>,
    version: u64,
    joined: Vec<usize>,
    snap: Vec<UnitSnap>,
    prev: Vec<UnitSnap>,
    inside: Vec<u64>,
    pending: Vec<Pending>,
    mod_file: Option<(PathBuf, SystemTime)>,
}

impl ModHost {
    /// A host over the sim loaded from `image`, an OSDT v3 image (rust/crates/data/FORMAT.md).
    /// `state.ticks` starts at 0.
    pub fn new(image: &[u8]) -> Result<Self, LoadError> {
        let sim = sim::load(image, MAX_UNITS)?;
        let mut lua = Lua::core();
        let state = lua.enter(|ctx| {
            let t = Table::new(&ctx);
            t.set(ctx, "ticks", 0i64).expect("seed state.ticks");
            ctx.stash(t)
        });
        Ok(Self {
            lua,
            sh: Rc::new(RefCell::new(Shared { sim, regions: Vec::new(), local: Vec::new(), cmds: Vec::new(), clock: 0, fired: 0, applied: 0, ui: Vec::new() })),
            state,
            hooks: None,
            version: 0,
            joined: Vec::new(),
            snap: Vec::new(),
            prev: Vec::new(),
            inside: vec![0; MAX_UNITS],
            pending: Vec::new(),
            mod_file: None,
        })
    }

    /// Compiles the mod in `src`, named `name` in error messages, runs its top level in its own
    /// environment, and makes it the active version. The mod must define on_tick.
    pub fn load_source(&mut self, name: &str, src: &str) -> Result<(), String> {
        let sh = self.sh.clone();
        let (prelude, env) = self
            .lua
            .try_enter(|ctx| {
                let env = Table::new(&ctx);
                for base in ["pcall", "error", "assert", "type", "tostring", "tonumber", "select", "pairs", "ipairs", "next"] {
                    env.set(ctx, base, ctx.globals().get(ctx, base))?;
                }
                bind(ctx, env, &sh);
                let known = Table::new(&ctx);
                for (event, _) in EVENTS {
                    known.set(ctx, event, true)?;
                }
                let rows_fn = {
                    let sh = sh.clone();
                    Callback::from_fn(&ctx, move |c, _, mut s| {
                        let comps = match s.get(0) {
                            Value::Table(t) => t,
                            _ => return Err(fail(c, "bad_argument: comps must be a list".into())),
                        };
                        let out = rows(c, &sh.borrow().sim, comps)?;
                        s.replace(c, out);
                        Ok(CallbackReturn::Return)
                    })
                };
                let note = {
                    let sh = sh.clone();
                    Callback::from_fn(&ctx, move |_, _, _| {
                        sh.borrow_mut().fired += 1;
                        Ok(CallbackReturn::Return)
                    })
                };
                let clock = {
                    let sh = sh.clone();
                    Callback::from_fn(&ctx, move |c, _, mut s| {
                        s.replace(c, sh.borrow().clock as i64);
                        Ok(CallbackReturn::Return)
                    })
                };
                let closure = Closure::load(ctx, Some("host-prelude"), prelude().as_bytes())?;
                let exec = Executor::start(ctx, closure.into(), (env, known, rows_fn, note, clock, MAX_PLAYERS as i64));
                Ok((ctx.stash(exec), ctx.stash(env)))
            })
            .map_err(|e| e.to_string())?;
        self.finish(&prelude)?;
        let hooks = self
            .lua
            .try_enter(|ctx| {
                let (emit, systems, timers) = ctx.fetch(&prelude).take_result::<(Value, Value, Value)>(ctx)??;
                let (Value::Function(emit), Value::Function(systems), Value::Function(timers)) = (emit, systems, timers) else {
                    return Err(fail(ctx, "host prelude returned a non-function".into()));
                };
                Ok(Hooks {
                    emit: ctx.stash(emit),
                    run_systems: ctx.stash(systems),
                    fire_timers: ctx.stash(timers),
                    on_unit_died: None,
                    on_cast: None,
                    on_march: None,
                    on_check_win: None,
                    target_priority: None,
                })
            })
            .map_err(|e| e.to_string())?;
        let exec = self
            .lua
            .try_enter(|ctx| {
                let closure = Closure::load_with_env(ctx, Some(name), src.as_bytes(), ctx.fetch(&env))?;
                Ok(ctx.stash(Executor::start(ctx, closure.into(), ())))
            })
            .map_err(|e| e.to_string())?;
        self.run(&exec)?;
        let defined = self.lua.enter(|ctx| matches!(ctx.fetch(&env).get(ctx, "on_tick"), Value::Function(_)));
        if !defined {
            return Err(format!("{name}: on_tick is not defined"));
        }
        let mut hooks = hooks;
        let mut named = |key: &str| {
            self.lua.enter(|ctx| match ctx.fetch(&env).get(ctx, ctx.intern(key.as_bytes())) {
                Value::Function(f) => Some(ctx.stash(f)),
                _ => None,
            })
        };
        hooks.on_unit_died = named("on_unit_died");
        hooks.on_cast = named("on_cast");
        hooks.on_march = named("on_march");
        hooks.on_check_win = named("on_check_win");
        hooks.target_priority = named("target_priority");
        self.hooks = Some(hooks);
        self.version += 1;
        Ok(())
    }

    /// Loads the mod file at `path` and remembers it, so `reload` can swap in later edits.
    pub fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let seen = mtime(path)?;
        let src = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        self.load_source(&mod_name(path), &src)?;
        self.mod_file = Some((path.to_path_buf(), seen));
        Ok(())
    }

    /// Swaps in the file behind `load_file` when its mtime moved. A mod that fails to load puts the
    /// sim, regions and queued commands back as they were and keeps the live mod in effect; the
    /// broken file is not retried until its mtime moves again. Returns Ok(true) when a new version
    /// went live.
    pub fn reload(&mut self) -> Result<bool, String> {
        let Some((path, seen)) = self.mod_file.clone() else { return Ok(false) };
        let now = mtime(&path)?;
        if now == seen {
            return Ok(false);
        }
        self.mod_file = Some((path.clone(), now));
        let src = std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let (sim, regions, cmds, local) = {
            let sh = self.sh.borrow();
            (sh.sim.save_state(), sh.regions.clone(), sh.cmds.clone(), sh.local.clone())
        };
        match self.load_source(&mod_name(&path), &src) {
            Ok(()) => Ok(true),
            Err(e) => {
                let mut sh = self.sh.borrow_mut();
                sh.sim.load_state(&sim).map_err(|_| "the saved sim state did not load back".to_string())?;
                sh.regions = regions;
                sh.cmds = cmds;
                sh.local = local;
                Err(e)
            }
        }
    }

    /// The count of mods this host has loaded.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// The sim this host advances, for a caller that commands or inspects units.
    pub fn sim(&self) -> std::cell::RefMut<'_, Sim> {
        std::cell::RefMut::map(self.sh.borrow_mut(), |s| &mut s.sim)
    }

    /// How many trigger bodies have run, and how many queued commands have been applied.
    pub fn counters(&self) -> (u64, u64) {
        let sh = self.sh.borrow();
        (sh.fired, sh.applied)
    }

    /// Spawns a unit of `kind` for player 0 at Q16.16 `x`, `y`. The unit's index, or None when
    /// the sim refuses the spawn.
    pub fn cmd_spawn(&mut self, kind: u32, x: i32, y: i32) -> Option<usize> {
        self.sim().spawn(kind, x, y)
    }

    /// Fires the sim trigger `name` on `unit`; false when the unit or the trigger is unknown.
    pub fn fire_trigger(&mut self, unit: usize, name: &str) -> bool {
        self.sim().fire_trigger(unit, name)
    }

    /// The event names a mod can handle or trigger on, the same list the mlua backend serves.
    pub fn events(&self) -> &'static [(&'static str, &'static str)] {
        &EVENTS
    }

    /// Records a player joining; the id is handed to on_player_joined at the next step. False
    /// when `player` is outside 0..MAX_PLAYERS.
    pub fn player_joined(&mut self, player: usize) -> bool {
        if player >= MAX_PLAYERS {
            return false;
        }
        self.joined.push(player);
        true
    }

    /// The UI event bodies (JSON object bodies) mods queued since the last call, in queue order.
    pub fn take_ui(&mut self) -> Vec<String> {
        std::mem::take(&mut self.sh.borrow_mut().ui)
    }

    /// The scalar fields a mod left in `state`, sorted, as `key=value` words: the same line mod_check
    /// prints after `mod-state` on the mlua host.
    pub fn state_line(&mut self) -> String {
        let mut fields: Vec<String> = self.lua.enter(|ctx| {
            ctx.fetch(&self.state)
                .iter()
                .filter_map(|(k, v)| {
                    let Value::String(k) = k else { return None };
                    let k = String::from_utf8_lossy(k.as_bytes()).into_owned();
                    match v {
                        Value::Integer(v) => Some(format!("{k}={v}")),
                        Value::Number(v) => Some(format!("{k}={v}")),
                        Value::Boolean(v) => Some(format!("{k}={v}")),
                        Value::String(v) => Some(format!("{k}={}", String::from_utf8_lossy(v.as_bytes()))),
                        _ => None,
                    }
                })
                .collect()
        });
        fields.sort();
        format!("mod-state {}", fields.join(" "))
    }

    /// The living unit handles, optionally only those `player` owns, in index order (game.units).
    pub fn units(&self, player: Option<u8>) -> Vec<usize> {
        let sh = self.sh.borrow();
        (0..sh.sim.count()).filter(|&i| sh.sim.alive(i) && player.map_or(true, |p| sh.sim.player(i) == p)).collect()
    }

    /// The catalog name of unit `unit`, None for an unknown handle (game.unit_kind).
    pub fn unit_kind(&self, unit: usize) -> Option<String> {
        self.sh.borrow().sim.kind_name(unit).map(str::to_owned)
    }

    /// The living unit count per player id, indexed 0..MAX_PLAYERS (game.players).
    pub fn players(&self) -> Vec<usize> {
        living_per_player(&self.sh.borrow().sim)
    }

    /// One step in the mlua host's order: the sim tick, zone and unit events, every system, the
    /// queued commands, due timers, then on_tick. A command queued by on_tick or a timer waits for
    /// the next step.
    pub fn advance(&mut self) -> Result<(), String> {
        {
            let mut sh = self.sh.borrow_mut();
            sh.clock += 1;
            sh.sim.tick();
        }
        self.collect();
        self.dispatch()?;
        let pending = std::mem::take(&mut self.pending);
        for event in pending {
            match event {
                Pending::Spawned(u) => self.emit("on_spawned", vec![Arg::Int(u as i64)])?,
                Pending::Attacked(u) => self.emit("on_attacked", vec![Arg::Int(u as i64)])?,
                Pending::Died(u, killer) => {
                    let death = self.death_of(u, killer);
                    self.died(&death)?
                }
                Pending::Enter(u, r) => self.emit_region("on_enter", r, u)?,
                Pending::Leave(u, r) => self.emit_region("on_leave", r, u)?,
                Pending::PlayerJoined(p) => self.emit("on_player_joined", vec![Arg::Int(p as i64)])?,
            }
        }
        if let Some(h) = &self.hooks {
            let f = h.run_systems.clone();
            self.call(&f, Vec::new())?;
        }
        self.apply_commands();
        if let Some(h) = &self.hooks {
            let f = h.fire_timers.clone();
            self.call(&f, Vec::new())?;
        }
        self.emit("on_tick", vec![Arg::State])?;
        Ok(())
    }

    /// Feeds one payable death of a room's sim to the mod's on_died, with the room's unit ids and the
    /// bounty, as the native host does. The commands the handlers queued are returned, not applied:
    /// their ids are the room's, so the room applies them and this world is untouched. The receiver
    /// is &mut self because piccolo runs its VM under a mutable borrow, so this differs from the
    /// native &self; the error is the piccolo host's String, not mlua::Error.
    pub fn feed_payout(&mut self, p: &Payout) -> Result<Vec<Cmd>, String> {
        if self.hooks.is_none() {
            return Ok(Vec::new());
        }
        let death = Death {
            victim: p.victim,
            victim_player: p.victim_player,
            killer: p.killer,
            killer_player: p.killer_player,
            killer_hero: p.killer_hero,
            bounty: p.bounty,
        };
        let before = self.sh.borrow().local.len();
        self.died(&death)?;
        Ok(self.sh.borrow_mut().local.split_off(before))
    }

    fn death_of(&self, unit: usize, killer: Option<usize>) -> Death {
        let sh = self.sh.borrow();
        Death {
            victim: unit as u32,
            victim_player: sh.sim.player(unit),
            killer: killer.map(|k| k as u32),
            killer_player: killer.map(|k| sh.sim.player(k)),
            killer_hero: killer.is_some_and(|k| sh.sim.is_hero(k)),
            bounty: sh.sim.unit_bounty(unit),
        }
    }

    // on_died(unit, killer, killer_player, bounty, victim_player, killer_hero), the handler then the triggers.
    // The commands the handlers queue here are the mod's own, so they move to `local` and apply before
    // the systems' commands, as the mlua host orders them.
    fn died(&mut self, d: &Death) -> Result<(), String> {
        let opt = |v: Option<i64>| v.map_or(Arg::Nil, Arg::Int);
        let before = self.sh.borrow().cmds.len();
        self.emit(
            "on_died",
            vec![
                Arg::Int(i64::from(d.victim)),
                opt(d.killer.map(i64::from)),
                opt(d.killer_player.map(i64::from)),
                Arg::Int(i64::from(d.bounty)),
                Arg::Int(i64::from(d.victim_player)),
                Arg::Bool(d.killer_hero),
            ],
        )?;
        if let Some(f) = self.hooks.as_ref().and_then(|h| h.on_unit_died.clone()) {
            match self.call_ret(&f, vec![Arg::Int(i64::from(d.victim)), Arg::State]) {
                Ok((delay, _)) => {
                    if let Some(cmd) = respawn_of(delay) {
                        self.sh.borrow_mut().cmds.push(Cmd::SetRespawn { unit: d.victim as usize, delay_s: cmd });
                    }
                }
                Err(e) => println!("ev=mod_hook_failed hook=on_unit_died error={:?}", e),
            }
        }
        let mut sh = self.sh.borrow_mut();
        let own = sh.cmds.split_off(before);
        sh.local.extend(own);
        Ok(())
    }

    // The hooks mlua dispatches each step, before the pending events: target_priority for each new aggro
    // acquisition, on_march for each destination request, on_cast for each cast that takes hold, and
    // on_check_win while no side has won. Every queue is taken each step, mod or not.
    fn dispatch(&mut self) -> Result<(), String> {
        let (acquired, requests, casts) = {
            let mut sh = self.sh.borrow_mut();
            (sh.sim.take_acquisitions(), sh.sim.take_destination_requests(), sh.sim.take_casts())
        };
        let Some(h) = self.hooks.as_ref() else { return Ok(()) };
        let (tp, march, cast, win) = (h.target_priority.clone(), h.on_march.clone(), h.on_cast.clone(), h.on_check_win.clone());
        if let Some(f) = tp {
            for (unit, target) in acquired {
                self.call_target_priority(&f, unit, target);
            }
        }
        if let Some(f) = march {
            for (unit, player) in requests {
                self.call_on_march(&f, unit, player);
            }
        }
        if let Some(f) = cast {
            for (caster, ability) in casts {
                self.call_on_cast(&f, caster, ability);
            }
        }
        if let Some(f) = win {
            if self.sh.borrow().sim.winner().is_none() {
                self.call_on_check_win(&f);
            }
        }
        Ok(())
    }

    // target_priority(unit, candidates, state): a class 0..3 queues a class override, false clears it, nil or
    // true leaves the unit alone. A Lua error or a malformed answer is logged and queues nothing.
    fn call_target_priority(&mut self, f: &StashedFunction, unit: usize, target: usize) {
        let answer = match self.call_ret(f, vec![Arg::Int(unit as i64), Arg::List(vec![target as i64]), Arg::State]) {
            Ok((r, _)) => r,
            Err(e) => return println!("ev=mod_hook_failed hook=target_priority error={:?}", e),
        };
        let class = match answer {
            Ret::Nil | Ret::Bool(true) => return,
            Ret::Bool(false) => None,
            Ret::Int(c) if (0..=3).contains(&c) => Some(c as u8),
            _ => return println!("ev=mod_hook_refused hook=target_priority"),
        };
        self.sh.borrow_mut().cmds.push(Cmd::SetClassOverride { unit, class });
    }

    // on_march(unit, player, state) returns the x and y of the point the unit walks to (fixed point). nil keeps
    // the unit where it stands. An error or a malformed answer queues nothing.
    fn call_on_march(&mut self, f: &StashedFunction, unit: usize, player: u8) {
        match self.call_ret(f, vec![Arg::Int(unit as i64), Arg::Int(i64::from(player)), Arg::State]) {
            Ok((Ret::Int(x), Ret::Int(y))) => match (i32::try_from(x), i32::try_from(y)) {
                (Ok(x), Ok(y)) => self.sh.borrow_mut().cmds.push(Cmd::AttackMove { unit, x, y }),
                _ => println!("ev=mod_hook_refused hook=on_march value=out_of_range"),
            },
            Ok((Ret::Nil | Ret::Bool(false), _)) => {}
            Ok(_) => println!("ev=mod_hook_refused hook=on_march"),
            Err(e) => println!("ev=mod_hook_failed hook=on_march error={:?}", e),
        }
    }

    // on_cast(caster, ability_name, state), with the ability's catalog name.
    fn call_on_cast(&mut self, f: &StashedFunction, caster: usize, ability: usize) {
        let name = {
            let sh = self.sh.borrow();
            let cat = sh.sim.catalog();
            match cat.abilities.get(ability) {
                Some(a) => cat.name(a.name).to_string(),
                None => return,
            }
        };
        if let Err(e) = self.call_ret(f, vec![Arg::Int(caster as i64), Arg::Text(name), Arg::State]) {
            println!("ev=mod_hook_failed hook=on_cast error={:?}", e);
        }
    }

    // on_check_win(state) returns the side that has won (0..MAX_PLAYERS), or nil or false while none has.
    fn call_on_check_win(&mut self, f: &StashedFunction) {
        match self.call_ret(f, vec![Arg::State]) {
            Ok((Ret::Nil | Ret::Bool(false), _)) => {}
            Ok((Ret::Int(s), _)) if (0..MAX_PLAYERS as i64).contains(&s) => {
                self.sh.borrow_mut().cmds.push(Cmd::DeclareSide { side: s as u8 });
            }
            Ok(_) => println!("ev=mod_hook_refused hook=on_check_win"),
            Err(e) => println!("ev=mod_hook_failed hook=on_check_win error={:?}", e),
        }
    }

    /// Runs the event `name` with a unit argument through the mod's handler and triggers, as a
    /// step would for that unit.
    pub fn emit_unit(&mut self, name: &str, unit: usize) -> Result<(), String> {
        self.emit(name, vec![Arg::Int(unit as i64)])
    }

    fn emit_region(&mut self, name: &str, region: usize, unit: usize) -> Result<(), String> {
        let region_name = self.sh.borrow().regions.get(region).map(|g| g.name.clone());
        match region_name {
            Some(r) => self.emit(name, vec![Arg::Text(r), Arg::Int(unit as i64)]),
            None => Ok(()),
        }
    }

    fn emit(&mut self, event: &str, mut args: Vec<Arg>) -> Result<(), String> {
        let Some(h) = &self.hooks else { return Ok(()) };
        let f = h.emit.clone();
        args.insert(0, Arg::Text(event.to_owned()));
        self.call(&f, args)
    }

    fn call(&mut self, f: &StashedFunction, args: Vec<Arg>) -> Result<(), String> {
        let exec = self.start(f, args)?;
        self.run(&exec)
    }

    // Runs a hook and returns its first two results as Ret values, so a hook's answer can be read
    // (on_march returns x and y). A Lua error is returned, for the caller to log and skip.
    fn call_ret(&mut self, f: &StashedFunction, args: Vec<Arg>) -> Result<(Ret, Ret), String> {
        let exec = self.start(f, args)?;
        self.finish(&exec)?;
        self.lua
            .try_enter(|ctx| {
                let (x, y) = ctx.fetch(&exec).take_result::<(Value, Value)>(ctx)??;
                Ok((Ret::of(x), Ret::of(y)))
            })
            .map_err(|e| e.to_string())
    }

    fn start(&mut self, f: &StashedFunction, args: Vec<Arg>) -> Result<StashedExecutor, String> {
        let state = &self.state;
        self.lua
            .try_enter(|ctx| {
                let values: Vec<Value> = args
                    .into_iter()
                    .map(|a| match a {
                        Arg::Int(i) => Value::Integer(i),
                        Arg::Bool(b) => Value::Boolean(b),
                        Arg::Nil => Value::Nil,
                        Arg::Text(t) => t.into_value(ctx),
                        Arg::State => Value::Table(ctx.fetch(state)),
                        Arg::List(items) => Value::Table(list_of(ctx, items.into_iter()).expect("list")),
                    })
                    .collect();
                Ok(ctx.stash(Executor::start(ctx, ctx.fetch(f), Variadic(values))))
            })
            .map_err(|e| e.to_string())
    }

    // Applies the queued commands in queue order. A command aimed at a unit that died since it
    // was queued does nothing.
    fn apply_commands(&mut self) {
        let mut guard = self.sh.borrow_mut();
        let sh = &mut *guard;
        let local = std::mem::take(&mut sh.local);
        let list = std::mem::take(&mut sh.cmds);
        let (sim, ui) = (&mut sh.sim, &mut sh.ui);
        for cmd in local.into_iter().chain(list) {
            match cmd {
                Cmd::Spawn { index, x, y, owner, route, .. } => {
                    sim.spawn_mod_unit(owner, index as u32, x, y, route);
                }
                Cmd::SetOwner { unit, player } => {
                    let _ = sim.set_owner(unit, player);
                }
                Cmd::SetFog { on } => sim.set_fog(on),
                Cmd::AddGold { player, amount } => sim.add_resources(player, amount, 0),
                Cmd::RemoveUnit { unit } => {
                    sim.remove_unit(unit);
                }
                Cmd::GiveItem { unit, item, count } => {
                    let _ = sim.grant_item(unit, item, count);
                }
                Cmd::Victory { player } => sim.declare_victory(player),
                Cmd::Defeat { player } => sim.declare_defeat(player),
                Cmd::Ui { body } => ui.push(body),
                Cmd::GiveAbility { unit, slot, name } => {
                    let _ = sim.give_ability(unit, slot, &name);
                }
                Cmd::AddUnitGold { unit, amount } => {
                    let _ = sim.add_unit_gold(unit, amount);
                }
                Cmd::Order { unit, x, y } if sim.alive(unit) => sim.order(unit, x, y),
                Cmd::Damage { caster, target, dmg } if sim.alive(caster) => {
                    let player = sim.player(caster);
                    sim.damage_target(caster as i32, player, dmg, target as i32);
                }
                Cmd::Buff { unit, kind, ticks } if sim.alive(unit) => {
                    let _ = sim.apply_buff(unit, kind, ticks);
                }
                Cmd::Cast { unit, slot, target, x, y } if sim.alive(unit) => {
                    let _ = sim.cast_ability(unit, slot, target, x, y);
                }
                Cmd::Attack { unit, target } if sim.alive(unit) => {
                    let _ = sim.order_attack_or_approach(unit, target);
                }
                Cmd::UseItem { unit, slot, target, x, y } if sim.alive(unit) => {
                    let _ = sim.use_item(unit, slot, target, x, y);
                }
                Cmd::Buy { unit, item } if sim.alive(unit) => {
                    let _ = sim.buy_item(unit, item);
                }
                Cmd::Sell { unit, slot } if sim.alive(unit) => {
                    let _ = sim.sell_item(unit, slot);
                }
                Cmd::Unstash { unit, slot } if sim.alive(unit) => {
                    let _ = sim.unstash(unit, slot);
                }
                Cmd::SetClassOverride { unit, class } => {
                    let _ = sim.set_class_override(unit, class);
                }
                Cmd::SetHoldOverride { unit, hold } => {
                    let _ = sim.set_hold_override(unit, hold);
                }
                Cmd::SkillUp { unit, slot } if sim.alive(unit) => {
                    let _ = sim.skill_up(unit, slot);
                }
                Cmd::RemovePlayer { player } => {
                    let _ = sim.remove_player(player);
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
                Cmd::AttackMove { unit, x, y } if sim.alive(unit) => {
                    let _ = sim.issue_order_kind(unit, sim::ORDER_ATTACK_MOVE, x, y);
                }
                Cmd::SetTeam { player, team } => {
                    let _ = sim.set_team(player, team);
                }
                Cmd::PackStash { unit } if sim.alive(unit) => {
                    sim.pack_stash(unit);
                }
                Cmd::SaveOrder { unit, x, y } => {
                    let _ = sim.save_order(unit, x, y);
                }
                Cmd::RestoreOrder { unit } => {
                    let _ = sim.restore_order(unit);
                }
                _ => continue,
            }
            sh.applied += 1;
        }
    }

    // Diffs the sim against the last step into spawn, attack, death, region and join events, in
    // unit index order then region index order.
    fn collect(&mut self) {
        self.snap.clear();
        self.pending.clear();
        self.pending.extend(self.joined.drain(..).map(Pending::PlayerJoined));
        let sh = self.sh.borrow();
        for i in 0..sh.sim.count() {
            let (x, y) = sh.sim.position_fixed(i);
            self.snap.push(UnitSnap { x, y, hp: sh.sim.hp(i), alive: sh.sim.alive(i), killer: sh.sim.killer_of(i) });
        }
        for (unit, s) in self.snap.iter().enumerate() {
            match self.prev.get(unit) {
                None => self.pending.push(Pending::Spawned(unit)),
                Some(p) => {
                    if s.hp < p.hp {
                        self.pending.push(Pending::Attacked(unit));
                    }
                    if p.alive && !s.alive {
                        self.pending.push(Pending::Died(unit, s.killer));
                    }
                }
            }
            let mut after = 0u64;
            if s.alive {
                for (r, g) in sh.regions.iter().enumerate() {
                    if g.contains(s.x, s.y) {
                        after |= 1 << r;
                    }
                }
            }
            let before = self.inside[unit];
            for r in 0..sh.regions.len() {
                let bit = 1u64 << r;
                match (before & bit != 0, after & bit != 0) {
                    (false, true) => self.pending.push(Pending::Enter(unit, r)),
                    (true, false) => self.pending.push(Pending::Leave(unit, r)),
                    _ => {}
                }
            }
            self.inside[unit] = after;
        }
        std::mem::swap(&mut self.prev, &mut self.snap);
    }

    fn run(&mut self, executor: &StashedExecutor) -> Result<(), String> {
        self.finish(executor)?;
        self.lua.try_enter(|ctx| ctx.fetch(executor).take_result::<()>(ctx)?).map_err(|e| e.to_string())
    }

    // Steps `executor` until it holds a result for the caller to take.
    fn finish(&mut self, executor: &StashedExecutor) -> Result<(), String> {
        for _ in 0..MAX_STEPS {
            let mut fuel = Fuel::with(FUEL_PER_STEP);
            if self.lua.enter(|ctx| ctx.fetch(executor).step(ctx, &mut fuel)) {
                return Ok(());
            }
        }
        Err(format!("instruction_limit: over {MAX_STEPS} steps in one call"))
    }
}

enum Arg {
    Int(i64),
    Bool(bool),
    Nil,
    Text(String),
    State,
    List(Vec<i64>),
}

// One death as on_died receives it. Built from this world's own deaths, or from a room's payable
// death (feed_payout). Field meanings match sim::Payout.
struct Death {
    victim: u32,
    victim_player: u8,
    killer: Option<u32>,
    killer_player: Option<u8>,
    killer_hero: bool,
    bounty: i32,
}
