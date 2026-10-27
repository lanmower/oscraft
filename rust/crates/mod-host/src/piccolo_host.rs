
use std::any::Any;
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use piccolo::fuel::Fuel;
use piccolo::{
    Callback, CallbackReturn, Closure, Context, Error, Executor, IntoValue, Lua, StashedExecutor, StashedFunction,
    Stack, StashedTable, Table, Value, Variadic,
};
use sim::{
    controller_code, AbilitySpec, Catalog, FxEvent, ItemSpec, LoadError, Refusal, ItemRequest, ItemVerdict, Sim, UnitSpec, UpgradeSpec, ABILITY_SLOTS, ITEM_SLOTS, MAX_PLAYERS,
    MOD_ALLIANCES, MOD_CONTROLLERS, NO_COMPONENT, NO_TAKE, ORDER_HOLD, TICKS_PER_SECOND, VERDICT_TAKES,
};

use crate::shared::{self, amount_arg, fixed, player_arg, PRELUDE_SRC};
use crate::{EVENTS, MAX_UNITS};

use seats::SeatState;

const FUEL_PER_STEP: i32 = 4096;
const MAX_STEPS: u32 = 2_500;
const MEMORY_CAP: usize = 256 << 20;
const HOOK_KEYS: &[&str] = &[
    "on_spawned",
    "on_attacked",
    "on_died",
    "on_enter",
    "on_leave",
    "on_player_joined",
    "on_player_left",
    "on_tick",
    "on_behaviour",
    "run_systems",
    "fire_timers",
    "on_unit_died",
    "on_cast",
    "on_cast_request",
    "on_proc",
    "on_hit",
    "on_march",
    "on_check_win",
    "on_request",
    "on_skill_up",
    "on_item_use",
    "on_action",
    "on_pointer",
    "on_state_replaced",
];
const MAX_REGIONS: usize = 64;
const NEUTRAL_OWNER: u8 = 0xFF;
const ITEM_FLAG_NO_BUY: u16 = 4;
const ITEM_FLAG_NO_SELL: u16 = 8;
const ITEM_FLAG_NO_DROP: u16 = 32;

#[derive(Clone)]
pub enum Cmd {
    Fx(FxEvent),
    Spawn { index: usize, x: i32, y: i32, owner: u8, route: i32 },
    Order { unit: usize, x: i32, y: i32 },
    Hold { unit: usize },
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
    AddGas { player: u8, amount: i32 },
    AddUnitGold { unit: usize, amount: i32 },
    AddXp { unit: usize, amount: u32 },
    AddStat { unit: usize, code: usize, delta: i32 },
    TimedStat { unit: usize, code: usize, delta: i32, ticks: i32 },
    RemoveUnit { unit: usize },
    GiveItem { unit: usize, item: usize, count: u32 },
    DropGround { item: usize, count: u32, x: i32, y: i32 },
    DropUnitItem { unit: usize, slot: usize, x: i32, y: i32 },
    PickupGround { unit: usize, id: u32 },
    RemoveGround { id: u32 },
    SetGroundCount { id: u32, count: u32 },
    SetItemCharges { unit: usize, slot: usize, count: u32 },
    ReplaceItem { unit: usize, slot: usize, item: usize, count: u32 },
    UseItemOn { unit: usize, slot: usize, prop: bool, target: u32 },
    SetItemOwner { unit: usize, slot: usize, player: usize },
    SetClassOverride { unit: usize, class: Option<u8> },
    SetHoldOverride { unit: usize, hold: Option<bool> },
    SetStrike { unit: usize, target: i32 },
    SkillUp { unit: usize, slot: usize },
    OrderHandle { handle: u32, x: i32, y: i32 },
    DamageHandle { caster: u32, target: u32, dmg: i32 },
    BuffHandle { handle: u32, kind: i32, ticks: i32 },
    SetAlliance { a: u8, b: u8, kind: usize },
    AddPlayer { player: u8, controller: u8 },
    RemovePlayer { player: u8 },
    EditCell { x: i32, y: i32, blocked: bool },
    SetTile { x: i32, y: i32, tile: u8 },
    SetXpRate { player: u8, percent: u16 },
    SetUnitScale { unit: usize, percent: u16 },
    SetRespawn { unit: usize, delay_s: Option<u32> },
    RespawnAs { unit: usize, kind: u32, heading: Option<i32> },
    SetHome { unit: usize, x: i32, y: i32 },
    SetInvulnerable { unit: usize, ticks: u32 },
    SetReduction { unit: usize, pct: u8 },
    SetHealing { unit: usize, hp_per_s: i32 },
    SetPeriod { unit: usize, ticks: i32 },
    Revive { unit: usize, x: i32, y: i32 },
    DeclareSide { side: u8 },
    AttackMove { unit: usize, x: i32, y: i32 },
    SetTeam { player: u8, team: u8 },
    PackStash { unit: usize },
    SaveOrder { unit: usize, x: i32, y: i32 },
    RestoreOrder { unit: usize },
    Victory { player: u8 },
    Defeat { player: u8 },
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
    PlayerLeft(usize),
}

struct Shared {
    sim: Sim,
    regions: Vec<Region>,
    local: Vec<Cmd>,
    cmds: Vec<Cmd>,
    counter_names: std::collections::HashSet<String>,
    clock: u64,
    fired: u64,
    applied: u64,
    ui: Vec<String>,
    generation: i64,
    seats: SeatState,
}

type Sh = Rc<RefCell<Shared>>;

struct Hooks {
    emit: StashedFunction,
    run_systems: StashedFunction,
    fire_timers: StashedFunction,
    on_unit_died: Option<StashedFunction>,
    on_cast: Option<StashedFunction>,
    on_cast_request: Option<StashedFunction>,
    on_proc: Option<StashedFunction>,
    on_hit: Option<StashedFunction>,
    on_march: Option<StashedFunction>,
    on_check_win: Option<StashedFunction>,
    on_request: Option<StashedFunction>,
    on_skill_up: Option<StashedFunction>,
    on_item_use: Option<StashedFunction>,
    on_action: Option<StashedFunction>,
    on_pointer: Option<StashedFunction>,
}

enum Own {
    Nil,
    Bool(bool),
    Int(i64),
    Str(String),
    List(Vec<i64>),
    Other,
}

fn own<'gc>(ctx: Context<'gc>, v: Value<'gc>) -> Own {
    match v {
        Value::Nil => Own::Nil,
        Value::Boolean(b) => Own::Bool(b),
        Value::Integer(n) => Own::Int(n),
        Value::Number(n) if n.fract() == 0.0 && n.abs() < 9.2e18 => Own::Int(n as i64),
        Value::String(s) => s.to_str().map_or(Own::Other, |t| Own::Str(t.to_owned())),
        Value::Table(t) => {
            let mut items = Vec::new();
            for i in 1..=t.length() {
                match t.get(ctx, i) {
                    Value::Integer(n) => items.push(n),
                    _ => return Own::Other,
                }
            }
            Own::List(items)
        }
        _ => Own::Other,
    }
}

fn whole(v: &Own, max: i64) -> Option<i64> {
    match v {
        Own::Int(n) if (0..=max).contains(n) => Some(*n),
        _ => None,
    }
}

fn parse_takes(v: &Own, items: usize) -> Option<[u32; VERDICT_TAKES]> {
    let mut out = [NO_TAKE; VERDICT_TAKES];
    match v {
        Own::Nil => Some(out),
        Own::List(list) => {
            if list.len() > VERDICT_TAKES {
                return None;
            }
            for (n, &x) in list.iter().enumerate() {
                if x < 0 || x as usize >= items {
                    return None;
                }
                out[n] = x as u32;
            }
            Some(out)
        }
        _ => None,
    }
}

fn denial(v: &Own) -> Refusal {
    match v {
        Own::Str(s) => match s.as_str() {
            "out-of-reach" | "no-shop" => Refusal::OutOfReach,
            "cannot-afford" => Refusal::CannotAfford,
            "unpawnable" => Refusal::Unpawnable,
            "consumable" => Refusal::Consumable,
            "locked" => Refusal::Locked,
            "no-item" => Refusal::NoItem,
            "inventory-full" => Refusal::InventoryFull,
            "missing-component" => Refusal::MissingComponent,
            _ => Refusal::NoItem,
        },
        _ => Refusal::NoItem,
    }
}

#[derive(Clone, Copy)]
enum Ret {
    Nil,
    Bool(bool),
    Int(i64),
    Other,
}

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

fn prelude() -> String {
    format!("local install = (function()\n{PRELUDE_SRC}\nend)()\n{HOST_PRELUDE}")
}

const HOST_PRELUDE: &str = r#"
local env, known, rows, note_fired, clock, max_players = ...
local function less_than(a, b) return a < b end
env.table.sort = function(t, cmp)
  cmp = cmp or less_than
  local n = #t
  if n < 2 then return end
  local src, dst = t, {}
  local width = 1
  while width < n do
    local i = 1
    while i <= n do
      local mid = i + width
      local hi = i + 2 * width
      if mid > n + 1 then mid = n + 1 end
      if hi > n + 1 then hi = n + 1 end
      local a, b, k = i, mid, i
      while a < mid and b < hi do
        if cmp(src[b], src[a]) then
          dst[k] = src[b]
          b = b + 1
        else
          dst[k] = src[a]
          a = a + 1
        end
        k = k + 1
      end
      while a < mid do
        dst[k] = src[a]
        a = a + 1
        k = k + 1
      end
      while b < hi do
        dst[k] = src[b]
        b = b + 1
        k = k + 1
      end
      i = i + 2 * width
    end
    src, dst = dst, src
    width = width * 2
  end
  if src ~= t then
    for i = 1, n do t[i] = src[i] end
  end
end
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
local function timer(name, delay, action, period)
  if type(delay) ~= "number" or delay < 0 then error("timer_delay: " .. tostring(delay) .. " is negative", 2) end
  if period ~= nil and (type(period) ~= "number" or period < 1) then error("timer_period: " .. tostring(period) .. " must be at least 1", 2) end
  local due = clock() + (delay < 1 and 1 or delay)
  for _, t in ipairs(timers) do
    if t.name == name then t.due, t.action, t.period = due, action, period return end
  end
  if #timers >= 256 then error("timer_limit: at most 256 timers", 2) end
  timers[#timers + 1] = { name = name, due = due, action = action, period = period }
 end
game.timer = timer
game.cancel_timer = env.cancel
local host = { define_region = env.define_region, unit_position = env.unit_position, in_region = env.in_region, region_bounds = env.region_bounds, map_size = env.map_size, schedule = env.schedule, cancel = env.cancel }
install(game, { global = {}, player = {}, unit = {}, score = {} }, host, max_players)
local function emit(handlers, event, ...)
  local handler = handlers[event]
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
    local t = remove(timers, hit)
    if t.period then timers[#timers + 1] = { name = t.name, due = now + t.period, action = t.action, period = t.period } end
    t.action()
  end
end
return emit, run_systems, fire_timers
"#;

trait Put<'gc> {
    fn put<K: IntoValue<'gc>, V: IntoValue<'gc>>(self, ctx: Context<'gc>, key: K, value: V);
}

impl<'gc> Put<'gc> for Table<'gc> {
    fn put<K: IntoValue<'gc>, V: IntoValue<'gc>>(self, ctx: Context<'gc>, key: K, value: V) {
        if let Err(e) = self.set(ctx, key, value) {
            println!("ev=table_put_refused error={e:?}");
        }
    }
}

fn fail<'gc>(ctx: Context<'gc>, msg: String) -> Error<'gc> {
    Error::from_value(msg.into_value(ctx))
}

fn panic_text(p: &(dyn Any + Send)) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        return (*s).to_owned();
    }
    p.downcast_ref::<String>().cloned().unwrap_or_else(|| "non-string panic payload".to_owned())
}

fn guard_native<'gc, R>(c: Context<'gc>, body: impl FnOnce() -> Result<R, Error<'gc>>) -> Result<R, Error<'gc>> {
    match panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(r) => r,
        Err(p) => Err(fail(c, format!("native_panic: {}", panic_text(&*p)))),
    }
}

fn guarded<T>(what: &str, body: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    match panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(r) => r,
        Err(p) => Err(format!("panic_in_{what}: {}", panic_text(&*p))),
    }
}

#[cfg(feature = "fault-injection")]
fn fault_tick(src: &str) -> Option<u32> {
    src.lines().find_map(|line| line.trim().strip_prefix("-- wasip3-fault: panic at")?.trim().parse().ok())
}

fn clear_table<'gc>(ctx: Context<'gc>, t: Table<'gc>) {
    let mut keys = Vec::new();
    let mut key = Value::Nil;
    while let piccolo::table::NextValue::Found { key: k, .. } = t.next(key) {
        keys.push(k);
        key = k;
    }
    for k in keys {
        let _ = t.set(ctx, k, Value::Nil);
    }
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

fn table_arg<'gc>(ctx: Context<'gc>, v: Value<'gc>, i: usize) -> Result<Table<'gc>, Error<'gc>> {
    match v {
        Value::Table(t) => Ok(t),
        _ => Err(fail(ctx, format!("bad_argument: argument {} must be a table", i + 1))),
    }
}

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

fn fx_event<'gc>(ctx: Context<'gc>, kind: &str, x: f64, y: f64, params: Value<'gc>) -> Result<FxEvent, String> {
    if kind.is_empty() {
        return Err("bad_argument: effect kind is empty".into());
    }
    if !x.is_finite() || !y.is_finite() {
        return Err(format!("bad_argument: effect position {x}, {y} must be finite"));
    }
    let at = |v: f64| v.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32;
    let mut e = FxEvent::new(kind, at(x), at(y));
    match params {
        Value::Nil => {}
        Value::Table(t) => {
            let int = |key: &str, default: i64| field_int(ctx, t, key, default).map(|v| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
            e.source = int("source", -1)?;
            e.target = int("target", -1)?;
            e.team = int("team", -1)?;
            e.duration = int("duration", 0)?;
            e.scale = int("scale", 0)?;
            if e.duration < 0 || e.scale < 0 {
                return Err("bad_field: effect duration and scale must not be negative".into());
            }
            e.value = int("value", 0)?;
            e.colour = field_text_opt(ctx, t, "colour")?.unwrap_or_default();
            e.text = field_text_opt(ctx, t, "text")?.unwrap_or_default();
            e.name = field_text_opt(ctx, t, "name")?.unwrap_or_default();
        }
        _ => return Err("bad_argument: effect params must be a table".into()),
    }
    Ok(e)
}

fn unit_row<'gc>(ctx: Context<'gc>, cat: &Catalog, index: usize) -> Table<'gc> {
    let u = &cat.units[index];
    let ext = cat.unit_ext.get(index).copied().unwrap_or_default();
    let row = Table::new(&ctx);
    row.put(ctx, "name", cat.name(u.name).to_owned());
    row.put(ctx, "index", index as i64);
    row.put(ctx, "cost", i64::from(u.resource_a));
    row.put(ctx, "gas", i64::from(u.gas));
    row.put(ctx, "build_ticks", i64::from(u.build_ticks));
    row.put(ctx, "capacity", i64::from(u.capacity));
    row.put(ctx, "radius_q16", i64::from(u.radius) * 65_536 / 1000);
    row.put(ctx, "rally_x_q16", i64::from(ext.rally_x) * 65_536 / 1000);
    row.put(ctx, "rally_y_q16", i64::from(ext.rally_y) * 65_536 / 1000);
    row.put(ctx, "flying", u.is_air());
    row.put(ctx, "structure", u.is_structure());
    row.put(ctx, "worker", u.is_worker());
    row.put(ctx, "melee", ext.melee);
    row.put(ctx, "resource", i64::from(u.resource_kind));
    row.put(ctx, "filter", i64::from(u.filter));
    row.put(ctx, "size", i64::from(ext.size));
    row.put(ctx, "transport", i64::from(ext.transport));
    row.put(ctx, "capacity_provided", i64::from(ext.capacity_provided));
    if let Some(p) = u.product_index().filter(|&p| p < cat.units.len()) {
        row.put(ctx, "product", cat.name(cat.units[p].name).to_owned());
    }
    row
}

fn research_row<'gc>(ctx: Context<'gc>, cat: &Catalog, index: usize) -> Table<'gc> {
    let r = &cat.research[index];
    let row = Table::new(&ctx);
    row.put(ctx, "name", cat.name(r.name).to_owned());
    let building = cat.units.get(r.building as usize).map_or(String::new(), |u| cat.name(u.name).to_owned());
    row.put(ctx, "building", building);
    row.put(ctx, "cost", i64::from(r.resource_a));
    row.put(ctx, "gas", i64::from(r.gas));
    row.put(ctx, "time", i64::from(r.time));
    row.put(ctx, "applies_mask", i64::from(r.applies_mask));
    row.put(ctx, "to_new", r.to_new);
    row
}

fn ability_row<'gc>(ctx: Context<'gc>, cat: &Catalog, index: usize) -> Table<'gc> {
    let a = &cat.abilities[index];
    let row = Table::new(&ctx);
    row.put(ctx, "name", cat.name(a.name).to_owned());
    row.put(ctx, "cooldown", a.cooldown as i64);
    row.put(ctx, "energy", a.energy as i64);
    row.put(ctx, "target", a.target as i64);
    row.put(ctx, "filter", a.filter as i64);
    row.put(ctx, "projectile", a.is_projectile());
    row.put(ctx, "projectile_speed", a.projectile_speed as i64);
    let first = a.effect_first as usize;
    let span = cat.effects.get(first..first + a.effect_count as usize).unwrap_or(&[]);
    let effects = Table::new(&ctx);
    for (i, e) in span.iter().enumerate() {
        let et = Table::new(&ctx);
        et.put(ctx, "kind", e.kind as i64);
        et.put(ctx, "p0", e.p0 as i64);
        et.put(ctx, "p1", e.p1 as i64);
        et.put(ctx, "p2", e.p2 as i64);
        et.put(ctx, "p3", e.p3 as i64);
        effects.put(ctx, i as i64 + 1, et);
    }
    row.put(ctx, "effects", effects);
    row
}

const PACK_TIMED_STAT_KEY: i32 = -1;

fn known_unit(sim: &Sim, i: i64) -> Result<usize, String> {
    usize::try_from(i).ok().filter(|&u| u < sim.count()).ok_or_else(|| format!("invalid_unit: {i}"))
}

fn raw_q16(v: i64) -> Result<i32, String> {
    i32::try_from(v).map_err(|_| format!("q16 value out of range: {v}"))
}

fn facing_heading(degrees: i64) -> Result<i32, String> {
    let turn = i64::from(sim::FULL_TURN);
    i32::try_from(degrees.rem_euclid(360) * turn / 360).map_err(|_| format!("invalid_facing: {degrees}"))
}

fn diplomacy_player(p: i64) -> Result<u8, String> {
    u8::try_from(p).map_err(|_| format!("invalid_player: {p} is outside 0..=255"))
}

const MAX_RESPAWN_S: i64 = u32::MAX as i64;

fn list_of<'gc>(ctx: Context<'gc>, items: impl Iterator<Item = i64>) -> Result<Table<'gc>, Error<'gc>> {
    let out = Table::new(&ctx);
    for (n, v) in items.enumerate() {
        out.put(ctx, n as i64 + 1, v);
    }
    Ok(out)
}

fn next_ground_id(sh: &Shared) -> i64 {
    let queued = sh.local.iter().chain(sh.cmds.iter()).filter(|c| matches!(c, Cmd::DropGround { .. } | Cmd::DropUnitItem { .. })).count();
    i64::from(sh.sim.ground_next_id()) + queued as i64
}

fn live_unit(sim: &Sim, i: i64) -> Result<usize, String> {
    usize::try_from(i).ok().filter(|&u| sim.alive(u)).ok_or_else(|| format!("invalid_unit: {i}"))
}

fn slot_of(slot: i64, limit: usize) -> Result<usize, String> {
    usize::try_from(slot).ok().filter(|&s| s < limit).ok_or_else(|| format!("invalid_slot: {slot} is outside 0..{limit}"))
}

const MAX_DEFINED_KINDS: usize = 4_096;
const MAX_COUNTER_NAMES: usize = 256;
const MAX_COUNTER_NAME_BYTES: usize = 64;

fn definition_room(known: bool, count: usize, what: &str) -> Result<(), String> {
    if known || count < MAX_DEFINED_KINDS {
        Ok(())
    } else {
        Err(format!("definition_limit: at most {MAX_DEFINED_KINDS} {what} in one catalog"))
    }
}

fn note_counter_name(names: &mut std::collections::HashSet<String>, name: &str) -> Result<(), String> {
    if name.len() > MAX_COUNTER_NAME_BYTES {
        return Err(format!("invalid_counter: name is over {MAX_COUNTER_NAME_BYTES} bytes"));
    }
    if names.contains(name) {
        return Ok(());
    }
    if names.len() >= MAX_COUNTER_NAMES {
        return Err(format!("counter_limit: at most {MAX_COUNTER_NAMES} counter names"));
    }
    names.insert(name.to_owned());
    Ok(())
}

fn buff_args(kind: i64, ticks: i64) -> Result<(i32, i32), String> {
    let kind = i32::try_from(kind).map_err(|_| format!("invalid_buff: kind {kind} out of i32 range"))?;
    let ticks = i32::try_from(ticks).ok().filter(|&t| t > 0).ok_or_else(|| format!("invalid_buff: ticks {ticks} must be a positive i32"))?;
    Ok((kind, ticks))
}

fn target_of(sim: &Sim, target: i64) -> Result<i32, String> {
    if target == -1 {
        Ok(-1)
    } else {
        live_unit(sim, target).map(|t| t as i32)
    }
}

macro_rules! native {
    ($ctx:ident, $table:ident, $name:literal, [$($cap:ident),*], |$c:ident, $s:ident| $body:block) => {{
        $(#[allow(unused_variables)] let $cap = $cap.clone();)*
        #[allow(unused_mut, unused_variables)]
        let callback = Callback::from_fn(&$ctx, move |$c, _, mut $s| guard_native($c, || $body));
        $table.put($ctx, $name, callback);
    }};
}

mod editor;
mod matrix;
mod scaling;
mod seats;
mod sound;

const MAX_STRING_BYTES: usize = 1 << 20;

fn display_bytes(v: Value) -> Vec<u8> {
    match v {
        Value::String(s) => s.as_bytes().to_vec(),
        Value::Integer(n) => n.to_string().into_bytes(),
        Value::Number(n) => format!("{n:?}").into_bytes(),
        Value::Boolean(b) => b.to_string().into_bytes(),
        Value::Nil => b"nil".to_vec(),
        other => other.type_name().as_bytes().to_vec(),
    }
}

fn whole_number(v: Value) -> Option<i64> {
    match v {
        Value::Integer(n) => Some(n),
        Value::Number(n) if n.is_finite() && n.fract() == 0.0 && n.abs() < 9.2e18 => Some(n as i64),
        _ => None,
    }
}

fn pad_field(sign: &[u8], body: &[u8], width: usize, left: bool, zero: bool) -> Vec<u8> {
    let len = sign.len() + body.len();
    let fill = width.saturating_sub(len);
    let mut out = Vec::with_capacity(len + fill);
    if left {
        out.extend_from_slice(sign);
        out.extend_from_slice(body);
        out.extend(std::iter::repeat(b' ').take(fill));
    } else if zero {
        out.extend_from_slice(sign);
        out.extend(std::iter::repeat(b'0').take(fill));
        out.extend_from_slice(body);
    } else {
        out.extend(std::iter::repeat(b' ').take(fill));
        out.extend_from_slice(sign);
        out.extend_from_slice(body);
    }
    out
}

fn format_lua<'gc>(c: Context<'gc>, s: &Stack<'gc, '_>) -> Result<Vec<u8>, Error<'gc>> {
    let fmt = match s.get(0) {
        Value::String(f) => f.as_bytes().to_vec(),
        _ => return Err(fail(c, "bad_argument: format must be a string".into())),
    };
    let mut out = Vec::new();
    let mut arg = 1usize;
    let mut i = 0usize;
    while i < fmt.len() {
        let ch = fmt[i];
        i += 1;
        if ch != b'%' {
            out.push(ch);
            continue;
        }
        if i >= fmt.len() {
            return Err(fail(c, "bad_format: the format ends with %".into()));
        }
        if fmt[i] == b'%' {
            out.push(b'%');
            i += 1;
            continue;
        }
        let (mut left, mut plus, mut space, mut zero, mut alt) = (false, false, false, false, false);
        while i < fmt.len() && matches!(fmt[i], b'-' | b'+' | b' ' | b'0' | b'#') {
            match fmt[i] {
                b'-' => left = true,
                b'+' => plus = true,
                b' ' => space = true,
                b'0' => zero = true,
                _ => alt = true,
            }
            i += 1;
        }
        let mut width = 0usize;
        while i < fmt.len() && fmt[i].is_ascii_digit() {
            width = width.saturating_mul(10).saturating_add(usize::from(fmt[i] - b'0'));
            i += 1;
        }
        let mut prec: Option<usize> = None;
        if i < fmt.len() && fmt[i] == b'.' {
            i += 1;
            let mut p = 0usize;
            while i < fmt.len() && fmt[i].is_ascii_digit() {
                p = p.saturating_mul(10).saturating_add(usize::from(fmt[i] - b'0'));
                i += 1;
            }
            prec = Some(p);
        }
        if i >= fmt.len() {
            return Err(fail(c, "bad_format: the format ends inside a conversion".into()));
        }
        if width > MAX_STRING_BYTES || prec.is_some_and(|p| p > MAX_STRING_BYTES) {
            return Err(fail(c, "bad_format: width or precision too large".into()));
        }
        let conv = fmt[i];
        i += 1;
        if arg >= s.len() {
            return Err(fail(c, format!("bad_argument: no value for conversion %{}", conv as char)));
        }
        let v = s.get(arg);
        arg += 1;
        let sign_of = |negative: bool| -> &'static [u8] {
            if negative {
                b"-"
            } else if plus {
                b"+"
            } else if space {
                b" "
            } else {
                b""
            }
        };
        let field = match conv {
            b'd' | b'i' => {
                let n = whole_number(v).ok_or_else(|| fail(c, format!("bad_argument: argument {arg} has no integer representation")))?;
                let mut digits = n.unsigned_abs().to_string().into_bytes();
                if let Some(p) = prec {
                    while digits.len() < p {
                        digits.insert(0, b'0');
                    }
                }
                pad_field(sign_of(n < 0), &digits, width, left, zero && prec.is_none())
            }
            b'u' | b'x' | b'X' | b'o' => {
                let n = whole_number(v).ok_or_else(|| fail(c, format!("bad_argument: argument {arg} has no integer representation")))? as u64;
                let mut digits = match conv {
                    b'u' => n.to_string(),
                    b'x' => format!("{n:x}"),
                    b'X' => format!("{n:X}"),
                    _ => format!("{n:o}"),
                }
                .into_bytes();
                if let Some(p) = prec {
                    while digits.len() < p {
                        digits.insert(0, b'0');
                    }
                }
                let prefix: &[u8] = match (alt, conv) {
                    (true, b'x') => b"0x",
                    (true, b'X') => b"0X",
                    (true, b'o') => b"0",
                    _ => b"",
                };
                pad_field(prefix, &digits, width, left, zero && prec.is_none())
            }
            b'c' => {
                let n = whole_number(v).ok_or_else(|| fail(c, format!("bad_argument: argument {arg} has no integer representation")))?;
                pad_field(b"", &[n as u8], width, left, false)
            }
            b's' => {
                let mut text = display_bytes(v);
                if let Some(p) = prec {
                    text.truncate(p);
                }
                pad_field(b"", &text, width, left, false)
            }
            b'f' | b'F' | b'g' | b'G' => {
                let x = match v {
                    Value::Integer(n) => n as f64,
                    Value::Number(n) => n,
                    _ => return Err(fail(c, format!("bad_argument: argument {arg} must be a number"))),
                };
                let body = if matches!(conv, b'f' | b'F') { format!("{:.*}", prec.unwrap_or(6), x.abs()) } else { format!("{}", x.abs()) };
                pad_field(sign_of(x.is_sign_negative() && x != 0.0), body.as_bytes(), width, left, zero && x.is_finite())
            }
            other => return Err(fail(c, format!("bad_format: unsupported conversion %{}", other as char))),
        };
        out.extend_from_slice(&field);
        if out.len() > MAX_STRING_BYTES {
            return Err(fail(c, "bad_format: result too large".into()));
        }
    }
    Ok(out)
}

fn str_pos(i: i64, len: usize) -> i64 {
    if i >= 0 {
        i
    } else {
        (len as i64 + i + 1).max(0)
    }
}

fn bind_lua_libs<'gc>(ctx: Context<'gc>, env: Table<'gc>, sh: &Sh) -> Result<(), Error<'gc>> {
    let string = Table::new(&ctx);
    if let Value::Table(std) = ctx.globals().get(ctx, "string") {
        for name in ["len", "sub", "lower", "upper", "reverse"] {
            string.set(ctx, name, std.get(ctx, name))?;
        }
    }
    native!(ctx, string, "format", [], |c, s| {
        let out = format_lua(c, &s)?;
        s.clear();
        s.push_back(Value::String(c.intern(&out)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, string, "rep", [], |c, s| {
        let text = match s.get(0) {
            Value::String(t) => t.as_bytes().to_vec(),
            _ => return Err(fail(c, "bad_argument: argument 1 must be a string".into())),
        };
        let n = a(c, &s, 1)?;
        let sep = match s.get(2) {
            Value::String(t) => t.as_bytes().to_vec(),
            _ => Vec::new(),
        };
        let n = usize::try_from(n).unwrap_or(0);
        if n.saturating_mul(text.len() + sep.len()) > MAX_STRING_BYTES {
            return Err(fail(c, "string.rep: result too large".into()));
        }
        let mut out = Vec::new();
        for k in 0..n {
            if k > 0 {
                out.extend_from_slice(&sep);
            }
            out.extend_from_slice(&text);
        }
        s.clear();
        s.push_back(Value::String(c.intern(&out)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, string, "byte", [], |c, s| {
        let text = match s.get(0) {
            Value::String(t) => t.as_bytes().to_vec(),
            _ => return Err(fail(c, "bad_argument: argument 1 must be a string".into())),
        };
        let first = str_pos(if matches!(s.get(1), Value::Nil) { 1 } else { a(c, &s, 1)? }, text.len()).max(1);
        let last = str_pos(if matches!(s.get(2), Value::Nil) { first } else { a(c, &s, 2)? }, text.len()).min(text.len() as i64);
        s.clear();
        for k in first..=last {
            s.push_back(Value::Integer(i64::from(text[(k - 1) as usize])));
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, string, "char", [], |c, s| {
        let mut out = Vec::with_capacity(s.len());
        for k in 0..s.len() {
            let code = a(c, &s, k)?;
            out.push(u8::try_from(code).map_err(|_| fail(c, format!("bad_argument: argument {} is not a byte", k + 1)))?);
        }
        s.clear();
        s.push_back(Value::String(c.intern(&out)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, string, "find", [], |c, s| {
        let (text, needle) = match (s.get(0), s.get(1)) {
            (Value::String(t), Value::String(n)) => (t.as_bytes().to_vec(), n.as_bytes().to_vec()),
            _ => return Err(fail(c, "bad_argument: find takes two strings".into())),
        };
        let plain = s.get(3).to_bool();
        if !plain && needle.iter().any(|b| b"^$()%.[]*+-?".contains(b)) {
            return Err(fail(c, "unsupported: string.find takes plain text only; pass true as the fourth argument".into()));
        }
        let init = str_pos(if matches!(s.get(2), Value::Nil) { 1 } else { a(c, &s, 2)? }, text.len()).max(1) as usize;
        let found = if init - 1 > text.len() { None } else { text[init - 1..].windows(needle.len().max(1)).position(|w| needle.is_empty() || w == &needle[..]) };
        match found {
            Some(p) => {
                let start = init + p;
                s.replace(c, (start as i64, (start + needle.len()) as i64 - 1));
            }
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    env.set(ctx, "string", string)?;

    let table = Table::new(&ctx);
    if let Value::Table(std) = ctx.globals().get(ctx, "table") {
        for name in ["pack", "unpack"] {
            table.set(ctx, name, std.get(ctx, name))?;
        }
    }
    native!(ctx, table, "insert", [], |c, s| {
        let t = table_arg(c, s.get(0), 0)?;
        let len = t.length();
        match s.len() {
            2 => {
                t.set(c, len + 1, s.get(1))?;
            }
            3 => {
                let pos = a(c, &s, 1)?;
                if pos < 1 || pos > len + 1 {
                    return Err(fail(c, "bad_argument: table.insert position out of bounds".into()));
                }
                let mut k = len;
                while k >= pos {
                    t.set(c, k + 1, t.get(c, k))?;
                    k -= 1;
                }
                t.set(c, pos, s.get(2))?;
            }
            _ => return Err(fail(c, "bad_argument: table.insert takes a table, an optional position and a value".into())),
        }
        s.clear();
        Ok(CallbackReturn::Return)
    });
    native!(ctx, table, "remove", [], |c, s| {
        let t = table_arg(c, s.get(0), 0)?;
        let len = t.length();
        let pos = if matches!(s.get(1), Value::Nil) { len } else { a(c, &s, 1)? };
        if len == 0 && matches!(s.get(1), Value::Nil) {
            s.replace(c, ());
            return Ok(CallbackReturn::Return);
        }
        if pos < 1 || pos > len + 1 {
            return Err(fail(c, "bad_argument: table.remove position out of bounds".into()));
        }
        let taken = t.get(c, pos);
        let mut k = pos;
        while k < len {
            t.set(c, k, t.get(c, k + 1))?;
            k += 1;
        }
        if pos <= len {
            t.set(c, len, Value::Nil)?;
        }
        s.replace(c, taken);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, table, "concat", [], |c, s| {
        let t = table_arg(c, s.get(0), 0)?;
        let sep = match s.get(1) {
            Value::String(t) => t.as_bytes().to_vec(),
            _ => Vec::new(),
        };
        let first = if matches!(s.get(2), Value::Nil) { 1 } else { a(c, &s, 2)? };
        let last = if matches!(s.get(3), Value::Nil) { t.length() } else { a(c, &s, 3)? };
        let mut out = Vec::new();
        let mut k = first;
        while k <= last {
            match t.get(c, k) {
                v @ (Value::String(_) | Value::Integer(_) | Value::Number(_)) => out.extend_from_slice(&display_bytes(v)),
                _ => return Err(fail(c, format!("bad_argument: table.concat found a non-string at {k}"))),
            }
            if k < last {
                out.extend_from_slice(&sep);
            }
            if out.len() > MAX_STRING_BYTES {
                return Err(fail(c, "table.concat: result too large".into()));
            }
            k += 1;
        }
        s.clear();
        s.push_back(Value::String(c.intern(&out)));
        Ok(CallbackReturn::Return)
    });
    env.set(ctx, "table", table)?;

    if let Value::Table(math) = env.get(ctx, "math") {
        native!(ctx, math, "random", [sh], |c, s| {
            let bits = sh.borrow_mut().sim.random_bits();
            let (low, high) = match (s.get(0), s.get(1)) {
                (Value::Nil, _) => {
                    s.replace(c, bits as f64 / (1u64 << 53) as f64);
                    return Ok(CallbackReturn::Return);
                }
                (_, Value::Nil) => (1, a(c, &s, 0)?),
                _ => (a(c, &s, 0)?, a(c, &s, 1)?),
            };
            if low > high {
                return Err(fail(c, "bad_argument: math.random interval is empty".into()));
            }
            let span = (i128::from(high) - i128::from(low) + 1) as u128;
            let pick = (u128::from(bits) * span) >> 53;
            s.replace(c, (i128::from(low) + pick as i128) as i64);
            Ok(CallbackReturn::Return)
        });
    }
    if let Value::Table(game) = env.get(ctx, "game") {
        native!(ctx, game, "random", [sh], |c, s| {
            let n = a(c, &s, 0)?;
            if n < 1 {
                s.replace(c, ());
                return Ok(CallbackReturn::Return);
            }
            let bits = sh.borrow_mut().sim.random_bits();
            s.replace(c, ((u128::from(bits) * n as u128) >> 53) as i64);
            Ok(CallbackReturn::Return)
        });
    }
    Ok(())
}

fn bind_unit_primitives<'gc>(ctx: Context<'gc>, game: Table<'gc>, sh: &Sh) {
    native!(ctx, game, "xp", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.xp(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "xp_curve", [sh], |c, s| {
        let sh = sh.borrow();
        s.replace(c, list_of(c, sh.sim.xp_curve().iter().map(|&t| i64::from(t)))?);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "stat_value", [sh], |c, s| {
        let (i, code) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let code = usize::try_from(code).map_err(|_| fail(c, format!("invalid_stat: {code}")))?;
        s.replace(c, i64::from(sh.sim.stat(u, code)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "item_bonus", [sh], |c, s| {
        let (i, name) = (a(c, &s, 0)?, text(c, s.get(1), 1)?);
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.item_bonus(u, &name)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "xp_bounty", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.xp_bounty(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "deny_xp_pct", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.deny_xp_pct(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "assist_radius_milli", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.assist_radius_milli(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "upgrade_bits", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.upgrade_bits(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "alive", [sh], |c, s| {
        let sh = sh.borrow();
        let i = a(c, &s, 0)?;
        let alive = usize::try_from(i).is_ok_and(|u| u < sh.sim.count() && sh.sim.alive(u));
        s.replace(c, alive);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "is_summoned", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.is_summoned(u));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "counter", [sh], |c, s| {
        let (i, name) = (a(c, &s, 0)?, text(c, s.get(1), 1)?);
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.counter(u, &name));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_counter", [sh], |c, s| {
        let (i, name, value) = (a(c, &s, 0)?, text(c, s.get(1), 1)?, a(c, &s, 2)?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        note_counter_name(&mut sh.counter_names, &name).map_err(|m| fail(c, m))?;
        sh.sim.set_counter(u, &name, value);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "take_recent_hitters", [sh], |c, s| {
        let (i, window) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let window = u32::try_from(window).map_err(|_| fail(c, "invalid_window: ticks must not be negative".into()))?;
        let ids = sh.sim.take_recent_hitters(u, window);
        s.replace(c, list_of(c, ids.into_iter().map(i64::from))?);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_spell_block", [sh], |c, s| {
        let (unit, ticks) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let ticks = u32::try_from(ticks).map_err(|_| fail(c, "invalid_ticks: must not be negative".into()))?;
        sh.sim.set_spell_block(u, ticks);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "spell_block_left", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.spell_block_left(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "item_lock_left", [sh], |c, s| {
        let (unit, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = usize::try_from(slot).map_err(|_| fail(c, "invalid_slot".into()))?;
        s.replace(c, i64::from(sh.sim.item_lock_left(u, slot)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "take_blocks", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let blocks = sh.sim.take_blocks();
        let out = Table::new(&c);
        for (n, (target, caster, ability)) in blocks.into_iter().enumerate() {
            let row = Table::new(&c);
            row.put(c, "target", target as i64);
            row.put(c, "caster", caster as i64);
            row.put(c, "ability", sh.sim.catalog().name(sh.sim.catalog().abilities[ability].name).to_owned());
            out.put(c, n as i64 + 1, row);
        }
        s.replace(c, out);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "recent_hitters", [sh], |c, s| {
        let (i, window) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let window = u32::try_from(window).map_err(|_| fail(c, "invalid_window: ticks must not be negative".into()))?;
        let ids = sh.sim.recent_hitters(u, window);
        s.replace(c, list_of(c, ids.into_iter().map(i64::from))?);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "heavy_hit_age", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        match sh.sim.heavy_hit_age(u) {
            Some(age) => s.replace(c, i64::from(age)),
            None => s.replace(c, Value::Nil),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "add_kill", [sh], |c, s| {
        let (i, n) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let n = u32::try_from(n).map_err(|_| fail(c, "invalid_kill: must not be negative".into()))?;
        sh.sim.add_kill(u, n);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "add_credit", [sh], |c, s| {
        let (i, n) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let n = u32::try_from(n).map_err(|_| fail(c, "invalid_credit: must not be negative".into()))?;
        sh.sim.add_credit(u, n);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "attr_field", [sh], |c, s| {
        let (i, name) = (a(c, &s, 0)?, text(c, s.get(1), 1)?);
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        match sh.sim.attr_field(u, &name) {
            Some(v) => s.replace(c, i64::from(v)),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_upgrade_bit", [sh], |c, s| {
        let (i, bit) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let on = matches!(s.get(2), Value::Boolean(true));
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let bit = u32::try_from(bit).map_err(|_| fail(c, format!("invalid_bit: {bit}")))?;
        let took = sh.sim.set_upgrade_bit(u, bit, on).is_ok();
        s.replace(c, took);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "enroll", [sh], |c, s| {
        let (i, x, y) = (a(c, &s, 0)?, fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        sh.sim.enroll_attributed(u, x, y);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "seat_start", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let start = sh.borrow().sim.catalog().starts.iter().find(|st| st.player == player).map(|st| (st.x, st.y));
        match start {
            Some((x, y)) => {
                let t = Table::new(&c);
                t.put(c, "x", i64::from(x));
                t.put(c, "y", i64::from(y));
                s.replace(c, t);
            }
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "spawn", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let kind = match s.get(1) {
            Value::Integer(n) => u32::try_from(n).ok(),
            other => {
                let name = text(c, other, 1)?;
                sh.borrow().sim.catalog().unit_index(&name).map(|k| k as u32)
            }
        };
        let (x, y) = (fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 3)?).map_err(|m| fail(c, m))?);
        let spawned = kind.and_then(|k| sh.borrow_mut().sim.spawn_for(player, k, x, y));
        match spawned {
            Some(u) => s.replace(c, u as i64),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "teleport", [sh], |c, s| {
        let (i, x, y) = (a(c, &s, 0)?, fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let took = sh.sim.teleport(u, x, y);
        s.replace(c, took);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "timed_spawn", [sh], |c, s| {
        let (caster, kind, life) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let (x, y) = (fixed(a(c, &s, 3)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 4)?).map_err(|m| fail(c, m))?);
        if life < 0 {
            return Err(fail(c, format!("invalid_timed_spawn: life {life} must not be negative")));
        }
        let (Ok(kind), Ok(life)) = (i32::try_from(kind), i32::try_from(life)) else {
            return Err(fail(c, "invalid_timed_spawn: kind or life out of i32 range".into()));
        };
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, caster).map_err(|m| fail(c, m))?;
        let before: Vec<bool> = (0..sh.sim.count()).map(|i| sh.sim.alive(i)).collect();
        sh.sim.timed_spawn_effect(u, kind, life, x, y);
        let placed = (0..sh.sim.count()).find(|&i| sh.sim.alive(i) && !before.get(i).copied().unwrap_or(false));
        match placed {
            Some(i) => s.replace(c, i as i64),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cast_point", [sh], |c, s| {
        let sh = sh.borrow();
        let (_, x, y) = sh.sim.last_cast(known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?);
        s.replace(c, (i64::from(x), i64::from(y)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cast_target", [sh], |c, s| {
        let sh = sh.borrow();
        let (target, _, _) = sh.sim.last_cast(known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?);
        match u32::try_from(target) {
            Ok(t) => s.replace(c, i64::from(t)),
            Err(_) => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "map_size", [sh], |c, s| {
        let sh = sh.borrow();
        let g = sh.sim.grid();
        s.replace(c, ((g.w as i64) << 16, (g.h as i64) << 16));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_add_xp", [sh], |c, s| {
        let (unit, amount) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let amount = u32::try_from(amount).map_err(|_| fail(c, format!("invalid_amount: {amount} is negative")))?;
        sh.cmds.push(Cmd::AddXp { unit: u, amount });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_timed_stat", [sh], |c, s| {
        let (unit, code, delta, ticks) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?, a(c, &s, 3)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let code = usize::try_from(code).map_err(|_| fail(c, format!("invalid_stat: {code}")))?;
        let delta = amount_arg(delta).map_err(|m| fail(c, m))?;
        let ticks = i32::try_from(ticks).ok().filter(|&t| t > 0).ok_or_else(|| fail(c, format!("invalid_ticks: {ticks}")))?;
        sh.cmds.push(Cmd::TimedStat { unit: u, code, delta, ticks });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_add_stat", [sh], |c, s| {
        let (unit, code, delta) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let code = usize::try_from(code).map_err(|_| fail(c, format!("invalid_stat: {code}")))?;
        let delta = amount_arg(delta).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::AddStat { unit: u, code, delta });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "taunter", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        match sh.sim.taunter_of(u) {
            Some(t) => s.replace(c, t as i64),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "refresh_attr_stats", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        sh.sim.refresh_attr_stats(u);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_skill_rank", [sh], |c, s| {
        let (i, slot, rank) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ABILITY_SLOTS).map_err(|m| fail(c, m))?;
        let rank = u8::try_from(rank).map_err(|_| fail(c, format!("invalid_rank: {rank} is outside 0..=255")))?;
        match sh.sim.set_skill_rank(u, slot, rank) {
            Some(held) => s.replace(c, i64::from(held)),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "spawn_neutral", [sh], |c, s| {
        let kind = match s.get(0) {
            Value::Integer(n) => u32::try_from(n).ok(),
            other => {
                let name = text(c, other, 0)?;
                sh.borrow().sim.catalog().unit_index(&name).map(|k| k as u32)
            }
        };
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let heading = match s.get(3) {
            Value::Nil => None,
            _ => Some(facing_heading(a(c, &s, 3)?).map_err(|m| fail(c, m))?),
        };
        let placed = kind.and_then(|k| sh.borrow_mut().sim.spawn_for(NEUTRAL_OWNER, k, x, y));
        if let (Some(u), Some(h)) = (placed, heading) {
            sh.borrow_mut().sim.set_heading(u, h);
        }
        match placed {
            Some(u) => s.replace(c, u as i64),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_turn_rate", [sh], |c, s| {
        let (unit, deg) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let deg = i32::try_from(deg).map_err(|_| fail(c, format!("invalid_turn_rate: {deg}")))?;
        let mut sh = sh.borrow_mut();
        let unit = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.set_turn_rate(unit, deg));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_hold_fire", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let on = s.get(1).to_bool();
        let mut sh = sh.borrow_mut();
        let unit = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.set_hold_fire(unit, on));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "holds_fire", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.holds_fire(u));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_heading", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.heading_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_heading", [sh], |c, s| {
        let (unit, heading) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let heading = i32::try_from(heading).map_err(|_| fail(c, format!("invalid_heading: {heading}")))?;
        let mut sh = sh.borrow_mut();
        let unit = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.set_heading(unit, heading));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "turn_step", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.turn_step_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "bearing", [], |c, s| {
        let (dx, dy) = (a(c, &s, 0)?, a(c, &s, 1)?);
        match sim::angle_of(dx, dy) {
            Some(angle) => s.replace(c, i64::from(angle)),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_move_tree_rule", [sh], |c, s| {
        let half = raw_q16(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let secs = num(c, s.get(1), 1)?;
        let ticks = (secs.max(0.0) * f64::from(TICKS_PER_SECOND)).round().min(f64::from(u32::MAX)) as u32;
        sh.borrow_mut().sim.set_move_tree_rule(half, ticks);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "tree_add", [sh], |c, s| {
        let (cx, cy, w, h) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?, a(c, &s, 3)?);
        let prop = match s.get(4) {
            Value::Nil => -1,
            _ => i32::try_from(a(c, &s, 4)?).map_err(|_| fail(c, "invalid_prop: id out of i32 range".to_string()))?,
        };
        let rect = (i32::try_from(cx), i32::try_from(cy), i32::try_from(w), i32::try_from(h));
        let placed = match rect {
            (Ok(cx), Ok(cy), Ok(w), Ok(h)) => sh.borrow_mut().sim.add_tree(cx, cy, w, h, prop).ok(),
            _ => None,
        };
        match placed {
            Some(id) => s.replace(c, i64::from(id)),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "blocker_add", [sh], |c, s| {
        let (cx, cy, w, h, secs) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?, a(c, &s, 3)?, num(c, s.get(4), 4)?);
        let ticks = (secs.max(0.0) * f64::from(TICKS_PER_SECOND)).round().min(f64::from(u32::MAX)) as u32;
        let rect = (i32::try_from(cx), i32::try_from(cy), i32::try_from(w), i32::try_from(h));
        let placed = match rect {
            (Ok(cx), Ok(cy), Ok(w), Ok(h)) => sh.borrow_mut().sim.add_blocker(cx, cy, w, h, ticks).ok(),
            _ => None,
        };
        match placed {
            Some(id) => s.replace(c, i64::from(id)),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "blocker_remove", [sh], |c, s| {
        let id = u32::try_from(a(c, &s, 0)?).ok();
        let done = id.is_some_and(|id| sh.borrow_mut().sim.remove_blocker(id).is_ok());
        s.replace(c, done);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "tree_kill", [sh], |c, s| {
        let id = u32::try_from(a(c, &s, 0)?).ok();
        let secs = num(c, s.get(1), 1)?;
        let ticks = (secs.max(0.0) * f64::from(TICKS_PER_SECOND)).round().min(f64::from(u32::MAX)) as u32;
        let done = id.is_some_and(|id| sh.borrow_mut().sim.kill_tree(id, ticks).is_ok());
        s.replace(c, done);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "tree_restore", [sh], |c, s| {
        let id = u32::try_from(a(c, &s, 0)?).ok();
        let done = id.is_some_and(|id| sh.borrow_mut().sim.restore_tree(id).is_ok());
        s.replace(c, done);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_tree_guard", [sh], |c, s| {
        let radius = raw_q16(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        sh.borrow_mut().sim.set_tree_guard(radius);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "trees_near", [sh], |c, s| {
        let (x, y, r) = (raw_q16(a(c, &s, 0)?).map_err(|m| fail(c, m))?, raw_q16(a(c, &s, 1)?).map_err(|m| fail(c, m))?, raw_q16(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let sh = sh.borrow();
        s.replace(c, list_of(c, sh.sim.trees_near(x, y, r).into_iter().map(i64::from))?);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "tree_info", [sh], |c, s| {
        let id = a(c, &s, 0)?;
        let sh = sh.borrow();
        match usize::try_from(id).ok().and_then(|k| sh.sim.obstacles().get(k).copied()) {
            Some(o) => {
                let row = Table::new(&c);
                row.put(c, "x", i64::from(o.x0));
                row.put(c, "y", i64::from(o.y0));
                row.put(c, "w", i64::from(o.w));
                row.put(c, "h", i64::from(o.h));
                row.put(c, "alive", o.alive);
                row.put(c, "until", i64::from(o.until));
                row.put(c, "blocker", o.temp);
                s.replace(c, row);
            }
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "tree_count", [sh], |c, s| {
        s.replace(c, sh.borrow().sim.obstacles().len() as i64);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_resources", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let gold = amount_arg(a(c, &s, 1)?).map_err(|m| fail(c, m))?;
        let gas = amount_arg(a(c, &s, 2)?).map_err(|m| fail(c, m))?;
        let mut sh = sh.borrow_mut();
        let (have_gold, have_gas) = (sh.sim.resource_a(player), sh.sim.resource_b(player));
        sh.sim.add_resources(player, gold.wrapping_sub(have_gold), gas.wrapping_sub(have_gas));
        Ok(CallbackReturn::Return)
    });
}

fn bind<'gc>(ctx: Context<'gc>, env: Table<'gc>, sh: &Sh) {
    let game = Table::new(&ctx);
    bind_unit_primitives(ctx, game, sh);
    seats::bind_seats(ctx, game, sh);
    editor::bind_editor(ctx, game, sh);
    scaling::bind_scaling(ctx, game, sh);
    sound::bind_sound(ctx, game, sh);

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
    native!(ctx, env, "unit_position", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let (x, y) = sh.sim.position_fixed(u);
        s.replace(c, (x as i64, y as i64));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "route_len", [sh], |c, s| {
        let (p, id) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let player = u8::try_from(p).map_err(|_| fail(c, "invalid_route: player out of range".into()))?;
        let id = u8::try_from(id).map_err(|_| fail(c, "invalid_route: route out of range".into()))?;
        let sh = sh.borrow();
        s.replace(c, sh.sim.route_len(player, id) as i64);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, env, "route_point", [sh], |c, s| {
        let (p, id, step) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let player = u8::try_from(p).map_err(|_| fail(c, "invalid_route: player out of range".into()))?;
        let id = u8::try_from(id).map_err(|_| fail(c, "invalid_route: route out of range".into()))?;
        let step = usize::try_from(step).map_err(|_| fail(c, "invalid_route: step out of range".into()))?;
        let sh = sh.borrow();
        let (x, y) = sh.sim.route_point(player, id, step);
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
    native!(ctx, env, "ability_info", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let sh = sh.borrow();
        let cat = sh.sim.catalog();
        let index = cat.abilities.iter().position(|a| cat.name(a.name) == name).ok_or_else(|| fail(c, format!("unknown_ability: {name}")))?;
        s.replace(c, ability_row(c, cat, index));
        Ok(CallbackReturn::Return)
    });

    native!(ctx, env, "ability_ext", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let sh = sh.borrow();
        let cat = sh.sim.catalog();
        let index = cat.abilities.iter().position(|a| cat.name(a.name) == name).ok_or_else(|| fail(c, format!("unknown_ability: {name}")))?;
        let ext = cat.ext.get(index).copied().unwrap_or_default();
        let t = Table::new(&c);
        t.put(c, "cast_range", i64::from(ext.cast_range));
        t.put(c, "cast_point", i64::from(ext.cast_point));
        t.put(c, "period", i64::from(ext.period));
        t.put(c, "chance", i64::from(ext.chance));
        t.put(c, "charges", i64::from(ext.charges));
        t.put(c, "charge_restore", i64::from(ext.charge_restore));
        t.put(c, "flags", i64::from(ext.flags));
        s.replace(c, t);
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
    native!(ctx, game, "cmd_hold", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::Hold { unit });
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
    native!(ctx, game, "cmd_drop_ground", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let count = a(c, &s, 1)?;
        let (x, y) = (fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 3)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let item = sh.sim.catalog().item_index(&name).ok_or_else(|| fail(c, format!("unknown_item: {name}")))?;
        let count = u32::try_from(count).ok().filter(|&n| n > 0).ok_or_else(|| fail(c, format!("invalid_count: {count} must be 1 or more")))?;
        let id = next_ground_id(&sh);
        sh.cmds.push(Cmd::DropGround { item, count, x, y });
        s.replace(c, id);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_drop_item", [sh], |c, s| {
        let (unit, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let (x, y) = (fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 3)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ITEM_SLOTS).map_err(|m| fail(c, m))?;
        let id = next_ground_id(&sh);
        sh.cmds.push(Cmd::DropUnitItem { unit, slot, x, y });
        s.replace(c, id);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_pickup_item", [sh], |c, s| {
        let (unit, id) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let id = u32::try_from(id).map_err(|_| fail(c, format!("invalid_ground_item: {id}")))?;
        sh.cmds.push(Cmd::PickupGround { unit, id });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_remove_ground", [sh], |c, s| {
        let id = a(c, &s, 0)?;
        let id = u32::try_from(id).map_err(|_| fail(c, format!("invalid_ground_item: {id}")))?;
        sh.borrow_mut().cmds.push(Cmd::RemoveGround { id });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_set_ground_charges", [sh], |c, s| {
        let (id, count) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let id = u32::try_from(id).map_err(|_| fail(c, format!("invalid_ground_item: {id}")))?;
        let count = u32::try_from(count).map_err(|_| fail(c, format!("invalid_count: {count} must be 0 or more")))?;
        sh.borrow_mut().cmds.push(Cmd::SetGroundCount { id, count });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_set_item_charges", [sh], |c, s| {
        let (unit, slot, count) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ITEM_SLOTS).map_err(|m| fail(c, m))?;
        let count = u32::try_from(count).map_err(|_| fail(c, format!("invalid_count: {count} must be 0 or more")))?;
        sh.cmds.push(Cmd::SetItemCharges { unit, slot, count });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_replace_item", [sh], |c, s| {
        let (unit, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let name = text(c, s.get(2), 2)?;
        let count = a(c, &s, 3)?;
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ITEM_SLOTS).map_err(|m| fail(c, m))?;
        let item = sh.sim.catalog().item_index(&name).ok_or_else(|| fail(c, format!("unknown_item: {name}")))?;
        let count = u32::try_from(count).ok().filter(|&n| n > 0).ok_or_else(|| fail(c, format!("invalid_count: {count} must be 1 or more")))?;
        sh.cmds.push(Cmd::ReplaceItem { unit, slot, item, count });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_use_item_on", [sh], |c, s| {
        let (unit, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let kind = text(c, s.get(2), 2)?;
        let target = a(c, &s, 3)?;
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ITEM_SLOTS).map_err(|m| fail(c, m))?;
        let prop = match kind.as_str() {
            "unit" => false,
            "prop" => true,
            other => return Err(fail(c, format!("invalid_target_kind: {other} is not unit or prop"))),
        };
        let target = u32::try_from(target).map_err(|_| fail(c, format!("invalid_target: {target}")))?;
        sh.cmds.push(Cmd::UseItemOn { unit, slot, prop, target });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "item_owner", [sh], |c, s| {
        let (unit, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let sh = sh.borrow();
        let unit = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ITEM_SLOTS).map_err(|m| fail(c, m))?;
        match sh.sim.item_owner(unit, slot) {
            Some(player) => s.replace(c, player as i64),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_set_item_owner", [sh], |c, s| {
        let (unit, slot, player) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let mut sh = sh.borrow_mut();
        let unit = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ITEM_SLOTS).map_err(|m| fail(c, m))?;
        let player = usize::try_from(player).ok().filter(|&p| p < sim::MAX_PLAYERS).ok_or_else(|| fail(c, format!("invalid_player: {player}")))?;
        sh.cmds.push(Cmd::SetItemOwner { unit, slot, player });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "ground_items", [sh], |c, s| {
        let sh = sh.borrow();
        let cat = sh.sim.catalog();
        let list = Table::new(&c);
        for (n, g) in sh.sim.ground_items().iter().enumerate() {
            let Some(item) = (g.kind as usize).checked_sub(1).and_then(|i| cat.items.get(i)) else { continue };
            let row = Table::new(&c);
            row.put(c, "id", i64::from(g.id));
            row.put(c, "item", i64::from(g.kind) - 1);
            row.put(c, "name", cat.name(item.name).to_owned());
            row.put(c, "count", i64::from(g.count));
            row.put(c, "x", i64::from(g.x));
            row.put(c, "y", i64::from(g.y));
            if g.owner != 0 {
                row.put(c, "owner", i64::from(g.owner) - 1);
            }
            list.put(c, n as i64 + 1, row);
        }
        s.replace(c, list);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "strike_reach", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let range = i64::from(raw_q16(a(c, &s, 1)?).map_err(|m| fail(c, m))?.max(0));
        let (ux, uy) = sh.sim.position_fixed(u);
        let r2 = range * range;
        let mut pairs = Vec::new();
        for j in 0..sh.sim.count() {
            let (jx, jy) = sh.sim.position_fixed(j);
            let (dx, dy) = (i64::from(jx) - i64::from(ux), i64::from(jy) - i64::from(uy));
            let d2 = dx * dx + dy * dy;
            if d2 <= r2 && sh.sim.can_target(u, j) {
                pairs.push((j as i64, d2));
            }
        }
        s.replace(c, list_of(c, pairs.into_iter().flat_map(|(j, d2)| [j, d2]))?);
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
    native!(ctx, game, "is_attributed", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.is_attributed(u));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "hostile", [sh], |c, s| {
        let p = diplomacy_player(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let q = diplomacy_player(a(c, &s, 1)?).map_err(|m| fail(c, m))?;
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
    native!(ctx, game, "props_near", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let reach = i32::try_from(a(c, &s, 1)?).map_err(|_| fail(c, "invalid_reach: out of i32 range".into()))?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        s.replace(c, list_of(c, sh.sim.props_near(u, reach).into_iter().map(i64::from))?);
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
            row.put(c, "slot", slot as i64);
            row.put(c, "item", i64::from(kind) - 1);
            row.put(c, "name", cat.name(item.name).to_owned());
            row.put(c, "count", i64::from(count));
            n += 1;
            out.put(c, n, row);
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
        t.put(c, "name", cat.name(it.name).to_owned());
        t.put(c, "cost", i64::from(it.cost));
        t.put(c, "charges", i64::from(it.charges));
        t.put(c, "stack_max", i64::from(it.stack_max));
        t.put(c, "tags", i64::from(it.tags));
        t.put(c, "stock_max", i64::from(it.stock_max));
        t.put(c, "stock_initial", i64::from(it.stock_initial));
        t.put(c, "stock_time_s", i64::from(it.stock_time_s));
        t.put(c, "stock_start_s", i64::from(it.stock_start_s));
        t.put(c, "no_buy", it.flags & ITEM_FLAG_NO_BUY != 0);
        t.put(c, "no_sell", it.flags & ITEM_FLAG_NO_SELL != 0);
        t.put(c, "consumable", it.flags & 1 != 0);
        t.put(c, "no_drop", it.flags & ITEM_FLAG_NO_DROP != 0);
        t.put(c, "components", list_of(c, it.components.iter().filter(|&&k| k != NO_COMPONENT).map(|&k| i64::from(k)))?);
        let stats = Table::new(&c);
        for (name, v) in sim::ITEM_STATS.iter().zip(it.stats) {
            if v != 0 {
                stats.put(c, *name, i64::from(v));
            }
        }
        t.put(c, "stats", stats);
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
    native!(ctx, game, "set_healing", [sh], |c, s| {
        let unit = usize::try_from(a(c, &s, 0)?).map_err(|_| fail(c, "invalid_unit".into()))?;
        let h = a(c, &s, 1)?;
        let hp_per_s = i32::try_from(h).ok().filter(|&v| v >= 0).ok_or_else(|| fail(c, format!("invalid_healing: {h} is outside 0..=2147483647")))?;
        sh.borrow_mut().cmds.push(Cmd::SetHealing { unit, hp_per_s });
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
    native!(ctx, game, "cmd_respawn_as", [sh], |c, s| {
        let (unit, name) = (a(c, &s, 0)?, text(c, s.get(1), 1)?);
        let mut sh = sh.borrow_mut();
        let unit = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let kind = sh.sim.catalog().unit_index(&name).ok_or_else(|| fail(c, format!("unknown_unit: {name}")))?;
        let heading = match s.get(2) {
            Value::Nil => None,
            _ => Some(facing_heading(a(c, &s, 2)?).map_err(|m| fail(c, m))?),
        };
        sh.cmds.push(Cmd::RespawnAs { unit, kind: kind as u32, heading });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "join_spawn_group", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let mut sh = sh.borrow_mut();
        let unit = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.join_spawn_group(unit).is_ok());
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
    native!(ctx, game, "cmd_strike", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let target = a(c, &s, 1)?;
        let mut sh = sh.borrow_mut();
        let unit = usize::try_from(unit).map_err(|_| fail(c, format!("invalid_unit: {unit}")))?;
        let target = i32::try_from(target).map_err(|_| fail(c, format!("invalid_target: {target}")))?;
        sh.cmds.push(Cmd::SetStrike { unit, target });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "weapon", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let (damage, range) = sh.sim.weapon(u);
        s.replace(c, (i64::from(damage), i64::from(range)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "can_target", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let o = known_unit(&sh.sim, a(c, &s, 1)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.can_target(u, o));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "status_left", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let name = text(c, s.get(1), 1)?;
        let kind = sh.sim.status_index(&name).ok_or_else(|| fail(c, format!("unknown status {name}")))?;
        s.replace(c, i64::from(sh.sim.status_left(u, kind)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "apply_status", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let name = text(c, s.get(1), 1)?;
        let ticks = i32::try_from(a(c, &s, 2)?).map_err(|_| fail(c, "invalid_apply_status: ticks out of i32 range".into()))?;
        let kind = sh.sim.status_index(&name).ok_or_else(|| fail(c, format!("unknown status {name}")))?;
        match s.get(3) {
            Value::Nil => sh.sim.grant_status(u, kind, ticks),
            _ => {
                let mul = i32::try_from(a(c, &s, 3)?).map_err(|_| fail(c, "invalid_apply_status: multiplier out of i32 range".into()))?;
                sh.sim.grant_status_scaled(u, kind, ticks, mul);
            }
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "apply_slow", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let ticks = i32::try_from(a(c, &s, 1)?).map_err(|_| fail(c, "invalid_apply_slow: ticks out of i32 range".into()))?;
        let mul = i32::try_from(a(c, &s, 2)?).map_err(|_| fail(c, "invalid_apply_slow: multiplier out of i32 range".into()))?;
        let kind = sh.sim.status_index("slow").ok_or_else(|| fail(c, "unknown status slow".into()))?;
        sh.sim.apply_status(u, kind, ticks, mul);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "clear_status", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let name = text(c, s.get(1), 1)?;
        let kind = sh.sim.status_index(&name).ok_or_else(|| fail(c, format!("unknown status {name}")))?;
        sh.sim.clear_status(u, kind);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_status_flags", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let name = text(c, s.get(0), 0)?;
        let kind = sh.sim.status_index(&name).ok_or_else(|| fail(c, format!("unknown status {name}")))?;
        let bits = fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.set_status_flags(kind, bits));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "clear_timed_mods", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let buffs = a(c, &s, 1)? != 0;
        sh.sim.clear_timed_mods(u, buffs);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "clear_dots", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        sh.sim.clear_dots(u);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "spell_damage", [sh], |c, s| {
        let (caster, target, amount, dtype) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?, a(c, &s, 3)?);
        let mut sh = sh.borrow_mut();
        let cu = known_unit(&sh.sim, caster).map_err(|m| fail(c, m))?;
        let t = known_unit(&sh.sim, target).map_err(|m| fail(c, m))?;
        let amount = i32::try_from(amount).ok().filter(|v| (0..=32767).contains(v)).ok_or_else(|| fail(c, format!("invalid_damage: {amount} is outside 0..=32767")))?;
        let dtype = i32::try_from(dtype).ok().filter(|&v| (1..=3).contains(&(v & sim::SPELL_KIND_MASK)) && sim::spell_type_valid(v)).ok_or_else(|| fail(c, format!("invalid_damage_type: {dtype} is not a kind 1..=3 plus an attack type")))?;
        let killed = sh.sim.spell_damage(cu as i32, t, amount, dtype);
        s.replace(c, killed);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "add_stat", [sh], |c, s| {
        let (unit, code, delta) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let code = usize::try_from(code).ok().filter(|&v| v < sim::STAT_COUNT).ok_or_else(|| fail(c, format!("invalid_stat: {code}")))?;
        let delta = amount_arg(delta).map_err(|m| fail(c, m))?;
        sh.sim.add_stat_bonus(u, code, delta);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_hp", [sh], |c, s| {
        let (unit, hp) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let hp = raw_q16(hp).map_err(|m| fail(c, m))?;
        let set = sh.sim.set_hp_within(u, hp);
        s.replace(c, set);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_flying", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let on = matches!(s.get(1), Value::Boolean(true));
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        sh.sim.set_flying(u, on);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "morph", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let kind = match s.get(1) {
            Value::Integer(n) => u32::try_from(n).ok(),
            other => {
                let name = text(c, other, 1)?;
                sh.borrow().sim.catalog().unit_index(&name).map(|k| k as u32)
            }
        };
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let done = kind.map_or(false, |k| sh.sim.morph(u, k));
        s.replace(c, done);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_mana", [sh], |c, s| {
        let (unit, mana) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let mana = amount_arg(mana).map_err(|m| fail(c, m))?;
        let set = sh.sim.set_mana_within(u, mana);
        s.replace(c, set);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "channel_left", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.channel_left(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "ability_gate", [sh], |c, s| {
        let sh = sh.borrow();
        let ai = usize::try_from(a(c, &s, 0)?).map_err(|_| fail(c, "ability index out of range".into()))?;
        match sh.sim.ability_gate(ai) {
            Some((flags, filter, target)) => s.replace(c, (i64::from(flags), i64::from(filter), i64::from(target))),
            None => s.replace(c, Value::Nil),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cast_in_range", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let ai = usize::try_from(a(c, &s, 1)?).map_err(|_| fail(c, "ability index out of range".into()))?;
        let (x, y) = (fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 3)?).map_err(|m| fail(c, m))?);
        s.replace(c, sh.sim.cast_in_range(u, ai, x, y));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "visible_to", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let t = known_unit(&sh.sim, a(c, &s, 1)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.visible(sh.sim.player(u), t));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "meets_filter", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let mask = u32::try_from(a(c, &s, 1)?).map_err(|_| fail(c, "filter mask out of range".into()))?;
        s.replace(c, sh.sim.meets_filter(u, mask));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "spawn_image", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let caster = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let (life, out, inc) = (
            fixed(a(c, &s, 3)?).map_err(|m| fail(c, m))?,
            fixed(a(c, &s, 4)?).map_err(|m| fail(c, m))?,
            fixed(a(c, &s, 5)?).map_err(|m| fail(c, m))?,
        );
        let image = sh.sim.spawn_image(caster, x, y, life, out, inc);
        s.replace(c, image.map_or(-1, |j| j as i64));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_stat_bounds", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let code = usize::try_from(a(c, &s, 0)?).map_err(|_| fail(c, "stat code out of range".into()))?;
        let (lo, hi) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        s.replace(c, sh.sim.set_stat_bounds(code, lo, hi));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_armour_scale", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let scale = raw_q16(a(c, &s, 1)?).map_err(|m| fail(c, m))?;
        sh.sim.set_armour_scale(u, scale);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "armour_of", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.eff_armour_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_speed_bounds", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let (floor, cap) = (raw_q16(a(c, &s, 1)?).map_err(|m| fail(c, m))?, raw_q16(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        sh.sim.set_speed_bounds(u, floor, cap);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_speed", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.base_speed_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "type_speed", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.type_speed_milli(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_miss_chance", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let chance = raw_q16(a(c, &s, 1)?).map_err(|m| fail(c, m))?;
        sh.sim.set_miss_chance(u, chance);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_miss_chance", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.data_miss_chance(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_magic_keep", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let pct = i32::try_from(a(c, &s, 1)?).map_err(|_| fail(c, "magic keep out of range".into()))?;
        sh.sim.set_magic_keep(u, pct);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "magic_base", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.magic_base_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_damage_keep", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let pct = i32::try_from(a(c, &s, 1)?).map_err(|_| fail(c, "damage keep out of range".into()))?;
        sh.sim.set_damage_keep(u, pct);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_response_keep", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let pct = i32::try_from(a(c, &s, 1)?).map_err(|_| fail(c, "response keep out of range".into()))?;
        sh.sim.set_response_keep(u, pct);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "response_of", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.response_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "class_override", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.class_override_of(u).map_or(-1, i64::from));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "strike_of", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.strike_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "aggro_target", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.aggro_target(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "can_strike", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let o = known_unit(&sh.sim, a(c, &s, 1)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.can_strike(u, o));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "strike_lands", [sh], |c, s| {
        let sh = sh.borrow();
        let o = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.strike_lands(o));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_aggro", [sh], |c, s| {
        let hold = u32::try_from(a(c, &s, 2)?).unwrap_or(0);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let t = known_unit(&sh.sim, a(c, &s, 1)?).map_err(|m| fail(c, m))?;
        sh.sim.set_aggro(u, t, hold);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "order_of", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let (kind, x, y) = sh.sim.order_of(u);
        s.replace(c, (i64::from(kind), i64::from(x), i64::from(y)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "issue_order", [sh], |c, s| {
        let kind = a(c, &s, 1)?;
        let (x, y) = (fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 3)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let kind = u8::try_from(kind).map_err(|_| fail(c, format!("invalid_order: {kind}")))?;
        let ok = sh.sim.issue_order_kind(u, kind, x, y).is_ok();
        s.replace(c, ok);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "queue_order", [sh], |c, s| {
        let kind = a(c, &s, 1)?;
        let (x, y) = (fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 3)?).map_err(|m| fail(c, m))?);
        let target = a(c, &s, 4)?;
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let kind = u8::try_from(kind).map_err(|_| fail(c, format!("invalid_order: {kind}")))?;
        let target = i32::try_from(target).map_err(|_| fail(c, "invalid_target: out of i32 range".into()))?;
        let ok = sh.sim.queue_order(u, kind, x, y, target).is_ok();
        s.replace(c, ok);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "queue_clear", [sh], |c, s| {
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let ok = sh.sim.queue_clear(u);
        s.replace(c, ok);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "chase_get", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let (target, kind, x, y) = sh.sim.chase_get(u);
        s.replace(c, (i64::from(target), i64::from(kind), i64::from(x), i64::from(y)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "chase_set", [sh], |c, s| {
        let target = a(c, &s, 1)?;
        let kind = a(c, &s, 2)?;
        let (x, y) = (fixed(a(c, &s, 3)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 4)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let target = if target < 0 { -1 } else { known_unit(&sh.sim, target).map_err(|m| fail(c, m))? as i32 };
        let kind = u8::try_from(kind).map_err(|_| fail(c, format!("invalid_order: {kind}")))?;
        sh.sim.chase_set(u, target, kind, x, y);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "acquire_of", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.acquire_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "ticks", [sh], |c, s| {
        let sh = sh.borrow();
        s.replace(c, i64::from(sh.sim.ticks()));
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
    native!(ctx, game, "skill_rank", [sh], |c, s| {
        let (i, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let slot = usize::try_from(slot).ok().filter(|&v| v < ABILITY_SLOTS).ok_or_else(|| fail(c, format!("invalid_slot: {slot}")))?;
        s.replace(c, i64::from(sh.sim.ability_rank(u, slot)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "skill_gate", [sh], |c, s| {
        let (i, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let slot = usize::try_from(slot).ok().filter(|&v| v < ABILITY_SLOTS).ok_or_else(|| fail(c, format!("invalid_slot: {slot}")))?;
        match sh.sim.next_rank_gate(u, slot) {
            Some(gate) => s.replace(c, i64::from(gate)),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "remove_player", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::RemovePlayer { player: p });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "is_spawn_group_unit", [sh], |c, s| {
        let sh = sh.borrow();
        let unit = usize::try_from(a(c, &s, 0)?).ok().is_some_and(|u| sh.sim.is_spawn_group_unit(u));
        s.replace(c, unit);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "rule", [sh], |c, s| {
        let key = text(c, s.get(0), 0)?;
        match sh.borrow().sim.rule(&key) {
            Some(v) => s.replace(c, i64::from(v)),
            None => s.replace(c, Value::Nil),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "is_night", [sh], |c, s| {
        let night = sh.borrow().sim.is_night();
        s.replace(c, night);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_clock", [sh], |c, s| {
        let night = match s.get(0) {
            Value::Boolean(b) => b,
            other => return Err(fail(c, format!("invalid_clock: night is {}", other.type_name()))),
        };
        let mut cycle = [0u32; 3];
        for (k, slot) in cycle.iter_mut().enumerate() {
            let v = a(c, &s, k + 1)?;
            *slot = u32::try_from(v).map_err(|_| fail(c, format!("invalid_clock: value {k} is out of range")))?;
        }
        sh.borrow_mut().sim.set_clock(night, cycle);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "consume_item_charge", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let slot = a(c, &s, 1)?;
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let held = usize::try_from(slot).ok().filter(|&k| k < sh.sim.inventory_slots() && sh.sim.inventory_slot(u, k).0 != 0);
        match held {
            Some(k) => {
                sh.sim.consume_item_charge(u, k);
                s.replace(c, true);
            }
            None => s.replace(c, false),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "cmd_set_home", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let x = i32::try_from(a(c, &s, 1)?).map_err(|_| fail(c, "invalid_point: x out of range".into()))?;
        let y = i32::try_from(a(c, &s, 2)?).map_err(|_| fail(c, "invalid_point: y out of range".into()))?;
        let mut sh = sh.borrow_mut();
        let unit = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::SetHome { unit, x, y });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "acquire", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let value = a(c, &s, 1)?;
        let game_units = u32::try_from(value).map_err(|_| fail(c, "invalid_unit: acquire must not be negative".into()))?;
        sh.borrow_mut().sim.set_acquire(&name, game_units).map_err(|why| fail(c, format!("invalid_unit: {name}: {why}")))?;
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "alliance", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let q = player_arg(a(c, &s, 1)?).map_err(|m| fail(c, m))?;
        match s.get(2) {
            Value::Nil => {
                let rel = sh.borrow().sim.relation(p, q).map(str::to_owned);
                s.replace(c, rel.map_or(Value::Nil, |r| r.into_value(c)));
            }
            kind_value => {
                let kind = text(c, kind_value, 2)?;
                let kind = MOD_ALLIANCES.iter().position(|k| *k == kind.as_str()).ok_or_else(|| {
                    fail(c, format!("invalid_alliance: {kind} is not {}", MOD_ALLIANCES.join(", ")))
                })?;
                sh.borrow_mut().cmds.push(Cmd::SetAlliance { a: p, b: q, kind });
            }
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "add_player", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let controller = text(c, s.get(1), 1)?;
        let controller = controller_code(&controller).ok_or_else(|| {
            fail(c, format!("invalid_controller: {controller} is not {}", MOD_CONTROLLERS.join(", ")))
        })?;
        sh.borrow_mut().cmds.push(Cmd::AddPlayer { player: p, controller });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let spec = table_arg(c, s.get(1), 1)?;
        let int = |key: &str, default: i64| field_int(c, spec, key, default).and_then(fixed).map_err(|m| fail(c, m));
        let weapon = match spec.get(c, "weapon") {
            Value::Nil => None,
            Value::Table(t) => {
                let get = |key: &str, default: i64| field_int(c, t, key, default).and_then(fixed).map_err(|m| fail(c, m));
                Some((get("damage", 0)?, get("range", 0)?, get("period", 0)?))
            }
            _ => return Err(fail(c, "bad_field: weapon must be a table".into())),
        };
        let abilities = match spec.get(c, "abilities") {
            Value::Nil => Vec::new(),
            Value::Table(t) => {
                let mut out = Vec::new();
                for k in 1..=t.length() {
                    out.push(text(c, t.get(c, k), k as usize - 1)?);
                }
                out
            }
            _ => return Err(fail(c, "bad_field: abilities must be a list".into())),
        };
        let model = field_text_opt(c, spec, "model").map_err(|m| fail(c, m))?.unwrap_or_default();
        let filter = u32::try_from(int("filter", 0)?).map_err(|_| fail(c, "invalid_unit: filter must not be negative".into()))?;
        let def = UnitSpec {
            name: name.clone(),
            model,
            health: int("health", 0)?,
            speed: int("speed", 0)?,
            radius: int("radius", 0)?,
            armor: int("armor", 0)?,
            shield: int("shield", 0)?,
            sight: int("sight", 0)?,
            filter,
            cost: int("cost", 0)?,
            bounty: int("bounty", 0)?,
            weapon,
            abilities,
        };
        {
            let sh = sh.borrow();
            let cat = sh.sim.catalog();
            definition_room(cat.unit_index(&name).is_some(), cat.units.len(), "units").map_err(|m| fail(c, m))?;
        }
        sh.borrow_mut().sim.define_unit(&def).map_err(|why| fail(c, format!("invalid_unit: {name}: {why}")))?;
        s.replace(c, name);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "item", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let spec = table_arg(c, s.get(1), 1)?;
        let int = |key: &str| field_int(c, spec, key, 0).and_then(fixed).map_err(|m| fail(c, m));
        let def = ItemSpec {
            name: name.clone(),
            cost: int("cost")?,
            armor: int("armor")?,
            damage: int("damage")?,
            health: int("health")?,
            ability: field_text_opt(c, spec, "ability").map_err(|m| fail(c, m))?,
        };
        {
            let sh = sh.borrow();
            let cat = sh.sim.catalog();
            let known = cat.items.iter().any(|it| cat.name(it.name) == name);
            definition_room(known, cat.items.len(), "items").map_err(|m| fail(c, m))?;
        }
        let index = sh.borrow_mut().sim.define_item(&def).map_err(|why| fail(c, format!("invalid_item: {name}: {why}")))?;
        s.replace(c, index as i64);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "upgrade", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let spec = table_arg(c, s.get(1), 1)?;
        let mut sh = sh.borrow_mut();
        let int = |key: &str, default: i64| field_int(c, spec, key, default).and_then(fixed).map_err(|m| fail(c, m));
        let names = |key: &str| -> Result<Vec<String>, Error> {
            match spec.get(c, c.intern(key.as_bytes())) {
                Value::Nil => Ok(Vec::new()),
                Value::Table(t) => {
                    let mut out = Vec::new();
                    for k in 1..=t.length() {
                        out.push(text(c, t.get(c, k), k as usize - 1)?);
                    }
                    Ok(out)
                }
                _ => Err(fail(c, format!("bad_field: {key} must be a list"))),
            }
        };
        let entry = table_arg(c, spec.get(c, "effect"), 1)?;
        let field = |key: &str| field_int(c, entry, key, 0).and_then(fixed);
        let kind = field_text(c, entry, "kind").map_err(|m| fail(c, m))?;
        let txt = |key: &str| field_text(c, entry, key);
        let effect = shared::effect_from(&sh.sim, &kind, &field, &txt).map_err(|m| fail(c, m))?;
        let filter = u32::try_from(int("filter", 0)?).map_err(|_| fail(c, "invalid_upgrade: filter must not be negative".into()))?;
        let def = UpgradeSpec {
            name: name.clone(),
            building: field_text(c, spec, "building").map_err(|m| fail(c, m))?,
            time: int("time", 0)?,
            cost: int("cost", 0)?,
            gas: int("gas", 0)?,
            effect,
            filter,
            requires: names("requires")?,
            unlocks: names("unlocks")?,
        };
        {
            let cat = sh.sim.catalog();
            let known = cat.research.iter().any(|r| cat.name(r.name) == name);
            definition_room(known, cat.research.len(), "upgrades").map_err(|m| fail(c, m))?;
        }
        sh.sim.define_upgrade(&def).map_err(|why| fail(c, format!("invalid_upgrade: {name}: {why}")))?;
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
        {
            let cat = sh.sim.catalog();
            let known = cat.abilities.iter().any(|a| cat.name(a.name) == name);
            definition_room(known, cat.abilities.len(), "abilities").map_err(|m| fail(c, m))?;
        }
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
    native!(ctx, game, "cmd_add_gas", [sh], |c, s| {
        let (player, amount) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let player = player_arg(player).map_err(|m| fail(c, m))?;
        let amount = amount_arg(amount).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::AddGas { player, amount });
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
    native!(ctx, game, "effect", [sh], |c, s| {
        let kind = text(c, s.get(0), 0)?;
        let (x, y) = (num(c, s.get(1), 1)?, num(c, s.get(2), 2)?);
        let event = fx_event(c, &kind, x, y, s.get(3)).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::Fx(event));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "message", [sh], |c, s| {
        let (player, msg) = (a(c, &s, 0)?, text(c, s.get(1), 1)?);
        let seconds = num_opt(c, s.get(2), 2)?;
        let mut sh = sh.borrow_mut();
        if matches!(s.get(3), Value::Nil) {
            let body = shared::ui_message(player, &msg, seconds).map_err(|m| fail(c, m))?;
            sh.cmds.push(Cmd::Ui { body });
        } else {
            let sender = player_arg(a(c, &s, 3)?).map_err(|m| fail(c, m))?;
            for ally in seats::allies_of(&sh.sim, sender) {
                let body = shared::ui_message(i64::from(ally), &msg, seconds).map_err(|m| fail(c, m))?;
                sh.cmds.push(Cmd::Ui { body });
            }
        }
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
                let grid = matrix::grid_arg(c, t)?;
                Some((field_text_opt(c, t, "title").map_err(|m| fail(c, m))?, field_text_opt(c, t, "text").map_err(|m| fail(c, m))?, player, grid))
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
    native!(ctx, game, "camera_distance", [sh], |c, s| {
        let player = a(c, &s, 0)?;
        let (zoom, seconds) = (num(c, s.get(1), 1)?, num_opt(c, s.get(2), 2)?.unwrap_or(0.0));
        if !zoom.is_finite() || zoom <= 0.0 || !seconds.is_finite() || seconds < 0.0 {
            return Err(fail(c, "invalid_camera: zoom must be positive and seconds not negative".to_string()));
        }
        let player = shared::ui_player(player).map_err(|m| fail(c, m))?;
        let body = format!(r#""kind":"camera_distance","player":{player},"zoom":{zoom},"seconds":{seconds}"#);
        sh.borrow_mut().cmds.push(Cmd::Ui { body });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "camera_lock", [sh], |c, s| {
        let player = shared::ui_player(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let locked = s.get(1).to_bool();
        let body = format!(r#""kind":"camera_lock","player":{player},"locked":{locked}"#);
        sh.borrow_mut().cmds.push(Cmd::Ui { body });
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "player_controller", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let code = sh.borrow().sim.controller(player);
        s.replace(c, MOD_CONTROLLERS.get(usize::from(code)).copied().unwrap_or("open").to_owned());
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "is_computer", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let code = sh.borrow().sim.controller(player);
        s.replace(c, code == sim::CONTROLLER_COMPUTER);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "props", [sh], |c, s| {
        let wanted = match s.get(0) {
            Value::Nil => None,
            other => Some(text(c, other, 0)?),
        };
        let sh = sh.borrow();
        let kinds = &sh.sim.catalog().names.prop_kinds;
        let list = Table::new(&c);
        let mut n = 0i64;
        for d in sh.sim.prop_list() {
            let Some(name) = kinds.get(usize::from(d.kind)) else { continue };
            if wanted.as_ref().is_some_and(|w| w != name) {
                continue;
            }
            let row = Table::new(&c);
            row.put(c, "id", i64::from(d.id));
            row.put(c, "kind", name.clone());
            row.put(c, "x", i64::from(d.x) * 65_536 / 1000);
            row.put(c, "y", i64::from(d.y) * 65_536 / 1000);
            n += 1;
            list.put(c, n, row);
        }
        s.replace(c, list);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "ui_event", [sh], |c, s| {
        let kind = text(c, s.get(0), 0)?;
        let payload = text(c, s.get(1), 1)?;
        let body = shared::ui_event(&kind, &payload).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::Ui { body });
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
    native!(ctx, game, "unit_info", [sh], |c, s| {
        let sh = sh.borrow();
        let cat = sh.sim.catalog();
        let index = match s.get(0) {
            Value::Integer(n) => usize::try_from(n).ok().filter(|&k| k < cat.units.len()),
            other => cat.unit_index(&text(c, other, 0)?),
        };
        match index {
            Some(k) => s.replace(c, unit_row(c, cat, k)),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "capacity_limits", [sh], |c, s| {
        let e = sh.borrow().sim.catalog().econ;
        let t = Table::new(&c);
        t.put(c, "base", i64::from(e.capacity_base));
        t.put(c, "max", i64::from(e.capacity_max));
        s.replace(c, t);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "resources", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let sh = sh.borrow();
        s.replace(c, (i64::from(sh.sim.resource_a(player)), i64::from(sh.sim.resource_b(player))));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "add_resources", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let gold = amount_arg(a(c, &s, 1)?).map_err(|m| fail(c, m))?;
        let gas = amount_arg(a(c, &s, 2)?).map_err(|m| fail(c, m))?;
        sh.borrow_mut().sim.add_resources(player, gold, gas);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "hide_unit", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let took = sh.sim.remove_unit(u);
        s.replace(c, took);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unhide_unit", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let back = sh.sim.revive_unit(u, x, y);
        s.replace(c, back);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "revive_at", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let (x, y) = (fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 2)?).map_err(|m| fail(c, m))?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let back = !sh.sim.alive(u) && sh.sim.is_attributed(u) && sh.sim.revive_at_point(u, x, y);
        s.replace(c, back);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "walkable", [sh], |c, s| {
        let (x, y) = (fixed(a(c, &s, 0)?).map_err(|m| fail(c, m))?, fixed(a(c, &s, 1)?).map_err(|m| fail(c, m))?);
        let sh = sh.borrow();
        s.replace(c, sh.sim.grid().walkable(x >> 16, y >> 16));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "research_info", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let sh = sh.borrow();
        let cat = sh.sim.catalog();
        match cat.research.iter().position(|r| cat.name(r.name) == name) {
            Some(k) => s.replace(c, research_row(c, cat, k)),
            None => s.replace(c, ()),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "research_bit", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let name = text(c, s.get(1), 1)?;
        let sh = sh.borrow();
        let held = match sh.sim.research_index(&name) {
            Some(r) => sh.sim.research_bit(p, r),
            None => false,
        };
        s.replace(c, held);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_research_bit", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let name = text(c, s.get(1), 1)?;
        let on = matches!(s.get(2), Value::Boolean(true));
        let mut sh = sh.borrow_mut();
        let took = match sh.sim.research_index(&name) {
            Some(r) => sh.sim.set_research_bit(p, r, on),
            None => false,
        };
        s.replace(c, took);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "prereqs", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let kind = text(c, s.get(1), 1)?;
        let target_kind: u8 = match kind.as_str() {
            "unit" => 0,
            "research" => 1,
            other => return Err(fail(c, format!("bad_argument: kind {other} is not unit or research"))),
        };
        let sh = sh.borrow();
        let cat = sh.sim.catalog();
        let target = if target_kind == 0 { cat.unit_index(&name) } else { cat.research.iter().position(|r| cat.name(r.name) == name) };
        let out = Table::new(&c);
        let mut n = 0i64;
        if let Some(t) = target {
            for q in cat.prereqs.iter().filter(|q| q.target_kind == target_kind && q.target as usize == t) {
                let req = if q.requirement_kind == 0 {
                    cat.units.get(q.requirement as usize).map(|u| ("unit", cat.name(u.name)))
                } else {
                    cat.research.get(q.requirement as usize).map(|r| ("research", cat.name(r.name)))
                };
                if let Some((req_kind, req_name)) = req {
                    n += 1;
                    let row = Table::new(&c);
                    row.put(c, "kind", req_kind.to_owned());
                    row.put(c, "name", req_name.to_owned());
                    out.put(c, n, row);
                }
            }
        }
        s.replace(c, out);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "apply_research", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let unit = a(c, &s, 1)?;
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let found = sh.sim.catalog().research.iter().position(|r| sh.sim.catalog().name(r.name) == name);
        let took = match found {
            Some(r) => sh.sim.apply_research(r, u),
            None => false,
        };
        s.replace(c, took);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "players", [sh], |c, s| {
        let counts = living_per_player(&sh.borrow().sim);
        let list = Table::new(&c);
        for (p, alive) in counts.iter().enumerate() {
            let row = Table::new(&c);
            row.put(c, "id", p as i64);
            row.put(c, "units", *alive as i64);
            list.put(c, p as i64 + 1, row);
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
    native!(ctx, game, "unit_class", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        s.replace(c, sh.sim.class_name(u).to_owned());
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_radius", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.radius_fixed(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_harvest", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.harvest_per_trip(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_amount", [sh], |c, s| {
        let i = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.resource_left(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "take_amount", [sh], |c, s| {
        let (i, want) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let want = amount_arg(want).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.take_resource(u, want)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "econ", [sh], |c, s| {
        let key = text(c, s.get(0), 0)?;
        match sh.borrow().sim.econ_value(&key) {
            Some(v) => s.replace(c, v),
            None => s.replace(c, Value::Nil),
        }
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
    native!(ctx, game, "start_cooldown", [sh], |c, s| {
        let (unit, slot, ticks) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let mut sh = sh.borrow_mut();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ABILITY_SLOTS).map_err(|m| fail(c, m))?;
        let ticks = amount_arg(ticks).map_err(|m| fail(c, m))?;
        sh.sim.set_ability_cooldown(u, slot, ticks.max(0));
        s.replace(c, true);
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
            row.put(c, "slot", slot as i64);
            row.put(c, "name", cat.name(ability.name).to_owned());
            row.put(c, "cooldown", i64::from(sh.sim.ability_cooldown(u, slot)));
            n += 1;
            list.put(c, n, row);
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
            row.put(c, "slot", slot as i64);
            row.put(c, "item", i64::from(kind) - 1);
            row.put(c, "name", cat.name(item.name).to_owned());
            row.put(c, "count", i64::from(count));
            row.put(c, "cooldown", i64::from(sh.sim.item_cooldown(u, slot)));
            if let Some(owner) = sh.sim.item_owner(u, slot) {
                row.put(c, "owner", owner as i64);
            }
            n += 1;
            list.put(c, n, row);
        }
        s.replace(c, list);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_buffs", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let list = Table::new(&c);
        for (n, (kind, left)) in sh.sim.buffs(u).enumerate() {
            let row = Table::new(&c);
            row.set(c, "kind", i64::from(kind))?;
            row.set(c, "left", i64::from(left))?;
            list.set(c, n as i64 + 1, row)?;
        }
        s.replace(c, list);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "sim_generation", [sh], |c, s| {
        s.replace(c, sh.borrow().generation);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_item_gen", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.item_generation(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "skill_info", [sh], |c, s| {
        let (i, slot) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, i).map_err(|m| fail(c, m))?;
        let slot = slot_of(slot, ABILITY_SLOTS).map_err(|m| fail(c, m))?;
        match sh.sim.rank_tree(u, slot) {
            Some(k) => {
                let t = Table::new(&c);
                t.put(c, "max_rank", i64::from(k.max_rank));
                let per_rank = |v: &[i32]| list_of(c, v.iter().take(usize::from(k.max_rank)).map(|&n| i64::from(n)));
                t.put(c, "gates", per_rank(&k.gates)?);
                t.put(c, "damage", per_rank(&k.damage)?);
                t.put(c, "cooldowns", per_rank(&k.cooldown)?);
                t.put(c, "energies", per_rank(&k.energy)?);
                t.put(c, "ultimate", k.ultimate);
                s.replace(c, t);
            }
            None => s.replace(c, Value::Nil),
        }
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "mana", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.mana_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "shield_of", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.shield_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_ability_cooldown", [sh], |c, s| {
        let (unit, slot, ticks) = (a(c, &s, 0)?, a(c, &s, 1)?, a(c, &s, 2)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let slot = usize::try_from(slot).map_err(|_| fail(c, format!("invalid_slot: {slot}")))?;
        let ticks = i32::try_from(ticks.clamp(0, i64::from(i32::MAX))).unwrap_or(0);
        sh.sim.set_ability_cooldown(u, slot, ticks);
        s.replace(c, true);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "raise_max_hp", [sh], |c, s| {
        let (unit, whole) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let floor = i32::try_from(whole.clamp(0, 30_000)).unwrap_or(0).saturating_mul(sim::fixed::ONE);
        let (hp, max) = (sh.sim.hp(u), sh.sim.hp_max(u));
        let raised = floor > max;
        if raised {
            sh.sim.set_health(u, hp + (floor - max), floor);
        }
        s.replace(c, raised);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_base_period", [sh], |c, s| {
        let (unit, ticks) = (a(c, &s, 0)?, a(c, &s, 1)?);
        let mut sh = sh.borrow_mut();
        let u = live_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        let ticks = i32::try_from(ticks).map_err(|_| fail(c, "invalid_period: out of i32 range".into()))?;
        let before = sh.sim.period_of(u);
        sh.sim.set_base_period(u, ticks);
        s.replace(c, i64::from(before));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "max_mana", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.unit_max_mana(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "max_hp", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.hp_max(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "kills_of", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.kills_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "deaths_of", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.deaths_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "credits_of", [sh], |c, s| {
        let sh = sh.borrow();
        let u = known_unit(&sh.sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        s.replace(c, i64::from(sh.sim.credits_of(u)));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_kinds", [sh], |c, s| {
        let sh = sh.borrow();
        let cat = sh.sim.catalog();
        let out = Table::new(&c);
        for (k, unit) in cat.units.iter().enumerate() {
            let row = Table::new(&c);
            row.put(c, "name", cat.name(unit.name).to_owned());
            row.put(c, "player_controlled", cat.unit_ext.get(k).is_some_and(|e| e.player_controlled));
            row.put(c, "melee", cat.unit_ext.get(k).is_some_and(|e| e.melee));
            if let Some(d) = sh.sim.kind_attr_def(k) {
                row.put(c, "primary", i64::from(d.primary));
            }
            row.put(c, "model", cat.name(unit.model).to_owned());
            out.put(c, k as i64 + 1, row);
        }
        s.replace(c, out);
        Ok(CallbackReturn::Return)
    });
    for name in ["cmd_add_gold", "cmd_add_stat", "cmd_add_unit_gold", "cmd_add_xp", "cmd_set_respawn"] {
        env.put(ctx, name, game.get(ctx, name));
    }
    #[cfg(test)]
    native!(ctx, env, "fault_panic", [], |c, s| {
        let _ = (c, &s);
        panic!("fault injected by the containment test")
    });
    env.put(ctx, "game", game);
}

fn living_per_player(sim: &Sim) -> Vec<usize> {
    let mut counts = vec![0usize; MAX_PLAYERS];
    for i in (0..sim.count()).filter(|&i| sim.alive(i)) {
        counts[sim.player(i) as usize % MAX_PLAYERS] += 1;
    }
    counts
}

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
        row.put(ctx, "id", i as i64);
        for name in &wanted {
            match name.as_str() {
                "position" => {
                    let (x, y) = sim.position_fixed(i);
                    let p = Table::new(&ctx);
                    p.put(ctx, "x", x as i64);
                    p.put(ctx, "y", y as i64);
                    row.put(ctx, "position", p);
                }
                "owner" => {
                    row.put(ctx, "owner", sim.player(i) as i64);
                }
                _ => {
                    row.put(ctx, "hp", sim.hp(i) as i64);
                }
            }
        }
        n += 1;
        out.put(ctx, n, row);
    }
    Ok(out)
}

fn mtime(path: &Path) -> Result<SystemTime, String> {
    std::fs::metadata(path).and_then(|m| m.modified()).map_err(|e| format!("stat {}: {e}", path.display()))
}

fn mod_name(path: &Path) -> String {
    path.file_stem().map_or_else(|| "mod".to_string(), |s| s.to_string_lossy().into_owned())
}

fn requires_of(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let code = line.split("--").next().unwrap_or("");
        let mut rest = code;
        while let Some(at) = rest.find("require") {
            let after_name = &rest[at + "require".len()..];
            let word_start = !rest[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.');
            let arg = after_name.trim_start();
            let arg = arg.strip_prefix('(').unwrap_or(arg).trim_start();
            if word_start {
                if let Some(q) = arg.chars().next().filter(|c| *c == '"' || *c == '\'') {
                    if let Some(end) = arg[1..].find(q) {
                        out.push(arg[1..1 + end].to_owned());
                    }
                }
            }
            rest = after_name;
        }
    }
    out
}

fn plan_loads(names: &[String], mods: &[(String, String)]) -> (Vec<usize>, Vec<Option<String>>, Vec<Vec<usize>>) {
    let n = names.len();
    let mut failed: Vec<Option<String>> = vec![None; n];
    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        if names[..i].contains(&names[i]) {
            failed[i] = Some(format!("duplicate module {}", names[i]));
        }
        for req in requires_of(&mods[i].1) {
            match names.iter().position(|m| *m == req) {
                Some(d) if !deps[i].contains(&d) => deps[i].push(d),
                Some(_) => {}
                None => {
                    failed[i].get_or_insert_with(|| format!("unknown_module: {req} is not a module of this pack"));
                }
            }
        }
    }
    let mut state = vec![0u8; n];
    let mut order = Vec::with_capacity(n);
    for i in 0..n {
        if state[i] == 0 {
            visit_load(i, &deps, names, &mut state, &mut Vec::new(), &mut failed, &mut order);
        }
    }
    (order, failed, deps)
}

fn visit_load(i: usize, deps: &[Vec<usize>], names: &[String], state: &mut [u8], stack: &mut Vec<usize>, failed: &mut [Option<String>], order: &mut Vec<usize>) {
    state[i] = 1;
    stack.push(i);
    for &d in &deps[i] {
        match state[d] {
            0 => visit_load(d, deps, names, state, stack, failed, order),
            1 => {
                let at = stack.iter().position(|&x| x == d).unwrap_or(0);
                let cycle: Vec<&str> = stack[at..].iter().map(|&x| names[x].as_str()).chain([names[d].as_str()]).collect();
                let msg = format!("require_cycle: {}", cycle.join(" -> "));
                for &x in &stack[at..] {
                    failed[x].get_or_insert_with(|| msg.clone());
                }
            }
            _ => {}
        }
    }
    stack.pop();
    state[i] = 2;
    order.push(i);
}

struct Module {
    name: String,
    hooks: Hooks,
    handlers: StashedTable,
    exports: Option<StashedTable>,
    env: StashedTable,
    off: Vec<String>,
    #[cfg(feature = "fault-injection")]
    fault_tick: Option<u32>,
}

type Peers = Rc<RefCell<Vec<StashedTable>>>;

type Registry = Rc<RefCell<std::collections::HashMap<String, Option<StashedTable>>>>;

pub struct ModHost {
    lua: RefCell<Lua>,
    sh: Sh,
    state: StashedTable,
    modules: Vec<Module>,
    registry: Registry,
    peers: Peers,
    version: u64,
    joined: Vec<usize>,
    left: Vec<usize>,
    snap: Vec<UnitSnap>,
    prev: Vec<UnitSnap>,
    inside: Vec<u64>,
    pending: Vec<Pending>,
    mod_file: Option<(PathBuf, SystemTime)>,
    hook_errors: Vec<String>,
}

impl ModHost {
    pub fn new(image: &[u8]) -> Result<Self, LoadError> {
        let sim = sim::load(image, MAX_UNITS)?;
        let mut lua = Lua::core();
        let state = lua.enter(|ctx| {
            let t = Table::new(&ctx);
            t.put(ctx, "ticks", 0i64);
            ctx.stash(t)
        });
        Ok(Self {
            lua: RefCell::new(lua),
            sh: Rc::new(RefCell::new(Shared {
                sim,
                regions: Vec::new(),
                local: Vec::new(),
                cmds: Vec::new(),
                counter_names: Default::default(),
                clock: 0,
                fired: 0,
                applied: 0,
                ui: Vec::new(),
                generation: 0,
                seats: SeatState::default(),
            })),
            state,
            modules: Vec::new(),
            registry: Rc::new(RefCell::new(std::collections::HashMap::new())),
            peers: Rc::new(RefCell::new(Vec::new())),
            version: 0,
            joined: Vec::new(),
            left: Vec::new(),
            snap: Vec::new(),
            prev: Vec::new(),
            inside: vec![0; MAX_UNITS],
            pending: Vec::new(),
            mod_file: None,
            hook_errors: Vec::new(),
        })
    }

    pub fn load_source(&mut self, name: &str, src: &str) -> Result<(), String> {
        self.load_mods(&[(name.to_owned(), src.to_owned())])
    }

    pub fn load_mods(&mut self, mods: &[(String, String)]) -> Result<(), String> {
        let names: Vec<String> = mods.iter().map(|(file, _)| mod_name(Path::new(file))).collect();
        let (order, mut failed, deps) = plan_loads(&names, mods);
        let mut old: Vec<Module> = std::mem::take(&mut self.modules);
        self.registry.borrow_mut().clear();
        self.peers.borrow_mut().clear();
        let mut built: Vec<Option<Module>> = (0..mods.len()).map(|_| None).collect();
        let mut errors = Vec::new();
        for &i in &order {
            let outcome = match failed[i].take() {
                Some(e) => Err(e),
                None => match deps[i].iter().find(|&&d| !self.registry.borrow().contains_key(&names[d])) {
                    Some(&d) => Err(format!("requires {}, which did not load", names[d])),
                    None => self.build_guarded(&names[i], &mods[i].1),
                },
            };
            match outcome {
                Ok(module) => {
                    self.registry.borrow_mut().insert(names[i].clone(), module.exports.clone());
                    self.peers.borrow_mut().push(module.env.clone());
                    built[i] = Some(module);
                }
                Err(e) => {
                    println!("ev=mod_load_failed module={} error={:?}", names[i], e);
                    errors.push(format!("{}: {e}", names[i]));
                    if let Some(p) = old.iter().position(|m| m.name == names[i]) {
                        let kept = old.swap_remove(p);
                        self.registry.borrow_mut().insert(names[i].clone(), kept.exports.clone());
                        self.peers.borrow_mut().push(kept.env.clone());
                        built[i] = Some(kept);
                    }
                }
            }
        }
        self.modules = built.into_iter().flatten().collect();
        self.version += 1;
        if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
    }

    fn build_guarded(&mut self, name: &str, src: &str) -> Result<Module, String> {
        let (sim, regions, cmds, local) = {
            let sh = self.sh.borrow();
            (sh.sim.save_state(), sh.regions.clone(), sh.cmds.clone(), sh.local.clone())
        };
        match guarded("load", || self.build_module(name, src)) {
            Ok(m) => Ok(m),
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

    pub fn set_sim(&mut self, sim: Sim) {
        self.sh.borrow_mut().sim = sim;
        self.sim_replaced();
    }

    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.sh.borrow_mut().sim.load_state(bytes).map_err(|e| format!("load_state: {e:?}"))?;
        self.sim_replaced();
        Ok(())
    }

    fn sim_replaced(&mut self) {
        {
            let mut sh = self.sh.borrow_mut();
            sh.cmds.clear();
            sh.local.clear();
            sh.counter_names.clear();
            sh.generation += 1;
        }
        if let Err(e) = self.emit("on_state_replaced", vec![Arg::State]) {
            println!("ev=on_state_replaced_failed error={e:?}");
        }
    }

    pub fn disabled_hooks(&self) -> Vec<String> {
        self.modules.iter().flat_map(|m| m.off.iter().map(move |h| format!("{}:{}", m.name, h))).collect()
    }

    pub fn hook_errors(&self) -> &[String] {
        &self.hook_errors
    }

    pub fn dispatch_action(&mut self, name: &str, args: &serde_json::Value) -> Result<(), String> {
        let seat = args.get("player").and_then(serde_json::Value::as_u64).and_then(|p| usize::try_from(p).ok());
        match (name, seat) {
            ("peer_left", Some(p)) => {
                self.player_left(p);
            }
            ("peer_back", Some(p)) => {
                self.player_joined(p);
            }
            _ => {}
        }
        for (i, f) in self.live_hooks("on_action", |h| h.on_action.clone()) {
            let values = vec![Arg::Text(name.to_owned()), Arg::Json(args.clone()), Arg::State];
            if let Err(e) = self.call(&f, values) {
                self.hook_failed(i, "on_action", &e);
            }
        }
        Ok(())
    }

    pub fn pointer(&mut self, kind: &str, x: f64, y: f64, button: i64) -> Result<(), String> {
        for (i, f) in self.live_hooks("on_pointer", |h| h.on_pointer.clone()) {
            let values = vec![Arg::Text(kind.to_owned()), Arg::Num(x), Arg::Num(y), Arg::Int(button), Arg::State];
            if let Err(e) = self.call(&f, values) {
                self.hook_failed(i, "on_pointer", &e);
            }
        }
        Ok(())
    }

    fn live_hooks(&self, key: &str, pick: impl Fn(&Hooks) -> Option<StashedFunction>) -> Vec<(usize, StashedFunction)> {
        self.modules
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.off.iter().any(|o| o == key))
            .filter_map(|(i, m)| pick(&m.hooks).map(|f| (i, f)))
            .collect()
    }

    fn is_live(&self, i: usize, key: &str) -> bool {
        self.modules.get(i).is_some_and(|m| !m.off.iter().any(|o| o == key))
    }

    fn hook_failed(&mut self, i: usize, hook: &str, error: &str) {
        let Some(m) = self.modules.get_mut(i) else { return };
        println!("ev=mod_hook_failed module={} hook={} error={:?}", m.name, hook, error);
        m.off.push(hook.to_owned());
        self.hook_errors.push(format!("{}:{}: {}", m.name, hook, error));
        if error.starts_with("memory_limit") {
            println!("ev=mod_retired module={} reason=memory_limit", m.name);
            if let Err(e) = guarded("retire", || {
                self.retire(i);
                Ok(())
            }) {
                println!("ev=mod_retire_failed module={i} error={e:?}");
            }
        }
    }

    fn new_env(&mut self) -> Result<(Hooks, StashedTable), String> {
        let sh = self.sh.clone();
        let registry = self.registry.clone();
        let peers = self.peers.clone();
        let (prelude, env) = self
            .lua.borrow_mut()
            .try_enter(|ctx| {
                let env = Table::new(&ctx);
                for base in ["pcall", "error", "assert", "type", "tostring", "tonumber", "select", "pairs", "ipairs", "next"] {
                    env.set(ctx, base, ctx.globals().get(ctx, base))?;
                }
                let deterministic_math = Table::new(&ctx);
                if let Value::Table(stdlib_math) = ctx.globals().get(ctx, "math") {
                    let mut key = Value::Nil;
                    while let piccolo::table::NextValue::Found { key: k, value } = stdlib_math.next(key) {
                        let seeded = matches!(k, Value::String(s) if s.as_bytes() == b"random" || s.as_bytes() == b"randomseed");
                        if !seeded {
                            deterministic_math.set(ctx, k, value)?;
                        }
                        key = k;
                    }
                }
                env.set(ctx, "math", deterministic_math)?;
                bind(ctx, env, &sh);
                bind_lua_libs(ctx, env, &sh)?;
                let fallback = Callback::from_fn(&ctx, move |c, _, mut s| {
                    guard_native(c, || {
                        let key = s.get(1);
                        let mut found = Value::Nil;
                        for peer in peers.borrow().iter() {
                            found = c.fetch(peer).get(c, key);
                            if !found.is_nil() {
                                break;
                            }
                        }
                        s.replace(c, found);
                        Ok(CallbackReturn::Return)
                    })
                });
                let meta = Table::new(&ctx);
                meta.set(ctx, "__index", fallback)?;
                env.set_metatable(&ctx, Some(meta));
                let require = Callback::from_fn(&ctx, move |c, _, mut s| {
                    guard_native(c, || {
                        let name = text(c, s.get(0), 0)?;
                        let found = registry.borrow().get(&name).cloned();
                        let out = match found {
                            Some(Some(t)) => Value::Table(c.fetch(&t)),
                            Some(None) => Value::Boolean(true),
                            None => return Err(fail(c, format!("unknown_module: {name} is not a module of this pack, or did not load"))),
                        };
                        s.replace(c, out);
                        Ok(CallbackReturn::Return)
                    })
                });
                env.set(ctx, "require", require)?;
                let known = Table::new(&ctx);
                for (event, _) in EVENTS {
                    known.set(ctx, event, true)?;
                }
                let rows_fn = {
                    let sh = sh.clone();
                    Callback::from_fn(&ctx, move |c, _, mut s| {
                        guard_native(c, || {
                            let comps = match s.get(0) {
                                Value::Table(t) => t,
                                _ => return Err(fail(c, "bad_argument: comps must be a list".into())),
                            };
                            let out = rows(c, &sh.borrow().sim, comps)?;
                            s.replace(c, out);
                            Ok(CallbackReturn::Return)
                        })
                    })
                };
                let note = {
                    let sh = sh.clone();
                    Callback::from_fn(&ctx, move |c, _, _| {
                        guard_native(c, || {
                            sh.borrow_mut().fired += 1;
                            Ok(CallbackReturn::Return)
                        })
                    })
                };
                let clock = {
                    let sh = sh.clone();
                    Callback::from_fn(&ctx, move |c, _, mut s| {
                        guard_native(c, || {
                            s.replace(c, sh.borrow().clock as i64);
                            Ok(CallbackReturn::Return)
                        })
                    })
                };
                let closure = Closure::load(ctx, Some("host-prelude"), prelude().as_bytes())?;
                let exec = Executor::start(ctx, closure.into(), (env, known, rows_fn, note, clock, MAX_PLAYERS as i64));
                Ok((ctx.stash(exec), ctx.stash(env)))
            })
            .map_err(|e| e.to_string())?;
        self.finish(&prelude)?;
        let hooks = self
            .lua.borrow_mut()
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
                    on_cast_request: None,
                    on_proc: None,
                    on_hit: None,
                    on_march: None,
                    on_check_win: None,
                    on_request: None,
                    on_skill_up: None,
                    on_item_use: None,
                    on_action: None,
                    on_pointer: None,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok((hooks, env))
    }

    fn run_chunk(&mut self, env: &StashedTable, name: &str, src: &str) -> Result<Option<StashedTable>, String> {
        let exec = self
            .lua.borrow_mut()
            .try_enter(|ctx| {
                let closure = Closure::load_with_env(ctx, Some(name), src.as_bytes(), ctx.fetch(env))?;
                Ok(ctx.stash(Executor::start(ctx, closure.into(), ())))
            })
            .map_err(|e| e.to_string())?;
        self.finish(&exec)?;
        self.lua.borrow_mut()
            .try_enter(|ctx| {
                let (first,) = ctx.fetch(&exec).take_result::<(Value,)>(ctx)??;
                Ok(match first {
                    Value::Table(t) => Some(ctx.stash(t)),
                    _ => None,
                })
            })
            .map_err(|e| e.to_string())
    }

    fn name_hooks(&self, mut hooks: Hooks, handlers: &StashedTable) -> Hooks {
        let named = |key: &str| {
            self.lua.borrow_mut().enter(|ctx| match ctx.fetch(handlers).get(ctx, ctx.intern(key.as_bytes())) {
                Value::Function(f) => Some(ctx.stash(f)),
                _ => None,
            })
        };
        hooks.on_unit_died = named("on_unit_died");
        hooks.on_cast = named("on_cast");
        hooks.on_cast_request = named("on_cast_request");
        hooks.on_proc = named("on_proc");
        hooks.on_hit = named("on_hit");
        hooks.on_march = named("on_march");
        hooks.on_check_win = named("on_check_win");
        hooks.on_request = named("on_request");
        hooks.on_skill_up = named("on_skill_up");
        hooks.on_item_use = named("on_item_use");
        hooks.on_action = named("on_action");
        hooks.on_pointer = named("on_pointer");
        hooks
    }

    fn build_module(&mut self, name: &str, src: &str) -> Result<Module, String> {
        let (hooks, env) = self.new_env()?;
        let exports = self.run_chunk(&env, name, src)?;
        let handlers = exports.clone().unwrap_or_else(|| env.clone());
        let hooks = self.name_hooks(hooks, &handlers);
        Ok(Module {
            name: name.to_owned(),
            hooks,
            handlers,
            exports,
            env,
            off: Vec::new(),
            #[cfg(feature = "fault-injection")]
            fault_tick: fault_tick(src),
        })
    }

    pub fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let seen = mtime(path)?;
        let src = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        self.load_source(&mod_name(path), &src)?;
        self.mod_file = Some((path.to_path_buf(), seen));
        Ok(())
    }

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

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn sim(&self) -> std::cell::Ref<'_, Sim> {
        std::cell::Ref::map(self.sh.borrow(), |s| &s.sim)
    }

    pub fn sim_mut(&self) -> std::cell::RefMut<'_, Sim> {
        std::cell::RefMut::map(self.sh.borrow_mut(), |s| &mut s.sim)
    }

    pub fn counters(&self) -> (u64, u64) {
        let sh = self.sh.borrow();
        (sh.fired, sh.applied)
    }

    pub fn cmd_spawn(&mut self, kind: u32, x: i32, y: i32) -> Option<usize> {
        self.sim_mut().spawn(kind, x, y)
    }

    pub fn fire_trigger(&mut self, unit: usize, name: &str) -> bool {
        self.sim_mut().fire_trigger(unit, name)
    }

    pub fn events(&self) -> &'static [(&'static str, &'static str)] {
        &EVENTS
    }

    pub fn player_joined(&mut self, player: usize) -> bool {
        if player >= MAX_PLAYERS {
            return false;
        }
        self.joined.push(player);
        true
    }

    pub fn player_left(&mut self, player: usize) -> bool {
        if player >= MAX_PLAYERS {
            return false;
        }
        self.left.push(player);
        true
    }

    pub fn flush_ui(&mut self) {
        let mut guard = self.sh.borrow_mut();
        let sh = &mut *guard;
        let (shown, kept): (Vec<Cmd>, Vec<Cmd>) = std::mem::take(&mut sh.cmds).into_iter().partition(|c| matches!(c, Cmd::Ui { .. }));
        sh.cmds = kept;
        for cmd in shown {
            if let Cmd::Ui { body } = cmd {
                sh.ui.push(body);
            }
        }
    }

    pub fn take_ui(&mut self) -> Vec<String> {
        std::mem::take(&mut self.sh.borrow_mut().ui)
    }

    pub fn state_line(&mut self) -> String {
        let mut fields: Vec<String> = self.lua.borrow_mut().enter(|ctx| {
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

    pub fn state_int(&self, key: &str) -> Option<i64> {
        self.lua.borrow_mut().enter(|ctx| match ctx.fetch(&self.state).get(ctx, ctx.intern(key.as_bytes())) {
            Value::Integer(v) => Some(v),
            _ => None,
        })
    }

    pub fn state_bool(&self, key: &str) -> Option<bool> {
        self.lua.borrow_mut().enter(|ctx| match ctx.fetch(&self.state).get(ctx, ctx.intern(key.as_bytes())) {
            Value::Boolean(v) => Some(v),
            _ => None,
        })
    }

    pub fn feed_request(&mut self, req: &ItemRequest) {
        self.settle(vec![*req]);
    }

    pub fn units(&self, player: Option<u8>) -> Vec<usize> {
        let sh = self.sh.borrow();
        (0..sh.sim.count()).filter(|&i| sh.sim.alive(i) && player.map_or(true, |p| sh.sim.player(i) == p)).collect()
    }

    pub fn unit_kind(&self, unit: usize) -> Option<String> {
        self.sh.borrow().sim.kind_name(unit).map(str::to_owned)
    }

    pub fn players(&self) -> Vec<usize> {
        living_per_player(&self.sh.borrow().sim)
    }

    pub fn advance(&mut self) -> Result<(), String> {
        {
            let mut sh = self.sh.borrow_mut();
            sh.clock += 1;
            sh.sim.tick();
        }
        let tick = self.sh.borrow().sim.ticks();
        self.lua.borrow_mut().enter(|ctx| {
            ctx.fetch(&self.state).put(ctx, "ticks", i64::from(tick));
        });
        self.collect();
        self.dispatch()?;
        let pending = std::mem::take(&mut self.pending);
        for event in pending {
            match event {
                Pending::Spawned(u) => self.emit("on_spawned", vec![Arg::Int(u as i64)])?,
                Pending::Attacked(u) if self.sh.borrow().sim.alive(u) => self.emit("on_attacked", vec![Arg::Int(u as i64)])?,
                Pending::Attacked(_) => {}
                Pending::Died(u, killer) => {
                    let death = self.death_of(u, killer);
                    self.died(&death)?
                }
                Pending::Enter(u, r) => self.emit_region("on_enter", r, u)?,
                Pending::Leave(u, r) => self.emit_region("on_leave", r, u)?,
                Pending::PlayerJoined(p) => self.emit("on_player_joined", vec![Arg::Int(p as i64)])?,
                Pending::PlayerLeft(p) => self.emit("on_player_left", vec![Arg::Int(p as i64)])?,
            }
        }
        for (i, f) in self.live_hooks("run_systems", |h| Some(h.run_systems.clone())) {
            if let Err(e) = self.call(&f, Vec::new()) {
                self.hook_failed(i, "run_systems", &e);
            }
        }
        self.apply_commands();
        for (i, f) in self.live_hooks("fire_timers", |h| Some(h.fire_timers.clone())) {
            if let Err(e) = self.call(&f, Vec::new()) {
                self.hook_failed(i, "fire_timers", &e);
            }
        }
        self.emit("on_tick", vec![Arg::State])?;
        Ok(())
    }

    pub fn feed_death(&mut self, death: &Death) -> Result<Vec<Cmd>, String> {
        if self.modules.is_empty() {
            return Ok(Vec::new());
        }
        let before = self.sh.borrow().local.len();
        self.died(death)?;
        Ok(self.sh.borrow_mut().local.split_off(before))
    }

    fn death_of(&self, unit: usize, killer: Option<usize>) -> Death {
        let mut sh = self.sh.borrow_mut();
        let bounty = sh.sim.roll_bounty(unit);
        Death {
            victim: unit as u32,
            victim_player: sh.sim.player(unit),
            killer: killer.map(|k| k as u32),
            killer_player: killer.map(|k| sh.sim.player(k)),
            bounty,
        }
    }

    fn died(&mut self, d: &Death) -> Result<(), String> {
        let opt = |v: Option<i64>| v.map_or(Arg::Nil, Arg::Int);
        let before = self.sh.borrow().cmds.len();
        let killer_attributed = d.killer.is_some_and(|k| self.sh.borrow().sim.is_attributed(k as usize));
        self.emit(
            "on_died",
            vec![
                Arg::Int(i64::from(d.victim)),
                opt(d.killer.map(i64::from)),
                opt(d.killer_player.map(i64::from)),
                Arg::Int(i64::from(d.bounty)),
                Arg::Int(i64::from(d.victim_player)),
                Arg::Bool(killer_attributed),
            ],
        )?;
        for (i, f) in self.live_hooks("on_unit_died", |h| h.on_unit_died.clone()) {
            match self.call_ret(&f, vec![Arg::Int(i64::from(d.victim)), Arg::State]) {
                Ok((delay, _)) => {
                    if let Some(cmd) = respawn_of(delay) {
                        self.sh.borrow_mut().cmds.push(Cmd::SetRespawn { unit: d.victim as usize, delay_s: cmd });
                    }
                }
                Err(e) => self.hook_failed(i, "on_unit_died", &e),
            }
        }
        let mut sh = self.sh.borrow_mut();
        let own = sh.cmds.split_off(before);
        sh.local.extend(own);
        Ok(())
    }

    fn dispatch(&mut self) -> Result<(), String> {
        let (requests, casts, hits, fired) = {
            let mut sh = self.sh.borrow_mut();
            (sh.sim.take_destination_requests(), sh.sim.take_casts(), sh.sim.take_hits(), sh.sim.take_fired())
        };
        let hit = self.live_hooks("on_hit", |h| h.on_hit.clone());
        for h in hits {
            for (i, f) in &hit {
                self.call_on_hit(*i, f, h);
            }
        }
        let march = self.live_hooks("on_march", |h| h.on_march.clone());
        for (unit, player) in requests {
            for (i, f) in &march {
                self.call_on_march(*i, f, unit, player);
            }
        }
        let cast = self.live_hooks("on_cast", |h| h.on_cast.clone());
        for (caster, ability) in casts {
            for (i, f) in &cast {
                self.call_on_cast(*i, f, caster, ability);
            }
        }
        let on_proc = self.live_hooks("on_proc", |h| h.on_proc.clone());
        let procs = self.sh.borrow_mut().sim.take_procs();
        for (caster, ability) in procs {
            for (i, f) in &on_proc {
                self.call_on_proc(*i, f, caster, ability);
            }
        }
        for f in fired {
            let args = vec![
                Arg::Int(i64::from(f.unit)),
                Arg::Text(f.trigger_name().to_owned()),
                Arg::Text(f.action_name().to_owned()),
                Arg::Int(i64::from(f.ability)),
                Arg::Int(i64::from(f.slot)),
                Arg::Int(i64::from(f.target_kind)),
            ];
            self.emit("on_behaviour", args)?;
        }
        let win =self.live_hooks("on_check_win", |h| h.on_check_win.clone());
        for (i, f) in &win {
            if self.sh.borrow().sim.winner().is_some() {
                break;
            }
            self.call_on_check_win(*i, f);
        }
        Ok(())
    }

    fn call_on_march(&mut self, i: usize, f: &StashedFunction, unit: usize, player: u8) {
        if !self.is_live(i, "on_march") {
            return;
        }
        match self.call_ret(f, vec![Arg::Int(unit as i64), Arg::Int(i64::from(player)), Arg::State]) {
            Ok((Ret::Int(x), Ret::Int(y))) => match (i32::try_from(x), i32::try_from(y)) {
                (Ok(x), Ok(y)) => self.sh.borrow_mut().cmds.push(Cmd::AttackMove { unit, x, y }),
                _ => println!("ev=mod_hook_refused hook=on_march value=out_of_range"),
            },
            Ok((Ret::Nil | Ret::Bool(false), _)) => {}
            Ok(_) => println!("ev=mod_hook_refused hook=on_march"),
            Err(e) => self.hook_failed(i, "on_march", &e),
        }
    }

    fn call_on_hit(&mut self, i: usize, f: &StashedFunction, hit: sim::Hit) {
        if !self.is_live(i, "on_hit") {
            return;
        }
        let args = vec![Arg::Int(i64::from(hit.attacker)), Arg::Int(i64::from(hit.victim)), Arg::Int(i64::from(hit.dealt)), Arg::State, Arg::Bool(hit.attack)];
        if let Err(e) = self.call_ret(f, args) {
            self.hook_failed(i, "on_hit", &e);
        }
    }

    fn call_on_cast(&mut self, i: usize, f: &StashedFunction, caster: usize, ability: usize) {
        if !self.is_live(i, "on_cast") {
            return;
        }
        let name = {
            let sh = self.sh.borrow();
            let cat = sh.sim.catalog();
            match cat.abilities.get(ability) {
                Some(a) => cat.name(a.name).to_string(),
                None => return,
            }
        };
        if let Err(e) = self.call_ret(f, vec![Arg::Int(caster as i64), Arg::Text(name), Arg::State]) {
            self.hook_failed(i, "on_cast", &e);
        }
    }

    fn call_on_proc(&mut self, i: usize, f: &StashedFunction, caster: usize, ability: usize) {
        if !self.is_live(i, "on_proc") {
            return;
        }
        let name = {
            let sh = self.sh.borrow();
            let cat = sh.sim.catalog();
            match cat.abilities.get(ability) {
                Some(a) => cat.name(a.name).to_string(),
                None => return,
            }
        };
        if let Err(e) = self.call_ret(f, vec![Arg::Int(caster as i64), Arg::Text(name), Arg::State]) {
            self.hook_failed(i, "on_proc", &e);
        }
    }

    fn call_on_check_win(&mut self, i: usize, f: &StashedFunction) {
        if !self.is_live(i, "on_check_win") {
            return;
        }
        match self.call_ret(f, vec![Arg::State]) {
            Ok((Ret::Nil | Ret::Bool(false), _)) => {}
            Ok((Ret::Int(s), _)) if (0..MAX_PLAYERS as i64).contains(&s) => {
                self.sh.borrow_mut().cmds.push(Cmd::DeclareSide { side: s as u8 });
            }
            Ok(_) => println!("ev=mod_hook_refused hook=on_check_win"),
            Err(e) => self.hook_failed(i, "on_check_win", &e),
        }
    }

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

    fn emit(&mut self, event: &str, args: Vec<Arg>) -> Result<(), String> {
        for (i, f) in self.live_hooks(event, |h| Some(h.emit.clone())) {
            #[cfg(feature = "fault-injection")]
            if event == "on_tick" {
                let tick = self.sh.borrow().sim.ticks();
                if self.modules[i].fault_tick == Some(tick) {
                    panic!("fault injected: {} on_tick at sim tick {tick}", self.modules[i].name);
                }
            }
            let mut values = vec![Arg::Table(self.modules[i].handlers.clone()), Arg::Text(event.to_owned())];
            values.extend(args.iter().cloned());
            if let Err(e) = self.call(&f, values) {
                self.hook_failed(i, event, &e);
            }
        }
        Ok(())
    }

    fn call(&mut self, f: &StashedFunction, args: Vec<Arg>) -> Result<(), String> {
        guarded("hook", || {
            let exec = self.start(f, args)?;
            self.run(&exec)
        })
    }

    fn call_ret(&mut self, f: &StashedFunction, args: Vec<Arg>) -> Result<(Ret, Ret), String> {
        guarded("hook", || {
            let exec = self.start(f, args)?;
            self.finish(&exec)?;
            self.lua.borrow_mut()
                .try_enter(|ctx| {
                    let (x, y) = ctx.fetch(&exec).take_result::<(Value, Value)>(ctx)??;
                    Ok((Ret::of(x), Ret::of(y)))
                })
                .map_err(|e| e.to_string())
        })
    }

    fn start(&mut self, f: &StashedFunction, args: Vec<Arg>) -> Result<StashedExecutor, String> {
        guarded("start", || {
            let state = &self.state;
            self.lua.borrow_mut()
                .try_enter(|ctx| {
                    let values: Vec<Value> = args
                        .into_iter()
                        .map(|a| match a {
                            Arg::Table(t) => Value::Table(ctx.fetch(&t)),
                            Arg::Int(i) => Value::Integer(i),
                            Arg::Bool(b) => Value::Boolean(b),
                            Arg::Nil => Value::Nil,
                            Arg::Text(t) => t.into_value(ctx),
                            Arg::Num(n) => Value::Number(n),
                            Arg::State => Value::Table(ctx.fetch(state)),
                            Arg::Json(j) => json_value(ctx, &j),
                        })
                        .collect();
                    Ok(ctx.stash(Executor::start(ctx, ctx.fetch(f), Variadic(values))))
                })
                .map_err(|e| e.to_string())
        })
    }

    fn apply_commands(&mut self) {
        let local = std::mem::take(&mut self.sh.borrow_mut().local);
        let list = std::mem::take(&mut self.sh.borrow_mut().cmds);
        for cmd in local.into_iter().chain(list) {
            if let Err(e) = guarded("command", || {
                self.apply_one(cmd);
                Ok(())
            }) {
                println!("ev=mod_command_failed error={e:?}");
            }
        }
    }

    fn apply_one(&mut self, cmd: Cmd) {
        if let Cmd::UseItemOn { unit, slot, prop, target } = cmd {
            self.item_used_on(unit, slot, prop, target);
            return;
        }
        let allowed = match cmd {
            Cmd::SkillUp { unit, slot } => self.skill_allowed(unit, slot),
            Cmd::Cast { unit, slot, target, x, y } => self.cast_allowed(unit, slot, target, x, y, false),
            Cmd::UseItem { unit, slot, target, x, y } => self.cast_allowed(unit, slot, target, x, y, true),
            _ => true,
        };
        let requests = {
            let mut guard = self.sh.borrow_mut();
            let sh = &mut *guard;
            if allowed && Self::apply_cmd(&mut sh.sim, &mut sh.ui, cmd) {
                sh.applied += 1;
            }
            sh.sim.drain_requests()
        };
        self.settle(requests);
    }

    fn item_used_on(&mut self, unit: usize, slot: usize, prop: bool, target: u32) {
        let name = {
            let sh = self.sh.borrow();
            let sim = &sh.sim;
            let (kind, count) = sim.inventory_slot(unit, slot);
            let target_ok = if prop {
                sim.prop_list().iter().any(|d| d.id == target)
            } else {
                (target as usize) < sim.count() && sim.alive(target as usize)
            };
            let cat = sim.catalog();
            match (kind as usize).checked_sub(1).and_then(|i| cat.items.get(i)) {
                Some(item) if sim.alive(unit) && count > 0 && target_ok => cat.name(item.name).to_owned(),
                _ => return,
            }
        };
        let target_kind = if prop { "prop" } else { "unit" };
        for (i, f) in self.live_hooks("on_item_use", |h| h.on_item_use.clone()) {
            if !self.is_live(i, "on_item_use") {
                continue;
            }
            let args = vec![
                Arg::Int(unit as i64),
                Arg::Int(slot as i64),
                Arg::Text(name.clone()),
                Arg::Text(target_kind.to_owned()),
                Arg::Int(i64::from(target)),
                Arg::State,
            ];
            if let Err(e) = self.call_ret(&f, args) {
                self.hook_failed(i, "on_item_use", &e);
            }
        }
    }

    fn skill_allowed(&mut self, unit: usize, slot: usize) -> bool {
        let Some((i, f)) = self.live_hooks("on_skill_up", |h| h.on_skill_up.clone()).into_iter().next() else { return true };
        match self.call_own(&f, vec![Arg::Int(unit as i64), Arg::Int(slot as i64), Arg::State]) {
            Ok(v) => matches!(v[0], Own::Bool(true)),
            Err(e) => {
                self.hook_failed(i, "on_skill_up", &e);
                false
            }
        }
    }

    pub fn cast_allowed(&mut self, unit: usize, slot: usize, target: i32, x: i32, y: i32, item: bool) -> bool {
        let found = {
            let sh = self.sh.borrow();
            let sim = &sh.sim;
            let index = if item { sim.item_ability_at(unit, slot) } else { sim.ability_in_slot(unit, slot) };
            match index {
                Some(ai) if sim.alive(unit) => {
                    let cat = sim.catalog();
                    cat.abilities.get(ai).map(|a| (ai, cat.name(a.name).to_string()))
                }
                _ => None,
            }
        };
        let Some((index, name)) = found else { return true };
        let Some((i, f)) = self.live_hooks("on_cast_request", |h| h.on_cast_request.clone()).into_iter().next() else { return true };
        let args = vec![
            Arg::Int(unit as i64),
            Arg::Int(slot as i64),
            Arg::Text(name),
            Arg::Int(index as i64),
            Arg::Int(i64::from(target)),
            Arg::Int(i64::from(x)),
            Arg::Int(i64::from(y)),
            Arg::Bool(item),
            Arg::State,
        ];
        match self.call_own(&f, args) {
            Ok(v) => matches!(v[0], Own::Bool(true)),
            Err(e) => {
                self.hook_failed(i, "on_cast_request", &e);
                false
            }
        }
    }

    fn settle(&mut self, requests: Vec<ItemRequest>) {
        for req in requests {
            if let Some(verdict) = self.answer(&req) {
                let more = {
                    let mut guard = self.sh.borrow_mut();
                    guard.sim.apply_verdict(&verdict);
                    guard.sim.drain_requests()
                };
                self.settle(more);
            }
        }
    }

    fn answer(&mut self, req: &ItemRequest) -> Option<ItemVerdict> {
        let hook = self.live_hooks("on_request", |h| h.on_request.clone()).into_iter().next();
        match (*req, hook) {
            (ItemRequest::Acquire { unit, item }, Some((i, f))) => Some(self.acquire_verdict(i, &f, unit, item)),
            (ItemRequest::Acquire { unit, item }, None) => {
                Some(ItemVerdict::Acquire { unit, item, allow: false, deny: Refusal::NoItem, debit: 0, takes: [NO_TAKE; VERDICT_TAKES] })
            }
            (ItemRequest::Release { unit, slot }, Some((i, f))) => Some(self.release_verdict(i, &f, unit, slot)),
            (ItemRequest::Release { unit, slot }, None) => Some(ItemVerdict::Release { unit, slot, allow: false, deny: Refusal::NoItem, credit: 0 }),
            (ItemRequest::TakeOut { unit, slot }, Some((i, f))) => Some(self.take_out_verdict(i, &f, unit, slot)),
            (ItemRequest::TakeOut { unit, slot }, None) => Some(ItemVerdict::TakeOut { unit, slot, allow: false, deny: Refusal::NoItem }),
            (ItemRequest::Carry { caster, target }, Some((i, f))) => {
                let args = vec![Arg::Text("carry".to_owned()), Arg::Int(caster as i64), Arg::Int(target as i64), Arg::State];
                if let Err(e) = self.call(&f, args) {
                    self.hook_failed(i, "on_request", &e);
                }
                None
            }
            (ItemRequest::Carry { .. }, None) => None,
        }
    }

    fn acquire_verdict(&mut self, i: usize, f: &StashedFunction, unit: usize, item: usize) -> ItemVerdict {
        let refuse = |deny| ItemVerdict::Acquire { unit, item, allow: false, deny, debit: 0, takes: [NO_TAKE; VERDICT_TAKES] };
        let items = self.sh.borrow().sim.catalog().items.len();
        match self.call_own(f, vec![Arg::Text("acquire".to_owned()), Arg::Int(unit as i64), Arg::Int(item as i64), Arg::State]) {
            Ok(v) => match (&v[0], &v[1], &v[2]) {
                (Own::Bool(true), debit, takes) => match (whole(debit, i64::from(i32::MAX)), parse_takes(takes, items)) {
                    (Some(debit), Some(takes)) => ItemVerdict::Acquire { unit, item, allow: true, deny: Refusal::NoItem, debit: debit as i32, takes },
                    _ => refuse(Refusal::NoItem),
                },
                (_, reason, _) => refuse(denial(reason)),
            },
            Err(e) => {
                self.hook_failed(i, "on_request", &e);
                refuse(Refusal::NoItem)
            }
        }
    }

    fn release_verdict(&mut self, i: usize, f: &StashedFunction, unit: usize, slot: usize) -> ItemVerdict {
        match self.call_own(f, vec![Arg::Text("release".to_owned()), Arg::Int(unit as i64), Arg::Int(slot as i64), Arg::State]) {
            Ok(v) => match (&v[0], &v[1]) {
                (Own::Bool(true), credit) => match whole(credit, i64::from(i32::MAX)) {
                    Some(credit) => ItemVerdict::Release { unit, slot, allow: true, deny: Refusal::NoItem, credit: credit as i32 },
                    None => ItemVerdict::Release { unit, slot, allow: false, deny: Refusal::NoItem, credit: 0 },
                },
                (_, reason) => ItemVerdict::Release { unit, slot, allow: false, deny: denial(reason), credit: 0 },
            },
            Err(e) => {
                self.hook_failed(i, "on_request", &e);
                ItemVerdict::Release { unit, slot, allow: false, deny: Refusal::NoItem, credit: 0 }
            }
        }
    }

    fn take_out_verdict(&mut self, i: usize, f: &StashedFunction, unit: usize, slot: usize) -> ItemVerdict {
        match self.call_own(f, vec![Arg::Text("take_out".to_owned()), Arg::Int(unit as i64), Arg::Int(slot as i64), Arg::State]) {
            Ok(v) => match &v[0] {
                Own::Bool(true) => ItemVerdict::TakeOut { unit, slot, allow: true, deny: Refusal::NoItem },
                _ => ItemVerdict::TakeOut { unit, slot, allow: false, deny: denial(&v[1]) },
            },
            Err(e) => {
                self.hook_failed(i, "on_request", &e);
                ItemVerdict::TakeOut { unit, slot, allow: false, deny: Refusal::NoItem }
            }
        }
    }

    fn call_own(&mut self, f: &StashedFunction, args: Vec<Arg>) -> Result<[Own; 3], String> {
        guarded("hook", || {
            let exec = self.start(f, args)?;
            self.finish(&exec)?;
            self.lua.borrow_mut()
                .try_enter(|ctx| {
                    let (x, y, z) = ctx.fetch(&exec).take_result::<(Value, Value, Value)>(ctx)??;
                    Ok([own(ctx, x), own(ctx, y), own(ctx, z)])
                })
                .map_err(|e| e.to_string())
        })
    }

    fn apply_cmd(sim: &mut Sim, ui: &mut Vec<String>, cmd: Cmd) -> bool {
        {
            match cmd {
                Cmd::Spawn { index, x, y, owner, route, .. } => {
                    sim.spawn_mod_unit(owner, index as u32, x, y, route);
                }
                Cmd::SetOwner { unit, player } => {
                    let _ = sim.set_owner(unit, player);
                }
                Cmd::SetFog { on } => sim.set_fog(on),
                Cmd::AddGold { player, amount } => sim.add_resources(player, amount, 0),
                Cmd::AddGas { player, amount } => sim.add_resources(player, 0, amount),
                Cmd::RemoveUnit { unit } => {
                    sim.remove_unit(unit);
                }
                Cmd::GiveItem { unit, item, count } => {
                    let _ = sim.grant_item(unit, item, count);
                }
                Cmd::DropGround { item, count, x, y } => {
                    let _ = sim.place_ground_item(item, count, x, y);
                }
                Cmd::DropUnitItem { unit, slot, x, y } => {
                    let _ = sim.drop_item_to_ground(unit, slot, x, y);
                }
                Cmd::PickupGround { unit, id } => {
                    let _ = sim.pickup_ground_item(unit, id);
                }
                Cmd::RemoveGround { id } => {
                    let _ = sim.remove_ground_item(id);
                }
                Cmd::SetGroundCount { id, count } => {
                    let _ = sim.set_ground_item_count(id, count);
                }
                Cmd::SetItemCharges { unit, slot, count } => {
                    let _ = sim.set_item_charges(unit, slot, count);
                }
                Cmd::ReplaceItem { unit, slot, item, count } => {
                    let _ = sim.replace_item(unit, slot, item as u32 + 1, count);
                }
                Cmd::SetItemOwner { unit, slot, player } => {
                    let _ = sim.set_item_owner(unit, slot, player);
                }
                Cmd::Victory { player } => sim.declare_victory(player),
                Cmd::Defeat { player } => sim.declare_defeat(player),
                Cmd::Ui { body } => ui.push(body),
                Cmd::Fx(event) => {
                    let _ = sim.push_fx(event);
                    return false;
                }
                Cmd::GiveAbility { unit, slot, name } => {
                    let _ = sim.give_ability(unit, slot, &name);
                }
                Cmd::AddUnitGold { unit, amount } => {
                    let _ = sim.add_unit_gold(unit, amount);
                }
                Cmd::AddXp { unit, amount } => {
                    sim.add_xp(unit, amount);
                }
                Cmd::AddStat { unit, code, delta } => sim.add_stat_bonus(unit, code, delta),
                Cmd::TimedStat { unit, code, delta, ticks } => {
                    sim.pack_timed_stat(unit, code, delta, ticks, PACK_TIMED_STAT_KEY);
                }
                Cmd::Order { unit, x, y } if sim.alive(unit) => sim.order(unit, x, y),
                Cmd::Hold { unit } if sim.alive(unit) => {
                    let _ = sim.issue_order_kind(unit, ORDER_HOLD, 0, 0);
                }
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
                Cmd::SetStrike { unit, target } => {
                    let _ = sim.set_strike_target(unit, target);
                }
                Cmd::SkillUp { unit, slot } if sim.alive(unit) => {
                    let _ = sim.rank_up(unit, slot);
                }
                Cmd::SetPeriod { unit, ticks } => {
                    let _ = sim.set_base_period(unit, ticks);
                }
                Cmd::Revive { unit, x, y } => {
                    let _ = sim.revive_unit(unit, x, y);
                }
                Cmd::RemovePlayer { player } => {
                    let _ = sim.remove_player(player);
                }
                Cmd::EditCell { x, y, blocked } => {
                    let _ = sim.set_cell_blocked(x, y, blocked);
                }
                Cmd::SetTile { x, y, tile } => {
                    let _ = sim.set_cell_tile(x, y, tile);
                }
                Cmd::SetXpRate { player, percent } => {
                    sim.set_xp_rate(usize::from(player), percent);
                }
                Cmd::SetUnitScale { unit, percent } => {
                    sim.set_unit_scale(unit, percent);
                }
                Cmd::OrderHandle { .. } | Cmd::DamageHandle { .. } | Cmd::BuffHandle { .. } => {}
                Cmd::SetAlliance { a, b, kind } => {
                    let _ = sim.set_relation(a, b, kind);
                }
                Cmd::AddPlayer { player, controller } => {
                    let _ = sim.add_player(player, controller);
                }
                Cmd::RespawnAs { unit, kind, heading } => {
                    let _ = sim.set_respawn_kind(unit, kind);
                    let _ = sim.set_respawn_heading(unit, heading);
                }
                Cmd::SetRespawn { unit, delay_s } => {
                    let _ = sim.set_respawn(unit, delay_s);
                }
                Cmd::SetHome { unit, x, y } => {
                    let _ = sim.set_group_home(unit, x, y);
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
                Cmd::DeclareSide { side } => sim.declare_winner(side),
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
                _ => return false,
            }
            true
        }
    }

    fn collect(&mut self) {
        self.snap.clear();
        self.pending.clear();
        self.pending.extend(self.joined.drain(..).map(Pending::PlayerJoined));
        self.pending.extend(self.left.drain(..).map(Pending::PlayerLeft));
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
        guarded("hook", || self.lua.borrow_mut().try_enter(|ctx| ctx.fetch(executor).take_result::<()>(ctx)?).map_err(|e| e.to_string()))
    }

    fn finish(&mut self, executor: &StashedExecutor) -> Result<(), String> {
        for _ in 0..MAX_STEPS {
            let mut fuel = Fuel::with(FUEL_PER_STEP);
            let done = guarded("vm", || Ok(self.lua.borrow_mut().enter(|ctx| ctx.fetch(executor).step(ctx, &mut fuel))))?;
            guarded("memory", || self.memory_check())?;
            if done {
                return Ok(());
            }
        }
        Err(format!("instruction_limit: over {MAX_STEPS} steps in one call"))
    }

    fn memory_check(&self) -> Result<(), String> {
        let mut lua = self.lua.borrow_mut();
        if lua.total_memory() <= MEMORY_CAP {
            return Ok(());
        }
        lua.gc_collect();
        let used = lua.total_memory();
        if used <= MEMORY_CAP {
            return Ok(());
        }
        Err(format!("memory_limit: {used} bytes of Lua memory in use, over the {MEMORY_CAP} byte cap"))
    }

    fn retire(&mut self, i: usize) {
        let Some(m) = self.modules.get_mut(i) else { return };
        for &hook in HOOK_KEYS {
            if !m.off.iter().any(|o| o == hook) {
                m.off.push(hook.to_owned());
            }
        }
        let mut tables = vec![m.env.clone(), m.handlers.clone()];
        tables.extend(m.exports.clone());
        let mut lua = self.lua.borrow_mut();
        lua.enter(|ctx| {
            for t in &tables {
                clear_table(ctx, ctx.fetch(t));
            }
        });
        lua.gc_collect();
    }
}

#[derive(Clone)]
enum Arg {
    Table(StashedTable),
    Int(i64),
    Bool(bool),
    Nil,
    Text(String),
    Num(f64),
    State,
    Json(serde_json::Value),
}

fn json_value<'gc>(ctx: Context<'gc>, v: &serde_json::Value) -> Value<'gc> {
    use serde_json::Value as J;
    match v {
        J::Null => Value::Nil,
        J::Bool(b) => Value::Boolean(*b),
        J::Number(n) => match n.as_i64() {
            Some(i) => Value::Integer(i),
            None => Value::Number(n.as_f64().unwrap_or(0.0)),
        },
        J::String(s) => Value::String(ctx.intern(s.as_bytes())),
        J::Array(items) => {
            let t = Table::new(&ctx);
            for (i, item) in items.iter().enumerate() {
                t.put(ctx, i as i64 + 1, json_value(ctx, item));
            }
            Value::Table(t)
        }
        J::Object(map) => {
            let t = Table::new(&ctx);
            for (k, item) in map {
                t.put(ctx, ctx.intern(k.as_bytes()), json_value(ctx, item));
            }
            Value::Table(t)
        }
    }
}

pub struct Death {
    pub victim: u32,
    pub victim_player: u8,
    pub killer: Option<u32>,
    pub killer_player: Option<u8>,
    pub bounty: i32,
}

#[cfg(all(test, feature = "build"))]
mod containment {
    use std::path::Path;

    use crate::ModHost;

    fn host() -> ModHost {
        let scenario = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
        let image = crate::build_image(&scenario).expect("moba scenario builds an image");
        ModHost::new(&image).expect("host")
    }

    #[test]
    fn a_panicking_native_is_a_hook_failure_and_the_host_keeps_advancing() {
        let mut host = host();
        let panicker: (String, String) = ("panicker.lua".into(), "return { on_tick = function(state) fault_panic() end }".into());
        let steady: (String, String) = ("steady.lua".into(), "return { on_tick = function(state) state.steady = (state.steady or 0) + 1 end }".into());
        host.load_mods(&[panicker, steady]).expect("both load");
        for _ in 0..3 {
            host.advance().expect("the panic is contained: the host keeps advancing");
        }
        let _ = host.sim().count();
        assert_eq!(host.state_int("steady"), Some(3), "the other module ran on every tick");
        assert!(host.disabled_hooks().contains(&"panicker:on_tick".to_string()), "the panicking hook is off: {:?}", host.disabled_hooks());
    }

    #[test]
    fn state_ticks_is_the_sim_tick_on_every_step() {
        let mut host = host();
        let clock: (String, String) = ("clock.lua".into(), "return { on_tick = function(state) state.seen = state.ticks end }".into());
        host.load_mods(&[clock]).expect("loads");
        for _ in 0..5 {
            host.advance().expect("advances");
        }
        assert_eq!(host.state_int("seen"), Some(i64::from(host.sim().ticks())), "state.ticks follows the sim, not the seed 0");
        assert!(host.state_int("seen").unwrap_or(0) > 0, "state.ticks has moved off 0");
    }
}
