use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use sim::{Grid, Sim};

use crate::order::{Order, OrderGate};

// Matches sim/src/lib.rs `TICKS_PER_SECOND` (16). The sim keeps it pub(crate), so it is not re-exported and must be kept in step by hand.
pub const TICKS_PER_SECOND: u32 = 16;

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
    tick: u64,
    overruns: u64,
    worst: Duration,
    orders_drained: u64,
    orders: Receiver<Order>,
}

pub struct Room {
    pub name: String,
    pub grid_w: i32,
    pub grid_h: i32,
    pub max_units: usize,
    pub loaded: bool,
    pub gate: OrderGate,
    queue: Mutex<Sender<Order>>,
    clock: Mutex<Clock>,
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
        let (sim, grid_w, grid_h, loaded) = match &cfg.data {
            Some(path) => {
                let sim = sim::load_file(path, cfg.max_units)
                    .map_err(|e| format!("--data {}: {e:?}", path.display()))?;
                let (w, h) = (sim.grid().w as i32, sim.grid().h as i32);
                (sim, w, h, true)
            }
            None => {
                let mut sim = Sim::new(cfg.max_units);
                sim.set_grid(Grid::open(cfg.grid_w as usize, cfg.grid_h as usize));
                (sim, cfg.grid_w, cfg.grid_h, false)
            }
        };
        Ok(Arc::new(Room {
            name: cfg.name,
            grid_w,
            grid_h,
            max_units: cfg.max_units,
            loaded,
            gate: OrderGate::new(cfg.token, cfg.rate),
            queue: Mutex::new(tx),
            clock: Mutex::new(Clock {
                sim,
                tick: 0,
                overruns: 0,
                worst: Duration::ZERO,
                orders_drained: 0,
                orders: rx,
            }),
        }))
    }

    pub fn submit(&self, order: Order) -> Result<(), String> {
        lock(&self.queue)
            .send(order)
            .map_err(|_| "room is closed".to_string())
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

    fn step(&self, budget: Duration) {
        let started = Instant::now();
        let mut c = lock(&self.clock);
        let mut drained = 0u64;
        while let Ok(order) = c.orders.try_recv() {
            c.sim.order_all(order.player as u8, order.x, order.y);
            drained += 1;
        }
        c.orders_drained += drained;
        c.sim.tick();
        c.tick += 1;
        let cost = started.elapsed();
        c.worst = c.worst.max(cost);
        if cost > budget {
            c.overruns += 1;
        }
    }
}

pub fn spawn_ticker(room: Arc<Room>) -> JoinHandle<()> {
    let budget = Duration::from_secs(1) / TICKS_PER_SECOND;
    thread::spawn(move || {
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
    })
}
