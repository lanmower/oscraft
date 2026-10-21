use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mod_host::Cmd;
use sim::{Grid, Sim};

use crate::budget::{
    TickHistogram, Timing, NORMAL_ORDER_CAP, SHED_AFTER_OVERRUNS, SHED_HOLD_TICKS, SHED_LOG_EVERY_TICKS,
    SHED_ORDER_CAP, SHED_PATHS_EVERY, SHED_VISION_EVERY,
};
use crate::mods::{self, ModLink};
use crate::order::{Order, OrderGate};
use crate::replay::{Command, Recording};
use crate::supervise::{supervise, Outcome};

// Matches sim/src/lib.rs `TICKS_PER_SECOND` (16). The sim keeps it pub(crate), so it is not re-exported and must be kept in step by hand.
pub const TICKS_PER_SECOND: u32 = 16;

// Mod commands the room applies per tick; the rest wait in the link for later ticks.
const MOD_CMDS_PER_TICK: usize = 64;

pub struct RoomConfig {
    pub name: String,
    pub grid_w: i32,
    pub grid_h: i32,
    pub max_units: usize,
    pub token: String,
    pub rate: u32,
    pub data: Option<PathBuf>,
    pub shed: bool,
}

struct Clock {
    sim: Sim,
    handles: HashMap<u32, usize>,
    tick: u64,
    overruns: u64,
    streak: u64,
    shed_until: u64,
    shed_on: bool,
    worst: Duration,
    orders_drained: u64,
    orders: Receiver<Order>,
    hist: TickHistogram,
    rec: Recording,
}

// Mod UI events (messages, banners, panels, camera moves, sounds) kept for clients to poll by id.
#[derive(Default)]
struct UiLog {
    next: u64,
    events: VecDeque<(u64, String)>,
}

const UI_LOG_CAP: usize = 256;

pub struct Room {
    pub name: String,
    pub grid_w: i32,
    pub grid_h: i32,
    pub max_units: usize,
    pub loaded: bool,
    pub gate: OrderGate,
    queue: Mutex<Sender<Order>>,
    queued: AtomicU64,
    shed: bool,
    feed: Mutex<Option<Arc<ModLink>>>,
    ui: Mutex<UiLog>,
    staged: Mutex<Option<Sim>>,
    clock: Mutex<Clock>,
    test_panic_at: AtomicU64,
    gold_by_mod: AtomicBool,
}

pub struct Snapshot {
    pub tick: u64,
    pub state_hash: u64,
    pub units: usize,
    pub winner: i32,
    pub resource_a: Vec<i32>,
    pub players: usize,
    pub sides: usize,
    pub records: Vec<String>,
}

pub struct Delta {
    pub tick: u32,
    pub units: Vec<u32>,
    pub records: Vec<String>,
}

// Most unit records one /state or /delta reply carries. A reply cut short still reports its exact
// unit or changed count, so a reader sees the gap as a count mismatch.
pub const RECORD_CAP: usize = 512;

fn unit_json(sim: &Sim, i: usize, with_alive: bool) -> String {
    let (x, y) = sim.position_fixed(i);
    let alive = if with_alive { "\"alive\":true," } else { "" };
    format!(
        "{{\"id\":{i},{alive}\"kind\":{},\"player\":{},\"x\":{x},\"y\":{y},\"hp\":{}}}",
        sim.kind_of(i),
        sim.player(i),
        sim.hp(i)
    )
}

fn live_records(sim: &Sim) -> Vec<String> {
    (0..sim.count())
        .filter(|&i| sim.alive(i))
        .take(RECORD_CAP)
        .map(|i| unit_json(sim, i, false))
        .collect()
}

fn changed_record(sim: &Sim, i: usize) -> String {
    if sim.alive(i) {
        unit_json(sim, i, true)
    } else {
        format!("{{\"id\":{i},\"alive\":false}}")
    }
}

pub enum DeltaError {
    Future { since: u32, tick: u32 },
}

pub struct Stats {
    pub tick: u64,
    pub overruns: u64,
    pub worst: Duration,
    pub orders_drained: u64,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Room {
    pub fn new(cfg: RoomConfig) -> Result<Arc<Room>, String> {
        let (tx, rx) = mpsc::channel();
        let watch = cfg.data.clone().filter(|path| path.is_dir());
        let (sim, grid_w, grid_h, loaded) = match &cfg.data {
            Some(path) => {
                let sim = mods::load_data(path, cfg.max_units)
                    .map_err(|e| format!("--data {}: {e}", path.display()))?;
                let (w, h) = (sim.grid().w as i32, sim.grid().h as i32);
                (sim, w, h, true)
            }
            None => {
                let mut sim = Sim::new(cfg.max_units);
                sim.set_grid(Grid::open(cfg.grid_w as usize, cfg.grid_h as usize));
                (sim, cfg.grid_w, cfg.grid_h, false)
            }
        };
        let image = cfg.data.as_ref().map_or_else(|| format!("open-grid:{grid_w}x{grid_h}"), |p| p.display().to_string());
        let rec = Recording::new(&image, cfg.max_units, &sim);
        let room = Arc::new(Room {
            name: cfg.name,
            grid_w,
            grid_h,
            max_units: cfg.max_units,
            loaded,
            gate: OrderGate::new(cfg.token, cfg.rate),
            queue: Mutex::new(tx),
            queued: AtomicU64::new(0),
            shed: cfg.shed,
            feed: Mutex::new(None),
            ui: Mutex::new(UiLog::default()),
            test_panic_at: AtomicU64::new(0),
            gold_by_mod: AtomicBool::new(false),
            staged: Mutex::new(None),
            clock: Mutex::new(Clock {
                sim,
                handles: HashMap::new(),
                tick: 0,
                overruns: 0,
                streak: 0,
                shed_until: 0,
                shed_on: false,
                worst: Duration::ZERO,
                orders_drained: 0,
                orders: rx,
                hist: TickHistogram::new(),
                rec,
            }),
        });
        if let Some(dir) = watch {
            mods::watch_data(Arc::downgrade(&room), dir, cfg.max_units);
        }
        Ok(room)
    }

    fn log_ui(&self, tick: u64, bodies: Vec<String>) {
        if bodies.is_empty() {
            return;
        }
        let mut log = lock(&self.ui);
        for body in bodies {
            log.next += 1;
            let id = log.next;
            log.events.push_back((id, format!("{{\"id\":{id},\"tick\":{tick},{body}}}")));
            if log.events.len() > UI_LOG_CAP {
                log.events.pop_front();
            }
        }
    }

    // The UI events with an id above `since`, and the newest id issued.
    pub fn ui_events_since(&self, since: u64) -> (u64, Vec<String>) {
        let log = lock(&self.ui);
        (log.next, log.events.iter().filter(|(id, _)| *id > since).map(|(_, e)| e.clone()).collect())
    }

    /// The replay so far as text: the initial data image name and hash, every command per tick and
    /// the state hash every few ticks, ending with a check at the current tick.
    pub fn replay_text(&self) -> String {
        let mut c = lock(&self.clock);
        let clock = &mut *c;
        clock.rec.render(&clock.sim)
    }

    /// Sets the sim's `gold_by_mod`: the mod owns kill gold, so the Rust payout stays off. The recording
    /// is restarted so its header carries the flag, which the init hash covers. Only before the first tick.
    pub fn set_gold_by_mod(&self, on: bool) -> Result<(), String> {
        let mut c = lock(&self.clock);
        if c.tick != 0 {
            return Err(format!("{}: gold_by_mod must be set before the first tick", self.name));
        }
        self.gold_by_mod.store(on, Ordering::Relaxed);
        c.sim.set_gold_by_mod(on);
        let image = c.rec.image.clone();
        let max_units = c.rec.max_units as usize;
        c.rec = Recording::new(&image, max_units, &c.sim);
        Ok(())
    }

    /// Streams the replay to `path` as the room ticks.
    pub fn set_replay_file(&self, path: &std::path::Path) -> Result<(), String> {
        let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        lock(&self.clock).rec.set_sink(Box::new(std::io::BufWriter::new(file)));
        Ok(())
    }

    pub fn attach_mod_feed(&self, link: Arc<ModLink>) {
        *lock(&self.feed) = Some(link);
    }

    // A rebuilt image waits here until the next tick boundary swaps it in.
    pub fn stage_data(&self, mut sim: Sim) -> Result<(), String> {
        let (w, h) = (sim.grid().w as i32, sim.grid().h as i32);
        if (w, h) != (self.grid_w, self.grid_h) {
            return Err(format!(
                "map is {}x{} in service and {w}x{h} in the new data; a map size change needs a restart",
                self.grid_w, self.grid_h
            ));
        }
        sim.set_gold_by_mod(self.gold_by_mod.load(Ordering::Relaxed));
        *lock(&self.staged) = Some(sim);
        Ok(())
    }

    pub fn arm_test_panic(&self, tick: u64) {
        self.test_panic_at.store(tick + 1, Ordering::Relaxed);
    }

    fn take_test_panic(&self, tick: u64) -> bool {
        if self.test_panic_at.load(Ordering::Relaxed) != tick + 1 {
            return false;
        }
        self.test_panic_at.store(0, Ordering::Relaxed);
        true
    }

    pub fn submit(&self, order: Order) -> Result<(), String> {
        self.queued.fetch_add(1, Ordering::Relaxed);
        lock(&self.queue).send(order).map_err(|_| {
            self.queued.fetch_sub(1, Ordering::Relaxed);
            "room is closed".to_string()
        })
    }

    pub fn tick_now(&self) -> u64 {
        lock(&self.clock).tick
    }

    pub fn snapshot(&self) -> Snapshot {
        let c = lock(&self.clock);
        Snapshot {
            tick: c.tick,
            state_hash: c.sim.state_hash(),
            units: c.sim.alive_count(),
            winner: c.sim.winner().map_or(-1, |side| side as i32),
            resource_a: (0..c.sim.player_count()).map(|p| c.sim.resource_a(p as u8)).collect(),
            players: c.sim.player_count(),
            sides: c.sim.side_count(),
            records: live_records(&c.sim),
        }
    }

    pub fn with_catalog<R>(&self, read: impl FnOnce(&sim::Catalog) -> R) -> R {
        read(lock(&self.clock).sim.catalog())
    }

    pub fn delta_since(&self, since: u32) -> Result<Delta, DeltaError> {
        let c = lock(&self.clock);
        let tick = c.sim.ticks();
        if since > tick {
            return Err(DeltaError::Future { since, tick });
        }
        let mut units = vec![0u32; c.sim.changed_count_since(since)];
        let written = c.sim.changed_since(since, &mut units);
        units.truncate(written);
        let records = units.iter().take(RECORD_CAP).map(|&i| changed_record(&c.sim, i as usize)).collect::<Vec<String>>();
        Ok(Delta { tick, units, records })
    }

    pub fn unit_position(&self, unit: usize) -> Option<(i32, i32)> {
        let c = lock(&self.clock);
        c.sim.alive(unit).then(|| c.sim.position_fixed(unit))
    }

    pub fn unit_health(&self, unit: usize) -> Option<(i32, i32)> {
        let c = lock(&self.clock);
        c.sim.alive(unit).then(|| (c.sim.hp(unit), c.sim.hp_max(unit)))
    }

    pub fn player_has_units(&self, player: u8) -> bool {
        let c = lock(&self.clock);
        (0..c.sim.count()).any(|i| c.sim.alive(i) && c.sim.player(i) == player)
    }

    pub fn stats(&self) -> Stats {
        let c = lock(&self.clock);
        Stats {
            tick: c.tick,
            overruns: c.overruns,
            worst: c.worst,
            orders_drained: c.orders_drained,
        }
    }

    pub fn timing(&self) -> Timing {
        let c = lock(&self.clock);
        Timing {
            tick: c.tick,
            hist: c.hist.clone(),
            queued: self.queued.load(Ordering::Relaxed),
            overruns: c.overruns,
        }
    }

    fn step(&self, budget: Duration) {
        let started = Instant::now();
        let mut c = lock(&self.clock);
        if self.take_test_panic(c.tick) {
            panic!("test-only panic armed by --test-panic-tick fired at tick {}", c.tick);
        }
        if let Some(mut sim) = lock(&self.staged).take() {
            // Rebuild the match on the new data by replaying the recording onto the fresh sim, so gold, kills, items
            // and camp progress carry over. Falls back to carrying the live units when the replay cannot run.
            let inner = &mut *c;
            let rebuilt = rebuild_by_replay(&mut inner.rec, &inner.sim, &mut sim);
            let (carried, refused, kept) = match rebuilt {
                Ok(()) => (0, 0, c.handles.clone()),
                Err(_) => {
                    let (carried, refused) = carry_live_units(&c.sim, &mut sim);
                    let kept = keep_handles(&c.sim, &mut sim, &c.handles);
                    let at = c.sim.ticks();
                    c.rec.mark_incomplete(at);
                    (carried, refused, kept)
                }
            };
            c.sim = sim;
            c.handles = kept;
            crate::emit!(
                Ev::DataReloaded,
                room = self.name,
                tick = c.tick,
                units = c.sim.catalog().units.len(),
                carried = carried,
                refused = refused
            );
        }
        let shedding = self.shed && c.tick < c.shed_until;
        let first_shed_tick = shedding && !c.shed_on;
        if shedding != c.shed_on {
            c.shed_on = shedding;
            let state = if shedding { "on" } else { "off" };
            crate::emit!(Ev::TickShedEdge, room = self.name, tick = c.tick, state = state);
        }
        let run_paths = !shedding || c.tick % SHED_PATHS_EVERY == 0;
        let run_vision = !shedding || c.tick % SHED_VISION_EVERY == 0;
        let cap = if shedding { SHED_ORDER_CAP } else { NORMAL_ORDER_CAP };
        let mut drained = 0u64;
        while (drained as usize) < cap {
            let Ok(order) = c.orders.try_recv() else {
                break;
            };
            let clock = &mut *c;
            clock.rec.apply(&mut clock.sim, Command::OrderAll { player: order.player as u8, x: order.x, y: order.y });
            drained += 1;
        }
        self.queued.fetch_sub(drained, Ordering::Relaxed);
        c.orders_drained += drained;
        let feed = lock(&self.feed).clone();
        if let Some(link) = feed {
            let clock = &mut *c;
            let mut ui = Vec::new();
            let applied = apply_mod_commands(&mut clock.sim, &mut clock.rec, &mut clock.handles, link.take_commands(MOD_CMDS_PER_TICK), &mut ui);
            self.log_ui(clock.tick, ui);
            if applied > 0 {
                crate::emit!(Ev::ModCmds, room = self.name, tick = c.tick, applied = applied);
            }
        }
        let clock = &mut *c;
        clock.rec.tick(&mut clock.sim, run_paths, run_vision);
        let deaths = clock.sim.drain_payouts();
        if let Some(link) = lock(&self.feed).clone() {
            if link.gold_by_mod() {
                link.push_payouts(deaths);
            }
        }
        c.tick += 1;
        let cost = started.elapsed();
        c.worst = c.worst.max(cost);
        c.hist.record(cost);
        if shedding {
            let deferred = self.queued.load(Ordering::Relaxed);
            if deferred > 0 && (first_shed_tick || c.tick % SHED_LOG_EVERY_TICKS == 0) {
                crate::emit!(Ev::TickShed, room = self.name, tick = c.tick, shed = deferred, cap = SHED_ORDER_CAP);
            }
        }
        if cost > budget {
            c.streak += 1;
            c.overruns += 1;
            if c.streak >= SHED_AFTER_OVERRUNS {
                c.shed_until = c.tick + SHED_HOLD_TICKS;
            }
            crate::emit!(Ev::TickOverrun, room = self.name, tick = c.tick, cost_us = cost.as_micros(), budget_us = budget.as_micros());
        } else {
            c.streak = 0;
        }
    }
}

use crate::log::Ev;

// Applies the mod's queued commands to the room's sim at the tick boundary. A spawn binds its
// handle to the room unit it got; handle commands resolve through that map, and a handle the map
// does not hold is refused and logged. Plain unit indices come from the mod's own world, so a
// target the room does not hold alive is skipped.
pub fn apply_mod_commands(sim: &mut Sim, rec: &mut Recording, handles: &mut HashMap<u32, usize>, cmds: Vec<Cmd>, ui: &mut Vec<String>) -> usize {
    let applied = cmds.len();
    for cmd in cmds {
        match cmd {
            Cmd::Spawn { handle, index, x, y, owner, lane } => match rec.spawn(sim, handle, index as u32, x, y, owner, lane) {
                Some(unit) => {
                    handles.insert(handle, unit);
                    crate::emit!(Ev::ModUnitBound, handle = handle, unit = unit);
                }
                None => crate::emit!(Ev::ModUnitRefused, handle = handle, reason = "spawn_failed"),
            },
            Cmd::OrderHandle { handle, x, y } => match mapped(handles, sim, handle) {
                Some(unit) => {
                    rec.apply(sim, Command::Order { unit, x, y });
                }
                None => refuse_unmapped(handle),
            },
            Cmd::DamageHandle { caster, target, dmg } => {
                match (mapped(handles, sim, caster), mapped(handles, sim, target)) {
                    (Some(c), Some(t)) => {
                        rec.apply(sim, Command::Damage { caster: c, target: t, dmg });
                    }
                    (None, _) => refuse_unmapped(caster),
                    (_, None) => refuse_unmapped(target),
                }
            }
            Cmd::BuffHandle { handle, kind, ticks } => match mapped(handles, sim, handle) {
                Some(unit) => {
                    rec.apply(sim, Command::Buff { unit, kind, ticks });
                }
                None => refuse_unmapped(handle),
            },
            Cmd::Order { unit, x, y } => {
                if sim.alive(unit) {
                    rec.apply(sim, Command::Order { unit, x, y });
                }
            }
            Cmd::Damage { caster, target, dmg } => {
                if sim.alive(caster) && sim.alive(target) {
                    rec.apply(sim, Command::Damage { caster, target, dmg });
                }
            }
            Cmd::Buff { unit, kind, ticks } => {
                if sim.alive(unit) {
                    rec.apply(sim, Command::Buff { unit, kind, ticks });
                }
            }
            // The sim validates a cast or attack and refuses what it cannot do, so a refused one changes nothing.
            Cmd::Cast { unit, slot, target, x, y } => {
                if sim.alive(unit) {
                    rec.apply(sim, Command::Cast { unit, slot, target, x, y });
                }
            }
            Cmd::Attack { unit, target } => {
                if sim.alive(unit) && sim.alive(target) {
                    rec.apply(sim, Command::Attack { unit, target });
                }
            }
            Cmd::UseItem { unit, slot, target, x, y } => {
                if sim.alive(unit) {
                    rec.apply(sim, Command::UseItem { unit, slot, target, x, y });
                }
            }
            Cmd::Buy { unit, item } => {
                if sim.alive(unit) {
                    rec.apply(sim, Command::Buy { unit, item });
                }
            }
            Cmd::SkillUp { unit, slot } => {
                if sim.alive(unit) {
                    rec.apply(sim, Command::SkillUp { unit, slot });
                }
            }
            Cmd::Sell { unit, slot } => {
                if sim.alive(unit) {
                    rec.apply(sim, Command::Sell { unit, slot });
                }
            }
            Cmd::Unstash { unit, slot } => {
                if sim.alive(unit) {
                    rec.apply(sim, Command::Unstash { unit, slot });
                }
            }
            Cmd::SetOwner { unit, player } => {
                rec.apply(sim, Command::SetOwner { unit, player });
            }
            Cmd::SetTeam { player, team } => {
                rec.apply(sim, Command::SetTeam { player, team });
            }
            Cmd::SetAlliance { a, b, kind } => {
                rec.apply(sim, Command::SetAlliance { a, b, kind });
            }
            Cmd::AddPlayer { player, controller } => {
                rec.apply(sim, Command::AddPlayer { player, controller });
            }
            Cmd::RemovePlayer { player } => {
                rec.apply(sim, Command::RemovePlayer { player });
            }
            Cmd::SetFog { on } => {
                rec.apply(sim, Command::Fog { on });
            }
            Cmd::SetClassOverride { unit, class } => {
                rec.apply(sim, Command::ClassOverride { unit: unit as u32, class: class.unwrap_or(255) });
            }
            Cmd::DefineAbility { name, cooldown, energy, target, filter, effects } => {
                rec.apply(sim, Command::DefineAbility { name, cooldown, energy, target, filter, effects });
            }
            Cmd::DefineUnit { spec } => {
                rec.apply(sim, Command::DefineUnit { spec });
            }
            Cmd::DefineItem { spec } => {
                rec.apply(sim, Command::DefineItem { spec });
            }
            Cmd::DefineUpgrade { spec } => {
                rec.apply(sim, Command::DefineUpgrade { spec });
            }
            Cmd::GrantUpgrade { player, name } => {
                rec.apply(sim, Command::GrantUpgrade { player, name });
            }
            Cmd::GiveAbility { unit, slot, name } => {
                rec.apply(sim, Command::GiveAbility { unit, slot, name });
            }
            Cmd::AddGold { player, amount } => {
                rec.apply(sim, Command::AddGold { player, amount });
            }
            Cmd::AddHeroGold { unit, amount } => {
                rec.apply(sim, Command::AddHeroGold { unit, amount });
            }
            Cmd::Victory { player } => {
                rec.apply(sim, Command::Victory { player });
            }
            Cmd::Defeat { player } => {
                rec.apply(sim, Command::Defeat { player });
            }
            Cmd::Ui { body } => {
                rec.apply(sim, Command::Ui { body: body.clone() });
                ui.push(body);
            }
        }
    }
    applied
}

// Carries the live units of the sim in service into a freshly loaded image without new ids. A
// live unit keeps its index and moves to its old point when the new map places a unit of the same
// player and the same kind name there; any other unit the new map placed is removed, so no unit is
// duplicated. A carried unit keeps its old health and maximum. Returns (carried, refused): refused
// counts the old live units that found no such slot.
fn carry_live_units(old: &Sim, new: &mut Sim) -> (usize, usize) {
    let mut carried = 0;
    for i in 0..new.count() {
        if !new.alive(i) {
            continue;
        }
        let same = i < old.count()
            && old.alive(i)
            && old.player(i) == new.player(i)
            && old.kind_name(i).is_some()
            && old.kind_name(i) == new.kind_name(i);
        if same {
            let (x, y) = old.position_fixed(i);
            new.set_position(i, x, y);
            new.set_health(i, old.hp(i), old.hp_max(i));
            carried += 1;
        } else {
            new.remove_unit(i);
        }
    }
    (carried, old.alive_count().saturating_sub(carried))
}

// Keeps each handle whose unit was live before the swap and whose kind name is unchanged. A unit
// the new map leaves out is respawned at its old index, kind, owner and point, so the handle still
// names the unit it was bound to. Any other handle is dropped and logged as refused with its reason.
fn keep_handles(old: &Sim, new: &mut Sim, handles: &HashMap<u32, usize>) -> HashMap<u32, usize> {
    let mut kept = HashMap::new();
    for (&handle, &unit) in handles {
        if !old.alive(unit) {
            continue;
        }
        let refuse = |reason: &str| crate::emit!(Ev::ModUnitRefused, handle = handle, unit = unit, reason = reason);
        let Some(name) = old.kind_name(unit) else {
            refuse("kind_absent");
            continue;
        };
        if new.kind_name(unit).is_some_and(|now| now != name) {
            refuse("kind_changed");
            continue;
        }
        if new.alive(unit) || respawn(old, new, unit) {
            kept.insert(handle, unit);
        } else {
            refuse(if new.catalog().unit_index(name).is_some() { "respawn_failed" } else { "kind_absent" });
        }
    }
    kept
}

fn respawn(old: &Sim, new: &mut Sim, unit: usize) -> bool {
    let Some(name) = old.kind_name(unit) else {
        return false;
    };
    let Some(kind) = new.catalog().unit_index(name) else {
        return false;
    };
    let (x, y) = old.position_fixed(unit);
    new.spawn_at(unit, kind as u32, old.player(unit), x, y)
}

fn mapped(handles: &HashMap<u32, usize>, sim: &Sim, handle: u32) -> Option<usize> {
    handles.get(&handle).copied().filter(|&unit| sim.alive(unit))
}

fn refuse_unmapped(handle: u32) {
    crate::emit!(Ev::ModUnitRefused, handle = handle, reason = "unmapped");
}

pub fn spawn_ticker(room: Arc<Room>, budget: Duration) -> JoinHandle<()> {
    thread::spawn(move || {
        supervise("ticker", Ev::TickerDied, Ev::TickerRestarted, || {
            run_ticker(&room, budget);
            Outcome::Retry
        })
    })
}

fn run_ticker(room: &Room, budget: Duration) {
    let mut next = Instant::now() + budget;
    loop {
        let now = Instant::now();
        if next > now {
            thread::sleep(next - now);
        }
        room.step(budget);
        while next <= Instant::now() {
            next += budget;
        }
    }
}

#[cfg(test)]
mod carry_tests {
    use super::*;

    #[test]
    fn carry_keeps_old_health() {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut old = mods::load_data(&data, 4096).expect("old image");
        let i = (0..4096).find(|&i| old.alive(i)).expect("a live unit");
        let max = old.hp_max(i);
        old.set_health(i, 7, max);
        let mut new = mods::load_data(&data, 4096).expect("new image");
        let (carried, _) = carry_live_units(&old, &mut new);
        assert!(carried > 0);
        assert_eq!(new.hp(i), 7, "a carried unit keeps its pre-swap health");
    }
}

// Rebuilds a running match on `fresh` (the new data) by replaying the room's recording onto it up to the live tick.
// The recording is kept as it was; a failed replay leaves `fresh` unused and the caller carries the live units instead.
fn rebuild_by_replay(rec: &mut crate::replay::Recording, live: &Sim, fresh: &mut Sim) -> Result<(), String> {
    let text = rec.render(live);
    let parsed = crate::replay::Recording::parse(&text).map_err(|e| format!("{e:?}"))?;
    let mut player = crate::replay::Replayer::new(parsed, fresh).map_err(|e| format!("{e:?}"))?;
    while fresh.ticks() < live.ticks() {
        player.step(fresh).map_err(|e| format!("{e:?}"))?;
    }
    Ok(())
}
