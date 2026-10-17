//! Browser entry point. `Game` loads a map image (the bytes sim::load reads) and steps the sim;
//! the page draws the units it reports.

use wasm_bindgen::prelude::*;

/// The unit cap the gate uses, so the browser and the parity checks load the same limits.
const MAX_UNITS: usize = 1024;

#[wasm_bindgen]
pub struct Game {
    sim: sim::Sim,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(image: &[u8]) -> Result<Game, JsValue> {
        let sim = sim::load(image, MAX_UNITS).map_err(|e| JsValue::from_str(&format!("sim::load: {e:?}")))?;
        let sim = sim;
        Ok(Game { sim })
    }

    pub fn tick(&mut self) {
        self.sim.tick();
    }

    pub fn ticks(&self) -> u32 {
        self.sim.ticks()
    }

    pub fn state_hash(&self) -> u64 {
        self.sim.state_hash()
    }

    /// Sim ids of the living units, in the same order as `units()`.
    pub fn unit_ids(&self) -> Vec<u32> {
        (0..self.sim.count())
            .filter(|&i| self.sim.alive(i))
            .map(|i| i as u32)
            .collect()
    }

    /// Orders unit `unit` to walk to tile (x, y). Returns false when the sim refuses the order.
    pub fn order_move(&mut self, unit: u32, x: f32, y: f32) -> bool {
        // sim's ORDER_MOVE (movement order kind 0); the sim keeps that constant crate-private.
        const ORDER_MOVE: u8 = 0;
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        self.sim.issue_order_kind(unit as usize, ORDER_MOVE, fx, fy).is_ok()
    }

    /// Hit points of unit `unit`, in whole points.
    pub fn unit_hp(&self, unit: u32) -> i32 {
        sim::fixed::to_int(self.sim.hp(unit as usize))
    }

    /// Gold (resource a) of player `player`, in whole units.
    pub fn gold(&self, player: u8) -> i32 {
        self.sim.resource_a(player)
    }

    /// The winning side, or -1 while the match runs.
    pub fn winner(&self) -> i32 {
        self.sim.winner().map_or(-1, |side| side as i32)
    }

    /// Casts ability slot `slot` of unit `unit` at tile (x, y), aimed at unit `target` or at the ground when
    /// `target` is -1. Returns false when the sim refuses the cast.
    pub fn cast(&mut self, unit: u32, slot: u32, target: i32, x: f32, y: f32) -> bool {
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        self.sim.cast_ability(unit as usize, slot as usize, target, fx, fy)
    }

    /// Orders unit `unit` to attack unit `target`, walking toward it when it is out of reach.
    /// Returns false when the sim refuses the order.
    pub fn attack(&mut self, unit: u32, target: u32) -> bool {
        self.sim.order_attack_or_approach(unit as usize, target as usize).is_ok()
    }

    /// Test hook for the page's CI probe: deals `damage` whole points to unit `unit`.
    pub fn hurt(&mut self, unit: u32, damage: i32) {
        self.sim.hurt(unit as usize, sim::fixed::from_int(damage));
    }

    /// Turns the fog of war on or off. The moba preview turns it off so every enemy is a valid target.
    pub fn set_fog(&mut self, on: bool) {
        self.sim.set_fog(on);
    }

    /// Living units as flat `[x, y, player]` triples, positions in tiles.
    pub fn units(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for i in 0..self.sim.count() {
            if !self.sim.alive(i) {
                continue;
            }
            let (x, y) = self.sim.position_fixed(i);
            out.push(sim::fixed::to_float(x) as f32);
            out.push(sim::fixed::to_float(y) as f32);
            out.push(self.sim.player(i) as f32);
        }
        out
    }
}
