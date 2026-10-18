//! The mod host on piccolo, a pure Rust Lua VM, for the wasip3 guest (no C objects). A mod runs in
//! its own environment table with the same game table (system, each, trigger, cmd_*, units,
//! players, unit_kind), the same unit queries and the same event dispatch as the mlua host. Not
//! carried over: terrain, tile, doodad and ability editing globals. A mod call runs under a step
//! budget so a loop cannot stall the guest.

use std::cell::RefCell;
use std::rc::Rc;

use piccolo::fuel::Fuel;
use piccolo::{
    Callback, CallbackReturn, Closure, Context, Error, Executor, IntoValue, Lua, StashedExecutor, StashedFunction,
    Stack, StashedTable, Table, Value, Variadic,
};
use sim::{LoadError, Sim, ABILITY_SLOTS, ITEM_SLOTS, MAX_PLAYERS};

use crate::{EVENTS, MAX_UNITS};

const FUEL_PER_STEP: i32 = 4096;
const MAX_STEPS: u32 = 2_500;
const MAX_REGIONS: usize = 64;

#[derive(Clone)]
enum Cmd {
    Spawn { index: usize, x: i32, y: i32 },
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
}

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
}

enum Pending {
    Spawned(usize),
    Attacked(usize),
    Died(usize),
    Enter(usize, usize),
    Leave(usize, usize),
    PlayerJoined(usize),
}

// What the native functions and the host both touch: the sim, regions, the queued commands and
// the counters the witness reads.
struct Shared {
    sim: Sim,
    regions: Vec<Region>,
    cmds: Vec<Cmd>,
    clock: u64,
    fired: u64,
    applied: u64,
}

type Sh = Rc<RefCell<Shared>>;

struct Hooks {
    emit: StashedFunction,
    run_systems: StashedFunction,
    fire_timers: StashedFunction,
}

// The Lua half of the host, run once per mod load with (env, known events, rows, note_fired, clock).
// It adds game.system, game.each, game.trigger, schedule and cancel to the mod environment and
// returns emit (the global handler, then every trigger of the event), run_systems and fire_timers.
// A trigger runs its actions only when every condition returns truthy.
const PRELUDE: &str = r#"
local env, known, rows, note_fired, clock = ...
local game = env.game
local systems, triggers, timers = {}, {}, {}
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
    if t.name == name then table.remove(timers, i) return true end
  end
  return false
end
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
    table.remove(timers, hit).action()
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

fn fixed(v: i64) -> Result<i32, String> {
    i32::try_from(v).map_err(|_| format!("Q16.16 value {v} out of i32 range"))
}

fn known_unit(sim: &Sim, i: i64) -> Result<usize, String> {
    usize::try_from(i).ok().filter(|&u| u < sim.count()).ok_or_else(|| format!("invalid_unit: {i}"))
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
        let mut sh = sh.borrow_mut();
        let index = sh.sim.catalog().unit_index(&kind).ok_or_else(|| fail(c, format!("unknown_unit: {kind}")))?;
        sh.cmds.push(Cmd::Spawn { index, x, y });
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
            sh: Rc::new(RefCell::new(Shared { sim, regions: Vec::new(), cmds: Vec::new(), clock: 0, fired: 0, applied: 0 })),
            state,
            hooks: None,
            version: 0,
            joined: Vec::new(),
            snap: Vec::new(),
            prev: Vec::new(),
            inside: vec![0; MAX_UNITS],
            pending: Vec::new(),
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
                let closure = Closure::load(ctx, Some("host-prelude"), PRELUDE.as_bytes())?;
                let exec = Executor::start(ctx, closure.into(), (env, known, rows_fn, note, clock));
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
                Ok(Hooks { emit: ctx.stash(emit), run_systems: ctx.stash(systems), fire_timers: ctx.stash(timers) })
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
        self.hooks = Some(hooks);
        self.version += 1;
        Ok(())
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
        let pending = std::mem::take(&mut self.pending);
        for event in pending {
            match event {
                Pending::Spawned(u) => self.emit("on_spawned", vec![Arg::Int(u as i64)])?,
                Pending::Attacked(u) => self.emit("on_attacked", vec![Arg::Int(u as i64)])?,
                Pending::Died(u) => self.emit("on_died", vec![Arg::Int(u as i64)])?,
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
        let ticks = self.sh.borrow().clock as i64;
        self.lua.enter(|ctx| {
            ctx.fetch(&self.state).set(ctx, "ticks", ticks).expect("state.ticks");
        });
        Ok(())
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
        let state = &self.state;
        let exec = self
            .lua
            .try_enter(|ctx| {
                let values: Vec<Value> = args
                    .into_iter()
                    .map(|a| match a {
                        Arg::Int(i) => Value::Integer(i),
                        Arg::Text(t) => t.into_value(ctx),
                        Arg::State => Value::Table(ctx.fetch(state)),
                    })
                    .collect();
                Ok(ctx.stash(Executor::start(ctx, ctx.fetch(f), Variadic(values))))
            })
            .map_err(|e| e.to_string())?;
        self.run(&exec)
    }

    // Applies the queued commands in queue order. A command aimed at a unit that died since it
    // was queued does nothing.
    fn apply_commands(&mut self) {
        let mut guard = self.sh.borrow_mut();
        let sh = &mut *guard;
        let list = std::mem::take(&mut sh.cmds);
        let sim = &mut sh.sim;
        for cmd in list {
            match cmd {
                Cmd::Spawn { index, x, y } => {
                    sim.spawn(index as u32, x, y);
                }
                Cmd::SetOwner { unit, player } => {
                    let _ = sim.set_owner(unit, player);
                }
                Cmd::SetFog { on } => sim.set_fog(on),
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
            self.snap.push(UnitSnap { x, y, hp: sh.sim.hp(i), alive: sh.sim.alive(i) });
        }
        for (unit, s) in self.snap.iter().enumerate() {
            match self.prev.get(unit) {
                None => self.pending.push(Pending::Spawned(unit)),
                Some(p) => {
                    if s.hp < p.hp {
                        self.pending.push(Pending::Attacked(unit));
                    }
                    if p.alive && !s.alive {
                        self.pending.push(Pending::Died(unit));
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
    Text(String),
    State,
}
