use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use sim::{Grid, Sim};

use crate::order::{Order, OrderGate};

pub const TICKS_PER_SECOND: u32 = 20;

pub struct RoomConfig {
    pub name: String,
    pub grid_w: i32,
    pub grid_h: i32,
    pub max_units: usize,
    pub token: String,
    pub rate: u32,
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
    pub gate: OrderGate,
    queue: Mutex<Sender<Order>>,
    clock: Mutex<Clock>,
}

pub struct Snapshot {
    pub tick: u64,
    pub state_hash: u64,
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
    pub fn new(cfg: RoomConfig) -> Arc<Room> {
        let (tx, rx) = mpsc::channel();
        let mut sim = Sim::new(cfg.max_units);
        sim.set_grid(Grid::open(cfg.grid_w as usize, cfg.grid_h as usize));
        Arc::new(Room {
            name: cfg.name,
            grid_w: cfg.grid_w,
            grid_h: cfg.grid_h,
            max_units: cfg.max_units,
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
        })
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
            state_hash: c.sim.state_hash() as u64,
        }
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
