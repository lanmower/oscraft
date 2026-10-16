use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mod_host::Cmd;
use sim::{Grid, Sim};

use crate::budget::{
    TickHistogram, Timing, NORMAL_ORDER_CAP, SHED_AFTER_OVERRUNS, SHED_LOG_EVERY_TICKS, SHED_ORDER_CAP,
};
use crate::mods::{self, ModLink};
use crate::order::{Order, OrderGate};
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
}

struct Clock {
    sim: Sim,
    handles: HashMap<u32, usize>,
    tick: u64,
    overruns: u64,
    streak: u64,
    worst: Duration,
    orders_drained: u64,
    orders: Receiver<Order>,
    hist: TickHistogram,
}

pub struct Room {
    pub name: String,
    pub grid_w: i32,
    pub grid_h: i32,
    pub max_units: usize,
    pub loaded: bool,
    pub gate: OrderGate,
    queue: Mutex<Sender<Order>>,
    queued: AtomicU64,
    feed: Mutex<Option<Arc<ModLink>>>,
    staged: Mutex<Option<Sim>>,
    clock: Mutex<Clock>,
    test_panic_at: AtomicU64,
}

pub struct Snapshot {
    pub tick: u64,
    pub state_hash: u64,
    pub units: usize,
}

pub struct Delta {
    pub tick: u32,
    pub units: Vec<u32>,
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
        let room = Arc::new(Room {
            name: cfg.name,
            grid_w,
            grid_h,
            max_units: cfg.max_units,
            loaded,
            gate: OrderGate::new(cfg.token, cfg.rate),
            queue: Mutex::new(tx),
            queued: AtomicU64::new(0),
            feed: Mutex::new(None),
            test_panic_at: AtomicU64::new(0),
            staged: Mutex::new(None),
            clock: Mutex::new(Clock {
                sim,
                handles: HashMap::new(),
                tick: 0,
                overruns: 0,
                streak: 0,
                worst: Duration::ZERO,
                orders_drained: 0,
                orders: rx,
                hist: TickHistogram::new(),
            }),
        });
        if let Some(dir) = watch {
            mods::watch_data(Arc::downgrade(&room), dir, cfg.max_units);
        }
        Ok(room)
    }

    pub fn attach_mod_feed(&self, link: Arc<ModLink>) {
        *lock(&self.feed) = Some(link);
    }

    // A rebuilt image waits here until the next tick boundary swaps it in.
    pub fn stage_data(&self, sim: Sim) -> Result<(), String> {
        let (w, h) = (sim.grid().w as i32, sim.grid().h as i32);
        if (w, h) != (self.grid_w, self.grid_h) {
            return Err(format!(
                "map is {}x{} in service and {w}x{h} in the new data; a map size change needs a restart",
                self.grid_w, self.grid_h
            ));
        }
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
        Ok(Delta { tick, units })
    }

    pub fn unit_position(&self, unit: usize) -> Option<(i32, i32)> {
        let c = lock(&self.clock);
        c.sim.alive(unit).then(|| c.sim.position_fixed(unit))
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
            let (carried, refused) = carry_live_units(&c.sim, &mut sim);
            c.sim = sim;
            let clock = &mut *c;
            clock.handles.retain(|_, unit| clock.sim.alive(*unit));
            crate::emit!(
                Ev::DataReloaded,
                room = self.name,
                tick = c.tick,
                units = c.sim.catalog().units.len(),
                carried = carried,
                refused = refused
            );
        }
        let shedding = c.streak >= SHED_AFTER_OVERRUNS;
        let first_shed_tick = c.streak == SHED_AFTER_OVERRUNS;
        let cap = if shedding { SHED_ORDER_CAP } else { NORMAL_ORDER_CAP };
        let mut drained = 0u64;
        while (drained as usize) < cap {
            let Ok(order) = c.orders.try_recv() else {
                break;
            };
            c.sim.order_all(order.player as u8, order.x, order.y);
            drained += 1;
        }
        self.queued.fetch_sub(drained, Ordering::Relaxed);
        c.orders_drained += drained;
        let feed = lock(&self.feed).clone();
        if let Some(link) = feed {
            let clock = &mut *c;
            let applied = apply_mod_commands(&mut clock.sim, &mut clock.handles, link.take_commands(MOD_CMDS_PER_TICK));
            if applied > 0 {
                crate::emit!(Ev::ModCmds, room = self.name, tick = c.tick, applied = applied);
            }
        }
        c.sim.tick();
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
fn apply_mod_commands(sim: &mut Sim, handles: &mut HashMap<u32, usize>, cmds: Vec<Cmd>) -> usize {
    let applied = cmds.len();
    for cmd in cmds {
        match cmd {
            Cmd::Spawn { handle, index, x, y } => match sim.spawn(index as u32, x, y) {
                Some(unit) => {
                    handles.insert(handle, unit);
                    crate::emit!(Ev::ModUnitBound, handle = handle, unit = unit);
                }
                None => crate::emit!(Ev::ModUnitRefused, handle = handle, reason = "spawn_failed"),
            },
            Cmd::OrderHandle { handle, x, y } => match mapped(handles, sim, handle) {
                Some(unit) => sim.order(unit, x, y),
                None => refuse_unmapped(handle),
            },
            Cmd::DamageHandle { caster, target, dmg } => {
                match (mapped(handles, sim, caster), mapped(handles, sim, target)) {
                    (Some(c), Some(t)) => {
                        let player = sim.player(c);
                        sim.damage_target(c as i32, player, dmg, t as i32);
                    }
                    (None, _) => refuse_unmapped(caster),
                    (_, None) => refuse_unmapped(target),
                }
            }
            Cmd::BuffHandle { handle, kind, ticks } => match mapped(handles, sim, handle) {
                Some(unit) => {
                    let _ = sim.apply_buff(unit, kind, ticks);
                }
                None => refuse_unmapped(handle),
            },
            Cmd::Order { unit, x, y } => {
                if sim.alive(unit) {
                    sim.order(unit, x, y);
                }
            }
            Cmd::Damage { caster, target, dmg } => {
                if sim.alive(caster) && sim.alive(target) {
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
    applied
}

// Carries the live units of the sim in service into a freshly loaded image without new ids. A
// live unit keeps its index and moves to its old point when the new map places a unit of the same
// player and the same kind name there; any other unit the new map placed is removed, so no unit is
// duplicated. Returns (carried, refused): refused counts the old live units that found no such slot.
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
            carried += 1;
        } else {
            new.remove_unit(i);
        }
    }
    (carried, old.alive_count().saturating_sub(carried))
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
