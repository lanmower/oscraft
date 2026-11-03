#![allow(dead_code)]

use std::path::Path;

use sim::fixed::{self, HALF};
use sim::{Grid, Sim};

pub const PLAYER: u8 = 0;
pub const FOE: u8 = 1;
pub const UNIT: &str = "sentinel_creep";
pub const FIELD_W: usize = 30;
pub const FIELD_H: usize = 30;
pub const WALL_X: i32 = 12;
pub const GAP_FROM_Y: i32 = 21;
pub const REACH_MILLI: f64 = 500.0;
pub const MAX_TICKS: u32 = 3000;

pub fn open_field() -> Sim {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
    let image = osdt_data::generate(&dir).expect("compile the moba scenario");
    let catalog = sim::load(&image, 4096)
        .expect("load the moba scenario")
        .catalog()
        .clone();
    let mut sim = Sim::with_catalog(64, Grid::open(FIELD_W, FIELD_H), catalog);
    sim.set_fog(false);
    assert!(sim.set_team(PLAYER, 0).is_ok());
    assert!(sim.set_team(FOE, 1).is_ok());
    sim
}

pub fn centre(cell: i32) -> i32 {
    fixed::from_int(cell) + HALF
}

pub fn cell(v: i32) -> i32 {
    fixed::to_int(v)
}

pub fn spawn(sim: &mut Sim, player: u8, cx: i32, cy: i32) -> usize {
    spawn_kind(sim, player, UNIT, cx, cy)
}

pub fn spawn_kind(sim: &mut Sim, player: u8, kind: &str, cx: i32, cy: i32) -> usize {
    let index = sim
        .catalog()
        .unit_index(kind)
        .expect("unit kind in the moba catalog");
    sim.spawn_for(player, index as u32, centre(cx), centre(cy))
        .expect("spawn fits the open field")
}

pub fn wall(sim: &mut Sim) {
    for y in 0..GAP_FROM_Y {
        assert!(sim.set_cell_blocked(WALL_X, y, true).is_ok());
    }
}

pub fn distance_milli(sim: &Sim, u: usize, x: i32, y: i32) -> f64 {
    let (ux, uy) = sim.position_fixed(u);
    let dx = fixed::to_float(ux - x);
    let dy = fixed::to_float(uy - y);
    (dx * dx + dy * dy).sqrt() * 1000.0
}

pub fn assert_never_on_blocked_cell(sim: &Sim, u: usize, tick: u32) {
    let (x, y) = sim.position_fixed(u);
    assert!(
        sim.grid().walkable(cell(x), cell(y)),
        "unit {u} stood on a blocked cell ({}, {}) at tick {tick}",
        cell(x),
        cell(y)
    );
}

pub fn in_reach_foe(sim: &Sim, u: usize) -> i32 {
    let ordered = sim.attack_order_of(u);
    if ordered >= 0 && sim.can_strike(u, ordered as usize) {
        return ordered;
    }
    (0..sim.count())
        .find(|&j| sim.can_strike(u, j))
        .map_or(-1, |j| j as i32)
}

pub fn drive_targeting(sim: &mut Sim, u: usize) {
    if !sim.alive(u) {
        return;
    }
    let strike = in_reach_foe(sim, u);
    assert!(sim.set_strike_target(u, strike).is_ok());
    if sim.order_of(u).0 == sim::ORDER_ATTACK_MOVE {
        assert!(sim.set_hold_override(u, Some(strike >= 0)).is_ok());
    }
}
